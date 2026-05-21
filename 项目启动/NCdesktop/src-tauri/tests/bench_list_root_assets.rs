//! task_009 AC-Bench：10000 root + 10000 derivative + 各自一行 pipeline_tasks /
//! extracted_content / conversion_meta 的 list_root_assets 性能基线。
//!
//! 默认 `#[ignore]`，手动运行：
//!   cargo test -p notecapt --test bench_list_root_assets -- --ignored --nocapture
//!
//! 阈值（PRD §9 / ADR-003）：
//!   - 警戒线 200ms：若超过则提示需考虑升级 V9（加 cached_state 列）
//!   - 硬上限 500ms：超过即 panic
//!
//! 测试在 tempfile 隔离的 SQLite 上构造数据，避免触碰真实工作区。

use app_lib::db::Database;
use std::time::Instant;

const N_ROOTS: usize = 10_000;
const HARD_LIMIT_MS: u128 = 500;
const WARN_THRESHOLD_MS: u128 = 200;

#[test]
#[ignore]
fn bench_list_root_assets_at_10k() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("bench.db");
    let db = Database::open(&db_path).expect("open db");

    let conn = db.conn.lock().unwrap();

    // 单 library + 单 project
    conn.execute(
        "INSERT INTO libraries (id, name, root_path, created_at) VALUES ('lib-bench', 'bench', '', '2026-05-13T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO projects (id, library_id, name, source_type, created_at, updated_at)
         VALUES ('p-bench', 'lib-bench', 'bench', 'test', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z')",
        [],
    )
    .unwrap();

    // 用单一事务批量插入 10k root + 10k derivative + 各自 pipeline_task /
    // extracted_content / conversion_meta（写入也是性能压点）
    conn.execute("BEGIN", []).unwrap();
    for i in 0..N_ROOTS {
        let root_id = format!("root_{i:06}");
        let der_id = format!("der_{i:06}");
        let pt_root = format!("pt_root_{i:06}");
        let ec_id = format!("ec_{i:06}");
        let cm_id = format!("cm_{i:06}");
        let imported_at = format!("2026-05-13T00:00:{:02}Z", i % 60);

        // root
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, original_name, file_path,
                file_size, mime_type, captured_at, imported_at, source_type, source_data,
                is_starred, source_asset_id, derivative_version)
             VALUES (?1, 'p-bench', 'pdf', ?2, ?2, ?3, 100, 'application/pdf',
                ?4, ?4, 'test', NULL, 0, NULL, 1)",
            rusqlite::params![
                root_id,
                format!("doc_{i:06}.pdf"),
                format!("/tmp/bench/{root_id}.pdf"),
                imported_at,
            ],
        )
        .unwrap();
        // derivative
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, original_name, file_path,
                file_size, mime_type, captured_at, imported_at, source_type, source_data,
                is_starred, source_asset_id, derivative_version)
             VALUES (?1, 'p-bench', 'markdown', ?2, ?2, ?3, 50, 'text/markdown',
                ?4, ?4, 'conversion', NULL, 0, ?5, 1)",
            rusqlite::params![
                der_id,
                format!("doc_{i:06}.md"),
                format!("/tmp/bench/{der_id}.md"),
                imported_at,
                root_id,
            ],
        )
        .unwrap();
        // pipeline_tasks (completed)
        conn.execute(
            "INSERT INTO pipeline_tasks (id, asset_id, task_type, status, retry_count,
                max_retries, priority, created_at, completed_at)
             VALUES (?1, ?2, 'extract', 'completed', 0, 3, 100, ?3, ?3)",
            rusqlite::params![pt_root, root_id, imported_at],
        )
        .unwrap();
        // extracted_content
        conn.execute(
            "INSERT INTO extracted_content (id, asset_id, status, retry_count, quality_level,
                created_at, updated_at)
             VALUES (?1, ?2, 'extracted', 0, 0, ?3, ?3)",
            rusqlite::params![ec_id, root_id, imported_at],
        )
        .unwrap();
        // conversion_meta
        conn.execute(
            "INSERT INTO conversion_meta (id, source_asset_id, derived_asset_id, converter_name,
                converter_version, source_mime, source_hash, quality_level, fallback_used,
                error_class, conversion_ms, converted_at)
             VALUES (?1, ?2, ?3, 'markitdown', 'builtin', 'application/pdf', 'h', 0, 0,
                NULL, 10, ?4)",
            rusqlite::params![cm_id, root_id, der_id, imported_at],
        )
        .unwrap();
    }
    conn.execute("COMMIT", []).unwrap();

    // 多跑 3 次取 min，规避冷热缓存噪声
    let mut samples: Vec<u128> = Vec::with_capacity(3);
    for _ in 0..3 {
        let t = Instant::now();
        let rows =
            app_lib::db::asset::list_root_assets(&conn, "p-bench").expect("list_root_assets ok");
        let elapsed = t.elapsed().as_millis();
        assert_eq!(rows.len(), N_ROOTS, "应返回 {N_ROOTS} 条 root");
        samples.push(elapsed);
    }
    let best = *samples.iter().min().unwrap();
    let worst = *samples.iter().max().unwrap();

    println!(
        "[bench_list_root_assets_at_10k] N={N_ROOTS}, samples_ms={:?}, best={}ms, worst={}ms",
        samples, best, worst
    );

    if best > WARN_THRESHOLD_MS {
        println!(
            "WARN: list_root_assets best={}ms 已超过 ADR-003 警戒线 {}ms — \
             需评估升级 V9 加 cached_state 列",
            best, WARN_THRESHOLD_MS
        );
    }
    assert!(
        best <= HARD_LIMIT_MS,
        "list_root_assets best 耗时 {}ms 超过硬上限 {}ms",
        best,
        HARD_LIMIT_MS
    );
}
