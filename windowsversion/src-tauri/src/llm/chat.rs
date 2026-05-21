use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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
    system: Option<String>,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

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


/// 把 messages 中所有 `role=="system"` 的 content 按出现顺序用 `\n\n` 合并为单条字符串，
/// 并返回剥离 system 后的剩余 messages（保持原顺序）。
///
/// Anthropic 协议把 system prompt 单独成字段，只接受一个 system 字符串。本函数为
/// `chat_completion` 的纯逻辑切片：调用方（如 `prompt_runtime::assemble_messages_for_*`）
/// 会按 "system_message → system_addon → user → output_format_guard" 顺序构造多条
/// system，本函数把它们按原顺序拼接，保留 "GUARD 永远最后压底" 语义。
///
/// 修复历史（task_004 AC-0）：之前用循环覆盖（`system_text = Some(msg.content.clone())`），
/// 多条 system 只有最后一条送达 Anthropic，前置上下文丢失。
fn merge_system_messages(messages: Vec<ChatMessage>) -> (Option<String>, Vec<ChatMessage>) {
    let mut system_parts: Vec<String> = Vec::new();
    let mut filtered_messages = Vec::new();
    for msg in messages {
        if msg.role == "system" {
            system_parts.push(msg.content);
        } else {
            filtered_messages.push(msg);
        }
    }
    let system_text = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };
    (system_text, filtered_messages)
}

/// 同步 Chat Completion（非流式）
pub async fn chat_completion(
    client: &LLMClient,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let (system_text, filtered_messages) = merge_system_messages(messages);

    let url = format!("{}/v1/messages", client.base_url.trim_end_matches('/'));

    if filtered_messages.is_empty() {
        return Err("Anthropic 协议要求至少包含一条用户消息（messages 不能为空）".to_string());
    }

    let request = AnthropicRequest {
        model: client.model.clone(),
        system: system_text,
        messages: filtered_messages,
        max_tokens: client.max_tokens,
        temperature: client.temperature,
        stream: false,
    };

    let response: AnthropicResponse = with_retry(|| async {
        let mut req = llm_http_client().post(&url).json(&request);
        for (k, v) in client.build_headers() {
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
pub async fn chat_completion_stream<F>(
    client: &LLMClient,
    messages: Vec<ChatMessage>,
    on_chunk: F,
) -> Result<String, String>
where
    F: Fn(&str),
{
    // 目前前端未接入流式，暂时留空返回，后续需适配 Anthropic Stream
    Err("Stream is currently unsupported in Anthropic mode".to_string())
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

    /// AC-0：多条 system 必须按顺序用 `\n\n` 合并为单条字符串，
    /// 而不是循环覆盖只保留最后一条。
    #[test]
    fn multiple_system_messages_are_joined_with_double_newline() {
        let messages = vec![sys("a"), sys("b"), sys("c"), usr("hello")];
        let (system_text, filtered) = merge_system_messages(messages);
        assert_eq!(system_text.as_deref(), Some("a\n\nb\n\nc"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].role, "user");
        assert_eq!(filtered[0].content, "hello");
    }

    /// 单条 system 应原样返回，不引入额外分隔符。
    #[test]
    fn single_system_message_returned_verbatim() {
        let messages = vec![sys("only one"), usr("hello")];
        let (system_text, filtered) = merge_system_messages(messages);
        assert_eq!(system_text.as_deref(), Some("only one"));
        assert_eq!(filtered.len(), 1);
    }

    /// 无 system → None；filtered 与原始 user/assistant 保持顺序一致。
    #[test]
    fn no_system_messages_yields_none_and_preserves_user_order() {
        let messages = vec![usr("first"), usr("second")];
        let (system_text, filtered) = merge_system_messages(messages);
        assert!(system_text.is_none());
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].content, "first");
        assert_eq!(filtered[1].content, "second");
    }

    /// system 与 user 交错排列，system 按出现顺序合并，user 顺序保留。
    /// 验证"GUARD 永远最后压底"语义：messages 末尾的 system（典型为
    /// `assemble_messages_for_*` 产出的 GUARD）会成为合并后字符串的末段。
    #[test]
    fn interleaved_system_and_user_preserved_in_order() {
        let messages = vec![
            sys("system_message"),
            sys("system_addon"),
            usr("user_body"),
            sys("GUARD"),
        ];
        let (system_text, filtered) = merge_system_messages(messages);
        assert_eq!(
            system_text.as_deref(),
            Some("system_message\n\nsystem_addon\n\nGUARD")
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, "user_body");
    }
}
