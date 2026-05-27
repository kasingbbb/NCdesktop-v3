//! task_006_mock_kc_server：`MockKcServer` basic 集成测试（input.md AC-5）。
//!
//! ## 4 个测试用例
//!
//! - [`mock_health_responds_200`] — 验证 health-only scenario，断言 GET /health 返 200 + 解析字段；
//! - [`mock_success_returns_enhanced_md`] — 验证 success scenario，断言 POST /ingest 返 200 + enhanced_markdown 字段；
//! - [`mock_internal_error_returns_500_with_code`] — 验证 internal_error scenario，断言 500 + detail.error_code = "KC_INTERNAL"；
//! - [`mock_timeout_blocks_for_delay`] — 验证 timeout scenario，断言 client 短超时下能拿到 `is_timeout() == true`。
//!
//! 本测试不依赖 NC 任何主代码（lib），只验证 `MockKcServer` helper 本身的契约。
//! task_007/008/011/022/023 后续会消费此 helper + lib 主代码。

mod common;

use std::time::Duration;

use common::mock_kc::{KcMockMeta, MockKcServer};
use reqwest::Client;
use serde_json::Value;

// =====================================================================
// Test 1：health-only scenario
// =====================================================================

#[tokio::test]
async fn mock_health_responds_200() {
    let mock = MockKcServer::start_with_health_only().await;
    let url = format!("{}/api/v1/health", mock.base_url());

    let resp = Client::new()
        .get(&url)
        .send()
        .await
        .expect("GET /health 应该成功");

    assert_eq!(resp.status(), 200, "health 应返 200");

    let body: Value = resp.json().await.expect("health 响应应是合法 JSON");
    assert_eq!(body["status"].as_str(), Some("ok"), "status 字段应为 'ok'");
    assert_eq!(
        body["ai_enabled"].as_bool(),
        Some(true),
        "ai_enabled 字段应为 true（mock 默认）",
    );
    assert_eq!(
        body["v1_ready"].as_bool(),
        Some(true),
        "v1_ready 字段应为 true",
    );

    mock.stop();
}

// =====================================================================
// Test 2：success scenario（KC-MOD-1 形态，含 enhanced_markdown）
// =====================================================================

#[tokio::test]
async fn mock_success_returns_enhanced_md() {
    let enhanced = "# 增强后的文档\n\n#AI #Mock\n\n正文内容...";
    let meta = KcMockMeta::default();
    let mock = MockKcServer::start_with_success(enhanced, meta.clone()).await;
    let url = format!("{}/api/v1/ingest", mock.base_url());

    let request_body = serde_json::json!({
        "markdown_text": "# 原始文档\n\nHello",
        "persist": false,
        "use_ai": true,
        "enable_qa": true,
        "enable_links": true
    });

    let resp = Client::new()
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .expect("POST /ingest 应该成功（mock 返 200）");

    assert_eq!(resp.status(), 200, "success scenario 应返 200");

    let body: Value = resp.json().await.expect("响应应是合法 JSON");
    assert_eq!(body["success"].as_bool(), Some(true));
    assert_eq!(
        body["enhanced_markdown"].as_str(),
        Some(enhanced),
        "enhanced_markdown 字段必须包含 KC-MOD-1 后的增强文本",
    );
    assert_eq!(body["doc_id"].as_str(), Some(meta.doc_id.as_str()));
    assert_eq!(body["title"].as_str(), Some(meta.title.as_str()));
    assert_eq!(
        body["kc_version"].as_str(),
        Some(meta.kc_version.as_str()),
    );
    // ai_tags 数组结构验证
    let ai_tags = body["ai_tags"].as_array().expect("ai_tags 应是数组");
    assert_eq!(ai_tags.len(), meta.ai_tags.len());

    mock.stop();
}

// =====================================================================
// Test 3：internal_error scenario（KC-MOD-3 结构化错误码）
// =====================================================================

#[tokio::test]
async fn mock_internal_error_returns_500_with_code() {
    let mock = MockKcServer::start_with_internal_error().await;
    let url = format!("{}/api/v1/ingest", mock.base_url());

    let request_body = serde_json::json!({
        "markdown_text": "# Test",
        "persist": false
    });

    let resp = Client::new()
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .expect("HTTP 请求应能完成（即便服务端返 500）");

    assert_eq!(resp.status(), 500, "internal_error scenario 应返 500");

    let body: Value = resp.json().await.expect("失败响应应是合法 JSON");
    assert_eq!(
        body["detail"]["error_code"].as_str(),
        Some("KC_INTERNAL"),
        "detail.error_code 必须是 KC_INTERNAL（KC-MOD-3 结构化错误码）",
    );
    assert!(
        body["detail"]["message"].as_str().is_some(),
        "detail.message 必须存在（供日志透传）",
    );
    assert_eq!(
        body["detail"]["retryable"].as_bool(),
        Some(false),
        "KC_INTERNAL 不可重试",
    );

    mock.stop();
}

// =====================================================================
// Test 4：timeout scenario（客户端超时短于 mock delay）
// =====================================================================

#[tokio::test]
async fn mock_timeout_blocks_for_delay() {
    // mock 延迟 500ms 响应；client 设 100ms 总超时 → 必触发 is_timeout()
    let delay = Duration::from_millis(500);
    let mock = MockKcServer::start_with_timeout(delay).await;
    let url = format!("{}/api/v1/ingest", mock.base_url());

    let short_timeout_client = Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("构造 reqwest client 应成功");

    let request_body = serde_json::json!({
        "markdown_text": "# Test",
        "persist": false
    });

    let result = short_timeout_client.post(&url).json(&request_body).send().await;

    let err = result.expect_err("100ms client 超时下应当返回 Err（mock 延迟 500ms）");
    assert!(
        err.is_timeout(),
        "reqwest Err 必须是 timeout 类型，实际：{:?}",
        err,
    );

    mock.stop();
}
