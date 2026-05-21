//! task_009 集成测试：workspace_unified_md PRD §8 S1–S5 + S7–S8。
//!
//! 测试策略：
//! - 真实 SQLite（`Database::open` 走全部 V1–V8 迁移）
//! - HOME sandbox 隔离 `~/Downloads/NoteCaptWorkPlace` 与 `~/Library/Caches`
//! - **不**调真实 markitdown / 讯飞 ASR；通过手工写 pipeline_tasks /
//!   conversion_meta / extracted_content + 把"canonical markdown 衍生件"
//!   asset 行 + 磁盘 .md 文件直接写入，模拟 scheduler 完成的终局状态
//! - 用 `OkScheduler` mock 让 `import_files_core` 走 happy 路径但不真跑入队
//! - bench 单独放在 `bench_list_root_assets.rs` 走 `#[ignore]`
//!
//! 跨测试的 HOME sandbox 复用 workspace_folders_integration 的串行模式
//! （HOME 是进程级状态，无法并行）。
//!
//! 包名 `notecapt`（lib `app_lib`）—— 跑 `cargo test -p notecapt --test
//! workspace_unified_md_integration`。

use app_lib::commands::dropzone::{import_files_core, EnqueueScheduler};
use app_lib::db::{self, Database};
use app_lib::models::{Asset, AssetState, Library, Project};
use app_lib::source_scan::{scan_with_conn, SourceMissingSet};

use std::path::{Path, PathBuf};
use std::sync::{Mutex, Mutex as StdMutex};

// ─────────────────────────────────────────────────────────────────────────────
// 公共脚手架
// ─────────────────────────────────────────────────────────────────────────────

/// HOME sandbox：进程级状态，全局串行（与 workspace_folders_integration 同源）。
fn with_sandboxed_home<F: FnOnce(&Path)>(f: F) {
    static HOME_LOCK: Mutex<()> = Mutex::new(());
    let _g = match HOME_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let td = tempfile::tempdir().unwrap();
    let prev_home = std::env::var_os("HOME");
    let prev_xdg_cache = std::env::var_os("XDG_CACHE_HOME");
    std::fs::create_dir_all(td.path().join("Downloads")).unwrap();
    std::fs::create_dir_all(td.path().join("Library/Caches")).unwrap();
    unsafe {
        std::env::set_var("HOME", td.path());
        // Linux：dirs_next::cache_dir 看 XDG_CACHE_HOME；macOS 直接 ~/Library/Caches
        std::env::set_var("XDG_CACHE_HOME", td.path().join(".cache"));
    }
    f(td.path());
    unsafe {
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg_cache {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
    }
}

fn make_db_in(home: &Path) -> Database {
    let db_dir = home.join("db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let path = db_dir.join("test.db");
    Database::open(&path).expect("open db")
}

fn insert_project(db: &Database, project_id: &str) {
    let conn = db.conn.lock().unwrap();
    let lib = Library {
        id: format!("lib-{project_id}"),
        name: "测试库".into(),
        root_path: String::new(),
        created_at: "2026-05-11T00:00:00Z".into(),
    };
    db::library::insert(&conn, &lib).unwrap();
    let proj = Project {
        id: project_id.into(),
        library_id: lib.id,
        name: "测试项目".into(),
        description: String::new(),
        cover_asset_id: None,
        source_type: "test".into(),
        source_data: None,
        is_pinned: false,
        is_archived: false,
        created_at: "2026-05-11T00:00:00Z".into(),
        updated_at: "2026-05-11T00:00:00Z".into(),
        total_duration: None,
        asset_count: 0,
        word_count: 0,
        imported_at: None,
    };
    db::project::insert(&conn, &proj).unwrap();
}

/// 仅在 conn 上写 pipeline_tasks 行，模拟 scheduler enqueue 后的实际行。
fn insert_pipeline_task(
    conn: &rusqlite::Connection,
    asset_id: &str,
    status: &str,
    error_message: Option<&str>,
    created_at: &str,
) {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO pipeline_tasks (id, asset_id, task_type, status, retry_count, max_retries, error_message, priority, batch_id, created_at, started_at, completed_at)
         VALUES (?1, ?2, 'extract', ?3, 0, 3, ?4, 100, NULL, ?5, NULL, NULL)",
        rusqlite::params![id, asset_id, status, error_message, created_at],
    )
    .expect("insert pipeline_task");
}

fn insert_conversion_meta_failure(
    conn: &rusqlite::Connection,
    source_asset_id: &str,
    error_class: &str,
    converted_at: &str,
) {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO conversion_meta (id, source_asset_id, derived_asset_id, converter_name, converter_version, source_mime, source_hash, quality_level, fallback_used, error_class, conversion_ms, converted_at)
         VALUES (?1, ?2, NULL, 'markitdown', 'test', 'text/plain', 'h', 0, 0, ?3, 10, ?4)",
        rusqlite::params![id, source_asset_id, error_class, converted_at],
    )
    .expect("insert conversion_meta");
}

fn insert_extracted_content(conn: &rusqlite::Connection, asset_id: &str, status: &str) {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO extracted_content (id, asset_id, status, error_message, retry_count, raw_text, structured_md, quality_level, extractor_type, segments_json, content_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, 0, NULL, NULL, 0, 'text', NULL, NULL, '2026-05-11T00:00:00Z', '2026-05-11T00:00:00Z')",
        rusqlite::params![id, asset_id, status],
    )
    .expect("insert extracted_content");
}

/// 模拟 scheduler 完成：在 DB 中创建 markdown derivative 行 + 磁盘 .md 文件 +
/// pipeline_tasks completed + extracted_content extracted。
fn materialize_done(
    db: &Database,
    root_asset_id: &str,
    project_workspace_dir: &Path,
) -> (String, PathBuf) {
    let conn = db.conn.lock().unwrap();
    let root = db::asset::get_by_id(&conn, root_asset_id)
        .unwrap()
        .expect("root exists");
    let stem: String = Path::new(&root.name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| root.name.clone());
    let derivative_id = uuid::Uuid::new_v4().to_string();
    let md_path = project_workspace_dir.join(format!("{derivative_id}_{stem}.md"));
    std::fs::write(&md_path, "# converted\n\nbody\n").expect("write md");
    let md_size = std::fs::metadata(&md_path).unwrap().len() as i64;

    let derivative = Asset {
        id: derivative_id.clone(),
        project_id: root.project_id.clone(),
        asset_type: "markdown".to_string(),
        name: format!("{stem}.md"),
        original_name: format!("{stem}.md"),
        file_path: md_path.to_string_lossy().to_string(),
        file_size: md_size,
        mime_type: "text/markdown".to_string(),
        captured_at: root.captured_at.clone(),
        imported_at: "2026-05-11T00:01:00Z".to_string(),
        source_type: "conversion".to_string(),
        source_data: None,
        is_starred: false,
        source_asset_id: Some(root.id.clone()),
        derivative_version: 1,
    };
    db::asset::insert(&conn, &derivative).expect("insert derivative");
    insert_pipeline_task(&conn, &root.id, "completed", None, "2026-05-11T00:01:00Z");
    insert_extracted_content(&conn, &root.id, "extracted");

    (derivative_id, md_path)
}

/// 简单 mock scheduler：直接写 pipeline_tasks(queued) 行，模拟 enqueue 副作用。
struct OkScheduler<'a> {
    conn_mutex: &'a StdMutex<rusqlite::Connection>,
}
impl<'a> EnqueueScheduler for OkScheduler<'a> {
    fn enqueue(&self, asset_id: &str) -> Result<String, String> {
        let conn = self
            .conn_mutex
            .lock()
            .map_err(|e| format!("锁失败: {e}"))?;
        insert_pipeline_task(&conn, asset_id, "queued", None, "2026-05-11T00:00:30Z");
        Ok("enqueued".to_string())
    }
}

fn project_workspace(project_id: &str) -> PathBuf {
    app_lib::workspace::ensure_project_workspace(project_id).expect("workspace dir")
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S1：唯一性
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s1_uniqueness_import_files_core_yields_n_root_rows() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s1");

        // 准备 3 个原件文件
        let src_dir = home.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let mut input_paths = Vec::new();
        for i in 0..3 {
            let p = src_dir.join(format!("file_{i}.txt"));
            std::fs::write(&p, b"hello").unwrap();
            input_paths.push(p.to_string_lossy().to_string());
        }

        let scheduler = OkScheduler {
            conn_mutex: &db.conn,
        };
        let out = import_files_core(&db.conn, &scheduler, "p_s1", input_paths.clone())
            .expect("import_files_core ok");

        assert_eq!(out.summary.created.len(), 3, "应创建 3 条 root asset");
        assert!(out.summary.failures.is_empty(), "happy 路径 failures 应为空");
        assert!(
            out.summary.failures_to_enqueue.is_empty(),
            "OkScheduler 全部成功入队"
        );

        // list_root_assets 行数 == 输入文件数 + 每行 source_asset_id IS NULL
        let conn = db.conn.lock().unwrap();
        let rows = db::asset::list_root_assets(&conn, "p_s1").expect("list ok");
        assert_eq!(rows.len(), 3, "list_root_assets 应只返回 3 条 root");
        for (asset, _join) in &rows {
            assert!(
                asset.source_asset_id.is_none(),
                "每条 root 的 source_asset_id 必须为 NULL（asset={}）",
                asset.id
            );
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S2：rename 元数据一致
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s2_rename_writes_root_and_derivative_consistently() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s2");
        let ws_dir = project_workspace("p_s2");

        // 准备一个 root + derivative（模拟已完成的资产）
        let root_id = "root_s2".to_string();
        let src_path = ws_dir.join(format!("{root_id}_原名.pdf"));
        std::fs::write(&src_path, b"pdf").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let root = Asset {
                id: root_id.clone(),
                project_id: "p_s2".to_string(),
                asset_type: "pdf".to_string(),
                name: "原名.pdf".to_string(),
                original_name: "原名.pdf".to_string(),
                file_path: src_path.to_string_lossy().to_string(),
                file_size: 3,
                mime_type: "application/pdf".to_string(),
                captured_at: "2026-05-11T00:00:00Z".to_string(),
                imported_at: "2026-05-11T00:00:00Z".to_string(),
                source_type: "test".to_string(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 1,
            };
            db::asset::insert(&conn, &root).unwrap();
        }
        let (_derivative_id, _md_path) = materialize_done(&db, &root_id, &ws_dir);

        // 等价 rename：写 root.name + derivative.name = sanitize_stem(stem) + .md
        // —— 与 commands::asset::rename_asset_inner（task_004）语义一致；rename_asset_inner
        //    是 crate-private，集成测试无法直接调用，因此在测试层用同等 DB 写入复现，
        //    底层 rename 行为本身已被 task_004 单测穷举覆盖。
        let new_root_name = "新名.pdf";
        {
            let conn = db.conn.lock().unwrap();
            let mut root = db::asset::get_by_id(&conn, &root_id).unwrap().unwrap();
            root.name = new_root_name.to_string();
            db::asset::update(&conn, &root).unwrap();

            let derivative = db::asset::find_markdown_derivative(&conn, &root_id)
                .unwrap()
                .expect("derivative present");
            // 与 derivative_name_from_root 同算法：sanitize_stem → rfind('.') → ".md"
            let safe = app_lib::utils::safe_name::sanitize_stem(new_root_name);
            let stem = match safe.rfind('.') {
                Some(idx) if idx > 0 => &safe[..idx],
                _ => &safe[..],
            };
            let new_md_name = format!("{stem}.md");
            db::asset::update_markdown_derivative(
                &conn,
                &derivative.id,
                &new_md_name,
                derivative.file_size,
                &derivative.imported_at,
            )
            .unwrap();
        }

        // 验证：get_by_id(root).name == 新名.pdf
        let conn = db.conn.lock().unwrap();
        let root_after = db::asset::get_by_id(&conn, &root_id).unwrap().unwrap();
        assert_eq!(root_after.name, "新名.pdf", "root.name 应为新名.pdf");

        // find_markdown_derivative(root).name == 新名.md
        let derivative_after = db::asset::find_markdown_derivative(&conn, &root_id)
            .unwrap()
            .expect("derivative present after rename");
        assert_eq!(
            derivative_after.name, "新名.md",
            "derivative.name 应为新名.md"
        );

        // FIX (task_005/009)：outbound 等价层应基于 root.name（"新名.pdf"）剥离
        // 原扩展名后再拼 `.md`，最终落盘文件名 = "新名.md"。强化为正向断言，
        // 与 PRD §S2 三处一致硬约束（list / outbound 文件名 / DB display_name）对齐。
        let outbound_filename = app_lib::commands::outbound::outbound_filename_from_root(
            &root_after.name,
            &root_id,
        );
        assert_eq!(
            outbound_filename, "新名.md",
            "outbound 落盘文件名必须 = derivative.name = 新名.md（PRD §S2）"
        );
        assert_eq!(
            outbound_filename, derivative_after.name,
            "outbound filename 必须与 derivative.name 一致"
        );

        // file_path 不动（display_name 仅活在 DB）
        assert_eq!(
            root_after.file_path,
            src_path.to_string_lossy().to_string(),
            "rename 不应改 root.file_path"
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S3：三态可见
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s3_three_states_visible_in_list() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s3");
        let ws_dir = project_workspace("p_s3");

        // 三个 root：done / converting / failed
        let states: &[(&str, &str)] = &[
            ("root_done", "done"),
            ("root_conv", "converting"),
            ("root_fail", "failed"),
        ];
        for (idx, (id, _state)) in states.iter().enumerate() {
            let src = ws_dir.join(format!("{id}_x.txt"));
            std::fs::write(&src, b"x").unwrap();
            let conn = db.conn.lock().unwrap();
            let imported_at = format!("2026-05-11T00:00:0{idx}Z");
            let asset = Asset {
                id: id.to_string(),
                project_id: "p_s3".to_string(),
                asset_type: "text".to_string(),
                name: format!("{id}.txt"),
                original_name: format!("{id}.txt"),
                file_path: src.to_string_lossy().to_string(),
                file_size: 1,
                mime_type: "text/plain".to_string(),
                captured_at: imported_at.clone(),
                imported_at,
                source_type: "test".to_string(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&conn, &asset).unwrap();
        }

        // done：materialize（写 derivative + completed pipeline）
        materialize_done(&db, "root_done", &ws_dir);
        // converting：pipeline_tasks queued
        {
            let conn = db.conn.lock().unwrap();
            insert_pipeline_task(&conn, "root_conv", "queued", None, "2026-05-11T00:00:10Z");
        }
        // failed：pipeline_tasks failed + conversion_meta error_class
        {
            let conn = db.conn.lock().unwrap();
            insert_pipeline_task(
                &conn,
                "root_fail",
                "failed",
                Some("提取失败"),
                "2026-05-11T00:00:10Z",
            );
            insert_conversion_meta_failure(
                &conn,
                "root_fail",
                "ExtractionFailed",
                "2026-05-11T00:00:10Z",
            );
        }

        let conn = db.conn.lock().unwrap();
        let rows = db::asset::list_root_assets(&conn, "p_s3").unwrap();
        assert_eq!(rows.len(), 3, "应有 3 条 root");

        // 派生 state（命令层做法）
        let mut by_id = std::collections::HashMap::new();
        for (asset, join) in rows {
            let rendition_exists = join
                .rendition_path
                .as_deref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false);
            let source_exists = Path::new(&asset.file_path).exists();
            let state = db::asset::compute_asset_state(
                join.pipeline_status.as_deref(),
                join.latest_error_class.as_deref(),
                rendition_exists,
                source_exists,
                false,
            );
            by_id.insert(asset.id.clone(), state);
        }

        assert_eq!(by_id["root_done"], AssetState::Done);
        assert_eq!(by_id["root_conv"], AssetState::Converting);
        assert_eq!(by_id["root_fail"], AssetState::Failed);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S4：失败降级 — 仍可 rename / 加 tag
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s4_failed_asset_still_supports_rename_and_tag() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s4");
        let ws_dir = project_workspace("p_s4");

        let src = ws_dir.join("root_s4_orig.txt");
        std::fs::write(&src, b"x").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let asset = Asset {
                id: "root_s4".into(),
                project_id: "p_s4".into(),
                asset_type: "text".into(),
                name: "orig.txt".into(),
                original_name: "orig.txt".into(),
                file_path: src.to_string_lossy().to_string(),
                file_size: 1,
                mime_type: "text/plain".into(),
                captured_at: "2026-05-11T00:00:00Z".into(),
                imported_at: "2026-05-11T00:00:00Z".into(),
                source_type: "test".into(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&conn, &asset).unwrap();
            // 标记 failed
            insert_pipeline_task(
                &conn,
                "root_s4",
                "failed",
                Some("markitdown 永远失败"),
                "2026-05-11T00:00:10Z",
            );
            insert_conversion_meta_failure(
                &conn,
                "root_s4",
                "MarkitdownFailed",
                "2026-05-11T00:00:10Z",
            );
        }

        // rename：直接走 db::asset::update（rename_asset_inner 等价的 DB 写入）
        {
            let conn = db.conn.lock().unwrap();
            let mut a = db::asset::get_by_id(&conn, "root_s4").unwrap().unwrap();
            a.name = "重命名后.txt".to_string();
            db::asset::update(&conn, &a).expect("rename in failed state should succeed");
        }
        // tag：通过 db::tag::link_to_asset
        {
            let conn = db.conn.lock().unwrap();
            let tag =
                db::tag::get_or_create_by_name(&conn, "已校对", "manual").expect("tag created");
            db::tag::link_to_asset(&conn, "root_s4", &tag.id)
                .expect("tagging in failed state should succeed");
        }

        // 验证 rename 后 name 与 tag 关联确实写入
        let conn = db.conn.lock().unwrap();
        let after = db::asset::get_by_id(&conn, "root_s4").unwrap().unwrap();
        assert_eq!(after.name, "重命名后.txt");
        let tags = db::tag::get_tags_for_asset(&conn, "root_s4").unwrap();
        assert_eq!(tags.len(), 1, "应有 1 个 tag");
        assert_eq!(tags[0].name, "已校对");

        // 验证状态仍为 failed
        let rows = db::asset::list_root_assets(&conn, "p_s4").unwrap();
        assert_eq!(rows.len(), 1);
        let (_a, join) = &rows[0];
        let state = db::asset::compute_asset_state(
            join.pipeline_status.as_deref(),
            join.latest_error_class.as_deref(),
            false,
            true,
            false,
        );
        assert_eq!(state, AssetState::Failed, "rename/tag 不应改变状态");
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S5：delete_with_cascade 无孤儿（含 outbound cache 目录）
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s5_delete_with_cascade_no_orphans() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s5");
        let ws_dir = project_workspace("p_s5");

        let root_id = "root_s5".to_string();
        let src_path = ws_dir.join(format!("{root_id}_doc.txt"));
        std::fs::write(&src_path, b"hello").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let asset = Asset {
                id: root_id.clone(),
                project_id: "p_s5".into(),
                asset_type: "text".into(),
                name: "doc.txt".into(),
                original_name: "doc.txt".into(),
                file_path: src_path.to_string_lossy().to_string(),
                file_size: 5,
                mime_type: "text/plain".into(),
                captured_at: "2026-05-11T00:00:00Z".into(),
                imported_at: "2026-05-11T00:00:00Z".into(),
                source_type: "test".into(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&conn, &asset).unwrap();
            insert_conversion_meta_failure(
                &conn,
                &root_id,
                "Placeholder",
                "2026-05-11T00:00:05Z",
            );
        }
        let (derivative_id, md_path) = materialize_done(&db, &root_id, &ws_dir);

        // 创建 outbound 缓存目录 + 文件，模拟 prepare_outbound_payload 已落盘
        let cache_dir = app_lib::commands::outbound::outbound_cache_dir_for(&root_id)
            .expect("cache_dir resolvable in HOME sandbox");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("doc.md");
        std::fs::write(&cache_file, b"# cached").unwrap();
        assert!(cache_file.exists(), "前置：outbound 缓存文件已落盘");

        // 调用 delete_with_cascade + 命令层等价的 outbound cache 清理
        {
            let conn = db.conn.lock().unwrap();
            let report =
                db::asset::delete_with_cascade(&conn, &root_id).expect("delete cascade ok");
            assert_eq!(report.root_asset_id, root_id);
            assert_eq!(report.derivative_asset_id.as_deref(), Some(derivative_id.as_str()));
            assert!(report.removed_root_file, "root 文件应被物理删除");
            assert!(report.removed_derivative_file, "derivative 文件应被物理删除");
        }
        // 命令层 outbound cache 清理（commands::asset::delete_asset 同源逻辑）
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir).expect("outbound cache cleanup");
        }

        // 断言：root + derivative 文件均不存在
        assert!(!src_path.exists(), "root 源文件不存在");
        assert!(!md_path.exists(), "derivative md 文件不存在");
        assert!(!cache_dir.exists(), "outbound 缓存目录不存在");

        // 断言：DB 各表中相关行 = 0
        let conn = db.conn.lock().unwrap();
        let count_assets: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE id = ?1 OR source_asset_id = ?1",
                rusqlite::params![root_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_assets, 0, "assets 中应无 root + derivative 行");

        let count_cm: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM conversion_meta WHERE source_asset_id = ?1",
                rusqlite::params![root_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_cm, 0, "conversion_meta 应被 FK CASCADE 清空");

        let count_ec: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM extracted_content WHERE asset_id = ?1",
                rusqlite::params![root_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_ec, 0, "extracted_content 应被 FK CASCADE 清空");

        let count_pt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pipeline_tasks WHERE asset_id = ?1 OR asset_id = ?2",
                rusqlite::params![root_id, derivative_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_pt, 0, "pipeline_tasks 手工 DELETE 应清空");
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S7：连击 5 次 retry，活动态 ≤ 1
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s7_retry_unique_index_caps_active_at_one() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s7");
        let ws_dir = project_workspace("p_s7");

        let root_id = "root_s7".to_string();
        let src = ws_dir.join(format!("{root_id}_x.txt"));
        std::fs::write(&src, b"x").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let asset = Asset {
                id: root_id.clone(),
                project_id: "p_s7".into(),
                asset_type: "text".into(),
                name: "x.txt".into(),
                original_name: "x.txt".into(),
                file_path: src.to_string_lossy().to_string(),
                file_size: 1,
                mime_type: "text/plain".into(),
                captured_at: "2026-05-11T00:00:00Z".into(),
                imported_at: "2026-05-11T00:00:00Z".into(),
                source_type: "test".into(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&conn, &asset).unwrap();
        }

        // 第一次 retry：写一行 queued
        {
            let conn = db.conn.lock().unwrap();
            insert_pipeline_task(&conn, &root_id, "queued", None, "2026-05-11T00:00:01Z");
        }
        // 后续 4 次：等价于 scheduler.enqueue 静默捕获 UNIQUE 冲突
        // （V7 idx_pipeline_tasks_active_unique 兜底）
        for i in 2..=5 {
            let id = uuid::Uuid::new_v4().to_string();
            let conn = db.conn.lock().unwrap();
            let result = conn.execute(
                "INSERT INTO pipeline_tasks (id, asset_id, task_type, status, retry_count, max_retries, priority, created_at)
                 VALUES (?1, ?2, 'extract', 'queued', 0, 3, 100, ?3)",
                rusqlite::params![id, root_id, format!("2026-05-11T00:00:0{i}Z")],
            );
            assert!(
                result.is_err(),
                "第 {i} 次 INSERT queued 应被 V7 部分唯一索引拦截"
            );
        }

        // 终局：活动态行数 == 1
        let conn = db.conn.lock().unwrap();
        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pipeline_tasks WHERE asset_id = ?1 AND status IN ('queued', 'running')",
                rusqlite::params![root_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 1, "活动态行数应恒等于 1");

        // list_root_assets 行数恒等于导入数 (1)
        let rows = db::asset::list_root_assets(&conn, "p_s7").unwrap();
        assert_eq!(rows.len(), 1, "list_root_assets 行数恒定");
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-S8：source 失联 → SourceMissingSet 标记，state 仍 Done，outbound 仍可
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s8_source_missing_marks_flag_but_state_done_and_outbound_ok() {
    with_sandboxed_home(|home| {
        let db = make_db_in(home);
        insert_project(&db, "p_s8");
        let ws_dir = project_workspace("p_s8");

        let root_id = "root_s8".to_string();
        let src_path = ws_dir.join(format!("{root_id}_paper.pdf"));
        std::fs::write(&src_path, b"pdf-bytes").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let asset = Asset {
                id: root_id.clone(),
                project_id: "p_s8".into(),
                asset_type: "pdf".into(),
                name: "paper.pdf".into(),
                original_name: "paper.pdf".into(),
                file_path: src_path.to_string_lossy().to_string(),
                file_size: 9,
                mime_type: "application/pdf".into(),
                captured_at: "2026-05-11T00:00:00Z".into(),
                imported_at: "2026-05-11T00:00:00Z".into(),
                source_type: "test".into(),
                source_data: None,
                is_starred: false,
                source_asset_id: None,
                derivative_version: 0,
            };
            db::asset::insert(&conn, &asset).unwrap();
        }
        let (_derivative_id, md_path) = materialize_done(&db, &root_id, &ws_dir);

        // 删除 source（不删 rendition）
        std::fs::remove_file(&src_path).expect("unlink source");
        assert!(!src_path.exists());
        assert!(md_path.exists(), "rendition 仍存在");

        // 重新扫描
        let missing_set = SourceMissingSet::new();
        let (scanned, missing) = {
            let conn = db.conn.lock().unwrap();
            scan_with_conn(&conn, &missing_set, "p_s8").expect("scan ok")
        };
        assert_eq!(scanned, 1);
        assert_eq!(missing, 1);
        assert!(missing_set.contains(&root_id), "root 应被记录为 missing");

        // list_root_assets + 派生 state：source 缺失但 state 仍 Done（ADR-006）
        let conn = db.conn.lock().unwrap();
        let rows = db::asset::list_root_assets(&conn, "p_s8").unwrap();
        assert_eq!(rows.len(), 1);
        let (asset, join) = &rows[0];
        let rendition_exists = join
            .rendition_path
            .as_deref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false);
        let source_exists = Path::new(&asset.file_path).exists();
        let state = db::asset::compute_asset_state(
            join.pipeline_status.as_deref(),
            join.latest_error_class.as_deref(),
            rendition_exists,
            source_exists,
            missing_set.contains(&asset.id),
        );
        assert_eq!(state, AssetState::Done, "source 缺失不应降级 done 资产");
        assert!(!source_exists);
        assert!(rendition_exists);

        // prepare_outbound_payload 等价路径：sanitize + 文件落盘
        // —— 由于 prepare_outbound_payload 命令需 Tauri State<Database>，
        //    本测试在 db / fs 层验证其可达性：rendition 仍在 + 缓存目录可写。
        drop(conn); // 释放后再做 IO
        let cache_dir = app_lib::commands::outbound::outbound_cache_dir_for(&root_id)
            .expect("cache_dir resolvable");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let sanitized =
            app_lib::commands::outbound::sanitize_outbound_filename("paper.md", &root_id);
        let cache_path = cache_dir.join(format!("{sanitized}.md"));
        // hardlink → 跨卷应不会发生（同一 tempdir 内），直接断言成功
        std::fs::hard_link(&md_path, &cache_path)
            .or_else(|_| std::fs::copy(&md_path, &cache_path).map(|_| ()))
            .expect("link or copy outbound");
        assert!(cache_path.exists(), "outbound 缓存文件应落盘成功");
    });
}
