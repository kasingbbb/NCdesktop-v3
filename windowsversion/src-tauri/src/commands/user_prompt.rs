//! 用户自定义 Prompt 命令层（custom_prompt_v1 / task_002 + task_003 回填）。
//!
//! 4 个 Tauri command：`list_user_prompts / get_user_prompt /
//! save_user_prompt / reset_user_prompt`。Architect output.md § 6.1 定义签名。
//!
//! 设计约束：
//! - 命名前缀全部用 `user_prompt`，与 PR-4 `commands/prompts.rs` 的 `prompt.override.*`
//!   命名空间区隔（Architect § 9 R6）。
//! - 4 个白名单 module：`tagging / para / concept / aggregation`。任何非白名单值
//!   立即拒绝并返回中文错误。
//! - 写命令（save / reset）必经 `ensure_writable(mode.inner())` 守卫，与
//!   `commands::categories::*` / `commands::prompts::*` 的范式一致。
//! - 字节长度上限 16 KiB（ADR-004 / R2）。保存时执行；调用前总字符长度校验
//!   由 `llm/prompt_runtime.rs::assert_total_chars_within` 承担。
//! - **task_003 回填**：`PromptInfo.default_text / display_title / required_placeholders`
//!   字段切换为 `llm::prompt_runtime` 的真实实现；`save_user_prompt` 中的
//!   `validate_placeholders_stub` 替换为
//!   `prompt_runtime::validate_required_placeholders`（ADR-003 Layer B）。

use crate::db::{user_prompt as db_user_prompt, Database};
use crate::llm::prompt_runtime;
use crate::startup::{ensure_writable, AppMode};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 4 个用户视角 module 的白名单（顺序即 UI 默认展示顺序）。
const MODULES: &[&str] = &["tagging", "para", "concept", "aggregation"];

/// `PromptInfo`：单个 module 的展示态（前端 `src/types/user-prompt.ts` 同构）。
///
/// task_003 起，`default_text / display_title / required_placeholders / max_bytes`
/// 全部由 `llm::prompt_runtime` 提供真实值（不再是 task_002 阶段的占位）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptInfo {
    pub module: String,
    pub display_title: String,
    pub default_text: String,
    pub user_text: Option<String>,
    pub is_custom: bool,
    pub builtin_version: String,
    pub updated_at: Option<String>,
    pub required_placeholders: Vec<String>,
    pub max_bytes: usize,
}

/// module 白名单校验（任何非 MODULES 中的值即拒绝）。
fn validate_module(module: &str) -> Result<(), String> {
    if MODULES.contains(&module) {
        Ok(())
    } else {
        Err(format!(
            "未知的 Prompt 模块: {module}（必须为 tagging / para / concept / aggregation 之一）"
        ))
    }
}

/// 字节长度校验（ADR-004 保存时分支）。本函数转调
/// `prompt_runtime::byte_len_check`，保持命令层与运行时层在阈值上单点同步。
fn validate_byte_len(text: &str) -> Result<(), String> {
    prompt_runtime::byte_len_check(text)
}

/// 占位符必含校验（ADR-003 Layer B / task_003 接入真实实现）。
///
/// 转调 `prompt_runtime::validate_required_placeholders`：concept 必含 `{content}`，
/// aggregation 必含 `{concept_name}`，tagging/para 无强制占位符。
fn validate_placeholders(module: &str, text: &str) -> Result<(), String> {
    prompt_runtime::validate_required_placeholders(module, text)
}

/// 把 DB 行（可能不存在）+ module 合成 `PromptInfo`。
///
/// `default_text` / `display_title` / `required_placeholders` / `max_bytes` 从
/// `llm::prompt_runtime` 读取（task_003 回填）。
fn assemble_prompt_info(
    module: &str,
    row: Option<db_user_prompt::UserPromptRow>,
) -> PromptInfo {
    let (user_text, is_custom, builtin_version, updated_at) = match row {
        Some(r) => (
            Some(r.prompt_text),
            r.is_custom,
            r.builtin_version,
            Some(r.updated_at),
        ),
        None => (None, false, "1.0".to_string(), None),
    };

    PromptInfo {
        module: module.to_string(),
        display_title: prompt_runtime::display_title(module).to_string(),
        default_text: prompt_runtime::default_for(module).to_string(),
        user_text,
        is_custom,
        builtin_version,
        updated_at,
        required_placeholders: prompt_runtime::required_placeholders(module)
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        max_bytes: prompt_runtime::MAX_USER_PROMPT_BYTES,
    }
}

/// 列出 4 个 module 的当前状态。恒定返回 4 条记录，按 MODULES 中的顺序。
#[tauri::command]
pub fn list_user_prompts(
    database: State<'_, Database>,
) -> Result<Vec<PromptInfo>, String> {
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;

    // 一次 list_all + HashMap 索引，避免 4 次单查。
    let rows = db_user_prompt::list_all(&conn)?;
    let mut by_module: std::collections::HashMap<String, db_user_prompt::UserPromptRow> =
        rows.into_iter().map(|r| (r.module.clone(), r)).collect();

    let mut out = Vec::with_capacity(MODULES.len());
    for m in MODULES {
        let row = by_module.remove(*m);
        out.push(assemble_prompt_info(m, row));
    }
    Ok(out)
}

/// 单条查询（编辑器进入时用）。
#[tauri::command]
pub fn get_user_prompt(
    database: State<'_, Database>,
    module: String,
) -> Result<PromptInfo, String> {
    validate_module(&module)?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let row = db_user_prompt::get(&conn, &module)?;
    Ok(assemble_prompt_info(&module, row))
}

/// 保存用户自定义 Prompt。
///
/// 守卫顺序：white-list → `ensure_writable` → 字节长度 → 必含占位符 → upsert。
#[tauri::command]
pub fn save_user_prompt(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    module: String,
    text: String,
) -> Result<(), String> {
    validate_module(&module)?;
    ensure_writable(mode.inner())?;
    validate_byte_len(&text)?;
    validate_placeholders(&module, &text)?;

    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    db_user_prompt::upsert(&conn, &module, &text)?;
    Ok(())
}

/// 重置：`None` = 删全部（一键恢复默认），`Some(m)` = 删单条。
#[tauri::command]
pub fn reset_user_prompt(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    module: Option<String>,
) -> Result<(), String> {
    ensure_writable(mode.inner())?;

    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    match module {
        None => db_user_prompt::delete_all(&conn)?,
        Some(m) => {
            validate_module(&m)?;
            db_user_prompt::delete(&conn, &m)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    // -------- 白名单校验 --------

    #[test]
    fn validate_module_accepts_four_whitelist() {
        for m in ["tagging", "para", "concept", "aggregation"] {
            assert!(validate_module(m).is_ok(), "{m} 应通过白名单");
        }
    }

    #[test]
    fn validate_module_rejects_unknown() {
        let err = validate_module("classify").expect_err("classify 不在 4-module 白名单内");
        // PR-4 的 classify/naming 等命名不能误闯本期白名单（R6）
        assert!(
            err.contains("未知的 Prompt 模块"),
            "错误消息应为中文，得到: {err}"
        );
        assert!(validate_module("").is_err());
        assert!(validate_module("Tagging").is_err(), "大小写敏感");
    }

    // -------- 字节长度校验（ADR-004 保存分支） --------

    #[test]
    fn byte_len_under_limit_passes() {
        assert!(validate_byte_len("").is_ok());
        let just_below = "a".repeat(prompt_runtime::MAX_USER_PROMPT_BYTES);
        assert!(validate_byte_len(&just_below).is_ok());
    }

    #[test]
    fn byte_len_over_limit_rejects_with_chinese_message() {
        let too_long = "a".repeat(prompt_runtime::MAX_USER_PROMPT_BYTES + 1);
        let err = validate_byte_len(&too_long).expect_err("应拒绝");
        assert!(err.contains("自定义 Prompt 过长"), "中文错误: {err}");
        assert!(err.contains(&format!("{}", prompt_runtime::MAX_USER_PROMPT_BYTES)));
    }

    #[test]
    fn byte_len_counts_bytes_not_chars() {
        // UTF-8 多字节字符：单个中文字 3 字节
        // 16 KiB / 3 ≈ 5461 字 → 5462 字 = 16386 字节，应拒
        let chinese = "中".repeat(prompt_runtime::MAX_USER_PROMPT_BYTES / 3 + 2);
        let err = validate_byte_len(&chinese).expect_err("超字节上限应拒绝");
        assert!(err.contains("字节"));
    }

    // -------- 占位符必含校验（ADR-003 Layer B / task_003 真实接入） --------

    #[test]
    fn validate_placeholders_tagging_para_accept_any_text() {
        // tagging 与 para 无强制占位符，纯文本片段均可接受
        for m in ["tagging", "para"] {
            assert!(validate_placeholders(m, "").is_ok());
            assert!(validate_placeholders(m, "任意自定义文字").is_ok());
        }
    }

    #[test]
    fn validate_placeholders_concept_rejects_missing_content() {
        let err = validate_placeholders("concept", "我的概念抽取规则但没有占位")
            .expect_err("应拒绝");
        assert!(err.contains("{content}"));
        assert!(err.contains("知识概念提取"), "应含模块中文名: {err}");
    }

    #[test]
    fn validate_placeholders_concept_accepts_when_required_present() {
        // 即便用户大幅改写文本，只要保留 {content} 必含占位符即放行
        assert!(validate_placeholders("concept", "请抽取概念：{content}").is_ok());
        assert!(validate_placeholders("concept", prompt_runtime::CONCEPT_DEFAULT).is_ok());
    }

    #[test]
    fn validate_placeholders_aggregation_rejects_missing_concept_name() {
        let err = validate_placeholders("aggregation", "聚合一下：{cases}")
            .expect_err("应拒绝");
        assert!(err.contains("{concept_name}"));
    }

    #[test]
    fn validate_placeholders_aggregation_accepts_when_required_present() {
        assert!(validate_placeholders("aggregation", "概念 {concept_name} 的聚合：{cases}").is_ok());
    }

    // -------- assemble_prompt_info 行为（task_003 起回填真实默认值） --------

    #[test]
    fn assemble_prompt_info_none_row_returns_real_defaults() {
        let info = assemble_prompt_info("tagging", None);
        assert_eq!(info.module, "tagging");
        assert_eq!(info.display_title, "文件打标签");
        // task_003 起：default_text 应为真实 TAGGING_DEFAULT（非占位字符串）
        assert_eq!(info.default_text, prompt_runtime::TAGGING_DEFAULT);
        assert!(info.user_text.is_none());
        assert!(!info.is_custom);
        assert_eq!(info.builtin_version, "1.0");
        assert!(info.updated_at.is_none());
        // tagging 无强制占位符
        assert!(info.required_placeholders.is_empty());
        assert_eq!(info.max_bytes, prompt_runtime::MAX_USER_PROMPT_BYTES);
    }

    #[test]
    fn assemble_prompt_info_concept_carries_real_required_placeholder() {
        let info = assemble_prompt_info("concept", None);
        assert_eq!(info.display_title, "知识概念提取");
        assert!(info.default_text.contains("{content}"));
        assert_eq!(info.required_placeholders, vec!["{content}".to_string()]);
    }

    #[test]
    fn assemble_prompt_info_aggregation_carries_real_required_placeholder() {
        let info = assemble_prompt_info("aggregation", None);
        assert_eq!(info.display_title, "知识聚合");
        assert!(info.default_text.contains("{concept_name}"));
        assert_eq!(
            info.required_placeholders,
            vec!["{concept_name}".to_string()]
        );
    }

    #[test]
    fn assemble_prompt_info_with_row_carries_user_text() {
        let row = db_user_prompt::UserPromptRow {
            module: "concept".into(),
            prompt_text: "我的概念抽取 prompt".into(),
            is_custom: true,
            builtin_version: "1.0".into(),
            updated_at: "2026-05-15T10:00:00+00:00".into(),
        };
        let info = assemble_prompt_info("concept", Some(row));
        assert_eq!(info.module, "concept");
        assert_eq!(info.display_title, "知识概念提取");
        assert_eq!(info.user_text.as_deref(), Some("我的概念抽取 prompt"));
        assert!(info.is_custom);
        assert_eq!(info.updated_at.as_deref(), Some("2026-05-15T10:00:00+00:00"));
    }

    // -------- 集成测试：直接驱动 DB 层 + 内部 assemble，绕开 Tauri State --------
    // `#[tauri::command]` 外壳依赖 Tauri Manager 的 State 机制，在 cargo test --lib
    // 环境下没有合适的 App 实例。我们改为直接调用底层 DB 函数 + assemble_prompt_info
    // 验证 save → get → reset → get 全链路语义。

    #[test]
    fn integration_save_get_reset_get_roundtrip() {
        let conn = fresh_conn();

        // save —— para 无强制占位符，任意纯文本通过
        validate_module("para").unwrap();
        validate_byte_len("我的 PARA prompt").unwrap();
        validate_placeholders("para", "我的 PARA prompt").unwrap();
        db_user_prompt::upsert(&conn, "para", "我的 PARA prompt").unwrap();

        // get → 应带 user_text + is_custom=true
        let row = db_user_prompt::get(&conn, "para").unwrap();
        let info = assemble_prompt_info("para", row);
        assert_eq!(info.user_text.as_deref(), Some("我的 PARA prompt"));
        assert!(info.is_custom);

        // reset 单条
        db_user_prompt::delete(&conn, "para").unwrap();

        // get → 应回到 None + is_custom=false（回退到内置默认）
        let row2 = db_user_prompt::get(&conn, "para").unwrap();
        let info2 = assemble_prompt_info("para", row2);
        assert!(info2.user_text.is_none());
        assert!(!info2.is_custom);
        // task_003 起：默认文本是真实 PARA_DEFAULT
        assert_eq!(info2.default_text, prompt_runtime::PARA_DEFAULT);
    }

    #[test]
    fn integration_save_concept_requires_placeholder_check() {
        // task_003 ADR-003 Layer B：concept 缺 {content} 应被拒
        let bad = "请抽取概念，但我忘了占位符";
        assert!(validate_placeholders("concept", bad).is_err());

        // 含 {content} 通过
        let good = "请按我的方式抽取：{content}";
        assert!(validate_placeholders("concept", good).is_ok());
    }

    #[test]
    fn integration_reset_none_deletes_all_four_modules() {
        let conn = fresh_conn();
        // 写入 4 个 module
        for m in ["tagging", "para", "concept", "aggregation"] {
            db_user_prompt::upsert(&conn, m, &format!("custom-{m}")).unwrap();
        }
        assert_eq!(db_user_prompt::list_all(&conn).unwrap().len(), 4);

        // reset_user_prompt(None) 语义 = delete_all
        db_user_prompt::delete_all(&conn).unwrap();

        assert!(db_user_prompt::list_all(&conn).unwrap().is_empty());
        // list_user_prompts 仍应恒返 4 条（user_text 全 None）
        let mut all = Vec::new();
        for m in MODULES {
            let row = db_user_prompt::get(&conn, m).unwrap();
            all.push(assemble_prompt_info(m, row));
        }
        assert_eq!(all.len(), 4);
        for info in &all {
            assert!(info.user_text.is_none(), "{} 应已被 reset", info.module);
            assert!(!info.is_custom);
        }
    }

    #[test]
    fn integration_list_returns_four_in_fixed_order_on_empty_db() {
        let conn = fresh_conn();
        // 模拟 list_user_prompts 主流程
        let rows = db_user_prompt::list_all(&conn).unwrap();
        let mut by_module: std::collections::HashMap<String, db_user_prompt::UserPromptRow> =
            rows.into_iter().map(|r| (r.module.clone(), r)).collect();
        let mut out = Vec::new();
        for m in MODULES {
            let row = by_module.remove(*m);
            out.push(assemble_prompt_info(m, row));
        }
        // 恒 4 条 + 顺序固定 = MODULES 的顺序
        assert_eq!(out.len(), 4);
        assert_eq!(out[0].module, "tagging");
        assert_eq!(out[1].module, "para");
        assert_eq!(out[2].module, "concept");
        assert_eq!(out[3].module, "aggregation");
        // 空 DB 上每条 user_text 都是 None
        for info in &out {
            assert!(info.user_text.is_none());
            assert!(!info.is_custom);
        }
    }

    #[test]
    fn integration_save_then_list_includes_user_text_for_saved_module_only() {
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "concept", "我的 concept").unwrap();

        let rows = db_user_prompt::list_all(&conn).unwrap();
        let mut by_module: std::collections::HashMap<String, db_user_prompt::UserPromptRow> =
            rows.into_iter().map(|r| (r.module.clone(), r)).collect();
        let mut out = Vec::new();
        for m in MODULES {
            let row = by_module.remove(*m);
            out.push(assemble_prompt_info(m, row));
        }
        let concept = out.iter().find(|i| i.module == "concept").unwrap();
        assert_eq!(concept.user_text.as_deref(), Some("我的 concept"));
        assert!(concept.is_custom);
        for other in out.iter().filter(|i| i.module != "concept") {
            assert!(other.user_text.is_none(), "{} 未保存应为 None", other.module);
            assert!(!other.is_custom);
        }
    }

    // -------- ensure_writable 守卫 --------

    #[test]
    fn ensure_writable_blocks_readonly_mode_for_writes() {
        // 验证写命令必经的 ensure_writable 守卫与 startup.rs 的行为一致
        let ro = AppMode::ReadOnly {
            reason: "测试只读".into(),
        };
        assert!(ensure_writable(&ro).is_err());

        let normal = AppMode::Normal;
        assert!(ensure_writable(&normal).is_ok());

        let degraded = AppMode::Degraded {
            reason: "x".into(),
            failed_count: 1,
        };
        assert!(ensure_writable(&degraded).is_ok(), "Degraded 仍允许写");
    }
}
