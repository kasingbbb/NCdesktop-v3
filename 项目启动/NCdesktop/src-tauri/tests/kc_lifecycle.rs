//! task_009：KC lifecycle integration 集成测试（input.md AC-5）。
//!
//! ## 3 个测试用例
//!
//! | # | 名称 | 验证 |
//! |--|--|--|
//! | 1 | `nc_setup_constructs_kc_singletons_without_blocking` | `KcProcessManager` + `KcClient` 构造 + `Arc<dyn PortProvider>` 转换 + 100ms 不阻塞 |
//! | 2 | `kc_starts_within_5s_with_mock_via_env_var` | mock KC + `KC_USE_MOCK_PORT` → 5s 内状态 Ready |
//! | 3 | `kc_stop_clears_state_after_mock_start` | mock 启动后 stop → 状态置 Stopped + PortProvider 返回 None |
//!
//! ## 设计依据
//!
//! - **input.md AC-5**：3 个 lifecycle 集成测试；
//! - 不依赖真实 Tauri runtime（用 `KcProcessManager::new_test_only_no_app` 跳过 AppHandle 绑定）；
//! - 不依赖真实 KC python venv（用 `KC_USE_MOCK_PORT` + wiremock 模拟）；
//! - 不依赖真实 LLM（mock 只回 health/ingest 静态响应）。
//!
//! ## 与 task_007 `kc_client_integration.rs` 的边界
//!
//! - `kc_client_integration.rs`：只测 `KcClient` HTTP 调用 + 错误分类，PortProvider 用 stub；
//! - 本测试：测 `KcProcessManager + KcClient` **共同 lifecycle**——重点是
//!   "manager 单例 + client 取 port 不脱节" + "setup 不阻塞" + "close 触发 stop"。
//!
//! ## clippy::await_holding_lock 抑制说明
//!
//! 本测试用 `std::sync::Mutex<()>` (`ENV_MUTEX`) 串行化 `KC_USE_MOCK_PORT` env 读写
//! （Rust 测试默认多线程并发跑，env 是全局可变状态）。两个 `#[tokio::test]` 在持锁期间
//! 调 `.await`——clippy 默认对此发警告（`await_holding_lock`）。
//!
//! 这里的锁仅用作"测试间互斥"，不传输跨任务数据；持锁期间 await 是有意为之，与 task_008
//! `src/kc/process.rs::tests` 内 7 处相同模式对齐（reviewer 已 verify）。统一在文件级
//! `#![allow(clippy::await_holding_lock)]` 抑制，避免逐函数标注。

#![allow(clippy::await_holding_lock)]

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use app_lib::kc::{KcClient, KcIngestOptions, KcIngestOutcome, KcProcessManager, KcStatus, PortProvider};
use common::mock_kc::MockKcServer;

// =====================================================================
// 测试间串行化：`KC_USE_MOCK_PORT` 是全局 env 变量，多线程并发跑会污染
// =====================================================================

use std::sync::Mutex as StdMutex;
static ENV_MUTEX: StdMutex<()> = StdMutex::new(());

// =====================================================================
// Test 1: AC-1 NC setup 构造 KC 单例不阻塞
//
// 验证：
// - `KcProcessManager::new_test_only_no_app()` 构造成功
// - `Arc<KcProcessManager>` 可作为 `Arc<dyn PortProvider>` 传入 `KcClient::new`
// - 100ms 内构造完成（不阻塞主线程）
// =====================================================================

#[tokio::test]
async fn nc_setup_constructs_kc_singletons_without_blocking() {
    let start = Instant::now();

    // 完全模拟 lib.rs setup() 内的 KC 注入路径——区别仅在 AppHandle 取不到
    // （集成测试无 Tauri runtime，用 `new_test_only_no_app` 跳过）。
    let kc_manager: Arc<KcProcessManager> = Arc::new(KcProcessManager::new_test_only_no_app());
    let kc_client: Arc<KcClient> =
        Arc::new(KcClient::new(kc_manager.clone() as Arc<dyn PortProvider>));

    let elapsed = start.elapsed();

    // 严格断言：100ms 上限（lib.rs setup 实际执行时间应远低于此）。
    // 这条断言守护 input.md "不阻塞 setup 返回" 不变量。
    assert!(
        elapsed < Duration::from_millis(100),
        "KC 单例构造耗时 {:?} 超过 100ms 上限——会阻塞 NC setup() 返回",
        elapsed
    );

    // 初始状态：Stopped（未调 start）。
    assert_eq!(kc_manager.current_status(), KcStatus::Stopped);
    // client 在 mgr 未 Ready 时，PortProvider 返回 None
    assert_eq!(kc_manager.current_port(), None);

    // sanity：client + manager 类型自洽（编译过即 OK，运行时也用一下）。
    drop(kc_client);
    drop(kc_manager);
}

// =====================================================================
// Test 2: AC-5 mock KC 模式下 5s 内 KC 状态变为 Ready
//
// 验证：
// - 起 mock KC server（wiremock）
// - 设 `KC_USE_MOCK_PORT=<mock 端口>`
// - 调 `KcProcessManager::start()` → 短路成功
// - 状态变为 Ready，远低于 5s 上限
// - `PortProvider::current_port()` 返回 mock 端口（client 据此可发请求）
// =====================================================================

#[tokio::test]
async fn kc_starts_within_5s_with_mock_via_env_var() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());

    // 起 mock KC server（提供 /api/v1/health + /api/v1/ingest）
    let enhanced_md = "# Enhanced\n\n#AI #Mock\n\n正文";
    let mock = MockKcServer::start_with_success(
        enhanced_md,
        common::mock_kc::KcMockMeta::default(),
    )
    .await;
    let mock_port = mock.port();

    // 设环境变量短路真实 spawn
    std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());

    let manager = Arc::new(KcProcessManager::new_test_only_no_app());

    // 调 start，计时
    let t0 = Instant::now();
    let start_result = manager.start().await;
    let elapsed = t0.elapsed();

    // 验证：start 成功 + 5s 内 + 状态 Ready + 端口正确
    assert!(
        start_result.is_ok(),
        "mock 短路 start 应成功，实际: {:?}",
        start_result
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "KC 启动耗时 {:?} 超过 5s 上限",
        elapsed
    );
    assert_eq!(manager.current_status(), KcStatus::Ready);
    assert_eq!(manager.current_port(), Some(mock_port));

    // 通过 KcClient 实际打 health → 走真实 HTTP（验证 PortProvider 链路自洽）。
    // client 持有的 Arc<dyn PortProvider> 应当通过 manager 透出 mock_port。
    let client = KcClient::new(manager.clone() as Arc<dyn PortProvider>);
    let outcome = client
        .ingest_text("# raw", &KcIngestOptions::default())
        .await
        .expect("ingest should succeed against mock");
    match outcome {
        KcIngestOutcome::Success { enhanced_md: ret, .. } => {
            assert_eq!(ret, enhanced_md, "enhanced_md 必须 round-trip");
        }
    }

    // 清理
    std::env::remove_var("KC_USE_MOCK_PORT");
    mock.stop();
}

// =====================================================================
// Test 3: AC-2 WindowEvent::CloseRequested 等价操作 → KC stop
//
// 验证：
// - mock 启动后 manager 处于 Ready
// - 调 `manager.stop()`（与 lib.rs `on_window_event` 闭包内 stop 调用同源）
// - 状态置 Stopped + PortProvider 返回 None（client 不再能拿到端口）
// =====================================================================

#[tokio::test]
async fn kc_stop_clears_state_after_mock_start() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());

    let mock = MockKcServer::start_with_success(
        "# Test\n\nx",
        common::mock_kc::KcMockMeta::default(),
    )
    .await;
    let mock_port = mock.port();

    std::env::set_var("KC_USE_MOCK_PORT", mock_port.to_string());

    let manager = Arc::new(KcProcessManager::new_test_only_no_app());
    manager.start().await.expect("start should succeed");
    assert_eq!(manager.current_status(), KcStatus::Ready);
    assert_eq!(manager.current_port(), Some(mock_port));

    // 等价于 lib.rs `on_window_event(WindowEvent::CloseRequested {..} | Destroyed)` 闭包内
    // `if let Some(mgr) = window.try_state::<Arc<KcProcessManager>>() { mgr.stop(); }`
    manager.stop();

    // 状态机校验
    assert_eq!(manager.current_status(), KcStatus::Stopped);
    assert_eq!(
        manager.current_port(),
        None,
        "Stopped 状态下 PortProvider 必须返 None（client 不应再能调 KC）"
    );

    // 二次 stop 必须幂等（与 lib.rs 钩子可能多次触发对齐）
    manager.stop();
    assert_eq!(manager.current_status(), KcStatus::Stopped);

    std::env::remove_var("KC_USE_MOCK_PORT");
    mock.stop();
}
