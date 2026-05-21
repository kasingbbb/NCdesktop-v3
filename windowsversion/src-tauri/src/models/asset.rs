use serde::{Deserialize, Serialize};

/// 工作区列表四态（task_001_architect ADR-003）。
///
/// 由 `db::asset::compute_asset_state` 实时派生，**不**持久化到 DB —— 任何
/// 触发器/缓存列方案都会破坏零迁移底线（见 session_context.md §3）。
///
/// 序列化为小写字符串以便前端做联合类型与文案映射：
/// `"done" | "converting" | "failed" | "offline"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetState {
    /// markdown rendition 已落盘且 pipeline 标记完成
    Done,
    /// pipeline_tasks 处于 queued / running
    Converting,
    /// pipeline_tasks failed 或最近一条 conversion_meta.error_class != NULL
    Failed,
    /// 既无活跃 pipeline，也无 rendition，亦非显式失败（含未入队 / 重启后丢失任务等）
    Offline,
}

/// 工作区列表 DTO（task_001_architect §六 数据模型 + ADR-002）。
///
/// 与 [`Asset`] 区分：
/// - `Asset` 是 DB 行的直接映射，用于非工作区视图（搜索 / 时间轴 / 知识中枢）；
/// - `WorkspaceAssetView` 是 root asset + 派生关联 + 实时四态的"工作区折叠视图"，
///   由 `db::asset::list_root_assets` 单一查询入口返回，避免双条目复发（PRD §9 R1）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAssetView {
    // ---- root asset 字段（与 Asset 同形）----
    pub id: String,
    pub project_id: String,
    pub asset_type: String,
    pub name: String,
    pub original_name: String,
    pub file_path: String, // source 绝对路径
    pub file_size: i64,
    pub mime_type: String,
    pub captured_at: String,
    pub imported_at: String,
    pub source_type: String,
    pub source_data: Option<String>,
    pub is_starred: bool,
    pub derivative_version: i32,

    // ---- 派生 / 关联（来自 LEFT JOIN 与 fs::stat）----
    /// canonical markdown 衍生件 id（若存在）
    pub rendition_id: Option<String>,
    /// canonical markdown 绝对路径（若存在）
    pub rendition_path: Option<String>,
    pub rendition_size: Option<i64>,
    /// 四态（实时派生）
    pub state: AssetState,
    /// 用户可见的失败原因（error_class / source-missing 等），中文文案在前端组合
    pub state_reason: Option<String>,
    /// 源文件在磁盘上不存在（来自 task_007 内存态 SourceMissingSet；
    /// 在 task_007 未注册前恒为 false）
    pub source_missing: bool,
    /// task_014 Fix-A4：extracted_content.extractor_type。
    /// 前端用于区分 `placeholder_*` (占位) vs 真 MD（即使 state="done"）。
    /// None = 该 root 尚无 extracted_content 行 / 空字符串。
    pub extractor_type: Option<String>,
    /// task_014 AC-4：最近一行 `conversion_meta.failure_code`。
    /// - `"legacy_unverified"`：旧版本"成功 + 空内容"被 V14 backfill 标注；
    /// - `"E_*"`（8 字面之一）：明确失败；
    /// - `None`：当前为成功 / 未尝试。
    /// 前端用于渲染三态 badge（与 `state` 互补：state 是工作区四态宏观视图，
    /// 本字段聚焦"提取链路"的微观失败原因）。
    pub extraction_failure_code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub asset_type: String,
    /// 工作区内展示名（可被 AI / 用户重命名）
    pub name: String,
    /// 拖入时的原始文件名，仅副本在应用目录内被整理，此字段用于对照原件
    #[serde(default)]
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub captured_at: String,
    pub imported_at: String,
    pub source_type: String,
    pub source_data: Option<String>,
    pub is_starred: bool,
    /// 若本 asset 是某个原件的 canonical markdown 衍生件，则指向原件 id；
    /// 原件本身该字段为 None。由 task_001_architect ADR-001 确立。
    #[serde(default)]
    pub source_asset_id: Option<String>,
    /// 该 root asset 已被成功转换的轮次计数（source 与 derivative 双写，
    /// 但只在真成功路径推进；placeholder 不推进，见 ADR-002 / ADR-006）。
    #[serde(default)]
    pub derivative_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIAnalysisRow {
    pub id: String,
    pub asset_id: String,
    pub summary: String,
    pub topics: String,
    pub ocr_text: Option<String>,
    pub language: String,
    pub suggested_tags: String,
    pub suggested_name: String,
}
