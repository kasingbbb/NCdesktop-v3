use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;

/// 不应再重试：避免超时/鉴权错误时累计等待数分钟
fn should_abort_retries(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("timed out")
        || e.contains("timeout")
        || e.contains("deadline")
        || err.contains("认证失败")
        || err.contains("401")
        || err.contains("403")
}

#[derive(Debug)]
pub enum RetryableError {
    RateLimited,
    ServerError(u16),
    NetworkError(String),
    NonRetryable(String),
}

impl RetryableError {
    pub fn from_status(status: u16, body: &str) -> Self {
        match status {
            429 => RetryableError::RateLimited,
            401 | 403 => RetryableError::NonRetryable(
                format!("认证失败 ({}): 请检查 API Key 配置", status)
            ),
            500..=599 => RetryableError::ServerError(status),
            _ => RetryableError::NonRetryable(
                format!("API 错误 ({}): {}", status, body)
            ),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, RetryableError::RateLimited | RetryableError::ServerError(_) | RetryableError::NetworkError(_))
    }

    pub fn to_user_message(&self) -> String {
        match self {
            RetryableError::RateLimited =>
                "请求频率超限，请稍后重试".to_string(),
            RetryableError::ServerError(code) =>
                format!("服务器暂时不可用 ({})，正在重试...", code),
            RetryableError::NetworkError(msg) =>
                format!("网络连接失败: {}。请检查网络或使用离线导出模式", msg),
            RetryableError::NonRetryable(msg) =>
                msg.clone(),
        }
    }
}

/// 指数退避重试策略（1s → 2s → 4s）
pub async fn with_retry<F, Fut, T>(mut operation: F) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last_error = String::new();

    for attempt in 0..MAX_RETRIES {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = e.clone();

                if should_abort_retries(&e) {
                    return Err(e);
                }

                if attempt < MAX_RETRIES - 1 {
                    let delay = BASE_DELAY_MS * 2u64.pow(attempt);
                    log::warn!(
                        "LLM 请求失败 (尝试 {}/{}): {}，{}ms 后重试",
                        attempt + 1,
                        MAX_RETRIES,
                        e,
                        delay
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                } else {
                    log::error!(
                        "LLM 请求最终失败 (尝试 {}/{}): {}",
                        attempt + 1,
                        MAX_RETRIES,
                        e
                    );
                }
            }
        }
    }

    Err(format!("请求失败（已重试 {} 次）: {}", MAX_RETRIES, last_error))
}
