//! task_007：`KcClient` HTTP 客户端（reqwest async + `Semaphore(1)` 串行化 + 60s 超时 + 错误分类）。
//!
//! ## 设计依据
//!
//! - **ADR-002**（Architect output.md §"ADR-002 KC HTTP 客户端"）：复用 NC 现有 reqwest 0.12，
//!   `reqwest::Client::builder().connect_timeout(5s).timeout(60s)`，HTTP/1.1 + rustls-tls + json features；
//! - **ADR-009**（同文档 §"ADR-009"）：`Semaphore(1)` 在**客户端层**，permits=1，单 NC 实例内串行；
//!   KC 服务端 4 并发不变（跨 NC 实例的潜在并发由本 NC 自身串行化控制）；
//! - **PRD §5.7**：60s 客户端总超时；
//! - **PRD §4.3**：不自动重试（错误向上抛给 enrichment step 决定降级路径）；
//! - **input.md AC-3/AC-4**：6 类响应→ `KcCallError` 变体映射严格表（HTTP 200/500 + 6 种 KC 错误码 + Malformed）；
//! - **input.md AC-5**：客户端层 1MB 输入预检（`markdown.len() > 1_048_576` 直接 `InputTooLarge`，
//!   不发请求，节省 HTTP cost）；
//! - **session_context §3 不可妥协底线 #1（LLM Key 不明文落盘到日志）**：`Internal.detail` 透传 KC
//!   返回的 `detail.message` 前先做 key mask（避免 KC 端日志含 key 传染到 NC 日志）。
//!
//! ## 模块结构
//!
//! - [`PortProvider`] —— trait 对象，由 task_008 `KcProcessManager` 实装（本 task 仅提供 trait 定义 + 测试 stub）；
//! - [`KcIngestOptions`] —— ingest 请求选项（4 字段，与 KC `IngestRequest` snake_case 严格一致）；
//! - [`KcIngestOutcome`] —— ingest 成功结果（仅 `Success` 变体；`PartialLlmUnavailable` 由 enrichment step
//!   从 `KcCallError::LlmUnavailable { partial_md }` 派生，不在客户端层暴露）；
//! - [`KcClient`] —— 主客户端，持有 `reqwest::Client` + `Arc<Semaphore>` + `Arc<dyn PortProvider>`；
//! - [`KcClient::ingest_text`] —— 公开 async 方法，签名稳定（ADR-002 §"客户端方法"）。
//!
//! ## 不变量
//!
//! 1. **Semaphore permits=1**：单 NC 实例内 KC ingest 永远串行；
//! 2. **60s 客户端总超时**：通过 `reqwest::Client::builder().timeout(60s)` 配置，不手动 tokio::select!；
//! 3. **错误细分 6 类**：与 `KcCallError` 6 变体严格一一对应（ADR-002 §"错误分类"）；
//! 4. **不自动重试**：上层（enrichment step）决定降级路径；
//! 5. **Key mask**：`Internal.detail` 经过 [`mask_secrets`] 处理，避免 KC 端泄漏 key 传染到 NC 日志。

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Semaphore;

use crate::kc::errors::{
    KcCallError, KcMeta, KcParagraphLink, KcQaPair, KcTagsSource,
};

// =====================================================================
// 1. 端口提供器 trait（FIX 第 1 轮：重导出 `kc::process::PortProvider`）
// =====================================================================

/// KC 子进程端口提供器——**重导出自 [`crate::kc::process::PortProvider`]**。
///
/// ## FIX 第 1 轮：单点定义协调
///
/// 历史背景：task_007（本模块）与 task_008（`kc::process`）并发开发期间，两个 dev 各自在自己模块
/// 独立定义了同名 trait `pub trait PortProvider: Send + Sync { fn current_port(&self) -> Option<u16> }`。
/// Rust 类型系统层面这是**两个不同类型**——`KcProcessManager`（task_008 实装 `kc::process::PortProvider`）
/// 不能当 `kc::client::PortProvider` 用，task_009 lifecycle integration 时编译失败。
///
/// 本次 FIX：删除本模块原有的 `pub trait PortProvider` 定义，改为 `pub use` 重导出
/// `kc::process::PortProvider`。两个 path（`kc::client::PortProvider` 与 `kc::process::PortProvider`）
/// 自此**指向同一个 trait 类型**，`KcClient::new(port_provider: Arc<dyn PortProvider>)` 接受
/// `Arc<KcProcessManager>`（其 impl 的是 `kc::process::PortProvider`，与本 path 等价）。
///
/// ## 设计语义（保留以便阅读）
///
/// **为什么用 trait 而不是直接持有端口**：KC 子进程支持崩溃恢复（ADR-001 §"重启时端口可能变化"），
/// 端口在运行时可能从 `None`（未启动） → `Some(p1)`（首次启动） → `None`（崩溃） → `Some(p2)`（重启），
/// `KcClient` 必须每次 ingest 时**实时取**当前端口，不能在 `new()` 时固化。
///
/// **Send + Sync 要求**：`KcClient` 持有 `Arc<dyn PortProvider>`，且 `ingest_text` 是 async（可能跨 await），
/// trait 对象必须可跨线程共享。
///
/// **同步取端口（非 async）**：实装侧用 `std::sync::atomic::AtomicU16` 或 `Mutex<Option<u16>>`，
/// 非阻塞读取——避免 `ingest_text` 在 acquire semaphore 之前就要 await。
///
/// trait 签名：`fn current_port(&self) -> Option<u16>`；`None` 表示 KC 未就绪
/// （未启动 / 崩溃中 / 重启中），`KcClient::ingest_text` 应直接返回 `KcCallError::Unreachable` 而不发请求。
pub use crate::kc::process::PortProvider;

// =====================================================================
// 2. ingest 请求选项（与 KC IngestRequest snake_case 严格一致）
// =====================================================================

/// KC ingest 请求的可选参数（4 字段，snake_case 与 KC `IngestRequest` Pydantic 模型对齐）。
///
/// **字段对齐**（参考 Architect output.md §"NC 调 KC 的 HTTP 调用合约"）：
/// - `use_ai` / `enable_qa` / `enable_links` —— KC 内部三个功能子开关（来自 `KcSettings` 用户配置）；
/// - `persist` —— KC OutputStage 控制开关（**始终 false**，ADR-006 §"层 1：信任 KC-MOD-2"）。
///
/// **持久化默认 false 的理由**：NC 接管 .md 落地（scheduler::materialize_md），KC 写 wiki/ 会
/// 污染用户工作区（违反 PRD 不可妥协底线 #3）。
#[derive(Debug, Clone)]
pub struct KcIngestOptions {
    /// 是否启用 AI 增强（调智谱 AI / OpenAI）；`false` 时 KC 走规则增强。
    pub use_ai: bool,
    /// 是否生成问答对（KC `enable_qa` 入参）。
    pub enable_qa: bool,
    /// 是否生成段落关联（KC `enable_links` 入参）。
    pub enable_links: bool,
    /// 是否让 KC 落地 wiki/ 文件（**始终 false**，ADR-006 层 1 防御）。
    pub persist: bool,
}

impl Default for KcIngestOptions {
    /// 默认值：三个 AI 开关全开、persist=false。
    fn default() -> Self {
        Self {
            use_ai: true,
            enable_qa: true,
            enable_links: true,
            persist: false,
        }
    }
}

// =====================================================================
// 3. ingest 成功结果
// =====================================================================

/// KC ingest 成功结果（仅 200 + `enhanced_markdown` 非空时返回 `Success` 变体）。
///
/// **为什么没有 `PartialLlmUnavailable` 变体**：input.md AC-4 把 KC 返回 500 +
/// `KC_LLM_UNAVAILABLE` 映射到 `KcCallError::LlmUnavailable { partial_md }`——属于"错误"路径，
/// 而非"成功"路径；enrichment step（task_011）从这个错误派生 `KcEnrichmentOutcome::PartialLlmUnavailable`。
/// 把"partial"放在错误侧的好处：客户端层不需要知道"partial 该如何降级"——那是 enrichment step 的职责。
#[derive(Debug, Clone)]
pub enum KcIngestOutcome {
    /// KC 返回 200 + `enhanced_markdown` 字段非空。
    Success {
        /// KC 返回的完整 v6 增强 markdown（不含 NC frontmatter，由 task_013 拼接）。
        enhanced_md: String,
        /// 元数据（11 字段，含客户端注入的 `response_size_bytes` / `duration_ms`）。
        meta: KcMeta,
    },
}

// =====================================================================
// 4. KcClient 主类型
// =====================================================================

/// KC HTTP 客户端（reqwest async + Semaphore 串行化 + 60s 超时）。
///
/// **不变量**（与 ADR-002 / ADR-009 严格一致）：
/// 1. `http` 是 `reqwest::Client`，构建时配置 connect=5s / write=5s / timeout=60s；
/// 2. `semaphore` permits=1，单 NC 实例内 KC ingest 串行；
/// 3. `port_provider` 是 `Arc<dyn PortProvider>`，每次 ingest 实时取端口。
///
/// **线程安全**：`reqwest::Client` 本身 `Clone + Send + Sync`（内部 Arc），`Semaphore` 也是；
/// `KcClient` 可被多个 task 持有 `Arc<KcClient>` 并发调用 `ingest_text`，串行化由 Semaphore 兜底。
pub struct KcClient {
    /// reqwest HTTP 客户端（持有连接池 + 超时配置）。
    http: reqwest::Client,
    /// 串行化信号量（permits=1，ADR-009）。
    semaphore: Arc<Semaphore>,
    /// 端口提供器（task_008 `KcProcessManager` 实装）。
    port_provider: Arc<dyn PortProvider>,
}

impl KcClient {
    /// 构造 `KcClient`（AC-2）。
    ///
    /// **reqwest 配置**（ADR-002 §"决策"）：
    /// - `connect_timeout = 5s`：连接握手最长 5s（KC 是 127.0.0.1，正常应 <100ms，5s 防极端冷启动）；
    /// - `timeout = 60s`：单次请求总超时（PRD §5.7）；
    /// - 不显式配 `write_timeout`（reqwest 0.12 默认不支持，由 `timeout` 兜底）；
    /// - 不启用 HTTP/2（KC 是 uvicorn HTTP/1.1）；
    /// - 不设 default headers（每次请求带 `X-Request-Id`，不能 client-global）。
    ///
    /// **失败处理**：`reqwest::Client::builder().build()` 在 features 完整时不会失败（rustls-tls 已编进 binary）；
    /// 用 `expect` 是合理的——构造失败属于编译/配置层 bug，应当让进程立即 panic 暴露问题。
    pub fn new(port_provider: Arc<dyn PortProvider>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest::Client::builder for KcClient must succeed (rustls-tls feature compiled)");
        Self {
            http,
            semaphore: Arc::new(Semaphore::new(1)),
            port_provider,
        }
    }

    /// **测试用**构造器：注入自定义 `reqwest::Client`（用于短超时测试）。
    ///
    /// 公共 `new` 写死 60s timeout，集成测试不能等 60s 才看到 timeout 路径；
    /// 测试用本 ctor 注入 100ms timeout 的 client，验证 reqwest 超时映射到 `KcCallError::Timeout`。
    ///
    /// **不在 production 中使用**：`#[cfg(any(test, feature = "test-helpers"))]` 限制——
    /// 但因为本 module 没启用 features 机制，这里用 `#[doc(hidden)]` + 命名约定（`new_with_*`）
    /// 提示调用方"非生产路径"。
    #[doc(hidden)]
    pub fn new_with_http_client(
        port_provider: Arc<dyn PortProvider>,
        http: reqwest::Client,
    ) -> Self {
        Self {
            http,
            semaphore: Arc::new(Semaphore::new(1)),
            port_provider,
        }
    }

    /// 调用 KC `/api/v1/ingest`，把 markdown 增强为 KC v6 增强 MD（AC-3）。
    ///
    /// ## 流程
    ///
    /// 1. **1MB 预检**（AC-5）：`markdown.len() > 1_048_576` → 直接 `InputTooLarge`，不发请求；
    /// 2. **acquire permit**：`semaphore.acquire_owned().await` 拿独占（RAII，函数返回前自动释放）；
    /// 3. **取端口**：`port_provider.current_port()`；`None` → `Unreachable`；
    /// 4. **构造请求**：POST `http://127.0.0.1:<port>/api/v1/ingest` + JSON body + `X-Request-Id` header；
    /// 5. **发送 + 计时**：记录 `Instant::now()` 用于 `meta.duration_ms`；
    /// 6. **响应分类**（AC-4）：
    ///    - reqwest 错误：connect/dns 错 → `Unreachable`；is_timeout → `Timeout`；其他 → `Internal`；
    ///    - HTTP 200：解析 body，按 `enhanced_markdown` 是否非空 → `Success` / `Malformed`；
    ///    - HTTP 500：解析 `detail.error_code`，6 种 KC 错误码 → 4 种 `KcCallError` 变体（含未知码 fallback）；
    ///    - HTTP 其他：当作 `Internal`（KC 不预期返 2xx-3xx 之外的 4xx）。
    ///
    /// **不重试**（PRD §4.3）：失败直接抛出，上层 enrichment step 决定降级。
    pub async fn ingest_text(
        &self,
        markdown: &str,
        options: &KcIngestOptions,
    ) -> Result<KcIngestOutcome, KcCallError> {
        // ---- 步骤 1：1MB 输入预检（AC-5）----
        // 客户端层先拒绝大输入，节省 HTTP 往返成本（KC 端也会拒，但拒的成本是 1 个 HTTP RTT）。
        // 边界严格：> 1_048_576 拒绝（== 1MB 通过；> 1MB 拒绝）。
        if markdown.len() > 1_048_576 {
            return Err(KcCallError::InputTooLarge);
        }

        // ---- 步骤 2：acquire Semaphore permit（RAII）----
        // .acquire_owned() 返回 OwnedSemaphorePermit，函数返回时 drop 释放。
        // .clone() Arc<Semaphore> 让 owned permit 不与 self 借用冲突。
        // expect() 是合理的——只有 semaphore 被 close() 才会失败，本 client 永不 close。
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("KcClient semaphore never closed");

        // ---- 步骤 3：实时取端口（acquire 之后取，避免锁定期间 port 已变更）----
        let port = match self.port_provider.current_port() {
            Some(p) => p,
            None => return Err(KcCallError::Unreachable),
        };

        // ---- 步骤 4：构造请求 ----
        let url = format!("http://127.0.0.1:{port}/api/v1/ingest");
        let request_id = uuid::Uuid::new_v4().to_string();
        // body 字段名严格 snake_case，与 KC `IngestRequest` Pydantic 模型对齐。
        let body = serde_json::json!({
            "markdown_text": markdown,
            "persist": options.persist,
            "use_ai": options.use_ai,
            "enable_qa": options.enable_qa,
            "enable_links": options.enable_links,
        });

        // ---- 步骤 5：发送 + 计时 ----
        let start = std::time::Instant::now();
        let response_result = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Request-Id", &request_id)
            .json(&body)
            .send()
            .await;

        // ---- 步骤 6：响应分类 ----
        let response = match response_result {
            Ok(r) => r,
            Err(e) => return Err(classify_reqwest_error(e)),
        };

        let status = response.status();
        // 一次性消费 body bytes（后续 JSON 解析失败时也能 fall back 到 `Malformed { reason }`）。
        // 同时 `bytes.len()` 用于 meta.response_size_bytes 注入。
        let body_bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => return Err(classify_reqwest_error(e)),
        };
        let response_size_bytes = body_bytes.len();
        let duration_ms = start.elapsed().as_millis() as u64;

        if status.is_success() {
            // ---- 200 路径：解析 enhanced_markdown，缺失 → Malformed ----
            parse_success_body(&body_bytes, response_size_bytes, duration_ms)
        } else if status.as_u16() == 500 {
            // ---- 500 路径：按 detail.error_code 分类 ----
            Err(classify_500_body(&body_bytes))
        } else {
            // ---- 其他状态码：当作 Internal（KC 不预期返非 200/500）----
            Err(KcCallError::Internal {
                detail: format!("unexpected status {}", status.as_u16()),
                code: "UNKNOWN".into(),
            })
        }
    }
}

// =====================================================================
// 5. 错误分类辅助
// =====================================================================

/// 把 `reqwest::Error` 分类成 `KcCallError`（AC-4 reqwest 错误侧）。
///
/// **分类优先级**（从特定到一般）：
/// 1. `is_timeout()` —— 客户端 60s 超时；
/// 2. `is_connect()` —— TCP connect 失败 / DNS 失败 / 端口拒绝；
/// 3. 其他 —— 当作 `Internal { code: "REQWEST_ERROR" }`（不应当频繁发生，主要是 hyper 内部错）。
///
/// **不把 `is_request()` / `is_body()` 单独分类**：这些大多是 client builder 阶段的配置问题，
/// 在 `KcClient::new` 阶段已经 `expect()` 兜底；运行时出现属于 NC bug，归到 `Internal` 即可。
fn classify_reqwest_error(e: reqwest::Error) -> KcCallError {
    if e.is_timeout() {
        return KcCallError::Timeout;
    }
    if e.is_connect() {
        return KcCallError::Unreachable;
    }
    // `is_request` 在某些 OS 层连接拒绝时也会触发（如 ECONNREFUSED）。
    // 用字符串特征兜底（reqwest 0.12 没暴露细分 API）。
    let msg = e.to_string();
    if msg.contains("Connection refused")
        || msg.contains("connection refused")
        || msg.contains("dns error")
        || msg.contains("os error 61") // ECONNREFUSED on macOS
        || msg.contains("os error 111") // ECONNREFUSED on Linux
    {
        return KcCallError::Unreachable;
    }
    KcCallError::Internal {
        detail: mask_secrets(&msg),
        code: "REQWEST_ERROR".into(),
    }
}

/// 解析 200 响应 body 为 `KcIngestOutcome::Success` 或 `KcCallError::Malformed`（AC-4 200 路径）。
///
/// **必含字段**（KC-MOD-1 后保证；缺失任一即 `Malformed`）：
/// - `enhanced_markdown` —— 非空字符串；
/// - 其他 meta 字段（`doc_id` / `kc_version` / `generated_at` / `paragraph_count`）—— 缺失时走 default
///   而非 Malformed（防御性容错，KC 端字段可能渐进增加；`enhanced_markdown` 才是核心契约）。
///
/// **meta.tags_source 判定逻辑**：
/// - KC 返回 `ai_tags` 非空 → `AiAndRule`（KC 正常成功路径）；
/// - `ai_tags` 为空 + `rule_tags` 非空 → `RuleOnly`（理论上 200 路径不该出现，但容错）；
/// - 都为空 → `AiAndRule`（默认；空数组也算"AI 增强成功，只是没标签"）。
fn parse_success_body(
    body_bytes: &[u8],
    response_size_bytes: usize,
    duration_ms: u64,
) -> Result<KcIngestOutcome, KcCallError> {
    // 用 Value 解析而非强类型 struct——KC-MOD-1 字段集可能渐进扩展，
    // 强类型在新字段加入时会需要同步改 struct 定义，Value 更宽容。
    let json: Value = match serde_json::from_slice(body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return Err(KcCallError::Malformed {
                reason: format!("body not valid JSON: {e}"),
            });
        }
    };

    // `enhanced_markdown` 是核心契约（KC-MOD-1）。
    let enhanced_md = match json.get("enhanced_markdown").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return Err(KcCallError::Malformed {
                reason: "missing or empty `enhanced_markdown`".into(),
            });
        }
    };

    // 其他 meta 字段：缺失 → default（容错，不影响主路径）。
    let doc_id = json
        .get("doc_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kc_version = json
        .get("kc_version")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let generated_at = json
        .get("kc_generated_at")
        .or_else(|| json.get("generated_at"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let paragraph_count = json
        .get("paragraph_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    let ai_tags = json
        .get("ai_tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let rule_tags = json
        .get("rule_tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ai_summary = json
        .get("ai_summary")
        .and_then(Value::as_str)
        .map(String::from);

    // ai_qa_pairs：数组 of {question, answer}
    let ai_qa_pairs = json
        .get("ai_qa_pairs")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let q = v.get("question").and_then(Value::as_str)?.to_string();
                    let a = v.get("answer").and_then(Value::as_str)?.to_string();
                    Some(KcQaPair {
                        question: q,
                        answer: a,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // ai_paragraph_links：数组 of {paragraph_id, related_text}
    let ai_paragraph_links = json
        .get("ai_paragraph_links")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let pid = v.get("paragraph_id").and_then(Value::as_str)?.to_string();
                    let rel = v.get("related_text").and_then(Value::as_str)?.to_string();
                    Some(KcParagraphLink {
                        paragraph_id: pid,
                        related_text: rel,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // tags_source 判定（200 路径默认 AI+rule；空数组也是 AI 增强）。
    let tags_source = KcTagsSource::AiAndRule;

    let meta = KcMeta {
        doc_id,
        kc_version,
        tags_source,
        ai_tags,
        rule_tags,
        ai_summary,
        ai_qa_pairs,
        ai_paragraph_links,
        generated_at,
        paragraph_count,
        response_size_bytes,
        duration_ms,
    };

    Ok(KcIngestOutcome::Success {
        enhanced_md,
        meta,
    })
}

/// 解析 500 响应 body 为 `KcCallError` 变体（AC-4 500 路径）。
///
/// **6 种 KC 错误码映射**：
/// - `KC_LLM_UNAVAILABLE` → `LlmUnavailable { partial_md: detail.partial_enhanced_markdown }`；
/// - `KC_INTERNAL` / `KC_PARSE_ERROR` / `KC_OUTPUT_ERROR` → `Internal { detail, code }`；
/// - `KC_INPUT_TOO_LARGE` → `InputTooLarge`；
/// - 其他 / body 不可解析 → `Internal { code: "UNKNOWN" }`（容错，避免 KC 新增错误码时漏分类）。
///
/// **detail mask**：透传到 `Internal.detail` 的字符串经过 [`mask_secrets`] 处理，
/// 防止 KC 端日志含 Key（即便理论上不会，也要兜底）传染到 NC 日志。
fn classify_500_body(body_bytes: &[u8]) -> KcCallError {
    let json: Value = match serde_json::from_slice(body_bytes) {
        Ok(v) => v,
        Err(_) => {
            // 500 + body 不可解析：归到 Internal/UNKNOWN（带原始 body 前 200 字节作 debug）。
            let preview = String::from_utf8_lossy(&body_bytes[..body_bytes.len().min(200)]);
            return KcCallError::Internal {
                detail: mask_secrets(&format!("500 with non-JSON body: {preview}")),
                code: "UNKNOWN".into(),
            };
        }
    };

    let error_code = json
        .get("detail")
        .and_then(|d| d.get("error_code"))
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN");

    let message = json
        .get("detail")
        .and_then(|d| d.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    match error_code {
        "KC_LLM_UNAVAILABLE" => {
            let partial_md = json
                .get("detail")
                .and_then(|d| d.get("partial_enhanced_markdown"))
                .and_then(Value::as_str)
                .map(String::from);
            KcCallError::LlmUnavailable { partial_md }
        }
        "KC_INPUT_TOO_LARGE" => KcCallError::InputTooLarge,
        "KC_INTERNAL" | "KC_PARSE_ERROR" | "KC_OUTPUT_ERROR" => KcCallError::Internal {
            detail: mask_secrets(&message),
            code: error_code.to_string(),
        },
        // 未知 KC 错误码：当作 Internal 但 code 保留 "UNKNOWN"
        // （日志能看到原 code，便于排查 KC 端新增错误码漏配）。
        _ => KcCallError::Internal {
            detail: mask_secrets(&format!("unknown error_code={error_code}: {message}")),
            code: "UNKNOWN".into(),
        },
    }
}

/// 屏蔽错误信息中可能含的 secret（API Key 等）。
///
/// **触发条件**（保守策略，宁可多 mask 不少 mask）：
/// - `sk-XXX` 形式（OpenAI Key 前缀）；
/// - `zhipu-XXX` / `glm-XXX` 形式（智谱 Key 常见前缀）；
/// - `api_key=XXX` / `api_key:"XXX"` / `apikey=XXX` 形式（query / config 透出）；
/// - `Bearer XXX` / `bearer XXX` 形式（Authorization header）。
///
/// **替换为 `<redacted>`**——保留信息结构（前后文）但清零 Key 内容。
///
/// **不做的**：不做完整的正则替换（performance + 复杂度），只用简单的子串前缀匹配。
fn mask_secrets(s: &str) -> String {
    // 用一个简单的逐字符状态机，性能足够（错误路径，每次 ingest 最多调一次）。
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();

    // 已知 secret 前缀列表（小写匹配）。
    // (prefix, prefix_len, redacted_repr)
    const PREFIXES: &[(&[u8], &str)] = &[
        (b"sk-", "sk-<redacted>"),
        (b"zhipu-", "zhipu-<redacted>"),
        (b"glm-", "glm-<redacted>"),
        (b"Bearer ", "Bearer <redacted>"),
        (b"bearer ", "bearer <redacted>"),
        (b"api_key=", "api_key=<redacted>"),
        (b"api_key:", "api_key:<redacted>"),
        (b"apikey=", "apikey=<redacted>"),
        (b"apikey:", "apikey:<redacted>"),
        (b"ZHIPUAI_API_KEY=", "ZHIPUAI_API_KEY=<redacted>"),
        (b"OPENAI_API_KEY=", "OPENAI_API_KEY=<redacted>"),
    ];

    while i < bytes.len() {
        let mut matched = false;
        for (prefix, repr) in PREFIXES {
            if bytes[i..].starts_with(prefix) {
                result.push_str(repr);
                // 跳过 prefix + 后续非空白字符（key 主体直到下一个空白 / 引号 / 逗号）。
                let mut j = i + prefix.len();
                while j < bytes.len() {
                    let c = bytes[j];
                    if c.is_ascii_whitespace() || c == b'"' || c == b'\'' || c == b',' || c == b';'
                    {
                        break;
                    }
                    j += 1;
                }
                i = j;
                matched = true;
                break;
            }
        }
        if !matched {
            // 复制单字节（UTF-8 多字节字符也按字节走，因为前缀全是 ASCII，不会切到字符中间）。
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

// =====================================================================
// 6. 单元测试（lib 内）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU16, Ordering};

    /// 单元测试用 PortProvider stub：固定端口 / 可置 None。
    struct StaticPortProvider {
        /// 0 表示 None（端口 0 不合法做约定值）。
        port: AtomicU16,
    }

    impl StaticPortProvider {
        fn with_port(port: u16) -> Arc<Self> {
            Arc::new(Self {
                port: AtomicU16::new(port),
            })
        }
        fn none() -> Arc<Self> {
            Arc::new(Self {
                port: AtomicU16::new(0),
            })
        }
    }

    impl PortProvider for StaticPortProvider {
        fn current_port(&self) -> Option<u16> {
            match self.port.load(Ordering::Acquire) {
                0 => None,
                p => Some(p),
            }
        }
    }

    // ---- AC-5：1MB 输入预检（不打 mock，纯客户端层）----

    /// 998K 输入（< 1MB）通过预检（仍会因端口不通失败，但失败码不是 InputTooLarge）。
    #[tokio::test]
    async fn ingest_998k_below_threshold_does_not_reject_at_size_check() {
        let provider = StaticPortProvider::none(); // 端口 None 触发 Unreachable
        let client = KcClient::new(provider);
        let markdown = "x".repeat(998 * 1024); // 998KB
        let options = KcIngestOptions::default();

        let err = client
            .ingest_text(&markdown, &options)
            .await
            .expect_err("端口 None 应当返 Err");
        // 998K 通过 size 预检，会进 acquire/port 流程，因 port=None 抛 Unreachable。
        assert!(
            matches!(err, KcCallError::Unreachable),
            "998K (< 1MB) 应通过 size 预检；实际：{err:?}"
        );
    }

    /// 正好 1MB（== 1_048_576 字节）通过预检。
    #[tokio::test]
    async fn ingest_exactly_1mb_passes_size_check() {
        let provider = StaticPortProvider::none();
        let client = KcClient::new(provider);
        let markdown = "x".repeat(1_048_576);
        let options = KcIngestOptions::default();

        let err = client
            .ingest_text(&markdown, &options)
            .await
            .expect_err("端口 None 应当返 Err");
        assert!(
            matches!(err, KcCallError::Unreachable),
            "1MB == 1_048_576 应通过 size 预检；实际：{err:?}"
        );
    }

    /// 1.1MB（> 1_048_576 字节）被 size 预检拒绝。
    #[tokio::test]
    async fn ingest_1_1mb_rejected_by_size_check() {
        let provider = StaticPortProvider::with_port(12345); // 即使端口合法，size 预检也应先拒绝
        let client = KcClient::new(provider);
        let markdown = "x".repeat(1_148_000); // ~1.1MB
        let options = KcIngestOptions::default();

        let err = client
            .ingest_text(&markdown, &options)
            .await
            .expect_err("> 1MB 应当被拒绝");
        assert!(
            matches!(err, KcCallError::InputTooLarge),
            "> 1MB 应当返 InputTooLarge；实际：{err:?}"
        );
    }

    // ---- AC-3 步骤 3：端口 None → Unreachable（不发请求）----

    #[tokio::test]
    async fn ingest_when_port_is_none_returns_unreachable() {
        let provider = StaticPortProvider::none();
        let client = KcClient::new(provider);
        let err = client
            .ingest_text("# hi", &KcIngestOptions::default())
            .await
            .expect_err("端口 None 应当返 Err");
        assert!(matches!(err, KcCallError::Unreachable));
    }

    // ---- mask_secrets 单测 ----

    #[test]
    fn mask_secrets_redacts_openai_key() {
        let msg = "Error calling sk-abc1234567890XYZ during ingest";
        let masked = mask_secrets(msg);
        assert!(!masked.contains("abc1234567890XYZ"), "key 内容必须被屏蔽");
        assert!(masked.contains("sk-<redacted>"), "应保留 sk- 前缀提示");
    }

    #[test]
    fn mask_secrets_redacts_zhipu_key() {
        let msg = "ZHIPUAI_API_KEY=zhipu-SUPERSECRET99 in env";
        let masked = mask_secrets(msg);
        assert!(!masked.contains("SUPERSECRET99"), "key 内容必须被屏蔽");
        assert!(masked.contains("ZHIPUAI_API_KEY=<redacted>"));
    }

    #[test]
    fn mask_secrets_redacts_bearer_token() {
        let msg = "Authorization: Bearer eyJhbGciOiJIUzI1Ni-secret-stuff";
        let masked = mask_secrets(msg);
        assert!(!masked.contains("eyJhbGciOiJIUzI1Ni-secret-stuff"));
        assert!(masked.contains("Bearer <redacted>"));
    }

    #[test]
    fn mask_secrets_preserves_non_secret_text() {
        let msg = "KC internal error: parse failure on line 3";
        assert_eq!(mask_secrets(msg), msg, "非 secret 文本必须原样保留");
    }

    // ---- KcIngestOptions 默认值（persist=false 不变量）----

    #[test]
    fn ingest_options_default_persist_is_false() {
        let opts = KcIngestOptions::default();
        assert!(!opts.persist, "persist 默认必须 false（ADR-006 层 1）");
        assert!(opts.use_ai);
        assert!(opts.enable_qa);
        assert!(opts.enable_links);
    }

    // ---- classify_500_body 直接测试（不走 HTTP，纯函数）----

    #[test]
    fn classify_500_kc_llm_unavailable_with_partial() {
        let body = br##"{"detail":{"error_code":"KC_LLM_UNAVAILABLE","message":"zhipu down","retryable":true,"partial_enhanced_markdown":"# partial"}}"##;
        let err = classify_500_body(body);
        match err {
            KcCallError::LlmUnavailable { partial_md } => {
                assert_eq!(partial_md.as_deref(), Some("# partial"));
            }
            other => panic!("expected LlmUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_kc_llm_unavailable_without_partial() {
        let body = br#"{"detail":{"error_code":"KC_LLM_UNAVAILABLE","message":"zhipu down","retryable":true}}"#;
        let err = classify_500_body(body);
        match err {
            KcCallError::LlmUnavailable { partial_md } => {
                assert!(partial_md.is_none(), "缺 partial_md 字段应为 None");
            }
            other => panic!("expected LlmUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_kc_internal_extracts_code() {
        let body = br#"{"detail":{"error_code":"KC_INTERNAL","message":"oops"}}"#;
        let err = classify_500_body(body);
        match err {
            KcCallError::Internal { detail, code } => {
                assert_eq!(code, "KC_INTERNAL");
                assert!(detail.contains("oops"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_kc_parse_error_maps_to_internal() {
        let body = br#"{"detail":{"error_code":"KC_PARSE_ERROR","message":"bad md"}}"#;
        let err = classify_500_body(body);
        match err {
            KcCallError::Internal { code, .. } => {
                assert_eq!(code, "KC_PARSE_ERROR");
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_kc_input_too_large() {
        let body = br#"{"detail":{"error_code":"KC_INPUT_TOO_LARGE","message":"too big"}}"#;
        let err = classify_500_body(body);
        assert!(matches!(err, KcCallError::InputTooLarge));
    }

    #[test]
    fn classify_500_unknown_code_falls_back_to_internal_unknown() {
        let body = br#"{"detail":{"error_code":"KC_NEW_ERROR_2027","message":"future"}}"#;
        let err = classify_500_body(body);
        match err {
            KcCallError::Internal { code, .. } => {
                assert_eq!(code, "UNKNOWN");
            }
            other => panic!("expected Internal/UNKNOWN, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_non_json_body_falls_back_to_internal_unknown() {
        let body = b"<html>500 Internal Server Error</html>";
        let err = classify_500_body(body);
        match err {
            KcCallError::Internal { code, .. } => {
                assert_eq!(code, "UNKNOWN");
            }
            other => panic!("expected Internal/UNKNOWN, got {other:?}"),
        }
    }

    #[test]
    fn classify_500_internal_message_masks_secrets() {
        // 防御性测试：即便 KC 端不应回传 key，detail 透传前必须 mask（ADR-007 §"日志屏蔽"）。
        let body = br#"{"detail":{"error_code":"KC_INTERNAL","message":"Auth failed for sk-leakyleakySECRET"}}"#;
        let err = classify_500_body(body);
        match err {
            KcCallError::Internal { detail, .. } => {
                assert!(!detail.contains("leakySECRET"), "detail 必须 mask 掉 key 内容");
                assert!(detail.contains("sk-<redacted>"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    // ---- parse_success_body 直接测试 ----

    #[test]
    fn parse_success_body_full_meta() {
        let body = br##"{
            "success": true,
            "doc_id": "doc-abc123",
            "title": "Test Doc",
            "enhanced_markdown": "# Enhanced\n\n#AI #ML",
            "ai_tags": ["AI", "ML"],
            "rule_tags": ["test"],
            "ai_summary": "summary",
            "ai_qa_pairs": [{"question":"Q1","answer":"A1"}],
            "ai_paragraph_links": [{"paragraph_id":"paragraph-0","related_text":"rel"}],
            "paragraph_count": 3,
            "kc_version": "0.9",
            "kc_generated_at": "2026-05-27T00:00:00Z"
        }"##;
        let outcome = parse_success_body(body, body.len(), 123)
            .expect("应当解析成功");
        match outcome {
            KcIngestOutcome::Success { enhanced_md, meta } => {
                assert_eq!(enhanced_md, "# Enhanced\n\n#AI #ML");
                assert_eq!(meta.doc_id, "doc-abc123");
                assert_eq!(meta.kc_version, "0.9");
                assert_eq!(meta.paragraph_count, 3);
                assert_eq!(meta.ai_tags, vec!["AI", "ML"]);
                assert_eq!(meta.rule_tags, vec!["test"]);
                assert_eq!(meta.ai_summary.as_deref(), Some("summary"));
                assert_eq!(meta.ai_qa_pairs.len(), 1);
                assert_eq!(meta.ai_qa_pairs[0].question, "Q1");
                assert_eq!(meta.ai_paragraph_links.len(), 1);
                assert_eq!(meta.ai_paragraph_links[0].paragraph_id, "paragraph-0");
                assert_eq!(meta.duration_ms, 123);
                assert_eq!(meta.response_size_bytes, body.len());
                assert_eq!(meta.tags_source, KcTagsSource::AiAndRule);
            }
        }
    }

    #[test]
    fn parse_success_body_missing_enhanced_md_returns_malformed() {
        let body = br#"{"success": true, "doc_id": "doc-x"}"#;
        let err = parse_success_body(body, body.len(), 0).expect_err("应当 Malformed");
        match err {
            KcCallError::Malformed { reason } => {
                assert!(reason.contains("enhanced_markdown"));
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn parse_success_body_empty_enhanced_md_returns_malformed() {
        let body = br#"{"success": true, "enhanced_markdown": ""}"#;
        let err = parse_success_body(body, body.len(), 0).expect_err("空 enhanced_md 应当 Malformed");
        assert!(matches!(err, KcCallError::Malformed { .. }));
    }

    #[test]
    fn parse_success_body_non_json_returns_malformed() {
        let body = b"not json at all";
        let err = parse_success_body(body, body.len(), 0).expect_err("非 JSON 应当 Malformed");
        match err {
            KcCallError::Malformed { reason } => {
                assert!(reason.contains("JSON"));
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn parse_success_body_missing_optional_meta_uses_defaults() {
        // 只有 enhanced_markdown，meta 其他字段全缺 → 走默认。
        let body = br##"{"enhanced_markdown": "# x"}"##;
        let outcome = parse_success_body(body, body.len(), 0).expect("应当解析成功");
        match outcome {
            KcIngestOutcome::Success { meta, .. } => {
                assert_eq!(meta.doc_id, "");
                assert_eq!(meta.kc_version, "unknown");
                assert_eq!(meta.paragraph_count, 0);
                assert!(meta.ai_tags.is_empty());
                assert!(meta.ai_summary.is_none());
            }
        }
    }
}
