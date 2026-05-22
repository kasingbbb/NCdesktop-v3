use std::sync::OnceLock;
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::client::LLMClient;
use super::retry::with_retry;

/// LLM HTTP：带连接/整体超时，避免错误 Base URL 或网络挂起时无限等待
fn llm_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            // 单次请求上限；超时后 with_retry 不再重复长时间等待
            .timeout(Duration::from_secs(75))
            .build()
            .expect("reqwest LLM client")
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<SystemBlock>>,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

/// Anthropic system 字段每个 block 的结构。
/// 标记 `cache_control: Some(ephemeral)` 的 block 进入 prompt cache，
/// 后续命中可显著降低 token 计费。
#[derive(Debug, Serialize)]
struct SystemBlock {
    #[serde(rename = "type")]
    type_field: String, // 总是 "text"
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    type_field: String, // 总是 "ephemeral"
}

/// 触发 prompt cache 的最小 text 长度（字符数，与 Anthropic 文档建议接近）。
/// Anthropic 限 4 个 cache 节点 → 短 system 不标，避免浪费配额。
const CACHE_CONTROL_MIN_LEN: usize = 1024;

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    pub content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
    pub thinking: Option<String>,
}

/// SSE 流式响应解析后的 event 类型（只关心 text_delta，其他忽略）。
#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<StreamDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
}

/// 把 messages 中所有 `role=="system"` 的 content 按出现顺序收集为 SystemBlock 列表，
/// 并返回剥离 system 后的剩余 messages（保持原顺序）。
///
/// 末尾那个 SystemBlock 若长度 ≥ `CACHE_CONTROL_MIN_LEN`，自动标
/// `cache_control: ephemeral`，让长 system（PARA prompt 等）进 Anthropic prompt cache，
/// 后续命中省 token；短 system 不标，避免占用 4 个 cache 节点配额。
///
/// 修复历史（task_004 AC-0）：之前用循环覆盖（`system_text = Some(msg.content.clone())`），
/// 多条 system 只有最后一条送达 Anthropic，前置上下文丢失。本函数保留 "GUARD 永远最后
/// 压底" 语义：调用方按 "system_message → system_addon → user → output_format_guard"
/// 顺序构造，最后一个 system block 在数组末尾。
fn merge_system_messages(messages: Vec<ChatMessage>) -> (Option<Vec<SystemBlock>>, Vec<ChatMessage>) {
    let mut system_parts: Vec<String> = Vec::new();
    let mut filtered_messages = Vec::new();
    for msg in messages {
        if msg.role == "system" {
            system_parts.push(msg.content);
        } else {
            filtered_messages.push(msg);
        }
    }
    if system_parts.is_empty() {
        return (None, filtered_messages);
    }

    let last_idx = system_parts.len() - 1;
    let blocks: Vec<SystemBlock> = system_parts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let cache_control = if i == last_idx && text.chars().count() >= CACHE_CONTROL_MIN_LEN {
                Some(CacheControl {
                    type_field: "ephemeral".to_string(),
                })
            } else {
                None
            };
            SystemBlock {
                type_field: "text".to_string(),
                text,
                cache_control,
            }
        })
        .collect();
    (Some(blocks), filtered_messages)
}

/// 构造 Anthropic /v1/messages 请求：url、request body、headers。
/// 流式与非流式只在 `stream` 字段不同。
fn build_anthropic_request(
    client: &LLMClient,
    messages: Vec<ChatMessage>,
    stream: bool,
) -> Result<(String, AnthropicRequest, Vec<(String, String)>), String> {
    let (system_blocks, filtered_messages) = merge_system_messages(messages);

    if filtered_messages.is_empty() {
        return Err("Anthropic 协议要求至少包含一条用户消息（messages 不能为空）".to_string());
    }

    let url = format!("{}/v1/messages", client.base_url.trim_end_matches('/'));
    let request = AnthropicRequest {
        model: client.model.clone(),
        system: system_blocks,
        messages: filtered_messages,
        max_tokens: client.max_tokens,
        temperature: client.temperature,
        stream,
    };
    let headers = client.build_headers();
    Ok((url, request, headers))
}

/// 同步 Chat Completion（非流式）
pub async fn chat_completion(
    client: &LLMClient,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let (url, request, headers) = build_anthropic_request(client, messages, false)?;

    let response: AnthropicResponse = with_retry(|| async {
        let mut req = llm_http_client().post(&url).json(&request);
        for (k, v) in &headers {
            req = req.header(k, v);
        }

        let res = req.send().await.map_err(|e| format!("网络请求失败: {e}"))?;
        let status = res.status();
        let text = res.text().await.map_err(|e| format!("读取响应失败: {e}"))?;
        if !status.is_success() {
            return Err(format!("LLM API 错误 ({status}): {text}"));
        }
        serde_json::from_str::<AnthropicResponse>(&text)
            .map_err(|e| format!("解析 API 响应失败: {e}"))
    })
    .await?;

    response
        .content
        .into_iter()
        .find_map(|c| c.text)
        .ok_or_else(|| "API 返回响应中未包含文本内容 (text block missing)".to_string())
}

/// 流式 Chat Completion — 通过 Tauri Event 推送
///
/// 协议契约（与 `src/lib/ai/useLLMStream.ts` 对齐）：
/// - text chunk 直接 emit（plain string payload）
/// - 流结束 emit `"[DONE]"`
/// - 流中错误 emit `"[ERROR] <msg>"` 并 return Err
///
/// 返回 accumulated 完整字符串，便于需要全文的 caller 使用。
pub async fn chat_completion_stream(
    client: &LLMClient,
    messages: Vec<ChatMessage>,
    app: &AppHandle,
    event_name: &str,
) -> Result<String, String> {
    let (url, request, headers) = build_anthropic_request(client, messages, true)?;

    // 流式不走 with_retry：流中途断开重试语义复杂，留给调用方决策。
    let mut req = llm_http_client().post(&url).json(&request);
    for (k, v) in &headers {
        req = req.header(k, v);
    }

    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("网络请求失败: {e}");
            let _ = app.emit(event_name, format!("[ERROR] {msg}"));
            return Err(msg);
        }
    };

    let status = res.status();
    if !status.is_success() {
        let text = res.text().await.unwrap_or_default();
        let msg = format!("LLM API 错误 ({status}): {text}");
        let _ = app.emit(event_name, format!("[ERROR] {msg}"));
        return Err(msg);
    }

    let mut accumulated = String::new();
    // 按字节累积，避免 chunk 边界切断 UTF-8 多字节序列（中文等）
    let mut byte_buf: Vec<u8> = Vec::new();
    let mut byte_stream = res.bytes_stream();
    let mut stopped = false;

    while let Some(chunk) = byte_stream.next().await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                let msg = format!("流读取失败: {e}");
                let _ = app.emit(event_name, format!("[ERROR] {msg}"));
                return Err(msg);
            }
        };
        byte_buf.extend_from_slice(&bytes);

        // 按 SSE event 分隔（双换行 \n\n = 0x0A 0x0A）切分；最后一段可能不完整，留在 buffer
        loop {
            let Some(idx) = byte_buf.windows(2).position(|w| w == b"\n\n") else {
                break;
            };
            // 提取 event 字节并从 buffer 移除（含分隔符）
            let event_bytes: Vec<u8> = byte_buf.drain(..idx + 2).collect();
            // SSE event 段含完整 UTF-8 边界（\n\n 后才切），可安全转 String
            let raw_event = match std::str::from_utf8(&event_bytes[..event_bytes.len() - 2]) {
                Ok(s) => s.to_string(),
                Err(_) => continue, // 异常字节序列：跳过该 event
            };

            // 提取所有 `data: ...` 行（一个 SSE event 可能有多行 data）
            let mut data_payload = String::new();
            for line in raw_event.lines() {
                if let Some(rest) = line.strip_prefix("data:") {
                    if !data_payload.is_empty() {
                        data_payload.push('\n');
                    }
                    data_payload.push_str(rest.trim_start());
                }
            }
            if data_payload.is_empty() {
                continue;
            }

            // 解析 JSON event。无法解析的忽略（兼容 ping/keepalive 等异常输入）
            let event: StreamEvent = match serde_json::from_str(&data_payload) {
                Ok(e) => e,
                Err(_) => continue,
            };

            match event.event_type.as_str() {
                "content_block_delta" => {
                    if let Some(delta) = event.delta {
                        // 只关心 text_delta；thinking_delta / input_json_delta 等忽略
                        if delta.delta_type.as_deref() == Some("text_delta") {
                            if let Some(text) = delta.text {
                                if !text.is_empty() {
                                    let _ = app.emit(event_name, text.clone());
                                    accumulated.push_str(&text);
                                }
                            }
                        }
                    }
                }
                "message_stop" => {
                    let _ = app.emit(event_name, "[DONE]".to_string());
                    stopped = true;
                    break;
                }
                _ => {
                    // message_start / content_block_start / content_block_stop /
                    // message_delta / ping / error 等：忽略
                }
            }
        }

        if stopped {
            break;
        }
    }

    // 流自然结束但未收到 message_stop（异常或被截断）：兜底 emit DONE
    if !stopped {
        let _ = app.emit(event_name, "[DONE]".to_string());
    }

    Ok(accumulated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sys(s: &str) -> ChatMessage {
        ChatMessage {
            role: "system".to_string(),
            content: s.to_string(),
        }
    }

    fn usr(s: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: s.to_string(),
        }
    }

    /// AC-0：多条 system 必须按顺序保留为独立 SystemBlock，
    /// 而不是循环覆盖只保留最后一条，也不再 join 为单字符串。
    #[test]
    fn multiple_system_messages_are_returned_as_blocks() {
        let messages = vec![sys("a"), sys("b"), sys("c"), usr("hello")];
        let (blocks_opt, filtered) = merge_system_messages(messages);
        let blocks = blocks_opt.expect("expected 3 system blocks");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text, "a");
        assert_eq!(blocks[0].type_field, "text");
        assert_eq!(blocks[1].text, "b");
        assert_eq!(blocks[2].text, "c");
        // 短 system 不标 cache_control
        assert!(blocks[0].cache_control.is_none());
        assert!(blocks[1].cache_control.is_none());
        assert!(blocks[2].cache_control.is_none());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].role, "user");
        assert_eq!(filtered[0].content, "hello");
    }

    /// 单条 system 应原样作为单个 block 返回。
    #[test]
    fn single_system_message_returned_as_single_block() {
        let messages = vec![sys("only one"), usr("hello")];
        let (blocks_opt, filtered) = merge_system_messages(messages);
        let blocks = blocks_opt.expect("expected one system block");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "only one");
        assert!(blocks[0].cache_control.is_none());
        assert_eq!(filtered.len(), 1);
    }

    /// 无 system → None；filtered 与原始 user/assistant 保持顺序一致。
    #[test]
    fn no_system_messages_yields_none_and_preserves_user_order() {
        let messages = vec![usr("first"), usr("second")];
        let (blocks_opt, filtered) = merge_system_messages(messages);
        assert!(blocks_opt.is_none());
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].content, "first");
        assert_eq!(filtered[1].content, "second");
    }

    /// system 与 user 交错排列，system 按出现顺序收集为独立 block，user 顺序保留。
    /// 验证"GUARD 永远最后压底"语义：messages 末尾的 system（典型为
    /// `assemble_messages_for_*` 产出的 GUARD）会成为 block 列表的最后一项。
    #[test]
    fn interleaved_system_and_user_preserved_in_order() {
        let messages = vec![
            sys("system_message"),
            sys("system_addon"),
            usr("user_body"),
            sys("GUARD"),
        ];
        let (blocks_opt, filtered) = merge_system_messages(messages);
        let blocks = blocks_opt.expect("expected 3 system blocks");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text, "system_message");
        assert_eq!(blocks[1].text, "system_addon");
        assert_eq!(blocks[2].text, "GUARD");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "user_body");
    }

    /// 末尾 system block 长度 ≥ 1024 字符 → 自动标 cache_control:ephemeral。
    #[test]
    fn last_long_system_block_gets_cache_control() {
        let long = "x".repeat(CACHE_CONTROL_MIN_LEN);
        let messages = vec![sys("short"), sys(&long), usr("hello")];
        let (blocks_opt, _) = merge_system_messages(messages);
        let blocks = blocks_opt.expect("expected 2 system blocks");
        assert_eq!(blocks.len(), 2);
        // 第一个 short 不标
        assert!(blocks[0].cache_control.is_none());
        // 末尾长 block 标 ephemeral
        let cc = blocks[1]
            .cache_control
            .as_ref()
            .expect("last long block should carry cache_control");
        assert_eq!(cc.type_field, "ephemeral");
    }

    /// 全部短 system → 没有任何 block 标 cache_control（不浪费 4 个 cache 节点配额）。
    #[test]
    fn short_system_blocks_no_cache_control() {
        let messages = vec![sys("short a"), sys("short b"), usr("hello")];
        let (blocks_opt, _) = merge_system_messages(messages);
        let blocks = blocks_opt.expect("expected 2 system blocks");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].cache_control.is_none());
        assert!(blocks[1].cache_control.is_none());
    }
}
