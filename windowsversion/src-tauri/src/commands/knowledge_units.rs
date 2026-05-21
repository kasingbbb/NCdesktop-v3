use crate::db::knowledge_units::{
    self, AssetInference, CreateKnowledgeUnit, CreateSnapshot, KnowledgeUnit,
    KnowledgeUnitSummary, UnderstandingSnapshot, VoiceMemo,
};
use crate::db::Database;
use tauri::State;

// ─── 知识单元 CRUD ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ku_get_list(
    db: State<'_, Database>,
    library_id: String,
) -> Result<Vec<KnowledgeUnitSummary>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_knowledge_units_summary(&conn, &library_id)
}

#[tauri::command]
pub async fn ku_get_detail(
    db: State<'_, Database>,
    id: String,
) -> Result<Option<KnowledgeUnit>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_knowledge_unit(&conn, &id)
}

#[tauri::command]
pub async fn ku_create(
    db: State<'_, Database>,
    unit: CreateKnowledgeUnit,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::insert_knowledge_unit(&conn, &unit)
}

#[tauri::command]
pub async fn ku_update_status(
    db: State<'_, Database>,
    id: String,
    status: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::update_knowledge_unit_status(&conn, &id, &status, &now)
}

#[tauri::command]
pub async fn ku_update_note(
    db: State<'_, Database>,
    id: String,
    user_note: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::update_knowledge_unit_note(&conn, &id, &user_note, &now)
}

#[tauri::command]
pub async fn ku_update_mirror_feedback(
    db: State<'_, Database>,
    id: String,
    feedback_json: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::update_knowledge_unit_mirror_feedback(&conn, &id, &feedback_json, &now)
}

#[tauri::command]
pub async fn ku_update_review_schedule(
    db: State<'_, Database>,
    id: String,
    next_review_due: Option<String>,
    depth_level: i64,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::update_knowledge_unit_review_schedule(
        &conn,
        &id,
        next_review_due.as_deref(),
        depth_level,
        &now,
    )
}

#[tauri::command]
pub async fn ku_delete(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::delete_knowledge_unit(&conn, &id)
}

#[tauri::command]
pub async fn ku_get_due_for_review(
    db: State<'_, Database>,
    library_id: String,
    limit: Option<i64>,
) -> Result<Vec<KnowledgeUnitSummary>, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_knowledge_units_due_for_review(&conn, &library_id, &today, limit.unwrap_or(3))
}

// ─── 理解快照 ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ku_get_snapshots(
    db: State<'_, Database>,
    knowledge_unit_id: String,
) -> Result<Vec<UnderstandingSnapshot>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_snapshots(&conn, &knowledge_unit_id)
}

#[tauri::command]
pub async fn ku_create_snapshot(
    db: State<'_, Database>,
    snapshot: CreateSnapshot,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::insert_snapshot(&conn, &snapshot)
}

// ─── 素材推断 ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ku_upsert_inference(
    db: State<'_, Database>,
    inference: AssetInference,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::upsert_asset_inference(&conn, &inference)
}

#[tauri::command]
pub async fn ku_get_inference(
    db: State<'_, Database>,
    asset_id: String,
) -> Result<Option<AssetInference>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_asset_inference(&conn, &asset_id)
}

// ─── 语音备注 ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ku_create_voice_memo(
    db: State<'_, Database>,
    memo: VoiceMemo,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::insert_voice_memo(&conn, &memo)
}

#[tauri::command]
pub async fn ku_classify_voice_memo(
    db: State<'_, Database>,
    id: String,
    memo_type: String,
    link_target_id: Option<String>,
    link_reason: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::update_voice_memo_classification(
        &conn,
        &id,
        &memo_type,
        link_target_id.as_deref(),
        link_reason.as_deref(),
    )
}

#[tauri::command]
pub async fn ku_get_unarchived_voice_memos(
    db: State<'_, Database>,
) -> Result<Vec<VoiceMemo>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_voice_memos_unarchived(&conn)
}

#[tauri::command]
pub async fn ku_get_voice_memos_for_unit(
    db: State<'_, Database>,
    knowledge_unit_id: String,
) -> Result<Vec<VoiceMemo>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    knowledge_units::get_voice_memos_for_unit(&conn, &knowledge_unit_id)
}
