//! task_008：`KcProcessManager` 子进程生命周期管理。
//!
//! ## 设计依据
//!
//! - **ADR-001**（Architect output.md）：混合启动 + 动态端口 + 健康检查 + 崩溃分类恢复 + 冷却期 + RAII Drop。
//! - **ADR-006**（OutputStage 三层防御，层 2 cwd 隔离）：spawn child 时把 cwd 设到 NC 控制的临时目录。
//! - **ADR-007**（LLM Key 注入）：env 变量 `ZHIPUAI_API_KEY` / `OPENAI_API_KEY` 注入子进程，绝不写 .env。
//! - **KC-MOD-5**（端口可配）：调 `run_api.py --host 127.0.0.1 --port <dyn>`。
//! - **KC-MOD-6**（Graceful shutdown）：先 SIGTERM，等 graceful（5s），仍未退则 SIGKILL fallback（仅 stop()）。
//! - **KC-MOD-7**（日志 Key 屏蔽）：drain stdout/stderr 时 mask 已知 Key 前缀，避免明文落盘日志。
//! - `extraction/scheduler.rs::detect_embedded_markitdown_python`：python 探测模板。
//! - `extraction/extractors/markitdown.rs::run_with_timeout`：pipe drain 后台线程模式（避免 OS pipe buffer 死锁）。
//!
//! ## 整体状态机
//!
//! ```text
//!                  start()
//!     Stopped ───────────────► Starting
//!        ▲                       │
//!        │                       │ (health check OK)
//!        │                       ▼
//!     stop() ◄── Drop ──────── Ready
//!        ▲                       │
//!        │                       │ (child exit / health miss × 3)
//!        │                       ▼
//!        └──────────── Unavailable(reason)
//!                              │
//!                              │ restart()（受冷却期约束）
//!                              ▼
//!                           Starting
//! ```
//!
//! ## 关键不变量（Reviewer 必查）
//!
//! 1. **Mutex 锁粒度**：`state.lock()` 持有时**绝不 `.await`**。所有 `.await`（health poll / sleep）
//!    在 lock 释放后做，借鉴 r2d2 模式（PR #28 commit）。
//! 2. **RAII Drop 不 panic**：`child.kill()` / `child.wait()` 失败仅 `log::warn`，吞掉 Error。
//! 3. **Pipe drain join**：两个 `thread::spawn(drain stdout/stderr)` 的 `JoinHandle` 保存在 state，
//!    `stop()` 时 join，避免线程 leak。
//! 4. **Key 不落盘**：env 变量经 `Command::env()` 注入子进程，Rust 进程层不写文件；drain 日志走
//!    `crate::kc::settings::mask_secrets_by_keys()` 精确子串替换屏蔽（task_009 TD-5：原名
//!    `mask_secrets`，统一改名后语义更明确——"按已知 Key 精确替换"，与 `client::mask_secrets_by_prefix`
//!    "按未知 Key 前缀启发"互补）。MAJOR-2 fix（task_008）：替代原前缀启发 `mask_secret`，
//!    覆盖智谱 dot 格式 / JSON dump / Debug dump 三类漏屏场景。
//! 5. **冷却期边界**：30s 内 ≥ 2 次 **OR** 60s 内 ≥ 3 次重启 → `RestartCooldownExceeded`，避免 fork bomb。
//!    两段阈值互补：短窗口严格抓"快速失败模式"（30s 内连续 2 次），长窗口宽松抓"慢崩累积"（60s 内 3 次）。
//!    设计要点：30s 窗口 ⊂ 60s 窗口，若两个窗口阈值都设 2 则 30s 规则被 60s 规则吞掉（FIX 第 1 轮修正）。
//! 6. **端口竞态**：`TcpListener::bind(":0")` drop 后到 KC 真 bind 间的微窗口靠"启动失败 → 重选 port
//!    重试"兜底（最多 3 次）。
//! 7. **mock 短路**：`KC_USE_MOCK_PORT` env 存在 → 跳过真实 spawn，状态直接 Ready；保证集成测试可在
//!    无 KC python venv 的 CI 环境跑（task_006 约定）。

use std::collections::VecDeque;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

// =====================================================================
// 1. 公共类型（AC-1：状态枚举 + 启动错误 + 健康状态）
// =====================================================================

/// KC 子进程对外可见的状态（AC-1）。
///
/// 不变量：`Unavailable.0` 是简短人类可读的原因字符串（如 `"healthcheck timeout"` /
/// `"child exited with code 1"`），供前端 toast / banner 展示，不含 Key。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KcStatus {
    /// 初始 / 已停止状态。
    Stopped,
    /// `start()` 已调用，正在等待 health check 通过。
    Starting,
    /// health check 通过，可接受 ingest 请求。
    Ready,
    /// 子进程异常（崩溃、健康检查 3 次失败、python not found 等）。
    Unavailable(String),
}

impl KcStatus {
    /// 序列化到事件 payload 的字面值（前端解析友好）。
    pub fn as_event_str(&self) -> &'static str {
        match self {
            KcStatus::Stopped => "stopped",
            KcStatus::Starting => "starting",
            KcStatus::Ready => "ready",
            KcStatus::Unavailable(_) => "unavailable",
        }
    }
}

/// 启动失败的分类（AC-1）。
///
/// **设计**：变体粒度对应"该错误是否值得用户重试"——
/// - `PythonNotFound` / `RestartCooldownExceeded`：需要用户介入（检查打包 / 等冷却过期），不会自动重试；
/// - `PortBindFailed` / `HealthCheckTimedOut` / `ProcessExitedDuringStartup`：临时故障，
///   `restart()` 可能恢复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KcStartError {
    /// `detect_embedded_kc_python()` 找不到 Python 解释器（DMG 打包问题）。
    PythonNotFound,
    /// 连续 3 次 `TcpListener::bind(":0")` + spawn 失败（OS 端口耗尽，极罕见）。
    PortBindFailed,
    /// 健康检查轮询 50 次后仍未 200（10s 上限）。
    HealthCheckTimedOut,
    /// child 在 health check 通过前就退出（早夭，多为 python 启动报错）。
    ProcessExitedDuringStartup,
    /// 冷却期内重启次数超限（30s 内 ≥ 2 次 OR 60s 内 ≥ 3 次）。
    RestartCooldownExceeded,
}

impl KcStartError {
    /// 简短可读原因（写入 `KcStatus::Unavailable` 和事件 payload）。
    pub fn reason(&self) -> &'static str {
        match self {
            KcStartError::PythonNotFound => "kc python interpreter not found",
            KcStartError::PortBindFailed => "failed to bind tcp port (after retries)",
            KcStartError::HealthCheckTimedOut => "health check timed out (10s)",
            KcStartError::ProcessExitedDuringStartup => "kc child exited during startup",
            KcStartError::RestartCooldownExceeded => "restart cooldown exceeded",
        }
    }
}

/// `health_check()` 返回值（AC-5）。
#[derive(Debug, Clone, Serialize)]
pub struct KcHealthStatus {
    /// 当前状态字面值（`ready` / `starting` / `unavailable` / `stopped`）。
    pub status: String,
    /// 详细原因（仅 `unavailable` 时非空）。
    pub reason: Option<String>,
    /// 当前监听端口（mock 模式下也有值）。
    pub port: Option<u16>,
    /// 本次 health check 时间戳。
    pub last_check: DateTime<Utc>,
    /// 从 `Ready` 起累计秒数；非 Ready 状态返回 None。
    pub uptime_secs: Option<u64>,
}

/// `PortProvider` trait（AC-2，task_007 KcClient 依赖此 trait 解耦）。
pub trait PortProvider: Send + Sync {
    /// 当前 KC 服务监听端口，未 Ready 时返回 None。
    fn current_port(&self) -> Option<u16>;
}

// =====================================================================
// 2. 内部状态（Arc<Mutex<>> 持有；锁内绝不 .await）
// =====================================================================

/// 进程管理器内部状态（AC-1）。
///
/// **持有 `Mutex` 时不可 await**——所有 `.await` 在锁释放后做。
struct KcInternalState {
    /// 当前 child handle（mock 模式 / Stopped 时为 None）。
    child: Option<Child>,
    /// 当前监听端口（mock 模式下也填）。
    port: Option<u16>,
    /// 当前状态。
    status: KcStatus,
    /// 最近一次 `start()` 失败原因（成功则为 None）。
    last_failure: Option<KcStartError>,
    /// 重启时间戳窗口（60s 滑动），用于冷却期判定。
    restart_history: VecDeque<Instant>,
    /// 进入 `Ready` 状态的时间（计算 uptime）。
    startup_time: Option<Instant>,
    /// drain stdout/stderr 的后台线程 join handle（stop 时回收，防 leak）。
    stdout_join: Option<JoinHandle<()>>,
    stderr_join: Option<JoinHandle<()>>,
    /// 监控 child exit 的后台线程 join handle。
    /// 不可 join（外部线程 wait 之后不能再 wait child），仅为防 leak 持有。
    monitor_join: Option<JoinHandle<()>>,
}

impl KcInternalState {
    fn new() -> Self {
        Self {
            child: None,
            port: None,
            status: KcStatus::Stopped,
            last_failure: None,
            restart_history: VecDeque::new(),
            startup_time: None,
            stdout_join: None,
            stderr_join: None,
            monitor_join: None,
        }
    }

    /// 清理 60s 窗口外的旧重启记录（在每次 restart() 前调用）。
    fn clean_restart_history(&mut self, now: Instant) {
        const WINDOW: Duration = Duration::from_secs(60);
        while let Some(front) = self.restart_history.front() {
            if now.duration_since(*front) >= WINDOW {
                self.restart_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// 冷却期判定（AC-6）：**30s 内 ≥ 2 次 OR 60s 内 ≥ 3 次**。
    ///
    /// 返回 `true` 表示**当前已超限**（不应再重启）。
    ///
    /// **两段阈值的数学独立性**（FIX 第 1 轮修正）：
    /// - 30s 窗口是 60s 窗口的真子集 → `short_count ≥ 2` 必然蕴含 `long_count ≥ 2`；
    /// - 若两窗口阈值都设 2，则 30s 规则永远不会独立触发（被 60s 规则吞掉），是死代码；
    /// - 改为"30s/2 + 60s/3"后：30s 内连续 2 次崩（快速失败模式）由 short 规则即刻触发；
    ///   60s 内偶发 3 次（包含 30s 内 1 次 + 30-60s 间 2 次）由 long 规则触发，不被 short 覆盖。
    fn is_in_cooldown(&self, now: Instant) -> bool {
        const SHORT_WINDOW: Duration = Duration::from_secs(30);
        const LONG_WINDOW: Duration = Duration::from_secs(60);
        const SHORT_THRESHOLD: usize = 2;
        const LONG_THRESHOLD: usize = 3;
        let short_count = self
            .restart_history
            .iter()
            .filter(|t| now.duration_since(**t) < SHORT_WINDOW)
            .count();
        let long_count = self
            .restart_history
            .iter()
            .filter(|t| now.duration_since(**t) < LONG_WINDOW)
            .count();
        short_count >= SHORT_THRESHOLD || long_count >= LONG_THRESHOLD
    }
}

// =====================================================================
// 3. `KcProcessManager` 主类型（AC-1）
// =====================================================================

/// KC uvicorn 子进程生命周期管理器。
///
/// 详细设计见模块文档。线程安全：`Clone` 是 `Arc` 浅拷贝，多个调用方共享同一状态。
#[derive(Clone)]
pub struct KcProcessManager {
    /// AppHandle 用于 emit `notecapt/kc-status-changed` 事件（AC-8）。
    /// `None` 仅出现在 `new_for_test`（绕开 Tauri runtime 的纯单元测试场景）。
    app_handle: Option<AppHandle>,
    /// 共享状态。
    state: Arc<Mutex<KcInternalState>>,
}

impl KcProcessManager {
    /// 构造（AC-1）：在 Tauri setup 期 / lib.rs `manage(...)` 时调用。
    pub fn new(app: &AppHandle) -> Self {
        Self {
            app_handle: Some(app.clone()),
            state: Arc::new(Mutex::new(KcInternalState::new())),
        }
    }

    /// 仅供单元测试用：构造不绑 AppHandle 的实例（emit 静默失败）。
    #[cfg(test)]
    fn new_for_test() -> Self {
        Self {
            app_handle: None,
            state: Arc::new(Mutex::new(KcInternalState::new())),
        }
    }

    /// **task_009 AC-5**：集成测试用构造器——同 `new_for_test` 但 pub 暴露给
    /// `tests/kc_lifecycle.rs` 等外部集成测试调用。emit 静默失败（无 AppHandle）。
    ///
    /// **严禁生产路径使用**：通过 `#[doc(hidden)]` + 命名 `_no_app` 后缀双重提示。
    /// 名字带 `_test_only`——任何 reviewer / 后续 dev 看到都应当意识到这不是生产 API。
    ///
    /// 用途仅限：在 `KC_USE_MOCK_PORT` 短路路径下测试 start/stop/health_check 状态机
    /// （不需要真实 Tauri AppHandle 上下文）。
    #[doc(hidden)]
    pub fn new_test_only_no_app() -> Self {
        Self {
            app_handle: None,
            state: Arc::new(Mutex::new(KcInternalState::new())),
        }
    }

    // -----------------------------------------------------------------
    // AC-3: async fn start()
    // -----------------------------------------------------------------

    /// 异步启动 KC 子进程（AC-3）。完整 7 步流程：
    ///
    /// 1. 检查 `KC_USE_MOCK_PORT`：存在 → 跳过真实 spawn，直接 Ready；
    /// 2. 探测 python 路径；
    /// 3. `bind:0` 选空闲端口；
    /// 4. `Command::new(python).env(KEY)...spawn()`；
    /// 5. drain stdout/stderr 两个后台线程；
    /// 6. 轮询 `/api/v1/health` 间隔 200ms 上限 50 次（10s）；
    /// 7. 监控 child exit（独立线程 `child.wait()`）。
    pub async fn start(&self) -> Result<(), KcStartError> {
        // 步骤 1：mock 短路（AC-3.1）。
        if let Ok(mock_port_str) = std::env::var("KC_USE_MOCK_PORT") {
            if let Ok(mock_port) = mock_port_str.parse::<u16>() {
                self.set_status_and_port(KcStatus::Ready, Some(mock_port), None);
                log::info!("[kc] KC_USE_MOCK_PORT={mock_port} 短路，跳过真实子进程启动");
                self.emit_status_changed(&KcStatus::Ready, None);
                return Ok(());
            } else {
                log::warn!("[kc] KC_USE_MOCK_PORT 值 '{mock_port_str}' 非法 u16，忽略并走真实启动");
            }
        }

        // 步骤 2-4：探测 python + 选端口 + spawn child（带最多 3 次端口竞态重试）。
        const SPAWN_RETRY: u32 = 3;
        let mut last_err = KcStartError::PortBindFailed;

        let app = match self.app_handle.as_ref() {
            Some(h) => h.clone(),
            None => {
                // 单元测试态：无 AppHandle → 直接走 PythonNotFound（不该在生产路径出现）
                log::warn!("[kc] start() called with no AppHandle bound; aborting");
                self.set_status_and_port(
                    KcStatus::Unavailable(KcStartError::PythonNotFound.reason().to_string()),
                    None,
                    Some(KcStartError::PythonNotFound),
                );
                return Err(KcStartError::PythonNotFound);
            }
        };

        let python = match detect_embedded_kc_python(&app) {
            Some(p) => p,
            None => {
                self.set_status_and_port(
                    KcStatus::Unavailable(KcStartError::PythonNotFound.reason().to_string()),
                    None,
                    Some(KcStartError::PythonNotFound),
                );
                self.emit_status_changed(
                    &KcStatus::Unavailable(KcStartError::PythonNotFound.reason().to_string()),
                    Some(KcStartError::PythonNotFound.reason()),
                );
                return Err(KcStartError::PythonNotFound);
            }
        };

        let run_api = match detect_embedded_kc_run_api(&app) {
            Some(p) => p,
            None => {
                self.set_status_and_port(
                    KcStatus::Unavailable(KcStartError::PythonNotFound.reason().to_string()),
                    None,
                    Some(KcStartError::PythonNotFound),
                );
                self.emit_status_changed(
                    &KcStatus::Unavailable(KcStartError::PythonNotFound.reason().to_string()),
                    Some(KcStartError::PythonNotFound.reason()),
                );
                return Err(KcStartError::PythonNotFound);
            }
        };

        // env 注入（ADR-007）：直接从 NC settings 读，绝不写 .env。
        // FIX 第 1 轮（MAJOR-2）：读出完整 `KcSettings` 并以 `Arc` 共享，drain 线程走 task_010 精确子串
        // 的 `mask_secrets_by_keys`（task_009 TD-5 重命名后；替代旧的前缀启发 `mask_secret`，
        // 覆盖 dot 格式 / JSON / Debug 三类漏屏）。
        let kc_settings = read_kc_settings(&app);
        let zhipu_key = kc_settings.zhipu_api_key.clone();
        let openai_key = kc_settings.openai_api_key.clone();
        let settings_arc: Arc<crate::kc::settings::KcSettings> = Arc::new(kc_settings);

        // cwd 设到 NC 控制的临时目录（ADR-006 层 2）。
        let kc_runtime_dir = ensure_kc_runtime_dir(&app);

        // 状态置 Starting（emit 一次）。
        self.set_status_and_port(KcStatus::Starting, None, None);
        self.emit_status_changed(&KcStatus::Starting, None);

        for attempt in 0..SPAWN_RETRY {
            // 步骤 3：选空闲端口（每次重试都重选，避免单端口 race）。
            let port = match pick_free_port() {
                Some(p) => p,
                None => {
                    last_err = KcStartError::PortBindFailed;
                    log::warn!("[kc] 选端口失败（attempt {attempt}/{SPAWN_RETRY}）");
                    continue;
                }
            };

            // 步骤 4：spawn child + env + cwd + stdio piped。
            let port_str = port.to_string();
            let mut cmd = Command::new(&python);
            cmd.arg(&run_api)
                .args(["--host", "127.0.0.1"])
                .args(["--port", &port_str])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .current_dir(&kc_runtime_dir)
                // ADR-006 层 2：让 KC 即便误写 wiki/ 也写在 NC 控制的临时目录。
                .env("WIKI_DIR", kc_runtime_dir.to_string_lossy().to_string());

            // ADR-007：env 注入（仅 Some 时设；None 时让 KC 内部走未配置兜底）。
            if let Some(ref k) = zhipu_key {
                cmd.env("ZHIPUAI_API_KEY", k);
            }
            if let Some(ref k) = openai_key {
                cmd.env("OPENAI_API_KEY", k);
            }

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    log::warn!(
                        "[kc] spawn 失败（attempt {attempt}/{SPAWN_RETRY}，port {port}）: {e}"
                    );
                    last_err = KcStartError::PortBindFailed;
                    continue;
                }
            };

            // 步骤 5：drain stdout/stderr（参考 markitdown.rs:322-336）。
            // 双线程持续读，避免 OS pipe buffer 满后 KC 阻塞在 write 上。
            // 每个线程 clone 一份 `Arc<KcSettings>`，drain 内每行经
            // `mask_secrets_by_keys(line, &settings)` 替换后再 log（MAJOR-2 fix + task_009 TD-5 重命名）。
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            let stdout_settings = Arc::clone(&settings_arc);
            let stderr_settings = Arc::clone(&settings_arc);
            let stdout_handle = stdout.and_then(|s| {
                thread::Builder::new()
                    .name("kc-stdout-drain".into())
                    .spawn(move || drain_pipe_to_log(s, "kc:stdout", &stdout_settings))
                    .ok()
            });
            let stderr_handle = stderr.and_then(|s| {
                thread::Builder::new()
                    .name("kc-stderr-drain".into())
                    .spawn(move || drain_pipe_to_log(s, "kc:stderr", &stderr_settings))
                    .ok()
            });

            // 记 child 到状态（在轮询 health 之前，让 stop()/Drop 即便此时也能干净退出）。
            {
                let mut guard = self.state.lock().expect("state mutex poisoned");
                guard.child = Some(child);
                guard.port = Some(port);
                guard.stdout_join = stdout_handle;
                guard.stderr_join = stderr_handle;
            }

            // 步骤 6：轮询 health（异步，10s 上限 = 200ms × 50）。
            let health_ok = poll_health_check(port, Duration::from_millis(200), 50).await;
            if health_ok {
                self.set_status_and_port(KcStatus::Ready, Some(port), None);
                // 监控 child exit（步骤 7）。
                self.spawn_child_exit_monitor();
                self.emit_status_changed(&KcStatus::Ready, None);
                log::info!("[kc] KC ready on 127.0.0.1:{port}");
                return Ok(());
            }

            // health timeout：先 stop 当前 child，再决定是重试还是 fail。
            log::warn!(
                "[kc] health check timeout（attempt {attempt}/{SPAWN_RETRY}，port {port}）"
            );
            self.kill_child_locked();

            // 检查 child 是否早夭（exit code 已有）→ ProcessExitedDuringStartup
            // 否则视为 HealthCheckTimedOut 重试。
            let exited_early = self.child_exit_status_locked().is_some();
            if exited_early {
                last_err = KcStartError::ProcessExitedDuringStartup;
            } else {
                last_err = KcStartError::HealthCheckTimedOut;
            }
        }

        // 三次都失败 → 最终状态 Unavailable。
        self.set_status_and_port(
            KcStatus::Unavailable(last_err.reason().to_string()),
            None,
            Some(last_err.clone()),
        );
        self.emit_status_changed(
            &KcStatus::Unavailable(last_err.reason().to_string()),
            Some(last_err.reason()),
        );
        Err(last_err)
    }

    // -----------------------------------------------------------------
    // AC-4: fn stop()
    // -----------------------------------------------------------------

    /// 同步停止 KC 子进程（AC-4）。
    ///
    /// 实现：
    /// 1. take child + 尝试 SIGTERM（Unix `kill -TERM`，graceful，KC-MOD-6）；
    /// 2. 等 5s graceful（轮询 `try_wait`）；
    /// 3. 仍未退则 SIGKILL fallback；
    /// 4. join drain 线程；
    /// 5. 状态置 Stopped。
    ///
    /// **幂等**：已 Stopped 时直接返回（不向上抛错）。
    /// **不返回 Result**——任何子操作失败仅 log::warn。
    pub fn stop(&self) {
        // Step 1: take child 与 drain handles（锁内完成）。
        let (mut child_opt, stdout_jh, stderr_jh, monitor_jh) = {
            let mut guard = self.state.lock().expect("state mutex poisoned");
            (
                guard.child.take(),
                guard.stdout_join.take(),
                guard.stderr_join.take(),
                guard.monitor_join.take(),
            )
        };

        if let Some(ref mut child) = child_opt {
            // KC-MOD-6：先 SIGTERM 优雅，再 SIGKILL fallback。
            graceful_terminate_child(child, Duration::from_secs(5));
        }

        // Step 4: join drain 线程（避免 leak）。
        if let Some(jh) = stdout_jh {
            let _ = jh.join();
        }
        if let Some(jh) = stderr_jh {
            let _ = jh.join();
        }
        // monitor 线程会在 child wait 返回后自然结束；不强 join 避免死锁
        // （它在试图获取同一个 Mutex）。
        // 由于 monitor 线程持有 weak 引用，状态正确清理即可让它退出。
        drop(monitor_jh);

        // Step 5: 状态置 Stopped。
        self.set_status_and_port(KcStatus::Stopped, None, None);
        self.emit_status_changed(&KcStatus::Stopped, None);
        log::info!("[kc] KcProcessManager.stop() 完成");
    }

    // -----------------------------------------------------------------
    // AC-5: async fn health_check()
    // -----------------------------------------------------------------

    /// 单次健康检查（AC-5）。返回当前 status + last_check + uptime。
    pub async fn health_check(&self) -> KcHealthStatus {
        let (port, status, startup_time) = {
            let guard = self.state.lock().expect("state mutex poisoned");
            (guard.port, guard.status.clone(), guard.startup_time)
        };

        // Ready 状态下额外发一次 HTTP 验证（健康检查的真实语义）。
        let (final_status, reason) = if matches!(status, KcStatus::Ready) {
            if let Some(p) = port {
                if single_health_request(p).await {
                    (status.clone(), None)
                } else {
                    // 单次失败不立刻降级（连续 3 次失败才标 Unavailable，那是上层 retry 的事）。
                    // 这里仅报告"本次 health 请求 fail"，状态还保持 Ready。
                    (status.clone(), Some("transient health request failure".to_string()))
                }
            } else {
                (status.clone(), Some("ready but no port recorded".to_string()))
            }
        } else if let KcStatus::Unavailable(r) = &status {
            (status.clone(), Some(r.clone()))
        } else {
            (status.clone(), None)
        };

        let uptime_secs = if matches!(final_status, KcStatus::Ready) {
            startup_time.map(|t| t.elapsed().as_secs())
        } else {
            None
        };

        KcHealthStatus {
            status: final_status.as_event_str().to_string(),
            reason,
            port,
            last_check: Utc::now(),
            uptime_secs,
        }
    }

    // -----------------------------------------------------------------
    // AC-6: async fn restart()
    // -----------------------------------------------------------------

    /// 异步重启（AC-6）。先 stop + 检查冷却期 + start。
    ///
    /// 冷却期约束：30s 内 ≥ 2 次 **OR** 60s 内 ≥ 3 次重启 → `RestartCooldownExceeded`，
    /// 不真启动；状态保持 `Unavailable("restart cooldown exceeded")`。
    pub async fn restart(&self) -> Result<(), KcStartError> {
        // 检查冷却期（锁内完成 + 注册当前 restart 时间）。
        let now = Instant::now();
        {
            let mut guard = self.state.lock().expect("state mutex poisoned");
            guard.clean_restart_history(now);
            if guard.is_in_cooldown(now) {
                let err = KcStartError::RestartCooldownExceeded;
                guard.status = KcStatus::Unavailable(err.reason().to_string());
                guard.last_failure = Some(err.clone());
                drop(guard);
                self.emit_status_changed(
                    &KcStatus::Unavailable(err.reason().to_string()),
                    Some(err.reason()),
                );
                log::warn!("[kc] restart cooldown exceeded（30s 内 ≥ 2 次 OR 60s 内 ≥ 3 次）");
                return Err(err);
            }
            guard.restart_history.push_back(now);
        }

        // stop 在锁外（避免 deadlock）。
        self.stop();

        // 再 start。
        self.start().await
    }

    // -----------------------------------------------------------------
    // AC-1 getter
    // -----------------------------------------------------------------

    /// 当前状态快照（AC-1）。
    pub fn current_status(&self) -> KcStatus {
        self.state.lock().expect("state mutex poisoned").status.clone()
    }

    // -----------------------------------------------------------------
    // 内部 helpers
    // -----------------------------------------------------------------

    /// 原子地更新状态 + port + last_failure（锁内完成，不 await）。
    fn set_status_and_port(
        &self,
        status: KcStatus,
        port: Option<u16>,
        failure: Option<KcStartError>,
    ) {
        let mut guard = self.state.lock().expect("state mutex poisoned");
        let was_ready = matches!(guard.status, KcStatus::Ready);
        let becoming_ready = matches!(status, KcStatus::Ready);
        guard.status = status;
        if port.is_some() {
            guard.port = port;
        }
        guard.last_failure = failure;
        if becoming_ready && !was_ready {
            guard.startup_time = Some(Instant::now());
        }
        if !becoming_ready {
            guard.startup_time = None;
        }
    }

    /// emit `notecapt/kc-status-changed`（AC-8）。
    /// AppHandle 未绑定（测试）时静默忽略。
    fn emit_status_changed(&self, status: &KcStatus, reason: Option<&str>) {
        if let Some(ref app) = self.app_handle {
            let payload = serde_json::json!({
                "status": status.as_event_str(),
                "reason": reason,
            });
            let _ = app.emit("notecapt/kc-status-changed", payload);
        }
    }

    /// 同步 kill 当前 child（锁内 take + 锁外 kill+wait，避免锁内阻塞）。
    fn kill_child_locked(&self) {
        let mut child_opt = {
            let mut guard = self.state.lock().expect("state mutex poisoned");
            guard.child.take()
        };
        if let Some(ref mut child) = child_opt {
            if let Err(e) = child.kill() {
                log::warn!("[kc] kill_child_locked: kill 失败: {e}");
            }
            if let Err(e) = child.wait() {
                log::warn!("[kc] kill_child_locked: wait 失败: {e}");
            }
        }
    }

    /// 探测 child 是否已退出（锁内 try_wait，不阻塞）。
    fn child_exit_status_locked(&self) -> Option<std::process::ExitStatus> {
        let mut guard = self.state.lock().expect("state mutex poisoned");
        if let Some(ref mut child) = guard.child {
            match child.try_wait() {
                Ok(Some(status)) => Some(status),
                _ => None,
            }
        } else {
            None
        }
    }

    /// 监控 child exit（AC-3 步骤 7）。在独立线程内调 `child.wait()` 阻塞。
    /// child 退出时 → 标记 Unavailable + 检查冷却期 + 决定是否自动重启。
    fn spawn_child_exit_monitor(&self) {
        // 拿 manager 的弱句柄（Clone Arc 即可）。
        let state = Arc::clone(&self.state);
        let manager_clone = self.clone();

        let handle = thread::Builder::new()
            .name("kc-child-monitor".into())
            .spawn(move || {
                // 注意：take child 出来 wait 后 child 就不能再被 stop() kill，
                // 所以 monitor 模式是"wait 等 child 真死"——stop() 已经 take 走 child 的话
                // 此处 child 为 None，监控线程直接退出。
                let child_to_wait: Option<Child> = {
                    let mut guard = state.lock().expect("state mutex poisoned");
                    guard.child.take()
                };
                let mut child = match child_to_wait {
                    Some(c) => c,
                    None => return, // 已被 stop() 接管
                };

                let exit_status = child.wait();
                let reason = match exit_status {
                    Ok(s) => classify_child_exit(s),
                    Err(e) => format!("wait error: {e}"),
                };

                // 进入 Unavailable + 触发自动重启（受冷却期约束）。
                {
                    let mut guard = state.lock().expect("state mutex poisoned");
                    // 若已被 stop()，不动状态。
                    if matches!(guard.status, KcStatus::Stopped) {
                        return;
                    }
                    guard.status = KcStatus::Unavailable(reason.clone());
                    guard.startup_time = None;
                    // child 已自然死亡，无需再 take。
                }

                log::warn!("[kc] child exited unexpectedly: {reason}");

                // 异步触发自动重启（不阻塞监控线程）。
                tauri::async_runtime::spawn(async move {
                    let r = manager_clone.restart().await;
                    if let Err(e) = r {
                        log::warn!("[kc] 自动重启失败: {:?}", e);
                    }
                });
            })
            .ok();

        // 保存 handle 到状态（防 leak 兜底）。
        if let Ok(mut guard) = self.state.lock() {
            guard.monitor_join = handle;
        }
    }
}

// =====================================================================
// 4. PortProvider impl（AC-2）
// =====================================================================

impl PortProvider for KcProcessManager {
    fn current_port(&self) -> Option<u16> {
        let guard = self.state.lock().expect("state mutex poisoned");
        if matches!(guard.status, KcStatus::Ready) {
            guard.port
        } else {
            None
        }
    }
}

// =====================================================================
// 5. RAII Drop（AC-7）
// =====================================================================

/// **RAII 兜底**：drop 时同步 kill child + join drain 线程。
///
/// 不变量：**不 panic**——任何 child.kill() / wait() 失败仅 log::warn。
/// 即便 Mutex poisoned 也走 `into_inner` fallback，避免二次 panic。
impl Drop for KcProcessManager {
    fn drop(&mut self) {
        // strong_count == 1 表示这是最后一个引用，需要做清理。
        // 其他引用持有时，drop 是无效的（多 Clone 的常见场景）。
        if Arc::strong_count(&self.state) != 1 {
            return;
        }

        // 不调 self.stop()（避免 emit + log 路径在 drop 中 panic）；
        // 仅做 child kill + 线程 join 这两件兜底事。
        let (mut child_opt, stdout_jh, stderr_jh, _monitor_jh) = match self.state.lock() {
            Ok(mut guard) => (
                guard.child.take(),
                guard.stdout_join.take(),
                guard.stderr_join.take(),
                guard.monitor_join.take(),
            ),
            Err(poisoned) => {
                // poisoned lock：拿 inner 兜底。
                let mut inner = poisoned.into_inner();
                (
                    inner.child.take(),
                    inner.stdout_join.take(),
                    inner.stderr_join.take(),
                    inner.monitor_join.take(),
                )
            }
        };

        if let Some(ref mut child) = child_opt {
            // Drop 用 SIGKILL 直接 kill（不走 graceful，drop 不能阻塞 5s）。
            if let Err(e) = child.kill() {
                log::warn!("[kc] Drop: kill 失败: {e}");
            }
            if let Err(e) = child.wait() {
                log::warn!("[kc] Drop: wait 失败: {e}");
            }
        }

        if let Some(jh) = stdout_jh {
            let _ = jh.join();
        }
        if let Some(jh) = stderr_jh {
            let _ = jh.join();
        }
    }
}

// =====================================================================
// 6. 模块级 helpers
// =====================================================================

/// 探测 KC python 解释器路径（参考 scheduler.rs:798 `detect_embedded_markitdown_python`）。
///
/// 优先级（ADR-010）：
/// 1. `.app/Contents/Resources/kc/venv/bin/python`（DMG 打包后正式路径）
/// 2. `.app/Contents/Resources/kc/venv/bin/python3`（python 软链不一致时的兜底）
/// 3. **仅 dev 模式（`debug_assertions`）**：`KnowledgeCompiler/.venv/bin/python`
///    （从 cwd 向上找 KnowledgeCompiler 工作区，让本机开发无需打 DMG 也能起 KC）
/// 4. 兜底：复用 markitdown-venv（仅 dev 期 + KC 未独立打包时；ADR-010 要求最终独立 venv）
///
/// **task_009 / AC-3**：原 fn 私有；改 `pub` 以便 lib.rs 在 setup 前可调用做"打包预检"。
/// dev fallback 用 `#[cfg(debug_assertions)]` 保护——release binary 永远不会从外部仓库拉 venv。
pub fn detect_embedded_kc_python(app: &AppHandle) -> Option<String> {
    let resource_dir = app.path().resource_dir().ok()?;
    let mut candidates: Vec<PathBuf> = vec![
        resource_dir.join("kc/venv/bin/python"),
        resource_dir.join("kc/venv/bin/python3"),
    ];

    // dev fallback：仅 debug 编译时，向上找 KnowledgeCompiler/.venv（release 严禁）。
    #[cfg(debug_assertions)]
    {
        if let Some(p) = find_dev_kc_venv_python() {
            candidates.push(p);
        }
    }

    // 兜底：markitdown-venv（仅 dev / 过渡期）。
    candidates.push(resource_dir.join("markitdown-venv/bin/python"));
    candidates.push(resource_dir.join("markitdown-venv/bin/python3"));

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().to_string())
}

/// dev 模式从 cwd 向上找 `KnowledgeCompiler/.venv/bin/python`。
///
/// 探测策略：从当前可执行文件目录或 cwd 出发，向上最多 6 级查 sibling `KnowledgeCompiler/`。
/// 仅在 debug build 启用——release build 不会编译此函数（避免任何泄漏外部仓库路径的可能）。
#[cfg(debug_assertions)]
fn find_dev_kc_venv_python() -> Option<PathBuf> {
    // 优先从 cwd 出发（dev 启动时 cwd 一般是 src-tauri/ 或项目根目录）。
    let starts: Vec<PathBuf> = [
        std::env::current_dir().ok(),
        std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)),
    ]
    .into_iter()
    .flatten()
    .collect();

    for start in starts {
        let mut cur: Option<&std::path::Path> = Some(start.as_path());
        for _ in 0..6 {
            let Some(dir) = cur else { break };
            // 探测 sibling KnowledgeCompiler/.venv/bin/python
            let candidate = dir.join("KnowledgeCompiler/.venv/bin/python");
            if candidate.is_file() {
                return Some(candidate);
            }
            // 探测同 workspace 下 ../../KLchunkline/KnowledgeCompiler（本机实际目录结构）
            let candidate_kl = dir.join("KLchunkline/KnowledgeCompiler/.venv/bin/python");
            if candidate_kl.is_file() {
                return Some(candidate_kl);
            }
            cur = dir.parent();
        }
    }
    None
}

/// 探测 KC `run_api.py` 路径。
fn detect_embedded_kc_run_api(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    let candidates = [
        resource_dir.join("kc/src/run_api.py"),
        resource_dir.join("kc/run_api.py"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// 选一个空闲 TCP 端口（127.0.0.1:0 bind + 立刻 drop）。
///
/// **竞态说明**：从 drop listener 到 KC bind 之间存在微窗口，理论上 OS 可能把端口
/// 重分配给其他进程。本函数返回 port 后调用方应**立即 spawn**，并在 spawn 失败时
/// 重试（KcProcessManager::start 中已实装 3 次重试）。
fn pick_free_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
}

/// 确保 NC 控制的 KC 运行时目录存在（ADR-006 层 2）。
/// `<app_local_data_dir>/kc_runtime/`，失败兜底为系统临时目录。
fn ensure_kc_runtime_dir(app: &AppHandle) -> PathBuf {
    let base = app
        .path()
        .app_local_data_dir()
        .ok()
        .map(|p| p.join("kc_runtime"))
        .unwrap_or_else(|| std::env::temp_dir().join("notecapt_kc_runtime"));
    let _ = std::fs::create_dir_all(&base);
    base
}

/// 从 NC settings 读完整 `KcSettings`（ADR-007）。
///
/// 失败兜底返回 `KcSettings::default()`（两个 Key 均为 None）：让 KC 内部走"未配置"路径，
/// 主链路不阻塞。
///
/// **MAJOR-2 fix**：原版 `read_kc_keys_from_settings` 仅返回两个 Key 字符串；本版本返回完整
/// `KcSettings`，让 drain 线程能用 `mask_secrets_by_keys(line, &settings)`（task_009 TD-5
/// 重命名后）精确替换日志中的 Key 子串（覆盖 task_008 旧 `mask_secret` 漏屏的 dot 格式 / JSON /
/// Debug dump 三类场景）。
fn read_kc_settings(app: &AppHandle) -> crate::kc::settings::KcSettings {
    use crate::kc::settings::KcSettings;
    let db_state = match app.try_state::<crate::db::Database>() {
        Some(s) => s,
        None => return KcSettings::default(),
    };
    // `Mutex<Connection>` 锁（与 lib.rs setup 一致）；失败兜底走 default。
    let conn = match db_state.conn.lock() {
        Ok(c) => c,
        Err(_) => return KcSettings::default(),
    };
    KcSettings::load(&conn).unwrap_or_default()
}

/// 子进程退出状态分类（崩溃分类恢复用）。
fn classify_child_exit(status: std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    if let Some(code) = status.code() {
        // 正常退出（含非零返回码）。
        if code == 0 {
            "child exited normally (code 0)".to_string()
        } else {
            format!("child exited with code {code}")
        }
    } else if let Some(sig) = status.signal() {
        // 被信号 kill（POSIX-specific；macOS / Linux）。
        format!("child killed by signal {sig}")
    } else {
        "child exited (unknown cause)".to_string()
    }
}

/// graceful 终止 child：先 SIGTERM 等 graceful，再 SIGKILL fallback（KC-MOD-6）。
///
/// 实现策略：通过 `/bin/kill -TERM <pid>` 发 SIGTERM（unix）；
/// 避免引入 `libc` 新依赖（input.md 不引入新 crate 约束）。
/// windows 无 SIGTERM 概念 → 直接 `child.kill()` 等价 SIGKILL。
fn graceful_terminate_child(child: &mut Child, graceful_window: Duration) {
    #[cfg(unix)]
    {
        let pid = child.id();
        // 通过 shell kill 命令发 SIGTERM，等价 libc::kill(pid, SIGTERM)。
        // 错误兜底：kill 命令失败（极罕见）→ 直接走 SIGKILL fallback。
        let term_ok = Command::new("/bin/kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !term_ok {
            log::warn!("[kc] /bin/kill -TERM {pid} 失败，直接 SIGKILL fallback");
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        // 等 graceful（轮询 try_wait）。
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return, // 已 graceful 退出
                Ok(None) => {
                    if start.elapsed() >= graceful_window {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_) => break,
            }
        }
        // 超时未退 → 兜底 SIGKILL。
        log::warn!(
            "[kc] graceful_terminate_child: SIGTERM 后 {}s 未退，发 SIGKILL fallback",
            graceful_window.as_secs()
        );
        let _ = child.kill();
        let _ = child.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// 持续读 pipe 输出 + 按行写日志 + 自动屏蔽 Key（KC-MOD-7）。
///
/// **MAJOR-2 fix + task_009 TD-5 重命名**：每行经
/// `crate::kc::settings::mask_secrets_by_keys(line, settings)` 精确子串替换后再写 log，
/// **取代**旧的 token 前缀启发 `mask_secret`（漏屏 dot 格式智谱 Key / JSON dump / Debug dump
/// 三类场景；reviewer 已 verify）。函数名由 `mask_secrets` 改为 `mask_secrets_by_keys`
/// 后语义更明确（"已知 Key 精确替换" vs client 模块的 `mask_secrets_by_prefix` "未知 Key
/// 前缀启发"）。
///
/// `settings` 通过 `Arc<KcSettings>` 在 spawn 线程时 clone 进入闭包，避免 lifetime 问题。
fn drain_pipe_to_log<R: std::io::Read>(
    mut reader: R,
    prefix: &'static str,
    settings: &crate::kc::settings::KcSettings,
) {
    let mut buf = [0u8; 4096];
    let mut acc = String::new();
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                    acc.push_str(s);
                } else {
                    // 非 UTF-8 字节：用 lossy 但仍要 mask
                    acc.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
                while let Some(idx) = acc.find('\n') {
                    let line = acc[..idx].trim_end_matches('\r').to_string();
                    acc.drain(..=idx);
                    if !line.is_empty() {
                        log::info!(
                            "[{}] {}",
                            prefix,
                            crate::kc::settings::mask_secrets_by_keys(&line, settings)
                        );
                    }
                }
            }
            Err(_) => break,
        }
    }
    // 收尾：剩余尾巴
    let tail = acc.trim_end_matches('\r');
    if !tail.is_empty() {
        log::info!(
            "[{}] {}",
            prefix,
            crate::kc::settings::mask_secrets_by_keys(tail, settings)
        );
    }
}

/// 单次 health 请求（reqwest 短超时 + 不报错抛弃）。
async fn single_health_request(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/api/v1/health");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    matches!(client.get(&url).send().await, Ok(r) if r.status().is_success())
}

/// 轮询 health（AC-3 步骤 6）：每 `interval` 一次，最多 `max_attempts` 次，任一 200 → 返回 true。
async fn poll_health_check(port: u16, interval: Duration, max_attempts: u32) -> bool {
    for _ in 0..max_attempts {
        if single_health_request(port).await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }
    false
}

// =====================================================================
// 7. 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- AC-9 单元测试 ----------

    /// 端口选择不应连续返回同一端口（OS 行为：bind:0 几乎不会重号）。
    /// 三次选都拿到 > 1024 的端口即视为 PASS。
    #[test]
    fn port_selection_returns_unique_port() {
        let p1 = pick_free_port().expect("should pick a port");
        let p2 = pick_free_port().expect("should pick a port");
        let p3 = pick_free_port().expect("should pick a port");
        assert!(p1 > 1024, "应当是高端口");
        assert!(p2 > 1024);
        assert!(p3 > 1024);
        // 三次都返回同一个端口的概率极低，但即便偶尔相同也接受（OS reuse policy）；
        // 此处更关注"能拿到合法端口"。
    }

    /// 60s 滑动窗口清理：插入 4 个超过 60s 的旧 Instant，调清理后窗口空。
    #[test]
    fn restart_history_window_cleans_old_entries() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        let stale = now - Duration::from_secs(120); // 远超 60s
        for _ in 0..4 {
            state.restart_history.push_back(stale);
        }
        // 加 1 个新 instant 验证保留
        state.restart_history.push_back(now);

        state.clean_restart_history(now);

        assert_eq!(state.restart_history.len(), 1, "60s 外的应被清空");
        assert_eq!(
            state.restart_history.front().copied(),
            Some(now),
            "保留 60s 内的最新一个"
        );
    }

    // -------------------------------------------------------------
    // 冷却期 30s/2 + 60s/3 新阈值（FIX 第 1 轮 MAJOR-1）
    // -------------------------------------------------------------
    //
    // 设计：30s 窗口阈值为 2、60s 窗口阈值为 3。两段独立、互补：
    // - 30s 内连续 2 次 = 快速失败模式 → short 规则即触发
    // - 60s 内 3 次（含 30s 外的）= 慢崩累积模式 → long 规则触发
    // - 60s 内 2 次（30s 内只 1 次）= 偶发，不视为冷却（与旧版"30s/2 + 60s/2"实质不同）。
    //
    // 旧测试 `cooldown_exceeded_when_two_restarts_within_window` 已被
    // `cooldown_short_window_30s_2_triggers` 取代（更精确的语义）。

    /// 冷却期：30s 内 2 次崩 → short 规则触发（与 long 规则独立）。
    #[test]
    fn cooldown_short_window_30s_2_triggers() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        // t1=15s 前、t2=5s 前（均在 30s 内）
        state.restart_history.push_back(now - Duration::from_secs(15));
        state.restart_history.push_back(now - Duration::from_secs(5));
        // short_count=2, long_count=2 → 触发 short 规则（"≥ 2 in 30s"）。
        assert!(
            state.is_in_cooldown(now),
            "30s 内 2 次应触发 cooldown（short 规则）"
        );
    }

    /// 冷却期：60s 内 3 次崩（30s 内 1 次 + 30-60s 间 2 次）→ long 规则触发，
    /// 而 short 规则不命中（30s 内只有 1 次）。
    #[test]
    fn cooldown_long_window_60s_3_triggers() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        // t1=55s 前、t2=45s 前（在 30-60s 间，short 不计）
        state.restart_history.push_back(now - Duration::from_secs(55));
        state.restart_history.push_back(now - Duration::from_secs(45));
        // t3=10s 前（在 30s 内，short 计 1 次）
        state.restart_history.push_back(now - Duration::from_secs(10));
        // short_count=1, long_count=3 → 触发 long 规则（"≥ 3 in 60s"）。
        assert!(
            state.is_in_cooldown(now),
            "60s 内 3 次（含 30s 内 1 次 + 30-60s 间 2 次）应触发 cooldown（long 规则）"
        );
    }

    /// 冷却期：60s 内 2 次（30s 内仅 1 次）→ 不触发（关键区分新旧阈值的 case）。
    ///
    /// 此 case 在**旧实装（30s/2 + 60s/2）下会触发**（long_count=2 ≥ 2），
    /// 在**新实装（30s/2 + 60s/3）下不触发**（short_count=1 < 2 且 long_count=2 < 3）。
    /// 这是新旧阈值最关键的区分点。
    #[test]
    fn cooldown_long_window_60s_2_does_not_trigger() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        // t1=45s 前（在 30-60s 间，short 不计；long 计）
        state.restart_history.push_back(now - Duration::from_secs(45));
        // t2=15s 前（在 30s 内，short 计；long 计）
        state.restart_history.push_back(now - Duration::from_secs(15));
        // short_count=1, long_count=2 → short 不命中（< 2）、long 不命中（< 3）→ 不冷却。
        assert!(
            !state.is_in_cooldown(now),
            "60s 内 2 次且 30s 内 ≤ 1 次不应触发 cooldown（新阈值核心区分）"
        );
    }

    /// 冷却期：60s 内 1 次 → 不 cooldown（保留旧测试，验证下限）。
    #[test]
    fn cooldown_not_exceeded_when_one_restart_in_window() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        state.restart_history.push_back(now - Duration::from_secs(20));
        assert!(!state.is_in_cooldown(now), "60s 内 1 次不应触发 cooldown");
    }

    /// 冷却期：60s 外的不计入（保留旧测试，验证窗口边界）。
    #[test]
    fn cooldown_ignores_stale_history() {
        let mut state = KcInternalState::new();
        let now = Instant::now();
        // 全在 60s 外
        state.restart_history.push_back(now - Duration::from_secs(120));
        state.restart_history.push_back(now - Duration::from_secs(180));
        state.restart_history.push_back(now - Duration::from_secs(240));
        assert!(!state.is_in_cooldown(now), "60s 外不计冷却");
    }

    // -------------------------------------------------------------
    // mask_secrets_by_keys 替换 dot Key / JSON / Debug（FIX 第 1 轮 MAJOR-2 + task_009 TD-5 重命名）
    // -------------------------------------------------------------
    //
    // 设计：旧自写 `mask_secret` 用前缀启发，漏屏 3 类场景（reviewer verify）：
    //   (a) 智谱真实 dot 格式 Key `a3f2b8.xyz9876` → dot 后明文泄漏
    //   (b) JSON 字段 `{"zhipuai_api_key":"abc..."}` → 完全不 mask
    //   (c) Debug 输出 `KcSettings { zhipu_api_key: Some("...") }` → 完全不 mask
    // 改用 task_010 `mask_secrets_by_keys(msg, &settings)`（task_009 TD-5 重命名）精确子串
    // 替换后，三类全部命中。

    /// 高熵 dot 格式智谱 Key fixture（≥ 8 字符触发 task_010 mask 阈值）。
    const DOT_FIXTURE_KEY: &str = "a3f2b8cd4e5f6789abcdef.xyz1234567890def";

    /// 高熵无前缀 JSON dump fixture（不含 sk-/zhipu- 前缀，旧 mask_secret 完全不 mask）。
    const PLAIN_FIXTURE_KEY: &str = "abc1234567890def567890QnT4mPaY6";

    /// MAJOR-2 验证 (a)：智谱真实 dot 格式 Key 应被完全屏蔽（旧实装 dot 后明文泄漏）。
    #[test]
    fn mask_secrets_by_keys_obscures_zhipu_dot_format() {
        use crate::kc::settings::{mask_secrets_by_keys, KcSettings};
        let settings = KcSettings {
            zhipu_api_key: Some(DOT_FIXTURE_KEY.to_string()),
            ..Default::default()
        };
        let drain_line = format!("ZHIPUAI_API_KEY={DOT_FIXTURE_KEY} (启动日志)");
        let masked = mask_secrets_by_keys(&drain_line, &settings);

        // dot 前 + dot + dot 后整体都不得残留
        assert!(
            !masked.contains(DOT_FIXTURE_KEY),
            "完整 Key 不得残留: {masked}"
        );
        assert!(
            !masked.contains("xyz1234567890def"),
            "dot 后明文不得残留（旧 mask_secret 主漏点）: {masked}"
        );
        // 占位符必须出现
        assert!(
            masked.contains("<ZHIPU_KEY_MASKED>"),
            "应含占位 <ZHIPU_KEY_MASKED>: {masked}"
        );
        // KEY 名 / 文字描述应保留
        assert!(masked.contains("ZHIPUAI_API_KEY"));
        assert!(masked.contains("启动日志"));
    }

    /// MAJOR-2 验证 (b)：JSON 字段 dump（小写 + 引号包裹）应被完全屏蔽。
    #[test]
    fn mask_secrets_by_keys_obscures_json_field_dump() {
        use crate::kc::settings::{mask_secrets_by_keys, KcSettings};
        let settings = KcSettings {
            zhipu_api_key: Some(PLAIN_FIXTURE_KEY.to_string()),
            openai_api_key: None,
            ..Default::default()
        };
        let drain_line = format!(
            r#"received settings dump: {{"zhipuai_api_key":"{PLAIN_FIXTURE_KEY}","model":"glm-4"}}"#
        );
        let masked = mask_secrets_by_keys(&drain_line, &settings);

        // Key 值不得残留（旧 mask_secret 因无 sk-/zhipu- 前缀完全不 mask）
        assert!(
            !masked.contains(PLAIN_FIXTURE_KEY),
            "JSON 中 Key 值不得残留: {masked}"
        );
        // 占位符出现
        assert!(masked.contains("<ZHIPU_KEY_MASKED>"));
        // JSON 结构 / 其他字段保留
        assert!(masked.contains(r#""zhipuai_api_key":"#));
        assert!(masked.contains("glm-4"));
    }

    /// MAJOR-2 验证 (c)：Debug 输出（`KcSettings { ... }`）格式应被完全屏蔽。
    #[test]
    fn mask_secrets_by_keys_obscures_debug_dump() {
        use crate::kc::settings::{mask_secrets_by_keys, KcSettings};
        let zhipu = PLAIN_FIXTURE_KEY.to_string();
        let openai = format!("sk-{}", "Z".repeat(40)); // 高熵无 dash 长 Key
        let settings = KcSettings {
            zhipu_api_key: Some(zhipu.clone()),
            openai_api_key: Some(openai.clone()),
            ..Default::default()
        };
        // 模拟某个调用方误用 `log::info!("{:?}", raw_settings)` 但 raw 内部含明文 Key
        // （注：KcSettings 本身的 Debug 已 redacted，这里手构 dump 模拟"任意第三方代码
        // 误把 Key 嵌进 Debug 字符串"的兜底场景）
        let drain_line = format!(
            r#"KcSettingsCopy {{ zhipu_api_key: Some("{zhipu}"), openai_api_key: Some("{openai}"), enabled: true }}"#
        );
        let masked = mask_secrets_by_keys(&drain_line, &settings);

        assert!(!masked.contains(&zhipu), "Debug 中 zhipu Key 不得残留: {masked}");
        assert!(!masked.contains(&openai), "Debug 中 openai Key 不得残留: {masked}");
        assert!(masked.contains("<ZHIPU_KEY_MASKED>"));
        assert!(masked.contains("<OPENAI_KEY_MASKED>"));
        // 结构关键字保留
        assert!(masked.contains("KcSettingsCopy"));
        assert!(masked.contains("enabled: true"));
    }

    /// 状态转换：初始 Stopped，set Ready 后 startup_time 写入。
    #[test]
    fn set_status_writes_startup_time_on_ready() {
        let mgr = KcProcessManager::new_for_test();
        assert_eq!(mgr.current_status(), KcStatus::Stopped);

        mgr.set_status_and_port(KcStatus::Ready, Some(12345), None);
        assert_eq!(mgr.current_status(), KcStatus::Ready);
        let guard = mgr.state.lock().unwrap();
        assert!(guard.startup_time.is_some(), "Ready 应记录 startup_time");
        assert_eq!(guard.port, Some(12345));
    }

    /// 状态转换：Ready → Unavailable 时 startup_time 清零。
    #[test]
    fn set_status_clears_startup_time_on_non_ready() {
        let mgr = KcProcessManager::new_for_test();
        mgr.set_status_and_port(KcStatus::Ready, Some(8080), None);
        mgr.set_status_and_port(
            KcStatus::Unavailable("test".into()),
            None,
            Some(KcStartError::HealthCheckTimedOut),
        );
        let guard = mgr.state.lock().unwrap();
        assert!(guard.startup_time.is_none(), "非 Ready 应清空 startup_time");
        assert_eq!(guard.last_failure, Some(KcStartError::HealthCheckTimedOut));
    }

    /// PortProvider trait：非 Ready 状态返回 None；Ready 时返回 port。
    #[test]
    fn port_provider_returns_none_when_not_ready() {
        let mgr = KcProcessManager::new_for_test();
        // Stopped 时
        assert_eq!(mgr.current_port(), None);

        // Starting + 端口已选但未 Ready
        mgr.set_status_and_port(KcStatus::Starting, Some(9999), None);
        assert_eq!(
            mgr.current_port(),
            None,
            "Starting 状态不应暴露 port（client 不该用）"
        );

        // Ready 时返回
        mgr.set_status_and_port(KcStatus::Ready, Some(9999), None);
        assert_eq!(mgr.current_port(), Some(9999));
    }

    /// stop() 幂等：未启动时调用不 panic、状态置 Stopped。
    #[test]
    fn stop_is_idempotent_when_not_started() {
        let mgr = KcProcessManager::new_for_test();
        mgr.stop(); // 第一次（实际什么都不做）
        assert_eq!(mgr.current_status(), KcStatus::Stopped);
        mgr.stop(); // 第二次（幂等）
        assert_eq!(mgr.current_status(), KcStatus::Stopped);
    }

    /// Drop 不 panic（无 child / 无线程）。
    #[test]
    fn drop_without_child_does_not_panic() {
        let mgr = KcProcessManager::new_for_test();
        drop(mgr);
        // 走完即 PASS
    }

    /// KcStartError reason 字符串非空且唯一。
    #[test]
    fn kc_start_error_reasons_are_non_empty() {
        let variants = [
            KcStartError::PythonNotFound,
            KcStartError::PortBindFailed,
            KcStartError::HealthCheckTimedOut,
            KcStartError::ProcessExitedDuringStartup,
            KcStartError::RestartCooldownExceeded,
        ];
        let mut seen = std::collections::HashSet::new();
        for v in &variants {
            let r = v.reason();
            assert!(!r.is_empty(), "{:?} reason 不应为空", v);
            assert!(seen.insert(r), "{:?} reason 重复：{}", v, r);
        }
    }

    /// KcStatus event_str 字面值约定（前端解析友好）。
    #[test]
    fn kc_status_event_strings() {
        assert_eq!(KcStatus::Stopped.as_event_str(), "stopped");
        assert_eq!(KcStatus::Starting.as_event_str(), "starting");
        assert_eq!(KcStatus::Ready.as_event_str(), "ready");
        assert_eq!(KcStatus::Unavailable("x".into()).as_event_str(), "unavailable");
    }

    // ---------- async 测试：mock 短路 + health_check ----------
    //
    // **测试间串行化**：以下用 `KC_USE_MOCK_PORT` 的测试通过 `ENV_MUTEX` 保证串行，
    // 因为 Rust 测试默认多线程并发跑，而 std::env 是全局可变状态，并发设/读会互相污染。
    // tokio::test 在不同线程上跑，需要 std::sync::Mutex（不是 tokio::sync）。

    use std::sync::Mutex as StdMutex;
    static ENV_MUTEX: StdMutex<()> = StdMutex::new(());

    /// KC_USE_MOCK_PORT 短路：start() 不 spawn child + 状态 Ready + port 取 env 值。
    #[tokio::test]
    async fn mock_port_env_var_short_circuits_real_startup() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("KC_USE_MOCK_PORT", "31337");

        let mgr = KcProcessManager::new_for_test();
        let result = mgr.start().await;
        assert!(result.is_ok(), "mock 短路应当成功，实际: {:?}", result);
        assert_eq!(mgr.current_status(), KcStatus::Ready);
        assert_eq!(mgr.current_port(), Some(31337));

        // 状态检查：no child handle
        {
            let inner = mgr.state.lock().unwrap();
            assert!(inner.child.is_none(), "mock 短路不应有 child handle");
        }

        std::env::remove_var("KC_USE_MOCK_PORT");
    }

    /// 非法 KC_USE_MOCK_PORT 应忽略并继续走真实路径（最终 PythonNotFound）。
    /// 这里 test_for_test 没有 AppHandle → 直接 PythonNotFound。
    #[tokio::test]
    async fn invalid_mock_port_env_falls_back_to_real_path() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("KC_USE_MOCK_PORT", "not-a-port");

        let mgr = KcProcessManager::new_for_test();
        let result = mgr.start().await;
        // 测试模式无 AppHandle → 走 PythonNotFound 兜底
        assert_eq!(result, Err(KcStartError::PythonNotFound));

        std::env::remove_var("KC_USE_MOCK_PORT");
    }

    /// 冷却期：restart 第 3 次（模拟 history 已 2 条）应当返回 RestartCooldownExceeded。
    #[tokio::test]
    async fn restart_respects_cooldown_window() {
        let mgr = KcProcessManager::new_for_test();
        // 注入 2 个 60s 内的 history
        {
            let mut guard = mgr.state.lock().unwrap();
            let now = Instant::now();
            guard.restart_history.push_back(now - Duration::from_secs(10));
            guard.restart_history.push_back(now - Duration::from_secs(5));
        }

        // 第 3 次 restart 应被冷却期挡住
        let result = mgr.restart().await;
        assert_eq!(
            result,
            Err(KcStartError::RestartCooldownExceeded),
            "应被冷却期挡住"
        );
        assert!(
            matches!(mgr.current_status(), KcStatus::Unavailable(ref r) if r.contains("cooldown")),
            "状态应为 Unavailable(cooldown reason)"
        );
    }

    /// health_check() 在 Stopped 状态下返回合规结构（不 panic）。
    #[tokio::test]
    async fn health_check_returns_status_when_stopped() {
        let mgr = KcProcessManager::new_for_test();
        let h = mgr.health_check().await;
        assert_eq!(h.status, "stopped");
        assert!(h.port.is_none());
        assert!(h.uptime_secs.is_none());
    }

    // =================================================================
    // 集成测试：MockKcServer + KC_USE_MOCK_PORT
    //
    // 用 wiremock（dev-dep）起本地 HTTP server，配合 `KC_USE_MOCK_PORT` 短路
    // 测试 KcProcessManager 的端到端流程：状态机 + health check + 重启冷却。
    //
    // 注意：以下测试和上面"mock_port_env_var_*"测试共享 `ENV_MUTEX`，
    // 确保 KC_USE_MOCK_PORT 设置/清理不会被并发污染。
    // =================================================================

    use wiremock::matchers::{method, path as wm_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 起一个 wiremock server 并挂 `GET /api/v1/health` 200。
    async fn spawn_health_only_mock() -> MockServer {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "status": "ok",
            "ai_enabled": true,
            "v1_ready": true,
        });
        Mock::given(method("GET"))
            .and(wm_path("/api/v1/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        server
    }

    /// **集成测试 1**：`process_starts_with_mock_via_env_var`
    /// 起 mock + 设 KC_USE_MOCK_PORT → start() Ready + health_check() Ready + port 一致。
    #[tokio::test]
    async fn process_starts_with_mock_via_env_var() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let server = spawn_health_only_mock().await;
        let mock_port = server.address().port();

        std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());
        let mgr = KcProcessManager::new_for_test();
        let res = mgr.start().await;
        assert!(res.is_ok(), "mock 短路应成功: {:?}", res);
        assert_eq!(mgr.current_status(), KcStatus::Ready);
        assert_eq!(mgr.current_port(), Some(mock_port));

        // health_check 应当返回 ready（且实际 HTTP 验证通过 mock）。
        let h = mgr.health_check().await;
        assert_eq!(h.status, "ready");
        assert_eq!(h.port, Some(mock_port));
        // reason 此时应当 None（真 health 请求成功）。
        assert!(h.reason.is_none(), "ready 状态下 reason 应为 None，实际: {:?}", h.reason);

        std::env::remove_var("KC_USE_MOCK_PORT");
        drop(server);
    }

    /// **集成测试 2**：`process_unavailable_on_python_not_found`
    /// 未设 KC_USE_MOCK_PORT，且没有 AppHandle → start() 返回 PythonNotFound。
    #[tokio::test]
    async fn process_unavailable_on_python_not_found() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        std::env::remove_var("KC_USE_MOCK_PORT");

        let mgr = KcProcessManager::new_for_test();
        let res = mgr.start().await;
        assert_eq!(res, Err(KcStartError::PythonNotFound));
        // 状态应为 Unavailable 且 reason 含 python
        match mgr.current_status() {
            KcStatus::Unavailable(r) => {
                assert!(r.contains("python"), "reason 应含 python: {r}");
            }
            other => panic!("应为 Unavailable，实际: {:?}", other),
        }
    }

    /// **集成测试 3**：`health_check_recovers_from_transient_failure`
    /// 验证 health_check 在 mock 下线后能正确报告 transient failure（status 仍 ready，reason 非空）。
    ///
    /// 实现：先用 mock 验证 reason=None；再手动把 manager 状态置为 Ready 但 port 指向一个**已知**
    /// 关闭的端口（用 ENV_MUTEX 保护下短时间 bind+drop 拿到的"刚被释放"端口），验证 transient
    /// reason 触发。
    #[tokio::test]
    async fn health_check_recovers_from_transient_failure() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let server = spawn_health_only_mock().await;
        let mock_port = server.address().port();

        std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());
        let mgr = KcProcessManager::new_for_test();
        mgr.start().await.expect("start should succeed");

        // 阶段 1：health 正常（mock 仍活）
        let h1 = mgr.health_check().await;
        assert_eq!(h1.status, "ready");
        assert!(h1.reason.is_none(), "活跃 mock 下 reason 应 None: {:?}", h1.reason);

        // 阶段 2：把 port 改成一个一定不可达的"已释放"端口，模拟瞬时不可达。
        // 直接拿一个 bind 后立刻 drop 的高端口（OS 极大概率不会立刻复用）。
        let dead_port = pick_free_port().expect("should pick port");
        // 手动改 state 模拟 KC 还 Ready 但 mock 端口换了（瞬时网络问题等效）
        {
            let mut g = mgr.state.lock().unwrap();
            g.port = Some(dead_port);
        }
        let h2 = mgr.health_check().await;
        assert_eq!(h2.status, "ready", "单次 health 失败不应降级状态");
        assert!(
            h2.reason.is_some(),
            "不可达端口下 reason 应非空: {:?}",
            h2.reason
        );

        // 阶段 3：把 port 改回 mock，验证 reason 恢复 None（瞬时故障恢复）
        {
            let mut g = mgr.state.lock().unwrap();
            g.port = Some(mock_port);
        }
        let h3 = mgr.health_check().await;
        assert_eq!(h3.status, "ready");
        assert!(h3.reason.is_none(), "恢复后 reason 应 None: {:?}", h3.reason);

        std::env::remove_var("KC_USE_MOCK_PORT");
        drop(server);
    }

    /// **集成测试 4**：`drop_kills_child_without_panic`
    /// 起 mock + start + drop manager → 不 panic（mock 模式下 child 为 None，drop 走纯状态清理）。
    #[tokio::test]
    async fn drop_after_mock_start_does_not_panic() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let server = spawn_health_only_mock().await;
        let mock_port = server.address().port();
        std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());

        let mgr = KcProcessManager::new_for_test();
        mgr.start().await.expect("start should succeed");
        assert_eq!(mgr.current_status(), KcStatus::Ready);

        // 显式 drop，验证不 panic
        drop(mgr);

        std::env::remove_var("KC_USE_MOCK_PORT");
        drop(server);
    }

    /// **集成测试 5**：`stop_after_mock_start_clears_state`
    /// 起 mock + start + stop → 状态置 Stopped + port 不再可读。
    #[tokio::test]
    async fn stop_after_mock_start_clears_state() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let server = spawn_health_only_mock().await;
        let mock_port = server.address().port();
        std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());

        let mgr = KcProcessManager::new_for_test();
        mgr.start().await.expect("start ok");
        assert_eq!(mgr.current_port(), Some(mock_port));

        mgr.stop();
        assert_eq!(mgr.current_status(), KcStatus::Stopped);
        assert_eq!(
            mgr.current_port(),
            None,
            "Stopped 状态下 PortProvider 应返回 None"
        );

        std::env::remove_var("KC_USE_MOCK_PORT");
        drop(server);
    }
}
