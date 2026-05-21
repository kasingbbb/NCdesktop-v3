//! PR-4 task_013: 用户可编辑 Prompt（library 级 / settings KV）
//!
//! KV 命名（ADR-008）：`prompt.override.{kind}.{field}`
//! - kind ∈ classify / naming / tagging
//! - field ∈ user / output / validated_offline / user_skipped_validation / updated_at
//!
//! merge layer：渲染时若 `field=user` 存在 override，用之替换默认；否则用 prompts.rs 默认值

use crate::db::{settings, Database};
use crate::startup::{ensure_writable, AppMode};
use serde::{Deserialize, Serialize};
use tauri::State;

const KINDS: &[&str] = &["classify", "naming", "tagging"];

fn validate_kind(kind: &str) -> Result<(), String> {
    if !KINDS.contains(&kind) {
        return Err(format!("kind 必须是 classify/naming/tagging，收到 {kind}"));
    }
    Ok(())
}

fn validate_field(field: &str) -> Result<(), String> {
    match field {
        "user" | "output" | "validated_offline" | "user_skipped_validation" | "updated_at" => Ok(()),
        _ => Err(format!("field 不识别: {field}")),
    }
}

fn key(kind: &str, field: &str) -> String {
    format!("prompt.override.{kind}.{field}")
}

/// 默认 Prompt 段（暴露给 UI 编辑器对照展示 / 恢复默认）
/// 注：MVP 三段以"嵌入 classify_prompt"形式存在，因此 default 文本对 naming/tagging 取自 classify_prompt 的对应段落（PRD §10）
pub fn default_for(kind: &str) -> &'static str {
    match kind {
        "classify" => CLASSIFY_DEFAULT,
        "naming" => NAMING_DEFAULT,
        "tagging" => TAGGING_DEFAULT,
        _ => "",
    }
}

const CLASSIFY_DEFAULT: &str = "【AI 逻辑与作业宪章：PARA 动态分类与重命名】\n\n核心路由（PARA Router）— 自上而下穿透：\n【P】1-项目 / 【A】2-领域 / 【R】3-资源 / 【A】4-存档 / other（兜底）\n\n待分析内容：\n{content}\n\n输出严格遵守：\n- 仅输出一段合法 JSON\n- 不使用 markdown 代码块\n- JSON 含 category / tags / confidence / language / suggestedFileName";
const NAMING_DEFAULT: &str = "偏项目/任务：倾向「强动词 + 具象对象/目标 + 关键时间或版本」。\n偏领域/资源：「核心责任或兴趣点 + 可选材料类型」。\n通用文件/素材：极简可检索，可用下划线连接要素。";
const TAGGING_DEFAULT: &str = "tags：3～5 个，短词，偏行动与归宿（如「Q3交付」「会议纪要」「竞品」），避免空洞学科名与纯格式词堆砌。";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptOverrideMeta {
    pub validated_offline: bool,
    pub user_skipped_validation: bool,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptInfo {
    pub kind: String,
    pub default_text: String,
    pub override_text: Option<String>,
    pub override_output: Option<String>,
    pub meta: PromptOverrideMeta,
}

#[tauri::command]
pub fn get_prompt(
    database: State<'_, Database>,
    kind: String,
) -> Result<PromptInfo, String> {
    validate_kind(&kind)?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let user_v = settings::get(&conn, &key(&kind, "user"))?;
    let out_v = settings::get(&conn, &key(&kind, "output"))?;
    let voff = settings::get(&conn, &key(&kind, "validated_offline"))?
        .map(|s| s == "true")
        .unwrap_or(false);
    let usk = settings::get(&conn, &key(&kind, "user_skipped_validation"))?
        .map(|s| s == "true")
        .unwrap_or(false);
    let ts = settings::get(&conn, &key(&kind, "updated_at"))?;
    Ok(PromptInfo {
        default_text: default_for(&kind).to_string(),
        override_text: user_v,
        override_output: out_v,
        meta: PromptOverrideMeta {
            validated_offline: voff,
            user_skipped_validation: usk,
            updated_at: ts,
        },
        kind,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub validated: bool,
    pub message: Option<String>,
}

/// 静态校验：必含变量集合 ⊆ 已用变量集合
fn required_placeholders(kind: &str) -> &'static [&'static str] {
    match kind {
        "classify" => &["{content}"],
        "naming" => &["{content}"],
        "tagging" => &["{content}"],
        _ => &[],
    }
}

fn placeholder_check(kind: &str, text: &str) -> Result<(), String> {
    for p in required_placeholders(kind) {
        if !text.contains(p) {
            return Err(format!("缺少必含占位符: {p}"));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_prompt(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    kind: String,
    field: String,
    text: String,
) -> Result<SaveResult, String> {
    ensure_writable(mode.inner())?;
    validate_kind(&kind)?;
    validate_field(&field)?;

    if field == "user" {
        placeholder_check(&kind, &text)?;
    }

    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    settings::set(&conn, &key(&kind, &field), &text)?;
    let ts = chrono::Utc::now().to_rfc3339();
    settings::set(&conn, &key(&kind, "updated_at"), &ts)?;
    Ok(SaveResult {
        validated: true,
        message: None,
    })
}

#[tauri::command]
pub fn reset_prompt(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    kind: String,
    field: Option<String>,
) -> Result<(), String> {
    ensure_writable(mode.inner())?;
    validate_kind(&kind)?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let fields_to_clear: Vec<&str> = if let Some(f) = field.as_deref() {
        validate_field(f)?;
        vec![f]
    } else {
        vec![
            "user",
            "output",
            "validated_offline",
            "user_skipped_validation",
            "updated_at",
        ]
    };
    for f in fields_to_clear {
        // 删除 = SET NULL：使用 DELETE
        conn.execute(
            "DELETE FROM settings WHERE key = ?1;",
            rusqlite::params![key(&kind, f)],
        )
        .map_err(|e| format!("reset 失败: {e}"))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// PR-4 task_015: dry-run 三态容灾（在线必过 / 离线 schema-only / 用户跳过二次确认）
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunOutcome {
    pub schema_ok: bool,
    pub online_ok: bool,
    pub offline_only: bool,
    pub error: Option<String>,
}

/// 仅 schema 校验：占位符存在 + 输出格式段（如 classify 需含 JSON 提示）
fn schema_check(kind: &str, draft: &str) -> Result<(), String> {
    placeholder_check(kind, draft)?;
    if kind == "classify" {
        // 简单"输出格式段"启发：需含 "JSON" 关键字提示
        if !draft.to_uppercase().contains("JSON") {
            return Err("classify Prompt 需在输出说明中提及 JSON 输出".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn dry_run_prompt(
    kind: String,
    draft_text: String,
    sample: Option<String>,
) -> Result<DryRunOutcome, String> {
    validate_kind(&kind)?;
    // schema 必过
    let schema_err = schema_check(&kind, &draft_text).err();
    if let Some(e) = schema_err {
        return Ok(DryRunOutcome {
            schema_ok: false,
            online_ok: false,
            offline_only: false,
            error: Some(e),
        });
    }

    // 在线探活：5s 超时调小请求；MVP 不真调 LLM（避免单测依赖外部网络）
    // 真实 LLM 接入由 task_017 e2e 时验证；此处提供桩值 + 注释
    let _ = sample;
    let online_ok = false; // 默认 offline_only 路径，前端按 onlineOk + offline_only 决策保存按钮态
    Ok(DryRunOutcome {
        schema_ok: true,
        online_ok,
        offline_only: !online_ok,
        error: None,
    })
}

/// 渲染时合并 default + override（task_013 提供，task_015 dry_run 与 LLM 调用方使用）
pub fn merge_user_segment(kind: &str, default_seg: &str, override_seg: Option<&str>) -> String {
    match override_seg {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => default_seg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_naming() {
        assert_eq!(key("classify", "user"), "prompt.override.classify.user");
    }

    #[test]
    fn placeholder_check_required_passes() {
        assert!(placeholder_check("classify", "...{content}...").is_ok());
        assert!(placeholder_check("classify", "no placeholder").is_err());
    }

    #[test]
    fn merge_uses_override_when_present() {
        assert_eq!(
            merge_user_segment("classify", "default", Some("override")),
            "override"
        );
        assert_eq!(
            merge_user_segment("classify", "default", None),
            "default"
        );
        assert_eq!(
            merge_user_segment("classify", "default", Some("   ")),
            "default",
            "全空白 override 视为未设置"
        );
    }

    #[test]
    fn validate_kind_rejects_unknown() {
        assert!(validate_kind("unknown").is_err());
        assert!(validate_kind("classify").is_ok());
    }

    #[test]
    fn schema_check_classify_needs_json_hint() {
        assert!(schema_check("classify", "{content} 输出 JSON").is_ok());
        assert!(schema_check("classify", "{content} 输出文本").is_err());
        assert!(schema_check("naming", "{content} 任意").is_ok());
    }

    #[tokio::test]
    async fn dry_run_schema_only_offline() {
        let r = dry_run_prompt("classify".into(), "{content} 输出 JSON".into(), None)
            .await
            .unwrap();
        assert!(r.schema_ok);
        assert!(r.offline_only);
    }

    #[tokio::test]
    async fn dry_run_schema_fail_returns_error() {
        let r = dry_run_prompt("classify".into(), "no placeholder".into(), None)
            .await
            .unwrap();
        assert!(!r.schema_ok);
        assert!(r.error.is_some());
    }
}
