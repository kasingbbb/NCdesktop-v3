//! task_006_mock_kc_server：`MockKcServer` — KC HTTP API 的 wiremock 模拟器。
//!
//! ## 用途
//!
//! 为 task_007（`KcClient` HTTP 客户端）、task_008（`KcProcessManager` 生命周期）、
//! task_011（`enrichment::enrich`）、task_022（失败注入测试）、task_023（e2e 集成测试）
//! 提供 **不依赖真实 KC 子进程 / 真实 LLM Key / 真实网络** 的本地 mock 端点。
//!
//! ## 设计依据
//!
//! - **ADR-011**（Architect output.md §"ADR-011 Mock KC server"）：选 `wiremock` 而非 `httpmock`，
//!   因为 wiremock async-first，与 NC 的 reqwest 0.12 async 客户端契合更好；
//! - **ADR-002 §"NC 调 KC 的 HTTP 调用合约"**：响应体 JSON schema 严格遵守；
//! - **KC-MOD-1/2/3**（`kc_api_integration_proposal.md` §三）：success 响应含 `enhanced_markdown`，
//!   失败响应含结构化 `detail.error_code`；
//! - **`extraction::failure_code::FailureCode::EKc*`**（task_003）：mock 的失败 scenario 与
//!   `KcCallError` → `FailureCode` 映射（`kc::errors`，task_005）严格对齐。
//!
//! ## 6 + 1 个 scenario
//!
//! | 方法 | KC 端点 | HTTP 状态 | 期望 NC `KcCallError` 变体 |
//! |--|--|--|--|
//! | `start_with_health_only()`       | `GET /api/v1/health`  | 200          | （仅健康检查，不触发 ingest） |
//! | `start_with_success(md, meta)`   | `POST /api/v1/ingest` | 200          | `Ok(IngestOutcome::Success)` |
//! | `start_with_unavailable()`       | （监听后立即 stop）   | （连接拒绝）  | `KcCallError::Unreachable` |
//! | `start_with_timeout(delay)`      | `POST /api/v1/ingest` | 200（延迟）  | `KcCallError::Timeout`（客户端超时） |
//! | `start_with_internal_error()`    | `POST /api/v1/ingest` | 500          | `KcCallError::Internal { code: "KC_INTERNAL" }` |
//! | `start_with_llm_unavailable(md)` | `POST /api/v1/ingest` | 500          | `KcCallError::LlmUnavailable { partial_md }` |
//! | `start_with_input_too_large()`   | `POST /api/v1/ingest` | 500          | `KcCallError::InputTooLarge` |
//!
//! ## `KC_USE_MOCK_PORT` 环境变量约定（AC-6）
//!
//! task_008 实装 `KcProcessManager` 时，启动逻辑必须支持以下分支：
//!
//! ```ignore
//! // src-tauri/src/kc/process.rs (task_008 实装时)
//! pub async fn start(&self) -> Result<(), KcStartError> {
//!     if let Ok(mock_port) = std::env::var("KC_USE_MOCK_PORT") {
//!         // 测试模式：跳过真 KC 启动，直接复用 mock 端口
//!         let port: u16 = mock_port.parse().expect("KC_USE_MOCK_PORT must be u16");
//!         self.set_port_and_status(port, KcStatus::Ready);
//!         return Ok(());
//!     }
//!     // 正常路径：拉起 python run_api.py 子进程
//!     ...
//! }
//! ```
//!
//! 测试代码示例：
//!
//! ```ignore
//! let mock = MockKcServer::start_with_success("...", meta).await;
//! std::env::set_var("KC_USE_MOCK_PORT", mock.port().to_string());
//! let manager = KcProcessManager::new(&app);
//! manager.start().await?; // 跳过真实子进程，使用 mock 端口
//! // ...
//! std::env::remove_var("KC_USE_MOCK_PORT");
//! mock.stop();
//! ```
//!
//! ## Lifecycle
//!
//! `MockKcServer` 持有 `wiremock::MockServer`，drop 时自动停止（wiremock 内部 join handle 会被 abort）。
//! 显式调用 `stop()` 只是为了语义清晰，效果与 drop 等价。
//!
//! ## 端口选择
//!
//! `wiremock::MockServer::start()` 默认绑 `127.0.0.1:0`（动态空闲端口），无端口冲突风险。

use std::net::SocketAddr;
use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =====================================================================
// 公共类型：与 `kc::errors::KcMeta` 字段对齐的 mock 元数据
// =====================================================================

/// Mock 用 KC 元数据（简化版，仅含 `start_with_success` 必需字段）。
///
/// **不**直接复用 `app_lib::kc::KcMeta`：因 `KcMeta` 是 lib 内部类型，integration test crate
/// 默认不能引用 lib 私有类型；本 mock 只关心 HTTP 响应体的"形状"，无需领域模型语义。
/// 调用方可用 `KcMockMeta::default()` 取一份合规默认值。
#[derive(Debug, Clone)]
pub struct KcMockMeta {
    pub doc_id: String,
    pub title: String,
    pub kc_version: String,
    pub ai_tags: Vec<String>,
    pub rule_tags: Vec<String>,
    pub ai_summary: Option<String>,
    pub paragraph_count: u32,
    pub annotation_count: u32,
    pub original_length: u32,
    pub enhanced_length: u32,
    pub extra_ratio: f64,
    pub generated_at: String,
}

impl Default for KcMockMeta {
    fn default() -> Self {
        Self {
            doc_id: "doc-mocktest".to_string(),
            title: "Mock 测试文档".to_string(),
            kc_version: "0.9".to_string(),
            ai_tags: vec!["AI".to_string(), "Mock".to_string()],
            rule_tags: vec!["test".to_string()],
            ai_summary: Some("Mock 测试摘要".to_string()),
            paragraph_count: 3,
            annotation_count: 0,
            original_length: 100,
            enhanced_length: 200,
            extra_ratio: 1.0,
            generated_at: "2026-05-27T00:00:00Z".to_string(),
        }
    }
}

// =====================================================================
// MockKcServer 主类型
// =====================================================================

/// KC HTTP API 的 wiremock 模拟器。
///
/// 详见模块文档 §"6 + 1 个 scenario"。
pub struct MockKcServer {
    /// 服务器监听的本地地址（含端口）。
    pub addr: SocketAddr,
    /// wiremock 实例（drop 时自动停止）。
    server: MockServer,
}

impl MockKcServer {
    /// 获取本 mock server 监听的端口（用于 `KC_USE_MOCK_PORT` 注入）。
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// 显式停止 mock server（等价于 drop self；为可读性保留）。
    pub fn stop(self) {
        // wiremock::MockServer 在 drop 时会 abort 内部 tokio task
        drop(self.server);
    }

    /// 获取 base URL（`http://127.0.0.1:<port>`），供调用方拼接路径。
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    // -----------------------------------------------------------------
    // Scenario 1：仅健康检查（不响应 ingest）
    // -----------------------------------------------------------------

    /// **Scenario 1**：`GET /api/v1/health` 返回 200 + 完整 `HealthResponse`。
    ///
    /// 用途：task_008 `KcProcessManager` 健康检查测试；task_023 e2e 启动阶段验证。
    /// 注意：本 scenario **不** 配置 `/ingest` 端点，调 ingest 会返 wiremock 默认 404。
    pub async fn start_with_health_only() -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;
        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // Scenario 2：成功响应（含 enhanced_markdown，KC-MOD-1 后形态）
    // -----------------------------------------------------------------

    /// **Scenario 2**：`POST /api/v1/ingest` 返回 200 + 完整 `IngestResponse`（含 `enhanced_markdown`）。
    ///
    /// 响应严格遵守 ADR-002 §"成功响应"schema：
    /// ```json
    /// {
    ///   "success": true,
    ///   "doc_id": "doc-...",
    ///   "title": "...",
    ///   "enhanced_markdown": "<KC-MOD-1 后必含>",
    ///   "ai_tags": [...],
    ///   "ai_summary": "...",
    ///   "ai_qa_pairs": [],
    ///   "ai_paragraph_links": [],
    ///   "paragraph_count": N,
    ///   "annotation_count": 0,
    ///   "original_length": N,
    ///   "enhanced_length": M,
    ///   "extra_ratio": 0.21,
    ///   "kc_version": "0.9",
    ///   "kc_generated_at": "...",
    ///   "rule_tags": [...]
    /// }
    /// ```
    ///
    /// 期望 NC 一侧（task_007 客户端）解析为 `Ok(IngestOutcome::Success { ... })`。
    pub async fn start_with_success(enhanced_md: &str, meta: KcMockMeta) -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;

        let response_body = json!({
            "success": true,
            "doc_id": meta.doc_id,
            "title": meta.title,
            "enhanced_markdown": enhanced_md,
            "enhanced_path": null,
            "index_path": null,
            "index_markdown": null,
            "ai_tags": meta.ai_tags,
            "rule_tags": meta.rule_tags,
            "ai_summary": meta.ai_summary,
            "ai_qa_pairs": [],
            "ai_paragraph_links": [],
            "paragraph_count": meta.paragraph_count,
            "annotation_count": meta.annotation_count,
            "original_length": meta.original_length,
            "enhanced_length": meta.enhanced_length,
            "extra_ratio": meta.extra_ratio,
            "kc_version": meta.kc_version,
            "kc_generated_at": meta.generated_at,
            "message": "文档摄入成功"
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/ingest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // Scenario 3：不可达（监听后立即停止，连接被拒）
    // -----------------------------------------------------------------

    /// **Scenario 3**：返回一个指向**无人监听端口**的 mock server。
    ///
    /// 实现方式：用 `std::net::TcpListener::bind("127.0.0.1:0")` 让 OS 分配一个空闲端口，
    /// 立即 `drop(listener)` 释放回端口池；保留该端口号到 `addr`。绝大多数 OS 不会立即
    /// 把刚释放的端口分配给后续 `MockServer::start()`（端口池随机选择），
    /// 因此调用方对 `addr` 发 HTTP 请求会得到 `connection refused`，
    /// NC `KcClient` 应映射为 `KcCallError::Unreachable`（ADR-002 §"错误分类"）。
    ///
    /// 注意：返回的 `MockKcServer.server` 是新起的"空 server"占位（不挂载任何 mock），
    /// 仅持有以满足 struct 完整性。占位 server 的端口与 `addr` **不同**（wiremock
    /// 自动选另一个空闲端口），调用方不应对占位 server 发请求。
    pub async fn start_with_unavailable() -> Self {
        // 通过 std::net::TcpListener 拿一个真正无人监听的端口：
        // 1. bind("127.0.0.1:0") 让 OS 分配空闲端口；
        // 2. local_addr() 取实际地址；
        // 3. drop(listener) 释放，端口回到空闲池（短时间内无人监听）。
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind 127.0.0.1:0 for unavailable scenario");
        let addr = listener
            .local_addr()
            .expect("local_addr of bound listener");
        drop(listener); // 释放端口

        // 占位 server（用一个空 MockServer 满足 struct 字段；其端口与 addr 不同）。
        let placeholder = MockServer::start().await;
        Self {
            addr, // 指向无人监听的端口
            server: placeholder,
        }
    }

    // -----------------------------------------------------------------
    // Scenario 4：超时（响应延迟超过客户端 timeout）
    // -----------------------------------------------------------------

    /// **Scenario 4**：`POST /api/v1/ingest` 响应**延迟 `delay`**（默认 200 即可触发短超时测试）。
    ///
    /// wiremock `ResponseTemplate::set_delay(delay)` 让 server 在返回前 sleep `delay` 时长。
    /// 调用方应**设置 reqwest client 总超时短于 `delay`**（如 `client_timeout < delay`）以触发
    /// `reqwest::Error::is_timeout() == true`，NC 一侧映射为 `KcCallError::Timeout`。
    ///
    /// **注意**：生产 client 默认 60s 超时，测试时务必用更短的客户端超时（task_022 推荐 100ms client + 500ms delay）。
    pub async fn start_with_timeout(delay: Duration) -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;

        // 配置 ingest 端点延迟响应（仍返 200 + 空 body，因为客户端预期会先超时）
        Mock::given(method("POST"))
            .and(path("/api/v1/ingest"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(delay)
                    .set_body_json(json!({"success": true, "doc_id": "doc-delayed"})),
            )
            .mount(&server)
            .await;

        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // Scenario 5：500 + KC_INTERNAL（KC 内部错误，KC-MOD-3 后结构化）
    // -----------------------------------------------------------------

    /// **Scenario 5**：`POST /api/v1/ingest` 返回 500 + `{detail: {error_code: "KC_INTERNAL"}}`。
    ///
    /// 响应严格遵守 ADR-002 §"失败响应"schema + KC-MOD-3 结构化错误码：
    /// ```json
    /// {
    ///   "detail": {
    ///     "error_code": "KC_INTERNAL",
    ///     "message": "...",
    ///     "retryable": false
    ///   }
    /// }
    /// ```
    ///
    /// 期望 NC 一侧（task_007 客户端）映射为 `KcCallError::Internal { code: "KC_INTERNAL", .. }`，
    /// 进而触发 `KcFallbackReason::InternalError(_)` → `FailureCode::EKcEnrichFailed`。
    pub async fn start_with_internal_error() -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;

        let response_body = json!({
            "detail": {
                "error_code": "KC_INTERNAL",
                "message": "KC 内部异常（mock）",
                "retryable": false
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/ingest"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response_body))
            .mount(&server)
            .await;

        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // Scenario 6：500 + KC_LLM_UNAVAILABLE（LLM 不可达，类型 C 带 partial_md）
    // -----------------------------------------------------------------

    /// **Scenario 6**：`POST /api/v1/ingest` 返回 500 + `{detail: {error_code: "KC_LLM_UNAVAILABLE", partial_enhanced_markdown: ...}}`。
    ///
    /// 响应包含 KC-MOD-3 "类型 C" `partial_enhanced_markdown`（规则标签 + 锚点 + 索引段，无 AI 增强）：
    /// ```json
    /// {
    ///   "detail": {
    ///     "error_code": "KC_LLM_UNAVAILABLE",
    ///     "message": "智谱 AI 连接失败",
    ///     "retryable": true,
    ///     "partial_enhanced_markdown": "..."
    ///   }
    /// }
    /// ```
    ///
    /// 期望 NC 一侧映射为 `KcCallError::LlmUnavailable { partial_md: Some(partial_md.into()) }`，
    /// 进而触发 `KcEnrichmentOutcome::PartialLlmUnavailable` 落地"规则增强 MD"。
    pub async fn start_with_llm_unavailable(partial_md: &str) -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;

        let response_body = json!({
            "detail": {
                "error_code": "KC_LLM_UNAVAILABLE",
                "message": "智谱 AI 连接失败（mock）",
                "retryable": true,
                "partial_enhanced_markdown": partial_md
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/ingest"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response_body))
            .mount(&server)
            .await;

        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // Scenario 7：500 + KC_INPUT_TOO_LARGE
    // -----------------------------------------------------------------

    /// **Scenario 7**：`POST /api/v1/ingest` 返回 500 + `{detail: {error_code: "KC_INPUT_TOO_LARGE"}}`。
    ///
    /// 期望 NC 一侧映射为 `KcCallError::InputTooLarge` → `FailureCode::EKcInputTooLarge`。
    pub async fn start_with_input_too_large() -> Self {
        let server = MockServer::start().await;
        Self::mount_health_ok(&server).await;

        let response_body = json!({
            "detail": {
                "error_code": "KC_INPUT_TOO_LARGE",
                "message": "输入 markdown 超过 KC 内部限制（mock）",
                "retryable": false
            }
        });

        Mock::given(method("POST"))
            .and(path("/api/v1/ingest"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response_body))
            .mount(&server)
            .await;

        let addr = *server.address();
        Self { addr, server }
    }

    // -----------------------------------------------------------------
    // 内部辅助：挂载 `/api/v1/health` 200 响应（所有 scenario 共享）
    // -----------------------------------------------------------------

    /// 给定一个 wiremock server，挂载 `GET /api/v1/health` 200 响应。
    ///
    /// 响应体形态参考 KC `HealthResponse`（`kc_api_integration_proposal.md` §1.2 + KC-MOD-4 期望字段）：
    /// ```json
    /// {
    ///   "status": "ok",
    ///   "ai_enabled": true,
    ///   "llm_reachable": true,
    ///   "venv_ok": true,
    ///   "v1_ready": true,
    ///   "v2_ready": true,
    ///   "version": "0.9"
    /// }
    /// ```
    ///
    /// 所有 ingest scenario 都附带 health endpoint，因为：
    /// 1. task_008 `KcProcessManager::start()` 会先发 health check 才标记 Ready；
    /// 2. 若 mock 不提供 health，task_007/011 集成测试会卡在"等待 ready"阶段。
    async fn mount_health_ok(server: &MockServer) {
        let health_body = json!({
            "status": "ok",
            "ai_enabled": true,
            "llm_reachable": true,
            "venv_ok": true,
            "v1_ready": true,
            "v2_ready": true,
            "version": "0.9"
        });

        Mock::given(method("GET"))
            .and(path("/api/v1/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(health_body))
            .mount(server)
            .await;
    }
}
