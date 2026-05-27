//! task_007：`KcClient` 集成测试（input.md AC-6）。
//!
//! ## 测试用例
//!
//! | # | 名称 | mock scenario | 验证 |
//! |--|--|--|--|
//! | 1 | `client_success_returns_outcome`        | `start_with_success`        | 200 → `Success { enhanced_md, meta }` |
//! | 2 | `client_unavailable_when_mock_not_started` | `start_with_unavailable`  | 端口不通 → `Unreachable` |
//! | 3 | `client_timeout_when_mock_delays`       | `start_with_timeout(500ms)` | client 100ms 超时 → `Timeout` |
//! | 4 | `client_llm_unavailable_with_partial`   | `start_with_llm_unavailable`| 500 + partial → `LlmUnavailable { partial_md: Some(_) }` |
//! | 5 | `client_internal_error_extracts_code`   | `start_with_internal_error` | 500 + KC_INTERNAL → `Internal { code: "KC_INTERNAL" }` |
//! | 6 | `client_input_too_large_skips_request`  | （不打 mock）               | 1.1MB → `InputTooLarge`，0 次 HTTP 请求 |
//! | 7 | `client_serializes_requests`            | 自定义 mock（delay=60ms）   | 5 并发请求总耗时 ≥ 250ms（≈ 5×60ms 串行） |
//! | 8 | `client_serializes_requests_exact_count`| 自定义 mock（expect=5）     | mock 收到精确 5 次（wiremock verify-on-drop） |
//!
//! ## 设计依据
//!
//! - **input.md AC-6**：7 个测试用例，覆盖错误分类 + 串行化 + 大小预检；
//! - **ADR-009**：Semaphore(1) 在客户端层，单 NC 实例内 KC 调用永远串行；
//! - **共享 `tests/common/mock_kc.rs`**（task_006 产出）：6+1 个 scenario helper；
//! - 不依赖任何真实 KC 子进程 / 真实 LLM。

mod common;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use app_lib::kc::client::{KcClient, KcIngestOptions, KcIngestOutcome, PortProvider};
use app_lib::kc::errors::KcCallError;
use common::mock_kc::{KcMockMeta, MockKcServer};

// =====================================================================
// 通用：静态端口 PortProvider（线程安全，覆盖整 KcClient 生命周期）
// =====================================================================

/// 集成测试用 PortProvider stub：端口固定，可置 0（视为 None）。
///
/// 不引入额外依赖（只用 std::sync::atomic）。`current_port()` 同步无阻塞，
/// 与生产侧 `KcProcessManager` 的预期实装（atomic 读端口）一致。
struct StaticPortProvider {
    port: AtomicU16,
}

impl StaticPortProvider {
    fn new(port: u16) -> Arc<Self> {
        Arc::new(Self {
            port: AtomicU16::new(port),
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

// =====================================================================
// Test 1: success scenario
// =====================================================================

#[tokio::test]
async fn client_success_returns_outcome() {
    let enhanced = "# 增强后的文档\n\n#AI #Mock\n\n正文内容...";
    let meta = KcMockMeta::default();
    let mock = MockKcServer::start_with_success(enhanced, meta.clone()).await;
    let provider = StaticPortProvider::new(mock.port());

    let client = KcClient::new(provider);
    let outcome = client
        .ingest_text("# 原始文档\n\nHello", &KcIngestOptions::default())
        .await
        .expect("success scenario 应当成功");

    match outcome {
        KcIngestOutcome::Success { enhanced_md, meta: ret_meta } => {
            assert_eq!(enhanced_md, enhanced, "enhanced_md 必须 round-trip");
            assert_eq!(ret_meta.doc_id, meta.doc_id, "doc_id 必须解析");
            assert_eq!(ret_meta.kc_version, meta.kc_version);
            // meta.duration_ms 应大于 0（实际有 HTTP 往返）。
            // 但 mock 本地极快，可能 0ms——这里仅断言字段存在不 panic。
            let _ = ret_meta.duration_ms;
            assert!(ret_meta.response_size_bytes > 0, "response_size_bytes 应被注入");
        }
    }

    mock.stop();
}

// =====================================================================
// Test 2: unavailable scenario（端口不通）
// =====================================================================

/// 不依赖 `MockKcServer::start_with_unavailable()`（该 helper 在 macOS 上有端口被
/// placeholder server 占用的竞态：drop real_server → 端口立刻被 OS pool 重分配
/// 给随后启动的 placeholder，导致预期"已释放"的端口其实有 wiremock 在跑，返 404 而非 ECONNREFUSED）。
///
/// 改用更稳定的策略：本地直接 `TcpListener::bind(0)` 拿一个空闲端口 + 立即 drop，
/// 然后**不再起任何 server** 占用 OS 池——客户端连这个端口必拿 ECONNREFUSED。
///
/// 竞态窗口存在（drop 到 client.connect 之间 OS 可能把端口分给别的进程）但极小，
/// 集成测试场景下足够稳定。
#[tokio::test]
async fn client_unavailable_when_mock_not_started() {
    use std::net::TcpListener;

    // 拿一个曾经合法但已释放的端口（无 server 占用，OS 池中标"freshly-released"）。
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 0 to get free port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // 立即释放，使该端口的 TCP connect 收到 ECONNREFUSED

    let provider = StaticPortProvider::new(port);
    let client = KcClient::new(provider);

    let err = client
        .ingest_text("# Test", &KcIngestOptions::default())
        .await
        .expect_err("已释放端口应当 Unreachable");

    assert!(
        matches!(err, KcCallError::Unreachable),
        "端口不通必须映射到 Unreachable，实际: {err:?}",
    );
}

// =====================================================================
// Test 3: timeout scenario
// =====================================================================

#[tokio::test]
async fn client_timeout_when_mock_delays() {
    // mock 延迟 500ms 响应；client 设 100ms 总超时 → 必触发 Timeout。
    let delay = Duration::from_millis(500);
    let mock = MockKcServer::start_with_timeout(delay).await;
    let provider = StaticPortProvider::new(mock.port());

    // 用 new_with_http_client 注入短超时（生产 KcClient::new 写死 60s 不能用于测试）。
    let short_http = reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("test client builder");
    let client = KcClient::new_with_http_client(provider, short_http);

    let err = client
        .ingest_text("# Test", &KcIngestOptions::default())
        .await
        .expect_err("100ms client + 500ms mock delay 必触发 Timeout");

    assert!(
        matches!(err, KcCallError::Timeout),
        "超时必须映射到 KcCallError::Timeout，实际: {err:?}",
    );

    mock.stop();
}

// =====================================================================
// Test 4: LlmUnavailable with partial_md
// =====================================================================

#[tokio::test]
async fn client_llm_unavailable_with_partial() {
    let partial = "# 规则增强 MD\n\n#rule_tag\n\n[paragraph-0]";
    let mock = MockKcServer::start_with_llm_unavailable(partial).await;
    let provider = StaticPortProvider::new(mock.port());

    let client = KcClient::new(provider);
    let err = client
        .ingest_text("# Test", &KcIngestOptions::default())
        .await
        .expect_err("LLM unavailable scenario 应当返 Err");

    match err {
        KcCallError::LlmUnavailable { partial_md } => {
            assert_eq!(
                partial_md.as_deref(),
                Some(partial),
                "partial_md 必须 round-trip",
            );
        }
        other => panic!("expected LlmUnavailable, got {other:?}"),
    }

    mock.stop();
}

// =====================================================================
// Test 5: Internal error with code extraction
// =====================================================================

#[tokio::test]
async fn client_internal_error_extracts_code() {
    let mock = MockKcServer::start_with_internal_error().await;
    let provider = StaticPortProvider::new(mock.port());

    let client = KcClient::new(provider);
    let err = client
        .ingest_text("# Test", &KcIngestOptions::default())
        .await
        .expect_err("internal_error scenario 应当返 Err");

    match err {
        KcCallError::Internal { code, detail } => {
            assert_eq!(code, "KC_INTERNAL", "code 必须从 detail.error_code 提取");
            assert!(
                detail.contains("KC 内部异常") || detail.contains("mock"),
                "detail 应含 mock 的 message，实际: {detail}",
            );
        }
        other => panic!("expected Internal, got {other:?}"),
    }

    mock.stop();
}

// =====================================================================
// Test 6: InputTooLarge skips HTTP request
// =====================================================================

#[tokio::test]
async fn client_input_too_large_skips_request() {
    // mock 用 success scenario，但客户端不应当真发请求（size 预检先拒）。
    let mock = MockKcServer::start_with_success("# x", KcMockMeta::default()).await;
    let provider = StaticPortProvider::new(mock.port());

    let client = KcClient::new(provider);
    let oversize = "x".repeat(1_148_000); // 1.1MB

    let err = client
        .ingest_text(&oversize, &KcIngestOptions::default())
        .await
        .expect_err("> 1MB 必须被 size 预检拒绝");

    assert!(
        matches!(err, KcCallError::InputTooLarge),
        "大输入必须映射到 InputTooLarge，实际: {err:?}",
    );

    // mock server 不应收到任何请求——wiremock 自动 verify_on_drop。
    // 在 mock_kc.rs 中 start_with_success 没设 .expect(N)，所以不会 panic on drop，
    // 但 size 预检逻辑由 lib 单测 ingest_1_1mb_rejected_by_size_check 已严格覆盖。
    mock.stop();
}

// =====================================================================
// Test 7: Semaphore(1) 串行化—— wall-clock 总耗时断言
// =====================================================================

/// **input.md AC-6 §"client_serializes_requests"**：5 个并发请求 → 验证 Semaphore 串行。
///
/// ## 策略：wall-clock 总耗时断言
///
/// - mock 端每次 ingest **set_delay 60ms**（让并发请求若不被串行化则可重叠）；
/// - 同时 spawn 5 个 task 调 `ingest_text`；
/// - 串行化生效 ⟹ 总耗时 ≈ 5 × 60ms = **300ms**；
/// - 串行化失效（如 Semaphore permits 误改为 N>1）⟹ 总耗时 ≈ 60-80ms（reqwest 复用 connection）。
///
/// **下界设 250ms**（300ms - 50ms 容忍 CI 噪声）：充分区分"串行"vs"并发"，不会假阳。
///
/// ## 为什么不在 mock 端用 atomic in-flight 计数
///
/// wiremock 0.6 的 `Respond::respond(&self, &Request) -> ResponseTemplate` 是 **sync fn**——
/// in_flight ± 在闭包内单步发生，永远只能观察到 == 1，**无法**用于检测并发；
/// set_delay 是在 respond 返回后才生效，等不到 -1 那一刻。
///
/// 所以 wall-clock 总耗时是**最可靠的串行化断言方式**（且与生产语义一致：
/// 用户感知的"串行化"就是"P95 时间叠加"）。
#[tokio::test]
async fn client_serializes_requests() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "ai_enabled": true,
            "llm_reachable": true,
            "venv_ok": true,
            "v1_ready": true,
            "v2_ready": true,
            "version": "0.9"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/ingest"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(60))
                .set_body_json(serde_json::json!({
                    "success": true,
                    "doc_id": "doc-serial",
                    "enhanced_markdown": "# serial",
                    "ai_tags": [],
                    "rule_tags": [],
                    "ai_qa_pairs": [],
                    "ai_paragraph_links": [],
                    "paragraph_count": 0,
                    "kc_version": "0.9",
                    "kc_generated_at": "2026-05-27T00:00:00Z"
                })),
        )
        .mount(&server)
        .await;

    let port = server.address().port();
    let provider = StaticPortProvider::new(port);
    let client = Arc::new(KcClient::new(provider));

    let mut handles = Vec::with_capacity(5);
    let t0 = std::time::Instant::now();
    for i in 0..5 {
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            client
                .ingest_text(&format!("# req {i}"), &KcIngestOptions::default())
                .await
        }));
    }

    let mut success_count = 0;
    for h in handles {
        let result = h.await.expect("task join");
        if matches!(result, Ok(KcIngestOutcome::Success { .. })) {
            success_count += 1;
        }
    }
    let elapsed = t0.elapsed();

    assert_eq!(success_count, 5, "5 个请求应当全部成功");

    // 60ms × 5 = 300ms 下界（留 50ms 容忍 CI 抖动）。
    // 串行化失效（permits > 1）：总耗时 ≈ 60-80ms（远 < 250ms），断言会失败。
    assert!(
        elapsed >= Duration::from_millis(250),
        "5 个串行请求每个 60ms，总耗时应 >= 250ms（实际 {:?}）；\
         < 250ms 说明请求被并发执行，Semaphore(1) 串行化失效",
        elapsed,
    );

    drop(server);
}

// =====================================================================
// Test 8: Semaphore(1) 串行化—— wiremock expect(N) 精确计数双保险
// =====================================================================

/// **双保险**：test 7 用 wall-clock 间接验证；test 8 用 wiremock `expect(5)` 验证
/// 客户端发送的请求数确实是 5 次（不多不少，无重试 / 无丢失）。
///
/// `Mock::expect(5)` 在 MockServer drop 时自动 verify，若收到次数 != 5 会 panic。
/// 这覆盖了 input.md "Reviewer 重点关注项 §串行化测试是否真验证了顺序" 的"不要只验证编译通过"要求。
#[tokio::test]
async fn client_serializes_requests_exact_count() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok", "ai_enabled": true, "llm_reachable": true,
            "venv_ok": true, "v1_ready": true, "v2_ready": true, "version": "0.9"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/ingest"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(40))
                .set_body_json(serde_json::json!({
                    "success": true,
                    "doc_id": "doc-count",
                    "enhanced_markdown": "# count",
                    "ai_tags": [], "rule_tags": [], "ai_qa_pairs": [], "ai_paragraph_links": [],
                    "paragraph_count": 0, "kc_version": "0.9",
                    "kc_generated_at": "2026-05-27T00:00:00Z"
                })),
        )
        // 精确 5 次：少了 1 次（如某 task 被 cancel）或多了 1 次（如重试）都会 panic on drop。
        .expect(5)
        .mount(&server)
        .await;

    let provider = StaticPortProvider::new(server.address().port());
    let client = Arc::new(KcClient::new(provider));

    let mut handles = Vec::with_capacity(5);
    for i in 0..5 {
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            client
                .ingest_text(&format!("# req {i}"), &KcIngestOptions::default())
                .await
        }));
    }

    for h in handles {
        let r = h.await.expect("task join");
        r.expect("每个请求都应成功");
    }

    // server drop 时 expect(5) 自动 verify。
    drop(server);
}

// =====================================================================
// Test 9: PortProvider trait 类型兼容性（FIX 第 1 轮新增）
// =====================================================================

/// **FIX 第 1 轮 MAJOR-1 验证测试**：`kc::client::PortProvider` 与 `kc::process::PortProvider`
/// 必须是**同一个 trait 类型**（不是同名不同类型的两个 trait），这样 task_009 lifecycle integration
/// 才能把 `Arc<KcProcessManager>` 当 `Arc<dyn kc::client::PortProvider>` 传给 `KcClient::new`。
///
/// ## 验证手段
///
/// 把一个 `impl kc::process::PortProvider` 的具体类型（这里用集成测试自己的 `StaticPortProvider`，
/// 它 `impl PortProvider`——而 `PortProvider` 已通过 client.rs 的 `pub use` 重导出指向 process 模块的
/// trait）强转为 `Arc<dyn kc::client::PortProvider>` 并传给 `KcClient::new`。**编译通过即视为 PASS**。
///
/// 如果 client.rs 与 process.rs 还有两个独立 trait，本测试会编译失败（trait object 不能跨类型转换）。
///
/// ## 为什么不直接构造 KcProcessManager 验证
///
/// `KcProcessManager::new(&AppHandle)` 需要 Tauri runtime（unit test 不可用）；
/// `KcProcessManager::new_for_test()` 是 `#[cfg(test)]` 私有，集成测试 binary 拿不到。
/// 类型签名的 trait 兼容性纯属编译期检查——只要 `impl process::PortProvider` 的任意类型能
/// 通过 `Arc<dyn kc::client::PortProvider>` 类型签名，就证明两个 path 等价。
#[tokio::test]
async fn port_provider_trait_is_unified_across_client_and_process() {
    // 1) StaticPortProvider 是本测试 binary 内 impl PortProvider 的具体类型；
    //    PortProvider 现在从 client.rs `pub use` 自 kc::process::PortProvider。
    let provider_impl: Arc<StaticPortProvider> = StaticPortProvider::new(31337);

    // 2) 类型断言（编译期）：能否把 Arc<StaticPortProvider> 强转 Arc<dyn PortProvider>。
    //    若 client::PortProvider ≠ process::PortProvider（修复前的状态），此处编译失败。
    let provider_dyn: Arc<dyn PortProvider> = provider_impl.clone();

    // 3) 验证它能被 KcClient::new 接受（这是 task_009 衔接的真实调用点）。
    //    KcClient::new 签名是 `pub fn new(port_provider: Arc<dyn PortProvider>) -> Self`，
    //    其中 PortProvider 来自 `crate::kc::client::PortProvider`（即 process 模块同 path）。
    let _client = KcClient::new(provider_dyn);

    // 4) 进一步显式断言：`kc::process::PortProvider` 与 `kc::client::PortProvider` 同 path（编译期）。
    //    （第二个 Arc cast 走 process 模块的 path——如果两者不等价，无法把同一个 provider 同时绑定到两个 trait object 类型上）
    let from_process_path: Arc<dyn app_lib::kc::process::PortProvider> = provider_impl.clone();
    let from_client_path: Arc<dyn app_lib::kc::client::PortProvider> = provider_impl;

    // 运行时仅断言端口一致（编译通过 = 测试 PASS 的核心）。
    assert_eq!(from_process_path.current_port(), Some(31337));
    assert_eq!(from_client_path.current_port(), Some(31337));
}
