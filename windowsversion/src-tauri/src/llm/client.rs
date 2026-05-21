use serde::{Deserialize, Serialize};
use std::env;

use crate::db;
use rusqlite::Connection;

pub const SETTING_LLM_API_KEY: &str = "llm.api_key";
pub const SETTING_LLM_BASE_URL: &str = "llm.base_url";
pub const SETTING_LLM_MODEL: &str = "llm.model";

// 私有发布兜底：构建时通过环境变量把 key/base/model 烤进二进制。
// 设置 / 环境变量都没配置时才会用到。**仅用于私下分发**，不要做公开发布。
// 用法：BUNDLED_LLM_API_KEY=xxx pnpm tauri:build
const BUNDLED_LLM_API_KEY: Option<&str> = option_env!("BUNDLED_LLM_API_KEY");
const BUNDLED_LLM_BASE_URL: Option<&str> = option_env!("BUNDLED_LLM_BASE_URL");
const BUNDLED_LLM_MODEL: Option<&str> = option_env!("BUNDLED_LLM_MODEL");

fn bundled(opt: Option<&str>) -> Option<String> {
    opt.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

#[derive(Debug, Clone)]
pub struct LLMClient {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub api_key_masked: String,
    pub base_url: String,
    pub model: String,
    pub is_configured: bool,
}

pub fn default_base_url() -> String {
    env::var("ARK_BASE_URL")
        .or_else(|_| env::var("OPENAI_BASE_URL"))
        .ok()
        .or_else(|| bundled(BUNDLED_LLM_BASE_URL))
        .unwrap_or_else(|| "https://ark.cn-beijing.volces.com/api/coding".to_string())
}

pub fn default_model() -> String {
    env::var("ARK_MODEL")
        .or_else(|_| env::var("OPENAI_MODEL"))
        .ok()
        .or_else(|| bundled(BUNDLED_LLM_MODEL))
        .unwrap_or_else(|| "ark-code-latest".to_string())
}

fn resolve_base_url(conn: &Connection) -> String {
    db::settings::get(conn, SETTING_LLM_BASE_URL)
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("ARK_BASE_URL").ok())
        .or_else(|| env::var("OPENAI_BASE_URL").ok())
        .or_else(|| bundled(BUNDLED_LLM_BASE_URL))
        .unwrap_or_else(default_base_url)
}

fn resolve_model(conn: &Connection) -> String {
    db::settings::get(conn, SETTING_LLM_MODEL)
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("ARK_MODEL").ok())
        .or_else(|| env::var("OPENAI_MODEL").ok())
        .or_else(|| bundled(BUNDLED_LLM_MODEL))
        .unwrap_or_else(default_model)
}

impl LLMClient {
    /// 仅环境变量（兼容旧逻辑与测试）
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("ARK_API_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .ok()
            .or_else(|| bundled(BUNDLED_LLM_API_KEY))
            .ok_or_else(|| {
                "未检测到 API Key（请在设置中填写或配置环境变量 ARK_API_KEY / OPENAI_API_KEY）".to_string()
            })?;

        if api_key.is_empty() {
            return Err("API Key 为空".to_string());
        }

        Ok(Self {
            api_key,
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: 4096,
            temperature: 0.7,
        })
    }

    /// 应用内设置优先，其次环境变量（与 `get_llm_config` / 拖放分类一致）
    pub fn from_db_or_env(conn: &Connection) -> Result<Self, String> {
        let api_key = db::settings::get(conn, SETTING_LLM_API_KEY)?
            .filter(|s| !s.trim().is_empty())
            .or_else(|| env::var("ARK_API_KEY").ok())
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .or_else(|| bundled(BUNDLED_LLM_API_KEY))
            .ok_or_else(|| {
                "未检测到 API Key（请在设置中填写或配置环境变量 ARK_API_KEY / OPENAI_API_KEY）"
                    .to_string()
            })?;

        if api_key.is_empty() {
            return Err("API Key 为空".to_string());
        }

        let base_url = resolve_base_url(conn);
        let model = resolve_model(conn);

        Ok(Self {
            api_key,
            base_url,
            model,
            max_tokens: 4096,
            temperature: 0.7,
        })
    }

    /// 设置页展示的 Base URL / Model（未配置 Key 时也返回可编辑默认值）
    pub fn display_defaults(conn: &Connection) -> (String, String) {
        (resolve_base_url(conn), resolve_model(conn))
    }

    pub fn get_config(&self) -> LLMConfig {
        let masked = if self.api_key.len() > 8 {
            format!("{}...", &self.api_key[..8])
        } else {
            "***".to_string()
        };

        LLMConfig {
            api_key_masked: masked,
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            is_configured: true,
        }
    }

    /// 是否有可用的 Key（应用内或环境变量或构建时烤入）
    pub fn is_available() -> bool {
        if let Ok(k) = env::var("ARK_API_KEY").or_else(|_| env::var("OPENAI_API_KEY")) {
            if !k.is_empty() {
                return true;
            }
        }
        bundled(BUNDLED_LLM_API_KEY).is_some()
    }

    pub fn is_available_in_conn(conn: &Connection) -> bool {
        if let Ok(Some(k)) = db::settings::get(conn, SETTING_LLM_API_KEY) {
            if !k.trim().is_empty() {
                return true;
            }
        }
        Self::is_available()
    }

    pub fn build_headers(&self) -> Vec<(String, String)> {
        vec![
            ("x-api-key".to_string(), self.api_key.clone()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ]
    }
}
