use crate::db::{self, Database};
use crate::models::{self, AssetState, WorkspaceAssetView};
use crate::workspace;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// SourceMissingSet 自 task_007 起归位到 `crate::source_scan`；
/// 这里保留 `pub use` 以兼容已有 import 路径（task_003 偏离 (a) 修正）。
pub use crate::source_scan::SourceMissingSet;

/// 工作区列表唯一对外命令（ADR-002）。
///
/// - 走 [`db::asset::list_root_assets`] 单查询拿到 root + 派生关联；
/// - 在命令层用 `Path::exists()` stat rendition / source（**db 层零 IO**）；
/// - 调用纯函数 [`db::asset::compute_asset_state`] 派生四态；
/// - source-missing 来自可选的 [`SourceMissingSet`]（task_007 注册前为 false）。
#[tauri::command]
pub fn get_assets(
    app: AppHandle,
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<WorkspaceAssetView>, String> {
    let rows = {
        let conn = database.conn()?;
        db::asset::list_root_assets(&conn, &project_id)?
        // 释放锁，防止后续 stat IO 长时间占用
    };

    // SourceMissingSet 在 task_007 注册；本 task 容忍未注册场景（input.md AC-4）。
    let missing_state: Option<State<'_, SourceMissingSet>> = app.try_state::<SourceMissingSet>();

    let mut out = Vec::with_capacity(rows.len());
    for (asset, join) in rows {
        // rendition 存在性：路径来自 LEFT JOIN，可能为 None
        let rendition_exists = match join.rendition_path.as_deref() {
            Some(p) => Path::new(p).exists(),
            None => false,
        };
        let source_exists = Path::new(&asset.file_path).exists();

        let source_missing_known = missing_state
            .as_ref()
            .map(|s| s.contains(&asset.id))
            .unwrap_or(false);

        out.push(build_workspace_view(
            asset,
            join,
            rendition_exists,
            source_exists,
            source_missing_known,
        ));
    }
    Ok(out)
}

/// 把 root asset + JOIN 行 + IO stat 结果拼成 [`WorkspaceAssetView`]。
///
/// 抽出来便于单测（避免依赖 Tauri AppHandle 与真实 fs）。
fn build_workspace_view(
    asset: models::Asset,
    join: db::asset::AssetListJoinRow,
    rendition_exists: bool,
    source_exists: bool,
    source_missing_known: bool,
) -> WorkspaceAssetView {
    let state = db::asset::compute_asset_state(
        join.pipeline_status.as_deref(),
        join.latest_error_class.as_deref(),
        rendition_exists,
        source_exists,
        source_missing_known,
    );

    // 失败原因优先取 conversion_meta.error_class（更结构化），
    // 否则回落到 pipeline_tasks.error_message。
    let state_reason = match state {
        AssetState::Failed => join
            .latest_error_class
            .clone()
            .or_else(|| join.pipeline_error.clone()),
        _ => None,
    };

    // hotfix-H3：工作区"显示 .md 衍生件"而非原文件
    // 当 root 有 .md derivative（join.rendition_id 存在）且 derivative 实际存在
    // （rendition_exists）时，用 derivative 的 name/file_path/file_size/mime/type
    // 覆盖 root 展示字段。asset.id 保留 root id —— 标签查询走 root.id 不受影响，
    // 删除 / 评级等命令仍以 root 为锚点（AC-3 / ADR-002）。
    let has_rendition = join.rendition_id.is_some() && rendition_exists;
    let display_name = if has_rendition {
        join.rendition_name.clone().unwrap_or(asset.name.clone())
    } else {
        asset.name.clone()
    };
    let display_file_path = if has_rendition {
        join.rendition_path.clone().unwrap_or(asset.file_path.clone())
    } else {
        asset.file_path.clone()
    };
    let display_file_size = if has_rendition {
        join.rendition_size.unwrap_or(asset.file_size)
    } else {
        asset.file_size
    };
    let display_mime = if has_rendition {
        join.rendition_mime.clone().unwrap_or(asset.mime_type.clone())
    } else {
        asset.mime_type.clone()
    };
    let display_asset_type = if has_rendition {
        join.rendition_asset_type
            .clone()
            .unwrap_or(asset.asset_type.clone())
    } else {
        asset.asset_type.clone()
    };

    WorkspaceAssetView {
        id: asset.id,
        project_id: asset.project_id,
        asset_type: display_asset_type,
        name: display_name,
        original_name: asset.original_name,
        file_path: display_file_path,
        file_size: display_file_size,
        mime_type: display_mime,
        captured_at: asset.captured_at,
        imported_at: asset.imported_at,
        source_type: asset.source_type,
        source_data: asset.source_data,
        is_starred: asset.is_starred,
        derivative_version: asset.derivative_version,
        rendition_id: join.rendition_id,
        rendition_path: join.rendition_path,
        rendition_size: join.rendition_size,
        state,
        state_reason,
        source_missing: source_missing_known || !source_exists,
        extractor_type: join.extractor_type,
        extraction_failure_code: join.latest_failure_code,
    }
}

/// 项目内素材 id → 标签名列表（工作区主题展示）
#[tauri::command]
pub fn get_project_asset_tag_map(
    database: State<'_, Database>,
    project_id: String,
) -> Result<HashMap<String, Vec<String>>, String> {
    let conn = database.conn()?;
    db::asset::get_tag_names_by_project(&conn, &project_id)
}

#[tauri::command]
pub fn get_assets_by_tag(
    database: State<'_, Database>,
    project_id: String,
    tag_id: String,
) -> Result<Vec<models::Asset>, String> {
    let conn = database.conn()?;
    db::asset::get_by_project_and_tag(&conn, &project_id, &tag_id)
}

#[tauri::command]
pub fn get_asset(
    database: State<'_, Database>,
    id: String,
) -> Result<Option<models::Asset>, String> {
    let conn = database.conn()?;
    db::asset::get_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_asset(
    database: State<'_, Database>,
    project_id: String,
    asset_type: String,
    name: String,
    file_path: String,
    file_size: i64,
    mime_type: String,
) -> Result<models::Asset, String> {
    let conn = database.conn()?;
    let now = chrono::Utc::now().to_rfc3339();
    let asset = models::Asset {
        id: uuid::Uuid::new_v4().to_string(),
        project_id,
        asset_type,
        name: name.clone(),
        original_name: name,
        file_path,
        file_size,
        mime_type,
        captured_at: now.clone(),
        imported_at: now,
        source_type: "manual_import".to_string(),
        source_data: None,
        is_starred: false,
        ..Default::default()
    };
    db::asset::insert(&conn, &asset)?;
    Ok(asset)
}

#[tauri::command]
pub fn update_asset(
    database: State<'_, Database>,
    asset: models::Asset,
) -> Result<(), String> {
    let conn = database.conn()?;
    db::asset::update(&conn, &asset)
}

/// display_name 上限（与 PRD §4.4 一致：UTF-8 ≤ 200 字节）。
const DISPLAY_NAME_MAX_BYTES: usize = 200;

/// 校验工作区 rename 的新展示名（ADR-007）。
///
/// - trim 后非空；
/// - UTF-8 字节长度 ≤ 200。
/// 错误文案统一中文（不可妥协的底线 §5）。
fn validate_display_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("新名称不能为空".to_string());
    }
    if trimmed.len() > DISPLAY_NAME_MAX_BYTES {
        return Err("新名称超长（请控制在 200 字节内）".to_string());
    }
    Ok(trimmed.to_string())
}

/// 由 root 新名生成 markdown 衍生件的展示名（不动磁盘文件）：
/// 1) sanitize_stem 清洗（把 `/` `:` 等非法字符替换为 `_`，避免 Path::file_stem
///    把 `a/b.pdf` 当路径切到 `b`）；
/// 2) 在清洗结果上按"最后一个 `.`"切掉原扩展名得到 stem；
/// 3) 拼 `.md`。
fn derivative_name_from_root(new_root_name: &str) -> String {
    let safe = crate::utils::safe_name::sanitize_stem(new_root_name);
    let stem = match safe.rfind('.') {
        // 仅当 `.` 不在首位才视为扩展名分隔符（避免 ".env" 退化为空 stem）
        Some(idx) if idx > 0 => &safe[..idx],
        _ => &safe[..],
    };
    format!("{stem}.md")
}

/// 在已持有连接的前提下双写 root.name 与 derivative.name，并返回新的
/// `WorkspaceAssetView`（抽出便于单测，调用方负责锁获取）。
fn rename_asset_inner(
    conn: &rusqlite::Connection,
    asset_id: &str,
    new_display_name: &str,
    rendition_exists_for_test: Option<bool>,
    source_exists_for_test: Option<bool>,
) -> Result<WorkspaceAssetView, String> {
    let trimmed = validate_display_name(new_display_name)?;

    let (mut root, derivative) = db::asset::resolve_asset_pair(conn, asset_id)?;

    // 双写：root.name + derivative.name —— 不动 file_path 与磁盘文件
    // （PRD 硬约束 §4：display_name 仅活在 DB）。
    root.name = trimmed.clone();
    db::asset::update(conn, &root)?;

    if let Some(ref d) = derivative {
        let new_md_name = derivative_name_from_root(&trimmed);
        // 复用 update_markdown_derivative，但 file_size / imported_at 保持原值
        // —— 当前函数签名要求三参数，传原值即可（不改大小、不改导入时间）。
        db::asset::update_markdown_derivative(
            conn,
            &d.id,
            &new_md_name,
            d.file_size,
            &d.imported_at,
        )?;
    }

    // 重新加载视图：复用 list_root_assets 路径，过滤出 root.id。
    // rename 是低频单次操作，多查一次项目列表的代价可接受，避免在 db 层新增
    // 单 root 查询路径（保持 ADR-002 单一查询入口）。
    let rows = db::asset::list_root_assets(conn, &root.project_id)?;
    let (asset, join) = rows
        .into_iter()
        .find(|(a, _)| a.id == root.id)
        .ok_or_else(|| "素材不存在".to_string())?;

    // stat：测试模式下用注入值，生产路径用真实 Path::exists。
    let rendition_exists = rendition_exists_for_test.unwrap_or_else(|| {
        match join.rendition_path.as_deref() {
            Some(p) => Path::new(p).exists(),
            None => false,
        }
    });
    let source_exists = source_exists_for_test
        .unwrap_or_else(|| Path::new(&asset.file_path).exists());

    Ok(build_workspace_view(
        asset,
        join,
        rendition_exists,
        source_exists,
        false, // SourceMissingSet 在命令层入口注入；inner 不依赖 AppHandle
    ))
}

/// 工作区 rename 唯一命令（ADR-007）。
///
/// - 入参以 asset_id 为唯一目标（**不接受 file_path**）；接受 root.id 或 markdown
///   derivative.id，由 [`db::asset::resolve_asset_pair`] 统一解算回 root；
/// - 校验：trim 后非空；UTF-8 ≤ 200 字节；
/// - 双写：`assets.name`（root）+ `assets.name`（markdown 衍生件，name 为
///   `sanitize_stem(stem(new_display_name)) + ".md"`）；
/// - **不动磁盘文件名 / file_path**（PRD 硬约束 §4：display_name 仅活在 DB）；
/// - 返回最新的 [`WorkspaceAssetView`]，便于前端就地 patch 不必再走 fetchAssets。
#[tauri::command]
pub fn rename_asset(
    app: AppHandle,
    database: State<'_, Database>,
    asset_id: String,
    new_display_name: String,
) -> Result<WorkspaceAssetView, String> {
    let view = {
        let conn = database.conn()?;
        rename_asset_inner(&conn, &asset_id, &new_display_name, None, None)?
        // 释放锁后再叠加 source-missing 标记
    };

    // source-missing 标记位（task_007 注册后生效，未注册时维持 inner 计算结果）。
    let missing_state: Option<State<'_, SourceMissingSet>> = app.try_state::<SourceMissingSet>();
    let source_missing_known = missing_state
        .as_ref()
        .map(|s| s.contains(&view.id))
        .unwrap_or(false);
    let view = WorkspaceAssetView {
        source_missing: view.source_missing || source_missing_known,
        ..view
    };
    Ok(view)
}

/// task_006 AC-5：工作区删除命令。
///
/// 签名与历史保持兼容（`id: String`，前端 `deleteAsset(id)` 不变）。内部委托给
/// [`db::asset::delete_with_cascade`] 完成 root + derivative + 关联表的 DB 级联
/// 清理；命令层补一刀 outbound 缓存目录的 `fs::remove_dir_all`（task_005 缓存
/// 路径口径），失败仅 warn 不阻断（AC-4 原则）。
#[tauri::command]
pub fn delete_asset(database: State<'_, Database>, id: String) -> Result<(), String> {
    let report = {
        let conn = database.conn()?;
        db::asset::delete_with_cascade(&conn, &id)?
        // 释放锁后再做磁盘清理，避免长时间持锁
    };

    // task_006 AC-4：清 outbound 缓存目录 —— 复用 task_005 路径助手保口径一致。
    // 失败仅 warn（缓存目录不存在 / 权限错误都不应阻断 DB 删除）。
    if let Some(cache_dir) = crate::commands::outbound::outbound_cache_dir_for(&report.root_asset_id) {
        if cache_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&cache_dir) {
                log::warn!(
                    "清理 outbound 缓存目录失败（不阻断删除）: {:?} — {e}",
                    cache_dir
                );
            }
        }
    } else {
        log::warn!(
            "无法定位系统缓存目录，跳过 outbound 清理 asset={}",
            report.root_asset_id
        );
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_asset_star(database: State<'_, Database>, id: String) -> Result<bool, String> {
    let conn = database.conn()?;
    db::asset::toggle_star(&conn, &id)
}

#[tauri::command]
pub fn get_asset_analysis(
    database: State<'_, Database>,
    asset_id: String,
) -> Result<Option<models::AIAnalysisRow>, String> {
    let conn = database.conn()?;
    db::asset::get_analysis(&conn, &asset_id)
}

fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match Path::new(file_name).extension() {
        Some(e) => (
            Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file_name)
                .to_string(),
            format!(".{}", e.to_string_lossy()),
        ),
        None => (file_name.to_string(), String::new()),
    };
    for i in 1..1000 {
        let candidate = dir.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}.{}{ext}", uuid::Uuid::new_v4()))
}

#[tauri::command]
pub fn move_asset_to_workspace_folder(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_relative_path: String,
    project_id: String,
) -> Result<(), String> {
    let workspace_root = workspace::project_workspace_dir(&project_id)?;
    let target_dir = if target_relative_path == "__ROOT__" {
        workspace_root.clone()
    } else {
        workspace_root.join(&target_relative_path)
    };
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("目标目录创建失败: {e}"))?;

    let workspace_canonical = workspace_root
        .canonicalize()
        .map_err(|e| format!("workspace 根目录规范化失败: {e}"))?;
    let target_canonical = target_dir
        .canonicalize()
        .map_err(|e| format!("目标目录规范化失败: {e}"))?;
    if !target_canonical.starts_with(&workspace_canonical) {
        return Err(format!(
            "目标路径越界：{:?} 不在 workspace {:?} 内",
            target_canonical, workspace_canonical
        ));
    }

    let mut conn = database.conn()?;

    let mut planned: Vec<(String, PathBuf, PathBuf, String)> = Vec::new();
    for id in &asset_ids {
        let asset = db::asset::get_by_id(&conn, id)?
            .ok_or_else(|| format!("素材不存在: {id}"))?;
        let src = PathBuf::from(&asset.file_path);
        if !src.exists() {
            return Err(format!("源文件缺失: {}", asset.file_path));
        }
        let file_name = src
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("非法文件名: {}", asset.file_path))?
            .to_string();
        let dest = unique_path(&target_dir, &file_name);
        let new_name = dest
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_name)
            .to_string();
        planned.push((asset.id.clone(), src, dest, new_name));
    }

    // —— 阶段 1：rename 物理文件，失败时反向回滚已 rename 的项 —— //
    // 修复：回滚 rename 失败原本是 `let _ = fs::rename(...)` 静默吞，现在改成
    // log::error!，让用户的源文件残留在目标目录时能在日志里追踪。
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (_id, src, dest, _name) in &planned {
        if let Err(e) = fs::rename(src, dest) {
            rollback_renames(&moved, &format!("rename 中途失败：{e}"));
            return Err(format!("移动失败 {:?} → {:?}: {e}", src, dest));
        }
        moved.push((src.clone(), dest.clone()));
    }

    // —— 阶段 2：DB 写入包进单个事务；任何一步失败都回滚事务 + 反向回滚 fs rename —— //
    // 修复：旧实现里 `update_name_and_path` 在 SQLite 默认 autocommit 模式下逐行写，
    // 中途失败（磁盘掉电 / UNIQUE 冲突）会留下 DB 与物理位置不一致——用户看到的现象
    // 是"我的素材凭空消失了"。现在所有 DB 写要么全部 commit，要么全部不写并物理回滚。
    let db_result = (|| -> Result<(), String> {
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;
        for (id, _src, dest, new_name) in &planned {
            let dest_str = dest.to_string_lossy().to_string();
            db::asset::update_name_and_path(&tx, id, new_name, &dest_str)?;
        }
        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(())
    })();

    if let Err(e) = db_result {
        log::error!(
            "move_asset_to_workspace_folder：DB 事务失败，回滚 {} 个已 rename 文件: {}",
            moved.len(),
            e
        );
        rollback_renames(&moved, &format!("DB 事务失败: {e}"));
        return Err(format!("移动失败（DB 写入已回滚物理文件）: {e}"));
    }

    Ok(())
}

/// 反向回滚一组 (src → dest) rename。任何一步回滚失败 log::error。
/// 此函数只在批量 fs 操作中途失败时被调用；成功路径不会触达。
fn rollback_renames(moved: &[(PathBuf, PathBuf)], cause: &str) {
    for (orig, target) in moved.iter().rev() {
        if let Err(re) = fs::rename(target, orig) {
            log::error!(
                "rename 回滚失败 {:?} → {:?}: {} (触发原因: {})",
                target,
                orig,
                re,
                cause
            );
        }
    }
}

#[tauri::command]
pub fn get_drag_icon_path(app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    if cfg!(debug_assertions) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("icons")
            .join("32x32.png");
        Ok(path.to_string_lossy().to_string())
    } else {
        // 2026-05-17 修复 release 拖到 Finder 完全无反应（root cause）：
        // Tauri 把 bundle.icon 配置里的所有 PNG 打包成 Resources/icon.icns
        // （macOS 标准），并 **不** 把原 PNG 复制到 Resources/icons/，因此
        // `resource_dir/icons/32x32.png` 在 release 永远不存在。
        // drag crate macOS impl 收到不存在的 image path → Error::ImageNotFound
        // → startDrag silent reject → NSDraggingSession 不启动 → Finder 看不到拖拽。
        // 修复：用 Resources/icon.icns（Tauri 一定会打入），
        // NSImage::initByReferencingFile 支持 .icns 格式。
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("resource_dir 失败: {e}"))?;
        let path = resource_dir.join("icon.icns");
        Ok(path.to_string_lossy().to_string())
    }
}

/// 跨项目移动素材（BatchToolbar"移动到"路径）。
///
/// - 物理文件：`fs::rename(src → target_workspace/<unique_name>)`
/// - 数据库：更新 `assets.project_id` 与 `assets.file_path`
/// - 冲突处理：`unique_path` 自动加 `(1)`/`(2)` 后缀避免覆盖
/// - 事务性：rename 阶段失败回滚已 rename 的文件（与 `move_asset_to_workspace_folder` 同模式）
/// - 已在目标项目的素材：跳过 IO，但仍返回最新行
/// - **不级联 derivative**：若传入是 root 且存在 markdown 衍生件，衍生件**不会**跟随移动，
///   留给后续重转换处理（与 BatchToolbar 简化模型一致；DnD 跨项目拖入已废弃）。
#[tauri::command]
pub fn move_assets(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_project_id: String,
) -> Result<Vec<models::Asset>, String> {
    let target_dir = workspace::ensure_project_workspace(&target_project_id)?;

    let mut conn = database.conn()?;

    db::project::get_by_id(&conn, &target_project_id)?
        .ok_or_else(|| format!("目标项目不存在: {target_project_id}"))?;

    // Plan: (asset_id, already_in_target, src, dest, src_existed_at_plan_time)
    // 修复：旧实现 `if src.exists()` 跳过物理 rename，但 DB 仍然 update 成新路径，
    // 导致"源文件已丢失"时 DB 指向**不存在的目标路径**。现在记录"plan 时 src 是否存在"，
    // DB 写入只对真正发生 rename 的项进行。
    let mut planned: Vec<(String, bool, PathBuf, PathBuf, bool)> = Vec::new();
    for id in &asset_ids {
        let asset = db::asset::get_by_id(&conn, id)?
            .ok_or_else(|| format!("素材不存在: {id}"))?;
        if asset.project_id == target_project_id {
            planned.push((asset.id, true, PathBuf::new(), PathBuf::new(), false));
            continue;
        }
        let src = PathBuf::from(&asset.file_path);
        let src_existed = src.exists();
        let file_name = src
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("非法文件名: {}", asset.file_path))?
            .to_string();
        let dest = unique_path(&target_dir, &file_name);
        planned.push((asset.id, false, src, dest, src_existed));
    }

    // —— 阶段 1：rename 物理文件 —— //
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (_id, skip, src, dest, src_existed) in &planned {
        if *skip || !*src_existed {
            continue;
        }
        if let Err(e) = fs::rename(src, dest) {
            rollback_renames(&moved, &format!("rename 中途失败：{e}"));
            return Err(format!("移动失败 {:?} → {:?}: {e}", src, dest));
        }
        moved.push((src.clone(), dest.clone()));
    }

    // —— 阶段 2：DB 写入包进事务；失败回滚事务 + 反向回滚 fs rename —— //
    let db_result = (|| -> Result<Vec<models::Asset>, String> {
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;
        let mut result: Vec<models::Asset> = Vec::with_capacity(planned.len());
        for (id, skip, _src, dest, src_existed) in &planned {
            if !*skip && *src_existed {
                let dest_str = dest.to_string_lossy().to_string();
                db::asset::update_project_and_path(&tx, id, &target_project_id, &dest_str)?;
            }
            let updated = db::asset::get_by_id(&tx, id)?
                .ok_or_else(|| format!("移动后读取素材失败: {id}"))?;
            result.push(updated);
        }
        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(result)
    })();

    match db_result {
        Ok(result) => Ok(result),
        Err(e) => {
            log::error!(
                "move_assets：DB 事务失败，回滚 {} 个已 rename 文件: {}",
                moved.len(),
                e
            );
            rollback_renames(&moved, &format!("DB 事务失败: {e}"));
            Err(format!("移动失败（DB 写入已回滚物理文件）: {e}"))
        }
    }
}

/// 跨项目复制素材（BatchToolbar"复制到"路径）。
///
/// - 物理文件：`fs::copy(src → target_workspace/<unique_name>)`
/// - 数据库：`INSERT` 新 Asset 行（新 UUID，`source_asset_id: None`，`derivative_version: 0`）
/// - 冲突处理：`unique_path` 自动加 `(1)`/`(2)` 后缀
/// - 失败处理：copy 阶段失败时删除已 copy 的目标文件回滚
/// - **不复制 derivative**：新行无 markdown 衍生件，需重新触发转换（task_008 流程）
#[tauri::command]
pub fn copy_assets(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_project_id: String,
) -> Result<Vec<models::Asset>, String> {
    let target_dir = workspace::ensure_project_workspace(&target_project_id)?;

    let mut conn = database.conn()?;

    db::project::get_by_id(&conn, &target_project_id)?
        .ok_or_else(|| format!("目标项目不存在: {target_project_id}"))?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut planned: Vec<(models::Asset, PathBuf, PathBuf)> = Vec::new();
    for id in &asset_ids {
        let asset = db::asset::get_by_id(&conn, id)?
            .ok_or_else(|| format!("素材不存在: {id}"))?;
        let src = PathBuf::from(&asset.file_path);
        let file_name = src
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("非法文件名: {}", asset.file_path))?
            .to_string();
        let dest = unique_path(&target_dir, &file_name);
        planned.push((asset, src, dest));
    }

    // —— 阶段 1：copy 物理文件，失败时清理已 copy —— //
    let mut copied: Vec<PathBuf> = Vec::new();
    for (_asset, src, dest) in &planned {
        if src.exists() {
            if let Err(e) = fs::copy(src, dest) {
                rollback_copies(&copied, &format!("copy 中途失败：{e}"));
                return Err(format!("复制失败 {:?} → {:?}: {e}", src, dest));
            }
            copied.push(dest.clone());
        }
    }

    // —— 阶段 2：DB INSERT 包进事务；失败回滚事务 + 清理已 copy 的目标文件 —— //
    // 修复：旧实现 db::asset::insert 在 autocommit 模式下逐行写，第 N+1 行失败时
    // 已经插入的 N 行留在 DB、对应的 N+1...M 物理文件留在磁盘但无 DB 行 → 孤儿文件累积。
    let db_result = (|| -> Result<Vec<models::Asset>, String> {
        let tx = conn
            .transaction()
            .map_err(|e| format!("开启事务失败: {e}"))?;
        let mut result: Vec<models::Asset> = Vec::with_capacity(planned.len());
        for (orig, _src, dest) in planned.iter() {
            let new_asset = models::Asset {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: target_project_id.clone(),
                asset_type: orig.asset_type.clone(),
                name: orig.name.clone(),
                original_name: orig.original_name.clone(),
                file_path: dest.to_string_lossy().to_string(),
                file_size: orig.file_size,
                mime_type: orig.mime_type.clone(),
                captured_at: orig.captured_at.clone(),
                imported_at: now.clone(),
                source_type: orig.source_type.clone(),
                source_data: orig.source_data.clone(),
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&tx, &new_asset)?;
            result.push(new_asset);
        }
        tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
        Ok(result)
    })();

    match db_result {
        Ok(result) => Ok(result),
        Err(e) => {
            log::error!(
                "copy_assets：DB 事务失败，清理 {} 个已 copy 文件: {}",
                copied.len(),
                e
            );
            rollback_copies(&copied, &format!("DB 事务失败: {e}"));
            Err(format!("复制失败（DB 写入已清理物理文件）: {e}"))
        }
    }
}

/// 反向清理一组 copy 出来的目标文件。
fn rollback_copies(copied: &[PathBuf], cause: &str) {
    for d in copied.iter().rev() {
        if let Err(re) = fs::remove_file(d) {
            log::error!(
                "copy 清理失败 {:?}: {} (触发原因: {})",
                d,
                re,
                cause
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::asset::AssetListJoinRow;
    use crate::models::Asset;

    fn mk_root_asset(id: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: "p1".to_string(),
            asset_type: "pdf".to_string(),
            name: format!("{id}.pdf"),
            original_name: format!("{id}.pdf"),
            file_path: format!("/tmp/{id}.pdf"),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            captured_at: "2025-01-01T00:00:00Z".to_string(),
            imported_at: "2025-01-01T00:00:00Z".to_string(),
            source_type: "dropzone_drag".to_string(),
            ..Default::default()
        }
    }

    fn empty_join() -> AssetListJoinRow {
        AssetListJoinRow {
            rendition_id: None,
            rendition_path: None,
            rendition_size: None,
            pipeline_status: None,
            pipeline_error: None,
            latest_error_class: None,
            latest_fallback_used: None,
            extraction_status: None,
            extractor_type: None,
            latest_failure_code: None,
            rendition_name: None,
            rendition_mime: None,
            rendition_asset_type: None,
        }
    }

    #[test]
    fn build_view_done_when_pipeline_completed_and_rendition_present() {
        let asset = mk_root_asset("r1");
        let mut join = empty_join();
        join.rendition_id = Some("md1".to_string());
        join.rendition_path = Some("/tmp/r1.md".to_string());
        join.rendition_size = Some(2048);
        join.pipeline_status = Some("completed".to_string());

        let view = build_workspace_view(asset, join, true, true, false);
        assert_eq!(view.state, AssetState::Done);
        assert!(view.state_reason.is_none(), "done 不应携带 reason");
        assert_eq!(view.rendition_id.as_deref(), Some("md1"));
        assert_eq!(view.rendition_path.as_deref(), Some("/tmp/r1.md"));
        assert!(!view.source_missing);
    }

    #[test]
    fn build_view_failed_uses_error_class_as_reason() {
        let asset = mk_root_asset("r2");
        let mut join = empty_join();
        join.pipeline_status = Some("failed".to_string());
        join.pipeline_error = Some("scheduler 报错".to_string());
        join.latest_error_class = Some("converter_timeout".to_string());

        let view = build_workspace_view(asset, join, false, true, false);
        assert_eq!(view.state, AssetState::Failed);
        // error_class 优先于 pipeline_error
        assert_eq!(view.state_reason.as_deref(), Some("converter_timeout"));
    }

    #[test]
    fn build_view_failed_falls_back_to_pipeline_error_when_no_error_class() {
        let asset = mk_root_asset("r3");
        let mut join = empty_join();
        join.pipeline_status = Some("failed".to_string());
        join.pipeline_error = Some("network".to_string());

        let view = build_workspace_view(asset, join, false, true, false);
        assert_eq!(view.state, AssetState::Failed);
        assert_eq!(view.state_reason.as_deref(), Some("network"));
    }

    #[test]
    fn build_view_offline_when_no_pipeline_no_meta() {
        let asset = mk_root_asset("r4");
        let view = build_workspace_view(asset, empty_join(), false, true, false);
        assert_eq!(view.state, AssetState::Offline);
        assert!(view.state_reason.is_none());
    }

    #[test]
    fn build_view_source_missing_marks_flag_but_keeps_state() {
        let asset = mk_root_asset("r5");
        let mut join = empty_join();
        join.rendition_path = Some("/tmp/r5.md".to_string());
        join.pipeline_status = Some("completed".to_string());

        // source 已不在盘上，但 rendition 仍在 → state 仍是 Done
        let view = build_workspace_view(asset, join, true, false, false);
        assert_eq!(view.state, AssetState::Done, "source 缺失不应降级 Done");
        assert!(view.source_missing, "source_missing 应被 stat 结果置 true");
    }

    // ---- rename_asset 单测（AC-2 / AC-3 / AC-4） ----

    use crate::db::asset as db_asset;
    use crate::db::migration::run_migrations;
    use rusqlite::{params, Connection};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory");
        run_migrations(&conn).expect("migrate");
        conn.execute(
            "INSERT OR IGNORE INTO libraries (id, name, root_path) VALUES (?1, ?2, ?3)",
            params!["lib_t", "lib", "/tmp/lib"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, ?3)",
            params!["p1", "lib_t", "proj"],
        )
        .unwrap();
        conn
    }

    fn mk_db_root(id: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: "p1".to_string(),
            asset_type: "pdf".to_string(),
            name: format!("{id}.pdf"),
            original_name: format!("{id}.pdf"),
            file_path: format!("/tmp/{id}.pdf"),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            captured_at: "2025-01-01T00:00:00Z".to_string(),
            imported_at: "2025-01-01T00:00:00Z".to_string(),
            source_type: "dropzone_drag".to_string(),
            ..Default::default()
        }
    }

    fn mk_db_md_derivative(id: &str, root_id: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: "p1".to_string(),
            asset_type: "markdown".to_string(),
            name: format!("{id}.md"),
            original_name: format!("{id}.md"),
            file_path: format!("/tmp/canonical/{id}.md"),
            file_size: 2048,
            mime_type: "text/markdown".to_string(),
            captured_at: "2025-01-02T00:00:00Z".to_string(),
            imported_at: "2025-01-02T00:00:00Z".to_string(),
            source_type: "derived".to_string(),
            source_asset_id: Some(root_id.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn rename_double_writes_root_and_derivative() {
        let conn = setup_conn();
        let root = mk_db_root("root1");
        db_asset::insert(&conn, &root).unwrap();
        let deriv = mk_db_md_derivative("d1", "root1");
        db_asset::insert(&conn, &deriv).unwrap();

        let view = rename_asset_inner(&conn, "root1", "新名.pdf", Some(true), Some(true))
            .expect("rename failed");

        assert_eq!(view.id, "root1");
        // hotfix-H3：工作区视图在 root 有 .md derivative 且 rendition_exists 时
        // 用 derivative 的 name 覆盖 root，显示 .md 文件名（用户测试反馈：原文件名不直观）
        assert_eq!(view.name, "新名.md");

        // root.name 更新（DB 层仍是 .pdf）
        let root_after = db_asset::get_by_id(&conn, "root1").unwrap().unwrap();
        assert_eq!(root_after.name, "新名.pdf");
        // file_path 不变（不动磁盘 —— PRD 硬约束 §4）
        assert_eq!(root_after.file_path, "/tmp/root1.pdf");

        // derivative.name 应去原扩展、拼 .md
        let deriv_after = db_asset::get_by_id(&conn, "d1").unwrap().unwrap();
        assert_eq!(deriv_after.name, "新名.md");
        // derivative.file_path 也不变
        assert_eq!(deriv_after.file_path, "/tmp/canonical/d1.md");
    }

    #[test]
    fn rename_via_derivative_id_resolves_to_root() {
        // AC-4：传入 derivative.id 也应正确解算回 root 后双写
        let conn = setup_conn();
        let root = mk_db_root("root2");
        db_asset::insert(&conn, &root).unwrap();
        let deriv = mk_db_md_derivative("d2", "root2");
        db_asset::insert(&conn, &deriv).unwrap();

        let view = rename_asset_inner(&conn, "d2", "另一名.docx", Some(true), Some(true))
            .expect("rename via derivative failed");

        // 返回视图 id 应是 root.id（工作区视图是 root 视角）
        assert_eq!(view.id, "root2");
        // hotfix-H3：rendition_exists → 用 derivative 名（"另一名.md"）覆盖 root（"另一名.docx"）
        assert_eq!(view.name, "另一名.md");

        let root_after = db_asset::get_by_id(&conn, "root2").unwrap().unwrap();
        assert_eq!(root_after.name, "另一名.docx");
        let deriv_after = db_asset::get_by_id(&conn, "d2").unwrap().unwrap();
        assert_eq!(deriv_after.name, "另一名.md");
    }

    #[test]
    fn rename_without_derivative_only_writes_root() {
        let conn = setup_conn();
        let root = mk_db_root("root_solo");
        db_asset::insert(&conn, &root).unwrap();

        let view = rename_asset_inner(&conn, "root_solo", "孤儿名.pdf", Some(false), Some(true))
            .expect("rename failed");
        assert_eq!(view.name, "孤儿名.pdf");
        assert!(view.rendition_id.is_none());

        let root_after = db_asset::get_by_id(&conn, "root_solo").unwrap().unwrap();
        assert_eq!(root_after.name, "孤儿名.pdf");
    }

    #[test]
    fn rename_rejects_empty_after_trim() {
        let conn = setup_conn();
        let root = mk_db_root("r_empty");
        db_asset::insert(&conn, &root).unwrap();

        let err = rename_asset_inner(&conn, "r_empty", "   ", Some(false), Some(true))
            .expect_err("应失败");
        assert_eq!(err, "新名称不能为空");

        // 原名未被改
        let after = db_asset::get_by_id(&conn, "r_empty").unwrap().unwrap();
        assert_eq!(after.name, "r_empty.pdf");
    }

    #[test]
    fn rename_rejects_over_200_bytes() {
        let conn = setup_conn();
        let root = mk_db_root("r_long");
        db_asset::insert(&conn, &root).unwrap();

        // 201 个 ASCII 字符 = 201 字节
        let long = "a".repeat(201);
        let err = rename_asset_inner(&conn, "r_long", &long, Some(false), Some(true))
            .expect_err("应失败");
        assert_eq!(err, "新名称超长（请控制在 200 字节内）");

        // 边界：200 字节应通过
        let ok = "a".repeat(200);
        rename_asset_inner(&conn, "r_long", &ok, Some(false), Some(true))
            .expect("200 字节应通过");
    }

    #[test]
    fn rename_rejects_when_asset_missing() {
        let conn = setup_conn();
        let err = rename_asset_inner(&conn, "nope", "x.pdf", Some(false), Some(true))
            .expect_err("应失败");
        assert_eq!(err, "素材不存在");
    }

    #[test]
    fn rename_derivative_name_uses_sanitize_stem() {
        // 新名含非法字符应被 sanitize_stem 清洗后再拼 .md
        let conn = setup_conn();
        let root = mk_db_root("r_sani");
        db_asset::insert(&conn, &root).unwrap();
        let deriv = mk_db_md_derivative("d_sani", "r_sani");
        db_asset::insert(&conn, &deriv).unwrap();

        // 用户输入含 `/`（合法 display_name，但磁盘非法）
        let _view = rename_asset_inner(&conn, "r_sani", "a/b.pdf", Some(true), Some(true))
            .expect("rename failed");

        let root_after = db_asset::get_by_id(&conn, "r_sani").unwrap().unwrap();
        // root.name 保留用户原始输入（display 层）
        assert_eq!(root_after.name, "a/b.pdf");

        let deriv_after = db_asset::get_by_id(&conn, "d_sani").unwrap().unwrap();
        // derivative.name 走 sanitize_stem：`/` → `_`
        assert_eq!(deriv_after.name, "a_b.md");
    }

    #[test]
    fn build_view_serializes_camel_case() {
        let asset = mk_root_asset("r6");
        let mut join = empty_join();
        join.pipeline_status = Some("queued".to_string());
        let view = build_workspace_view(asset, join, false, true, false);

        let json = serde_json::to_string(&view).expect("ser");
        // DTO 必须 camelCase；枚举 lowercase
        assert!(json.contains("\"projectId\":"), "expect projectId, got {json}");
        assert!(json.contains("\"assetType\":"));
        assert!(json.contains("\"renditionPath\":"));
        assert!(json.contains("\"sourceMissing\":"));
        assert!(json.contains("\"state\":\"converting\""));
    }
}
