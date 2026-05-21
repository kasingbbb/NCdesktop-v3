/// 技能封装与 MCP 导出命令（Step 11）
///
/// 命令列表：
///   skill_export_package    — 将已验证技能导出为 SkillPackage JSON 文件
///   skill_start_mcp_server  — 启动 localhost MCP 服务器
///   skill_stop_mcp_server   — 停止 MCP 服务器
///   skill_get_mcp_server_status — 获取服务器状态（端口/URL）
///   skill_get_mcp_config    — 生成 Claude Desktop / Cursor 配置片段

use crate::db::knowledge_units::get_knowledge_unit;
use crate::db::skills::get_skill;
use crate::db::Database;
use crate::mcp::server::{McpServerManager, McpServerStatus, skill_tool_name};
use serde::{Deserialize, Serialize};
use tauri::State;

// ─── SkillPackage ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackageKu {
    pub id: String,
    pub title: String,
    pub core_insight: String,
    pub summary: Option<String>,
    pub status: String,
    pub depth_level: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackage {
    /// 格式版本
    pub version: String,
    pub exported_at: String,
    pub skill_id: String,
    pub skill_name: String,
    pub skill_description: Option<String>,
    pub skill_status: String,
    pub verified_at: Option<String>,
    pub progress: f64,
    /// 知识单元摘要列表
    pub knowledge_units: Vec<SkillPackageKu>,
    /// 最后一次评估原始 JSON（透传）
    pub last_evaluation: Option<serde_json::Value>,
    /// MCP Tool 定义（供 claude_desktop_config.json 或 .mcp.json 使用）
    pub mcp_tool_definition: serde_json::Value,
}

// ─── skill_export_package ─────────────────────────────────────────────────────

/// 将已验证的技能导出为 SkillPackage JSON 文件。
/// output_path：完整文件路径，若为空则返回 JSON 字符串（不写文件）。
#[tauri::command]
pub async fn skill_export_package(
    skill_id: String,
    output_path: String,
    db: State<'_, Database>,
) -> Result<String, String> {
    let skill = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        get_skill(&conn, &skill_id)?
    };

    let skill = skill.ok_or_else(|| format!("技能 {skill_id} 不存在"))?;

    if skill.status != "verified" {
        return Err("只能导出已验证（verified）的技能".to_string());
    }

    // 加载知识单元
    let mut kus: Vec<SkillPackageKu> = Vec::new();
    for kid in &skill.ku_ids {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        if let Ok(Some(ku)) = get_knowledge_unit(&conn, kid) {
            kus.push(SkillPackageKu {
                id: ku.id,
                title: ku.title,
                core_insight: ku.core_insight,
                summary: ku.summary,
                status: ku.status,
                depth_level: ku.depth_level,
            });
        }
    }

    let last_evaluation: Option<serde_json::Value> = skill
        .last_evaluation
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let tool_name = skill_tool_name(&skill.id);
    let mcp_tool_definition = serde_json::json!({
        "name": tool_name,
        "description": format!(
            "Query the '{}' knowledge base. Verified skill from NoteCapt.{}",
            skill.name,
            skill.description.as_deref().map(|d| format!(" {d}")).unwrap_or_default()
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Your question about this skill",
                }
            },
            "required": ["query"],
        }
    });

    let package = SkillPackage {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        skill_description: skill.description.clone(),
        skill_status: skill.status.clone(),
        verified_at: skill.verified_at.clone(),
        progress: skill.progress,
        knowledge_units: kus,
        last_evaluation,
        mcp_tool_definition,
    };

    let json = serde_json::to_string_pretty(&package)
        .map_err(|e| format!("序列化失败: {e}"))?;

    if output_path.is_empty() {
        // 直接返回 JSON 字符串
        return Ok(json);
    }

    std::fs::write(&output_path, &json)
        .map_err(|e| format!("写文件失败 {output_path}: {e}"))?;

    Ok(output_path)
}

// ─── skill_start_mcp_server ───────────────────────────────────────────────────

#[tauri::command]
pub async fn skill_start_mcp_server(
    library_id: String,
    mcp: State<'_, McpServerManager>,
) -> Result<McpServerStatus, String> {
    let db_path = mcp.db_path.clone();
    mcp.start(library_id, db_path)
}

// ─── skill_stop_mcp_server ────────────────────────────────────────────────────

#[tauri::command]
pub async fn skill_stop_mcp_server(
    mcp: State<'_, McpServerManager>,
) -> Result<bool, String> {
    Ok(mcp.stop())
}

// ─── skill_get_mcp_server_status ─────────────────────────────────────────────

#[tauri::command]
pub async fn skill_get_mcp_server_status(
    mcp: State<'_, McpServerManager>,
) -> Result<McpServerStatus, String> {
    Ok(mcp.status())
}

// ─── skill_get_mcp_config ─────────────────────────────────────────────────────

/// 返回可粘贴进 Claude Desktop 或 Cursor 的 MCP 配置 JSON 片段。
#[tauri::command]
pub async fn skill_get_mcp_config(
    port: u16,
) -> Result<String, String> {
    let config = serde_json::json!({
        "mcpServers": {
            "notecapt-skills": {
                "type": "http",
                "url": format!("http://127.0.0.1:{port}"),
            }
        }
    });
    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}
