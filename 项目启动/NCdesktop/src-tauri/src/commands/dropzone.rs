use crate::db::{self, Database};
use crate::extraction::scheduler::PipelineScheduler;
use crate::llm::client::LLMClient;
use crate::models;
use crate::workspace;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

const SETTING_ACTIVE_PROJECT: &str = "ui.active_project_id";

/// task_002 / ADR-006：将 enqueue 抽象成最小 trait，使 `import_files_core`
/// 可以脱离 Tauri AppHandle 接受单测。
///
/// 生产实现 `AppHandleEnqueue` 仍委托 `PipelineScheduler::enqueue(&app, id)`，
/// 该静态方法会自取 `app.state::<Database>()` 的锁写 `pipeline_tasks` /
/// `extracted_content`；因此核心函数**必须**在调用 `enqueue` 时**不**持有
/// DB MutexGuard（否则死锁）。
pub trait EnqueueScheduler {
    fn enqueue(&self, asset_id: &str) -> Result<String, String>;
}

/// 生产路径：透传到 `PipelineScheduler::enqueue`。
pub struct AppHandleEnqueue<'a> {
    pub app: &'a AppHandle,
}

impl<'a> EnqueueScheduler for AppHandleEnqueue<'a> {
    fn enqueue(&self, asset_id: &str) -> Result<String, String> {
        PipelineScheduler::enqueue(self.app, asset_id)
    }
}

/// 核心导入产物：除 `ImportDropSummary` 外，回吐每个 asset 的 AI 分类输入，
/// 由命令薄包装层决定是否触发后台 AI 旁路。该结构仅进程内传递，不序列化给前端。
pub struct ImportCoreOutput {
    pub summary: ImportDropSummary,
    /// 与 `summary.created` 一一对应：每条 (asset, classify_input)。
    pub ai_pending_jobs: Vec<(models::Asset, String)>,
}

/// 单条拖入导入结果（扁平序列化：与 `Asset` 字段同层，便于前端沿用 `Asset` 类型）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDropCreated {
    #[serde(flatten)]
    pub asset: models::Asset,
    /// LLM 分类与写入 `ai_analyses` 是否成功
    pub ai_classified: bool,
    /// 失败或未配置时的说明；成功一般为 `None`
    pub ai_note: Option<String>,
    /// `true` 表示已提交后台任务，前端可显示「分析中」
    #[serde(default)]
    pub ai_pending: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportDropSummary {
    pub created: Vec<ImportDropCreated>,
    pub failures: Vec<String>,
    /// 本次导入落库的项目名称（便于悬浮窗提示用户去主页哪里找）
    pub import_project_name: String,
    /// ADR-006：已落库但入队失败的 asset_id 列表。
    /// 不删 asset 行 / 不删源文件，由 M5 重试或 M9 离线自愈兜底；UI 可据此提示。
    #[serde(default)]
    pub failures_to_enqueue: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportDropFinishedPayload {
    project_id: String,
    import_project_name: String,
}

fn resolve_import_project_id(conn: &rusqlite::Connection) -> Result<String, String> {
    match db::settings::get(conn, SETTING_ACTIVE_PROJECT)? {
        Some(pid) if !pid.is_empty() => {
            if db::project::get_by_id(conn, &pid)?.is_some() {
                return Ok(pid);
            }
        }
        _ => {}
    }

    let libraries = db::library::get_all(conn)?;
    for lib in libraries {
        let projects = db::project::get_by_library(conn, &lib.id)?;
        if let Some(p) = projects.first() {
            return Ok(p.id.clone());
        }
    }

    Err("没有可用的项目：请先在主窗口创建或选中一个项目".to_string())
}

/// 保证存在可导入目标：优先当前选中/首个项目；否则自动建「默认知识库 + 悬浮窗导入」项目。
fn ensure_import_project_id(conn: &rusqlite::Connection) -> Result<String, String> {
    if let Ok(id) = resolve_import_project_id(conn) {
        return Ok(id);
    }

    let library_id = match db::library::get_all(conn)?.first() {
        Some(lib) => lib.id.clone(),
        None => {
            let lib = models::Library {
                id: uuid::Uuid::new_v4().to_string(),
                name: "默认知识库".to_string(),
                root_path: String::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            db::library::insert(conn, &lib)?;
            // custom_para_v1：与 commands::library::create_library 对称，
            // 自动建默认库时也需 seed 4 个 PARA 内置类目，否则 LLM 分类首次落盘没目标。
            if let Err(e) = db::categories::seed_builtin_categories(conn, &lib.id) {
                log::warn!("自动建默认知识库后 seed 内置类目失败: {}", e);
            }
            lib.id
        }
    };

    let project = if let Some(p) = db::project::get_by_library(conn, &library_id)?.first() {
        p.clone()
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let p = models::Project {
            id: uuid::Uuid::new_v4().to_string(),
            library_id: library_id.clone(),
            name: "悬浮窗导入".to_string(),
            description: String::new(),
            cover_asset_id: None,
            source_type: "dropzone_auto".to_string(),
            source_data: None,
            is_pinned: false,
            is_archived: false,
            created_at: now.clone(),
            updated_at: now,
            total_duration: None,
            asset_count: 0,
            word_count: 0,
            imported_at: None,
        };
        db::project::insert(conn, &p)?;
        p
    };

    db::settings::set(conn, SETTING_ACTIVE_PROJECT, &project.id)?;
    Ok(project.id)
}

fn sanitize_file_stem(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '"' | '*' | '?' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .take(120)
        .collect();
    let t = t.trim().trim_matches('.').to_string();
    if t.is_empty() {
        "file".to_string()
    } else {
        t
    }
}

fn try_rename_or_copy_remove(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// custom_para_v1 / V17：把 LLM 返回的 `category` 字符串解析到一条 active 类目行
/// （含 `new:` 前缀剥离、alias 跳转、未知 slug 自动 upsert 为 `llm_generated`）。
///
/// 返回 `None` 表示 LLM 给出 `other` / 空类目 / 「明确不要整理」的兜底信号，
/// 调用方应跳过物理整理（行为与 V17 之前一致）。
fn resolve_or_create_category(
    conn: &Connection,
    library_id: &str,
    raw_category: &str,
) -> Option<crate::db::categories::CategoryRow> {
    let trimmed = raw_category.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("other") || trimmed.eq_ignore_ascii_case("none") {
        return None;
    }

    // 1. 剥离 "new:" 前缀（请求新建）
    let (requested_label, is_new_request) = if let Some(rest) = trimmed.strip_prefix("new:") {
        (rest.trim().to_string(), true)
    } else if let Some(rest) = trimmed.strip_prefix("NEW:") {
        (rest.trim().to_string(), true)
    } else {
        (trimmed.to_string(), false)
    };
    if requested_label.is_empty() {
        return None;
    }

    let slug = crate::db::categories::sanitize_slug(&requested_label);
    if slug == "other" {
        return None;
    }

    // 2. 不是显式 new: → 先查现有类目 / alias
    if !is_new_request {
        match crate::db::categories::resolve_for_slug(conn, library_id, &slug) {
            Ok(Some(row)) => return Some(row),
            Ok(None) => {
                // 未命中：当作 LLM 自动生成新类目（与 new: 等价处理）
                log::debug!(
                    "AI 整理：未知类目 {slug}（lib={library_id}），自动 upsert 为 llm_generated"
                );
            }
            Err(e) => {
                log::warn!("AI 整理：查询类目失败 {slug}: {e}");
                return None;
            }
        }
    }

    // 3. 自动 upsert 一条 llm_generated 行
    match crate::db::categories::upsert_llm_generated(conn, library_id, &slug, &requested_label) {
        Ok(row) => Some(row),
        Err(e) => {
            log::warn!("AI 整理：upsert 自动类目失败 {slug}: {e}");
            None
        }
    }
}

/// 将素材移入 `~/Downloads/NoteCaptWorkPlace/<projectId>/organized/<slug>/`，并按模型建议重命名（保留 `assetId` 前缀防冲突）
///
/// custom_para_v1 / V17：`category_slug` 由 [`resolve_or_create_category`] 提供，
/// 不再用 `sanitize_path_segment` 直接渲染裸字符串；同时返回 `category_slug` 三元组，
/// 让上层把它写入 `assets.category_slug`。
fn organize_asset_file_after_classify(
    conn: &Connection,
    library_id: &str,
    asset: &models::Asset,
    r: &crate::llm::classify_parse::ClassifyResult,
) -> Option<(String, String, String)> {
    let old = Path::new(&asset.file_path);
    if !old.is_file() {
        return None;
    }

    let project_root = match workspace::project_workspace_dir(&asset.project_id) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("AI 整理：无法解析工作区目录: {e}");
            return None;
        }
    };
    if !old.starts_with(&project_root) {
        log::debug!(
            "AI 整理：跳过（路径不在本项目工作区目录内） {}",
            old.display()
        );
        return None;
    }

    let category = resolve_or_create_category(conn, library_id, &r.category)?;
    let category_slug = category.slug;

    let stem = if !r.suggested_file_name.is_empty() {
        sanitize_file_stem(&r.suggested_file_name)
    } else {
        Path::new(&asset.name)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(sanitize_file_stem)
            .unwrap_or_else(|| "file".to_string())
    };

    let ext = old
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty());

    let organized_dir = project_root.join("organized");
    let new_dir = organized_dir.join(&category_slug);

    if let Err(e) = fs::create_dir_all(&new_dir) {
        log::warn!("AI 整理：创建目录失败 {}: {e}", new_dir.display());
        return None;
    }

    let base_name = match ext {
        Some(e) => format!("{}_{}.{}", asset.id, stem, e),
        None => format!("{}_{}", asset.id, stem),
    };
    let new_path = new_dir.join(&base_name);

    // 检查是否已经是这个路径了
    if new_path == old {
        log::debug!("AI 整理：路径未变化，跳过");
        return None;
    }

    if let Err(e) = try_rename_or_copy_remove(old, &new_path) {
        log::warn!(
            "AI 整理：移动文件失败 {} -> {}: {e}",
            old.display(),
            new_path.display()
        );
        return None;
    }

    let display_name = match ext {
        Some(e) => format!("{stem}.{e}"),
        None => stem,
    };

    Some((new_path.to_string_lossy().to_string(), display_name, category_slug))
}

/// 当分类为 `other` 等导致未进入 `organized/` 时，仍在项目工作区内将磁盘文件改为 `{assetId}_{建议主名}.ext`，避免仅改库名、磁盘仍为 `uuid_原名`。
fn rename_in_place_when_no_organize(
    asset: &models::Asset,
    r: &crate::llm::classify_parse::ClassifyResult,
) -> Option<(String, String)> {
    if r.suggested_file_name.trim().is_empty() {
        return None;
    }
    let project_root = match workspace::project_workspace_dir(&asset.project_id) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("原地重命名：{}", e);
            return None;
        }
    };
    let old = Path::new(&asset.file_path);
    if !old.is_file() {
        return None;
    }
    if !old.starts_with(&project_root) {
        log::debug!(
            "原地重命名：跳过（不在工作区内） {}",
            old.display()
        );
        return None;
    }

    let stem = sanitize_file_stem(&r.suggested_file_name);
    let ext = old
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty());

    let base_name = match ext {
        Some(e) => format!("{}_{}.{}", asset.id, stem, e),
        None => format!("{}_{}", asset.id, stem),
    };

    let parent = old.parent()?;
    let new_path = parent.join(&base_name);
    if new_path == old {
        return None;
    }
    if new_path.exists() {
        log::warn!(
            "原地重命名：目标已存在，跳过 {}",
            new_path.display()
        );
        return None;
    }

    if let Err(e) = fs::rename(old, &new_path) {
        log::warn!(
            "原地重命名失败 {} -> {}: {e}",
            old.display(),
            new_path.display()
        );
        return None;
    }

    let display_name = match ext {
        Some(e) => format!("{stem}.{e}"),
        None => stem,
    };

    Some((new_path.to_string_lossy().to_string(), display_name))
}

/// 后台任务：对已通过拖放入库的素材执行 LLM 分类并写回 `ai_analyses` / 标签（单独 DB 连接）
///
/// custom_para_v1 / V17：integration 路径上引入 `categories` 表查询/upsert，
/// `organize_asset_file_after_classify` 用 categories.slug 而非 LLM 字面字符串作为目录名；
/// 同时把解析出的 slug 写入 `assets.category_slug` 作为弱外键（不强制 FK）。
async fn apply_llm_classify_to_asset(
    database: &Database,
    asset: &models::Asset,
    classify_input: String,
) -> Result<(), String> {
    let r = crate::commands::llm::llm_classify_with_db(database, classify_input).await?;

    // V17：先在短锁内解析 library_id（asset.project_id → project.library_id），
    // 再在锁外做文件 IO；organize 需要 DB 读，因此重新取一次锁。
    let library_id = {
        let conn = database.conn()?;
        match db::project::get_by_id(&conn, &asset.project_id)? {
            Some(p) => p.library_id,
            None => {
                log::warn!(
                    "AI 整理：未找到 asset 所属 project {}，回退到首个 library",
                    asset.project_id
                );
                db::library::get_all(&conn)?
                    .first()
                    .map(|l| l.id.clone())
                    .unwrap_or_default()
            }
        }
    };

    // organize 需要 DB conn 走 resolve_or_create_category；保持短锁 + 锁外文件 IO 的惯例
    // 但这里 organize 内部既要查 DB 又要做 fs::rename，trade-off：让 organize 持锁，
    // 优于跨锁/锁外往返。生产路径下 organize 调用频率低（每个导入素材一次），可接受。
    let organized = {
        let conn = database.conn()?;
        organize_asset_file_after_classify(&conn, &library_id, asset, &r)
    };
    let had_organize = organized.is_some();
    let in_place = if !had_organize {
        rename_in_place_when_no_organize(asset, &r)
    } else {
        None
    };
    let had_in_place = in_place.is_some();

    // organized 是 (path, display_name, category_slug)；in_place 是 (path, display_name)
    let (final_path, final_name, organized_category_slug) = if let Some((p, n, slug)) = organized {
        (p, n, Some(slug))
    } else if let Some((p, n)) = in_place {
        (p, n, None)
    } else {
        (asset.file_path.clone(), asset.name.clone(), None)
    };

    let suggested_name_row = if !r.suggested_file_name.is_empty() {
        let ext = Path::new(&final_path)
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty());
        match ext {
            Some(e) => format!("{}.{}", r.suggested_file_name.trim(), e),
            None => r.suggested_file_name.trim().to_string(),
        }
    } else {
        final_name.clone()
    };

    let ai_row = models::AIAnalysisRow {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: asset.id.clone(),
        summary: "".to_string(),
        topics: r.category.clone(),
        ocr_text: None,
        language: r.language.clone(),
        suggested_tags: serde_json::to_string(&r.tags).unwrap_or_else(|_| "[]".to_string()),
        suggested_name: suggested_name_row.clone(),
    };

    let conn = database.conn()?;

    db::asset::upsert_analysis(&conn, &ai_row)
        .map_err(|e| format!("写入 AI 分析失败: {e}"))?;

    // 无论文件是否移动，都要更新 asset 表中的名称
    // organized：final_name/final_path 为 organized 目标；仅 in_place：磁盘已改名，名称用 suggested_name_row，路径用 final_path
    let (target_name, target_path) = if had_organize {
        (final_name, final_path)
    } else if had_in_place {
        (suggested_name_row, final_path)
    } else {
        (suggested_name_row, asset.file_path.clone())
    };

    db::asset::update_name_and_path(&conn, &asset.id, &target_name, &target_path)
        .map_err(|e| format!("更新素材元数据失败: {e}"))?;

    // V17：写 assets.category_slug 弱外键（仅 organized 路径才有值；in_place / 跳过保持 NULL）
    if let Some(slug) = organized_category_slug.as_deref() {
        if let Err(e) = db::asset::set_category_slug(&conn, &asset.id, Some(slug)) {
            log::warn!("AI 整理：写 category_slug 失败 asset={} slug={}: {}", asset.id, slug, e);
        }
    }

    for tag_name in r.tags {
        let tag_name = tag_name.trim();
        if tag_name.is_empty() {
            continue;
        }
        match db::tag::get_or_create_by_name(&conn, tag_name, "ai") {
            Ok(tag) => {
                if let Err(e) = db::tag::link_to_asset(&conn, &asset.id, &tag.id) {
                    log::warn!(
                        "拖放 AI 标签关联失败（{} -> {}）: {}",
                        asset.name,
                        tag_name,
                        e
                    );
                }
            }
            Err(e) => log::warn!("拖放 AI 标签「{tag_name}」: {e}"),
        }
    }

    // R6（防止标签传播多处实现）：原件被 AI 打标后，同步标签到该原件已有的 markdown 衍生件
    // 失败仅 warn，不阻断主流程（AC-3）
    if let Err(e) = db::tag::sync_tags_to_canonical_derivatives(&conn, &asset.id) {
        log::warn!(
            "AI 打标后同步标签到衍生件失败 asset_id={}: {}",
            asset.id,
            e
        );
    }

    Ok(())
}

fn spawn_dropzone_ai_job(app: &AppHandle, asset: models::Asset, classify_input: String) {
    let db_path = match app.path().app_data_dir() {
        Ok(p) => p.join("notecapt.db"),
        Err(e) => {
            log::error!("拖放 AI 后台：无法解析数据目录: {e}");
            return;
        }
    };

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let db = match Database::open(&db_path) {
            Ok(d) => d,
            Err(e) => {
                log::error!("拖放 AI 后台：打开数据库失败: {e}");
                return;
            }
        };
        let id = asset.id.clone();
        let project_id = asset.project_id.clone();
        match apply_llm_classify_to_asset(&db, &asset, classify_input).await {
            Ok(()) => {
                log::info!("拖放 AI 后台分类完成 ({id})");
                // 发送事件通知前端：AI 处理完成
                let _ = app_handle.emit(
                    "notecapt/dropzone-ai-finished",
                    serde_json::json!({
                        "assetId": id,
                        "projectId": project_id,
                    }),
                );
            }
            Err(e) => log::warn!("拖放 AI 后台分类失败 ({id}): {e}"),
        }
    });
}

fn path_asset_meta(path: &Path) -> (String, String, String) {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名")
        .to_string();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    // task_H2 修订：扩展名映射补全 + infer crate magic bytes 兜底。
    // 与 sync.rs::guess_mime_by_extension 字面对齐（避免 drift；未来抽公共 util）。
    let (asset_type, mime_owned): (&str, String) = match ext.as_str() {
        // 图片
        "jpg" | "jpeg" => ("image", "image/jpeg".into()),
        "png" => ("image", "image/png".into()),
        "gif" => ("image", "image/gif".into()),
        "webp" => ("image", "image/webp".into()),
        "heic" | "heif" => ("image", "image/heic".into()),
        "bmp" => ("image", "image/bmp".into()),
        "tiff" | "tif" => ("image", "image/tiff".into()),
        "svg" => ("image", "image/svg+xml".into()),
        // 文档
        "pdf" => ("pdf", "application/pdf".into()),
        "rtf" => ("docx", "application/rtf".into()),
        "html" | "htm" => ("html", "text/html".into()),
        "xml" => ("other", "application/xml".into()),
        "json" => ("other", "application/json".into()),
        // 表格 / 数据
        "csv" => ("csv", "text/csv".into()),
        "tsv" => ("csv", "text/tab-separated-values".into()),
        // Office
        "docx" => (
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
        ),
        "xlsx" => (
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".into(),
        ),
        "pptx" => (
            "pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation".into(),
        ),
        "doc" => ("docx", "application/msword".into()),
        "xls" => ("xlsx", "application/vnd.ms-excel".into()),
        "ppt" => ("pptx", "application/vnd.ms-powerpoint".into()),
        // 电子书 / 归档
        "epub" => ("epub", "application/epub+zip".into()),
        "zip" => ("other", "application/zip".into()),
        // 音频（task_010 路由到 iflytek）
        "mp3" => ("audio_clip", "audio/mpeg".into()),
        "wav" => ("audio_clip", "audio/wav".into()),
        "m4a" | "aac" => ("audio_clip", "audio/mp4".into()),
        "flac" => ("audio_clip", "audio/flac".into()),
        "ogg" => ("audio_clip", "audio/ogg".into()),
        "opus" => ("audio_clip", "audio/opus".into()),
        // 视频（task_010 reject）
        "mp4" => ("video", "video/mp4".into()),
        "mov" => ("video", "video/quicktime".into()),
        "webm" => ("video", "video/webm".into()),
        "mkv" => ("video", "video/x-matroska".into()),
        // 文本
        "md" | "markdown" => ("markdown", "text/markdown".into()),
        "txt" => ("markdown", "text/plain".into()),
        // 未知扩展名：用 infer crate 读 magic bytes 嗅探
        _ => {
            if let Ok(Some(kind)) = infer::get_from_path(path) {
                let m = kind.mime_type().to_string();
                let t = if m.starts_with("image/") {
                    "image"
                } else if m.starts_with("audio/") {
                    "audio_clip"
                } else if m.starts_with("video/") {
                    "video"
                } else if m == "application/pdf" {
                    "pdf"
                } else if m == "application/epub+zip" {
                    "epub"
                } else {
                    "other"
                };
                (t, m)
            } else {
                ("other", "application/octet-stream".into())
            }
        }
    };

    (asset_type.to_string(), mime_owned, name)
}

/// task_002 / ADR-006：原子导入核心 — 不持 `AppHandle`，便于单测。
///
/// 流程（**对每个 path 顺序执行**）：
///   1. `fs::copy` 源文件到 `project_workspace_dir/{asset_id}_{safe_name}`
///   2. `db::asset::insert(root)`（短锁；失败 → 计入 `failures`，物理文件保留为孤儿）
///   3. `scheduler.enqueue(asset_id)`
///        - 成功 → `created.push(...)`；
///        - 失败 → **不**删 asset 行 / **不**删源文件（ADR-006）；
///          asset_id 记入 `failures_to_enqueue`；asset 仍计入 `created`
///          （前端可见 offline 占位）。
///
/// 注：`enqueue` 调用期间**绝不**持有 DB MutexGuard，避免与 scheduler 内部
/// 重新锁 `Database` 产生死锁。`import_files_core` 本身是同步函数，没有 await，
/// 因此天然不会跨 await 持锁。
pub fn import_files_core<S: EnqueueScheduler>(
    pool: &Pool<SqliteConnectionManager>,
    scheduler: &S,
    project_id: &str,
    paths: Vec<String>,
) -> Result<ImportCoreOutput, String> {
    let mut created: Vec<ImportDropCreated> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut failures_to_enqueue: Vec<String> = Vec::new();
    let mut ai_pending_jobs: Vec<(models::Asset, String)> = Vec::new();

    let now = chrono::Utc::now().to_rfc3339();

    let project_asset_dir = workspace::ensure_project_workspace(project_id)?;
    log::info!(
        "拖入工作区目录: {}",
        project_asset_dir.display()
    );

    // 一次性读出 AI 是否可用（短锁）
    let ai_pending_global = {
        let conn = pool
            .get()
            .map_err(|e| format!("数据库连接获取失败: {e}"))?;
        LLMClient::is_available_in_conn(&conn)
    };

    for path_str in paths {
        let path = Path::new(&path_str);
        if !path.exists() {
            failures.push(format!("路径不存在: {path_str}"));
            continue;
        }
        if path.is_dir() {
            failures.push(format!("暂不支持导入文件夹: {path_str}"));
            continue;
        }

        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                failures.push(format!("无法读取文件: {path_str} — {e}"));
                continue;
            }
        };

        let (asset_type, mime_type, name) = path_asset_meta(path);

        // ── 视频导入：先用 ffmpeg 提取音频，丢弃视频本体，改为导入 .m4a ──
        //   需求：导入 mp4（及其他视频）时自动抽录音、丢弃视频、置入录音文件，
        //   随后正常走音频转写（audio_asr_iflytek）管线。
        //   做法：把音频抽到系统临时目录的 .m4a，再按音频资产 copy 进工作区；
        //   原视频不进工作区（"丢弃"），source_data 仍记录其来源路径作溯源。
        //   ffmpeg 不可用 / 抽取失败 → 记入 failures 并跳过（不把无法处理的视频塞进工作区）。
        let mut copy_src: PathBuf = path.to_path_buf();
        let mut temp_audio: Option<PathBuf> = None;
        let (asset_type, mime_type, name) = if asset_type == "video" {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(sanitize_file_stem)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "video".to_string());
            let tmp = std::env::temp_dir()
                .join(format!("nc_import_{}_{}.m4a", uuid::Uuid::new_v4(), stem));
            match crate::extraction::video_audio::extract_audio_to(path, &tmp) {
                Ok(r) => {
                    log::info!(
                        "视频导入→提取音频: {} → {}（stream_copy={}, {}ms），丢弃视频本体",
                        path_str,
                        tmp.display(),
                        r.stream_copy,
                        r.elapsed_ms
                    );
                    copy_src = tmp.clone();
                    temp_audio = Some(tmp);
                    (
                        "audio_clip".to_string(),
                        "audio/mp4".to_string(),
                        format!("{stem}.m4a"),
                    )
                }
                Err(e) => {
                    failures
                        .push(format!("视频提取音频失败（已跳过，未导入视频）: {path_str} — {e}"));
                    continue;
                }
            }
        } else {
            (asset_type, mime_type, name)
        };

        // file_size 取实际将导入的文件（视频场景为抽取出的 .m4a）
        let file_size = fs::metadata(&copy_src)
            .map(|m| m.len() as i64)
            .unwrap_or_else(|_| meta.len() as i64);
        let asset_id = uuid::Uuid::new_v4().to_string();

        let safe_name = name
            .chars()
            .map(|c| if c == '/' || c == ':' { '_' } else { c })
            .collect::<String>();
        let dest_path = project_asset_dir.join(format!("{}_{}", &asset_id, safe_name));
        if let Err(e) = fs::copy(&copy_src, &dest_path) {
            failures.push(format!(
                "复制失败: {} -> {} — {}",
                path_str,
                dest_path.display(),
                e
            ));
            if let Some(t) = &temp_audio {
                let _ = fs::remove_file(t);
            }
            continue;
        }
        // 工作区已落盘，删除临时音频（视频场景）
        if let Some(t) = &temp_audio {
            let _ = fs::remove_file(t);
        }

        let asset = models::Asset {
            id: asset_id.clone(),
            project_id: project_id.to_string(),
            asset_type,
            name: name.clone(),
            original_name: name,
            file_path: dest_path.to_string_lossy().to_string(),
            file_size,
            mime_type,
            captured_at: now.clone(),
            imported_at: now.clone(),
            source_type: "dropzone_drag".to_string(),
            source_data: Some(path_str.clone()),
            is_starred: false,
            ..Default::default()
        };

        // —— 短锁块：仅做 asset 行 INSERT；guard drop 后再 enqueue —— //
        // 修复：insert 失败时清理已 copy 到 workspace 的孤儿文件
        //（旧实现 fs::copy 已经把源文件复制到 dest_path，DB insert 失败后既无 asset 行
        //  也无任何引用，目标目录会累积"UUID 前缀+原名"的幽灵文件，用户从 UI 看不到，
        //  但持续占磁盘；UNIQUE 冲突 / 锁竞争场景下会规律性触发）。
        {
            let conn = pool
                .get()
                .map_err(|e| format!("数据库连接获取失败: {e}"))?;
            if let Err(e) = db::asset::insert(&conn, &asset) {
                drop(conn);
                if let Err(re) = fs::remove_file(&dest_path) {
                    log::error!(
                        "dropzone insert 失败后清理 dest_path 也失败：{} — {} (原 insert 错误: {})",
                        dest_path.display(),
                        re,
                        e
                    );
                } else {
                    log::warn!(
                        "dropzone insert 失败，已清理孤儿文件 {} — {}",
                        dest_path.display(),
                        e
                    );
                }
                failures.push(format!("{path_str}: {e}"));
                continue;
            }
        }

        // ADR-006：失败的 enqueue 不让 asset 行消失，也不删源文件。
        match scheduler.enqueue(&asset.id) {
            Ok(_) => {}
            Err(e) => {
                log::warn!(
                    "dropzone 入队失败 asset={}: {} — ADR-006 保留 asset 行与源文件",
                    asset.id,
                    e
                );
                failures_to_enqueue.push(asset.id.clone());
            }
        }

        // 轻量 AI 识别：构造分类输入（不阻塞，仅准备数据，命令层决定是否 spawn）
        let mut classify_input = format!(
            "文件名：{}\nMIME：{}\n资产类型：{}\n",
            asset.name, asset.mime_type, asset.asset_type
        );
        if asset.mime_type.starts_with("text/") || asset.asset_type == "markdown" {
            let mut buf = String::new();
            if let Ok(mut f) = fs::File::open(&dest_path) {
                let mut raw = vec![0u8; 32 * 1024];
                if let Ok(n) = f.read(&mut raw) {
                    raw.truncate(n);
                    buf = String::from_utf8_lossy(&raw).to_string();
                }
            }
            if !buf.trim().is_empty() {
                classify_input.push_str("\n内容片段（截断）：\n");
                classify_input.push_str(&buf);
            }
        }

        if ai_pending_global {
            ai_pending_jobs.push((asset.clone(), classify_input));
        }

        created.push(ImportDropCreated {
            asset,
            ai_classified: false,
            ai_note: if ai_pending_global {
                None
            } else {
                Some("未配置 AI，已跳过自动分类".to_string())
            },
            ai_pending: ai_pending_global,
        });
    }

    let summary = ImportDropSummary {
        created,
        failures,
        import_project_name: String::new(), // 由命令层填充（核心层不查 project 表）
        failures_to_enqueue,
    };

    Ok(ImportCoreOutput {
        summary,
        ai_pending_jobs,
    })
}

#[tauri::command]
pub async fn import_drop_paths(
    app: AppHandle,
    database: State<'_, Database>,
    paths: Vec<String>,
) -> Result<ImportDropSummary, String> {
    if paths.is_empty() {
        return Ok(ImportDropSummary::default());
    }

    // 注意：该命令是 async，不能在 await 期间持有 SQLite 的 MutexGuard
    let (project_id, import_project_name) = {
        let conn = database.conn()?;
        let pid = ensure_import_project_id(&conn)?;
        let pname = db::project::get_by_id(&conn, &pid)?
            .map(|p| p.name)
            .unwrap_or_else(|| "当前项目".to_string());
        (pid, pname)
    };

    // import_files_core 是同步且可能耗时的（尤其视频导入要跑 ffmpeg 抽音频，大文件可达数秒）。
    // 若直接在该 async 命令里同步执行，会阻塞 Tauri 的 async runtime worker，悬浮窗/主窗 UI
    // 卡死（用户实测：mp4 拖入悬浮窗即卡死）。这里挪到 blocking 线程池执行。
    // 注：import_files_core 全程不跨 await 持有 DB MutexGuard，放进 blocking 线程安全。
    let pool = database.pool.clone();
    let app_for_core = app.clone();
    let project_id_for_core = project_id.clone();
    let core_out = tokio::task::spawn_blocking(move || {
        let scheduler_adapter = AppHandleEnqueue { app: &app_for_core };
        import_files_core(&pool, &scheduler_adapter, &project_id_for_core, paths)
    })
    .await
    .map_err(|e| format!("导入任务执行失败（blocking join）: {e}"))??;

    // 若 created 非空且至少有一个成功入队，则唤醒调度循环
    let any_enqueued = core_out.summary.created.len() > core_out.summary.failures_to_enqueue.len();

    // AI 旁路：在命令层 spawn（核心层不 spawn，避免裸 tokio::spawn 扩散）
    for (asset, classify_input) in core_out.ai_pending_jobs {
        spawn_dropzone_ai_job(&app, asset, classify_input);
    }

    // W3-1：循环结束统一唤醒一次调度循环（start 自身幂等）
    if any_enqueued {
        let scheduler = app.state::<PipelineScheduler>();
        scheduler.start(app.clone());
    }

    let mut summary = core_out.summary;
    summary.import_project_name = import_project_name.clone();

    if let Err(e) = app.emit(
        "notecapt/import-drop-finished",
        ImportDropFinishedPayload {
            project_id: project_id.clone(),
            import_project_name,
        },
    ) {
        log::warn!("广播 import-drop-finished 失败: {e}");
    }

    Ok(summary)
}

/// 创建悬浮窗（系统浮动面板级别）
#[tauri::command]
pub async fn create_dropzone_window(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("dropzone").is_some() {
        log::info!("悬浮窗已存在");
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "dropzone", WebviewUrl::App("/dropzone".into()))
        .title("")
        // 默认略大于卡片；用户可拖动标题条移动、拖边角缩放（见前端）
        .inner_size(220.0, 248.0)
        .min_inner_size(140.0, 168.0)
        .max_inner_size(960.0, 1280.0)
        .resizable(true)
        .always_on_top(true)
        .decorations(false)
        .skip_taskbar(true)
        .build()
        .map_err(|e| format!("创建悬浮窗失败: {e}"))?;

    Ok(())
}

/// 关闭悬浮窗
#[tauri::command]
pub async fn close_dropzone_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("dropzone") {
        win.close().map_err(|e| format!("关闭悬浮窗失败: {e}"))?;
    }
    Ok(())
}

/// 显示/隐藏悬浮窗
#[tauri::command]
pub async fn toggle_dropzone_window(app: AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("dropzone") {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            win.hide().map_err(|e| format!("隐藏悬浮窗失败: {e}"))?;
        } else {
            win.show().map_err(|e| format!("显示悬浮窗失败: {e}"))?;
        }
        Ok(!visible)
    } else {
        create_dropzone_window(app).await?;
        Ok(true)
    }
}

// =====================================================================
// task_002 / ADR-006 — `import_files_core` 的单元测试
//
// 设计要点：
// - 不依赖 Tauri AppHandle / State / 网络；通过 `EnqueueScheduler` trait 注入
//   一个测试用假 scheduler（happy = 仅记录 + 同步写一行 pipeline_tasks；
//   failure = 永远返回 Err）。
// - 使用项目工作区真实路径 `~/Downloads/NoteCaptWorkPlace/<uuid>/`，
//   project_id 用一次性 UUID 隔离；测试结束 best-effort 清理该子目录。
// - DB 用临时目录里的 SQLite 文件（不是 :memory:，因为 `Database::open`
//   只接受文件路径并会跑 migration）。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{Library, Project};
    use rusqlite::params;
    use std::sync::Mutex as StdMutex;
    use tempfile::TempDir;

    /// 永远成功的 scheduler；副作用：往 `pipeline_tasks` 写一条 status='queued' 的行，
    /// 用以模拟真实 `PipelineScheduler::enqueue` 的可观察后果（满足 AC-4）。
    struct OkScheduler<'a> {
        pool: &'a Pool<SqliteConnectionManager>,
        enqueued: StdMutex<Vec<String>>,
    }

    impl<'a> EnqueueScheduler for OkScheduler<'a> {
        fn enqueue(&self, asset_id: &str) -> Result<String, String> {
            // 短锁：模拟真实 scheduler 在自己的事务里写 extracted_content + pipeline_tasks
            let conn = self
                .pool
                .get()
                .map_err(|e| format!("连接获取失败: {e}"))?;
            let task_id = uuid::Uuid::new_v4().to_string();
            let ec_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            conn.execute(
                "INSERT INTO extracted_content (id, asset_id, status, retry_count, quality_level, extractor_type, created_at, updated_at)
                 VALUES (?1, ?2, 'pending', 0, 0, '', ?3, ?3)",
                params![ec_id, asset_id, now],
            )
            .map_err(|e| format!("插入 extracted_content 失败: {e}"))?;
            conn.execute(
                "INSERT INTO pipeline_tasks (id, asset_id, task_type, status, retry_count, max_retries, priority, created_at)
                 VALUES (?1, ?2, 'extract', 'queued', 0, 3, 100, ?3)",
                params![task_id, asset_id, now],
            )
            .map_err(|e| format!("插入 pipeline_tasks 失败: {e}"))?;
            drop(conn);
            self.enqueued
                .lock()
                .unwrap()
                .push(asset_id.to_string());
            Ok(task_id)
        }
    }

    /// 永远失败的 scheduler，用于 AC-3。
    struct FailingScheduler {
        called: StdMutex<Vec<String>>,
    }
    impl EnqueueScheduler for FailingScheduler {
        fn enqueue(&self, asset_id: &str) -> Result<String, String> {
            self.called
                .lock()
                .unwrap()
                .push(asset_id.to_string());
            Err("boom: simulated enqueue failure".to_string())
        }
    }

    /// 在临时目录上开一个迁移到位的 DB，写入一条 library + 一条 project，返回 (db, project_id, _guard)。
    fn fresh_db_with_project() -> (Database, String, TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db_path = tmp.path().join("notecapt_test.db");
        let db = Database::open(&db_path).expect("open db");

        let library_id = uuid::Uuid::new_v4().to_string();
        let project_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        {
            let conn = db.conn().unwrap();
            let lib = Library {
                id: library_id.clone(),
                name: "测试库".into(),
                root_path: String::new(),
                created_at: now.clone(),
            };
            crate::db::library::insert(&conn, &lib).expect("insert lib");
            let proj = Project {
                id: project_id.clone(),
                library_id,
                name: "测试项目".into(),
                description: String::new(),
                cover_asset_id: None,
                source_type: "test".into(),
                source_data: None,
                is_pinned: false,
                is_archived: false,
                created_at: now.clone(),
                updated_at: now.clone(),
                total_duration: None,
                asset_count: 0,
                word_count: 0,
                imported_at: None,
            };
            crate::db::project::insert(&conn, &proj).expect("insert project");
        }
        (db, project_id, tmp)
    }

    /// 在临时目录写两个源文件供 import_files_core 消费。返回 (临时目录 guard, vec<path str>)。
    fn write_two_source_files() -> (TempDir, Vec<String>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let p1 = dir.path().join("alpha.txt");
        let p2 = dir.path().join("beta.md");
        std::fs::write(&p1, b"hello-alpha").expect("write 1");
        std::fs::write(&p2, b"# beta").expect("write 2");
        (
            dir,
            vec![
                p1.to_string_lossy().to_string(),
                p2.to_string_lossy().to_string(),
            ],
        )
    }

    /// 测试结束时尽力清理 workspace 子目录（不影响其他测试与用户数据）。
    fn cleanup_workspace(project_id: &str) {
        if let Ok(dir) = workspace::project_workspace_dir(project_id) {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn happy_path_inserts_root_and_enqueues() {
        crate::testing::init_test_logger();

        let (db, project_id, _db_tmp) = fresh_db_with_project();
        let (_src_tmp, paths) = write_two_source_files();

        let scheduler = OkScheduler {
            pool: &db.pool,
            enqueued: StdMutex::new(Vec::new()),
        };

        let out = import_files_core(&db.pool, &scheduler, &project_id, paths)
            .expect("core 调用应成功");

        // 1) 两条 created，零失败
        assert_eq!(out.summary.created.len(), 2, "应有 2 条 created");
        assert!(out.summary.failures.is_empty(), "不应有 failures");
        assert!(
            out.summary.failures_to_enqueue.is_empty(),
            "happy path 不应有 failures_to_enqueue"
        );

        // 2) DB 中两条 root asset（source_asset_id IS NULL）
        let conn = db.conn().unwrap();
        let root_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE project_id = ?1 AND source_asset_id IS NULL",
                params![project_id],
                |r| r.get(0),
            )
            .expect("count roots");
        assert_eq!(root_count, 2, "应有 2 条 root asset");

        // 3) FakeScheduler 写出了 2 条 pipeline_tasks status='queued'
        let queued_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pipeline_tasks WHERE status='queued' AND asset_id IN
                 (SELECT id FROM assets WHERE project_id = ?1)",
                params![project_id],
                |r| r.get(0),
            )
            .expect("count tasks");
        assert_eq!(queued_count, 2, "应有 2 条 queued pipeline_tasks");
        drop(conn);

        // 4) scheduler 内部记录了两个 asset_id
        let enqueued = scheduler.enqueued.lock().unwrap();
        assert_eq!(enqueued.len(), 2);

        cleanup_workspace(&project_id);
    }

    #[test]
    fn enqueue_failure_keeps_asset() {
        crate::testing::init_test_logger();

        let (db, project_id, _db_tmp) = fresh_db_with_project();
        let (_src_tmp, paths) = write_two_source_files();

        let scheduler = FailingScheduler {
            called: StdMutex::new(Vec::new()),
        };

        let out = import_files_core(&db.pool, &scheduler, &project_id, paths.clone())
            .expect("核心调用不应整体失败（enqueue 失败仅记 warn）");

        // 1) 仍 2 条 created（asset 行被保留）
        assert_eq!(out.summary.created.len(), 2);
        assert!(out.summary.failures.is_empty());

        // 2) 两条都进 failures_to_enqueue
        assert_eq!(out.summary.failures_to_enqueue.len(), 2, "ADR-006: 两条都应记入");

        // 3) DB 中两条 root asset 仍存在
        let conn = db.conn().unwrap();
        let asset_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE project_id = ?1",
                params![project_id],
                |r| r.get(0),
            )
            .expect("count assets");
        assert_eq!(asset_count, 2, "asset 行不应被回滚");

        // 4) 没有任何 pipeline_tasks 被写入（FailingScheduler 不会写）
        let queued_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pipeline_tasks WHERE asset_id IN
                 (SELECT id FROM assets WHERE project_id = ?1)",
                params![project_id],
                |r| r.get(0),
            )
            .expect("count tasks");
        assert_eq!(queued_count, 0);

        // 5) 物理源文件（落到 workspace 的副本）应仍存在
        for c in &out.summary.created {
            let p = Path::new(&c.asset.file_path);
            assert!(
                p.exists(),
                "ADR-006: workspace 内源文件应保留: {}",
                p.display()
            );
        }
        drop(conn);

        // 6) scheduler.enqueue 确被调用两次（顺序无关）
        assert_eq!(scheduler.called.lock().unwrap().len(), 2);

        cleanup_workspace(&project_id);
    }

    // =================================================================
    // custom_para_v1 / V17：resolve_or_create_category + organize 单测
    // =================================================================

    use crate::llm::classify_parse::ClassifyResult;

    fn make_classify(category: &str) -> ClassifyResult {
        ClassifyResult {
            category: category.to_string(),
            tags: vec![],
            confidence: 0.9,
            language: "zh".into(),
            suggested_file_name: "测试文件".into(),
        }
    }

    #[test]
    fn resolve_returns_none_for_other_or_empty() {
        let (db, _pid, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();
        assert!(resolve_or_create_category(&conn, &lib_id, "other").is_none());
        assert!(resolve_or_create_category(&conn, &lib_id, "OTHER").is_none());
        assert!(resolve_or_create_category(&conn, &lib_id, "").is_none());
        assert!(resolve_or_create_category(&conn, &lib_id, "   ").is_none());
        assert!(resolve_or_create_category(&conn, &lib_id, "none").is_none());
    }

    #[test]
    fn resolve_hits_builtin_by_slug() {
        let (db, _pid, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();
        let cat = resolve_or_create_category(&conn, &lib_id, "1-项目").unwrap();
        assert_eq!(cat.slug, "1-项目");
        assert!(cat.is_builtin);
    }

    #[test]
    fn resolve_new_prefix_creates_llm_generated() {
        let (db, _pid, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();
        let cat = resolve_or_create_category(&conn, &lib_id, "new:读书笔记").unwrap();
        assert_eq!(cat.slug, "读书笔记");
        assert!(!cat.is_builtin);
        // 表中确应存在
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE library_id=?1 AND slug='读书笔记' AND source='llm_generated'",
                params![lib_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn resolve_unknown_slug_auto_upserts_as_llm_generated() {
        let (db, _pid, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();
        // LLM 输出未知 slug（既不在 categories 也不在 aliases）→ 自动 upsert
        let cat = resolve_or_create_category(&conn, &lib_id, "竞品分析").unwrap();
        assert_eq!(cat.slug, "竞品分析");
        assert!(!cat.is_builtin);
    }

    #[test]
    fn resolve_follows_alias_to_target() {
        let (db, _pid, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();
        // 自定义类目 "读书笔记" + 别名 "学习资料"
        crate::db::categories::upsert_llm_generated(&conn, &lib_id, "读书笔记", "读书笔记").unwrap();
        conn.execute(
            "INSERT INTO category_aliases (library_id, alias_slug, target_slug) VALUES (?1, '学习资料', '读书笔记')",
            params![lib_id],
        )
        .unwrap();
        let cat = resolve_or_create_category(&conn, &lib_id, "学习资料").unwrap();
        assert_eq!(cat.slug, "读书笔记", "alias 应跳到 target");
    }

    #[test]
    fn organize_creates_dir_using_resolved_slug_and_returns_triple() {
        // 端到端覆盖：LLM 返回 "new:读书笔记" → organize 应建 organized/读书笔记/<id>_测试文件.txt
        // + 返回 category_slug = "读书笔记"
        crate::testing::init_test_logger();
        let (db, project_id, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();

        // 准备一个真实落地在 project workspace 内的源文件
        let proj_root = workspace::ensure_project_workspace(&project_id).unwrap();
        let src = proj_root.join("__test_src.txt");
        std::fs::write(&src, b"hello").unwrap();

        let asset = models::Asset {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            asset_type: "text".into(),
            name: "__test_src.txt".into(),
            original_name: "__test_src.txt".into(),
            file_path: src.to_string_lossy().to_string(),
            file_size: 5,
            mime_type: "text/plain".into(),
            captured_at: "".into(),
            imported_at: "".into(),
            source_type: "test".into(),
            source_data: None,
            is_starred: false,
            ..Default::default()
        };
        let r = make_classify("new:读书笔记");
        let triple = organize_asset_file_after_classify(&conn, &lib_id, &asset, &r)
            .expect("应整理成功");
        let (new_path, _display, slug) = triple;
        assert_eq!(slug, "读书笔记");
        assert!(
            new_path.contains("/organized/读书笔记/"),
            "目录应包含 organized/读书笔记/: {new_path}"
        );
        assert!(Path::new(&new_path).exists(), "新路径文件应已存在");

        drop(conn);
        cleanup_workspace(&project_id);
    }

    #[test]
    fn organize_skipped_when_category_is_other() {
        let (db, project_id, _tmp) = fresh_db_with_project();
        let conn = db.conn().unwrap();
        let lib_id: String = conn
            .query_row("SELECT id FROM libraries LIMIT 1", [], |r| r.get(0))
            .unwrap();
        crate::db::categories::seed_builtin_categories(&conn, &lib_id).unwrap();

        let proj_root = workspace::ensure_project_workspace(&project_id).unwrap();
        let src = proj_root.join("__test_other.txt");
        std::fs::write(&src, b"x").unwrap();
        let asset = models::Asset {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            asset_type: "text".into(),
            name: "__test_other.txt".into(),
            file_path: src.to_string_lossy().to_string(),
            ..Default::default()
        };
        let r = make_classify("other");
        assert!(
            organize_asset_file_after_classify(&conn, &lib_id, &asset, &r).is_none(),
            "other 应跳过物理整理"
        );
        drop(conn);
        cleanup_workspace(&project_id);
    }
}
