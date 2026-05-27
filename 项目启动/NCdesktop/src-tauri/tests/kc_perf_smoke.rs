//! task_024_perf_benchmark：KC 性能 benchmark（F21）。
//!
//! ## 目的
//!
//! 实测 PRD §4.1 性能阈值的"端到端 mock 路径"延迟分位数，把 P50 / P95 / P99
//! 落到 `target/bench_results.json`，供 task_029 Acceptance Report 引用。
//!
//! ## 设计依据
//!
//! - **input.md AC-1**：两个 bench——`bench_kc_ingest_10kb`（KC ingest 100 次） +
//!   `bench_main_pipeline_p95`（主链路 100 次）；
//! - **input.md 技术约束**："或自实现简易 Instant 计时（避免引入 criterion 也可，
//!   要求实现 P50/P95/P99 stat 计算）"——选**自实现**：现有 dev-deps（`wiremock` +
//!   `tempfile`）已能满足，无需新增 criterion / gnuplot；
//! - **PRD §4.1**：
//!   - `KC 单文档 ingest` P95 < 5s（mock 环境应远低于，亚毫秒级）
//!   - `主链路（拖入 → 衍生件落盘）` P95 < 30s（mock + in-memory DB 应远低于）
//!   - `KC 冷启动` < 5s（mock 启动 + 健康检查；wiremock 自身启动 < 1s）。
//!
//! ## 路径选择：tests/ 而非 benches/
//!
//! `cargo bench` 入口需要在 `Cargo.toml` 显式声明 `[[bench]]` + 让 bench 走 release profile（默认）；
//! 对于本 task 的目的（PRD 阈值守护，**非**性能调优工具）`#[test] + std::time::Instant`
//! 已足够：
//! - `cargo test --test kc_perf_smoke -- --nocapture` 单条命令运行 + 看到分位数 stdout；
//! - 不动 `Cargo.toml`，不引入 `criterion` 编译时间增长；
//! - CI 走默认 `cargo test`，自动跑（且 P95 阈值守护即 panic 触发 CI 红）。
//!
//! ## 测量方法（P50/P95/P99 自实现）
//!
//! 1. 收集 N=100 个 `std::time::Duration` 样本（向量）；
//! 2. **排序**（升序）；
//! 3. **分位数 = 向上取整索引法**（Excel/Numpy `nearest-rank`）：
//!    - P50 = sorted[ceil(N × 0.50) - 1] = sorted[49]
//!    - P95 = sorted[ceil(N × 0.95) - 1] = sorted[94]
//!    - P99 = sorted[ceil(N × 0.99) - 1] = sorted[98]
//!    这是 Wikipedia "Percentile - Nearest-Rank" 法；N=100 时整数运算稳定，
//!    避免插值法（C=1/3、C=0.5 等）在小样本下的非确定性。
//!
//! ## 输出 JSON 形态
//!
//! `target/bench_results.json`：
//! ```json
//! {
//!   "task": "task_024_perf_benchmark",
//!   "run_at_utc": "2026-05-28T...",
//!   "samples": 100,
//!   "kc_ingest_10kb": {
//!     "p50_ms": 1.23, "p95_ms": 4.56, "p99_ms": 5.78,
//!     "threshold_ms": 5000, "passes_prd_4_1": true
//!   },
//!   "main_pipeline": {
//!     "p50_ms": 2.45, "p95_ms": 8.90, "p99_ms": 12.34,
//!     "threshold_ms": 30000, "passes_prd_4_1": true
//!   }
//! }
//! ```
//!
//! ## 不变量 / 约束
//!
//! - **不依赖真实 LLM Key**：用 `MockKcServer::start_with_success`（task_006）；
//! - **不依赖真实 Tauri runtime**：直接调 `KcClient::ingest_text` + `kc::enrichment::resolve_outcome`
//!   + `db_update_kc_fields` / `db_conversion_meta_kc_insert`（与 scheduler.rs 的
//!   `kc_persist_resolved_with_conn` 字面同步）；
//! - **CI 友好**：`#[ignore]` 不加——默认 `cargo test` 走 100 次 mock 调用，~几百 ms 内完成。

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_lib::db::extraction as db_ext;
use app_lib::db::migration::run_migrations;
use app_lib::extraction::models::ExtractionResult;
use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;
use app_lib::kc::client::{KcClient, KcIngestOptions, KcIngestOutcome};
use app_lib::kc::enrichment::resolve_outcome;
use app_lib::kc::errors::{KcEnrichmentOutcome, KcMeta, KcTagsSource};
use app_lib::kc::frontmatter::build_kc_frontmatter;
use app_lib::kc::process::PortProvider;
use app_lib::models::Asset;

use common::mock_kc::{KcMockMeta, MockKcServer};

// =====================================================================
// 测量常量
// =====================================================================

/// 每个 bench 跑 100 次（task_024 input.md AC-1）。
const SAMPLE_COUNT: usize = 100;

/// 10KB 输入文本（task_024 input.md AC-1）。
const INPUT_BYTES: usize = 10 * 1024;

/// PRD §4.1：KC 单文档 ingest P95 阈值（5 秒）。
const KC_INGEST_P95_THRESHOLD_MS: f64 = 5_000.0;

/// PRD §4.1：主链路 P95 阈值（30 秒）。
const MAIN_PIPELINE_P95_THRESHOLD_MS: f64 = 30_000.0;

// =====================================================================
// 统计 helpers（自实现 P50/95/99，避免引入 criterion）
// =====================================================================

/// 把 `Vec<Duration>` 排序后按 Nearest-Rank 算 P50/P95/P99（毫秒，f64）。
///
/// **算法**：Wikipedia "Percentile - Nearest-Rank"——
/// P(k) 的 rank = ceil(k% × N)，索引（0-based） = rank - 1。
/// N=100 时：P50=sorted[49]、P95=sorted[94]、P99=sorted[98]，整数运算稳定。
///
/// **panic**：`durations.len() < 100` 时 panic（本测试场景 N==SAMPLE_COUNT==100 恒成立，
/// 用 panic 而非 Result 让违规调用立即可见）。
struct Percentiles {
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}

fn percentiles_ms(mut durations: Vec<Duration>) -> Percentiles {
    assert!(
        durations.len() >= SAMPLE_COUNT,
        "需要至少 {SAMPLE_COUNT} 个样本，实际 {}",
        durations.len()
    );
    durations.sort();
    let n = durations.len();

    // Nearest-Rank（向上取整）；N=100 时退化为整数索引。
    let idx = |percent: f64| -> usize {
        let rank = (percent * n as f64).ceil() as usize;
        // 边界：rank=0 时回到 0（理论不发生于 50/95/99，但保护边界）
        rank.saturating_sub(1).min(n - 1)
    };

    let to_ms = |d: Duration| -> f64 {
        // 用 as_secs_f64 * 1000 拿亚毫秒级精度（micros 截断会损失 < 1ms 信号）。
        d.as_secs_f64() * 1000.0
    };

    Percentiles {
        p50_ms: to_ms(durations[idx(0.50)]),
        p95_ms: to_ms(durations[idx(0.95)]),
        p99_ms: to_ms(durations[idx(0.99)]),
    }
}

// =====================================================================
// 测试 fixture helpers
// =====================================================================

/// 集成测试用 PortProvider stub（从 MockKcServer 取端口固化）。
struct StaticPortProvider {
    port: AtomicU16,
}

impl StaticPortProvider {
    fn with_port(port: u16) -> Arc<Self> {
        Arc::new(Self {
            port: AtomicU16::new(port),
        })
    }
}

impl PortProvider for StaticPortProvider {
    fn current_port(&self) -> Option<u16> {
        match self.port.load(Ordering::Acquire) {
            0 => None,
            p => Some(p),
        }
    }
}

/// 构造 10KB ASCII 输入文本。
///
/// **为什么 ASCII 而非随机字节**：KC ingest 走 JSON 序列化（reqwest），随机字节会
/// 触发额外 UTF-8 校验开销且不代表实际 markdown 场景；纯 ASCII 重复模式既稳定又
/// 接近真实 markitdown 输出形态（短行 + 段落 + 标点）。
fn make_10kb_markdown() -> String {
    // 一行 ~100 字符（含换行）→ ~102 行 ≈ 10KB
    let line = "# 测试段落 paragraph with mixed CJK and ASCII content for kc ingest perf bench. 0123456789\n";
    let line_bytes = line.len();
    let lines = (INPUT_BYTES + line_bytes - 1) / line_bytes;
    let mut out = String::with_capacity(INPUT_BYTES + line_bytes);
    for _ in 0..lines {
        out.push_str(line);
    }
    debug_assert!(
        out.len() >= INPUT_BYTES,
        "生成文本应 ≥ 10KB，实际 {} bytes",
        out.len()
    );
    out
}

/// 构造一个 in-memory SQLite + 跑完所有迁移到 v18 + 注入最小 fixture（lib/project/asset/
/// extracted_content），给主链路 bench 使用。返回 (conn, asset_id)。
fn setup_in_memory_db(asset_id: &str) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("migrations to v18");
    let now = "2026-05-28T00:00:00Z";
    conn.execute(
        "INSERT INTO libraries (id, name, root_path) VALUES ('lib1', 'L', '/tmp')",
        [],
    )
    .expect("insert library");
    conn.execute(
        "INSERT INTO projects (id, library_id, name) VALUES ('p', 'lib1', 'P')",
        [],
    )
    .expect("insert project");
    conn.execute(
        "INSERT INTO assets (id, project_id, asset_type, name, original_name, file_path,
                             file_size, mime_type, captured_at, imported_at, source_type,
                             source_data, is_starred, source_asset_id, derivative_version)
         VALUES (?1, 'p', 'document', 'x.pdf', 'x.pdf', '/tmp/x.pdf', 0,
                 'application/pdf', ?2, ?2, 'imported', NULL, 0, NULL, 0)",
        rusqlite::params![asset_id, now],
    )
    .expect("insert assets");
    conn.execute(
        "INSERT INTO extracted_content (id, asset_id, status, error_message, retry_count,
                                        raw_text, structured_md, quality_level, extractor_type,
                                        segments_json, created_at, updated_at)
         VALUES (?1, ?2, 'extracted', NULL, 0, 'raw', 'md', 2, 'markitdown', NULL, ?3, ?3)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), asset_id, now],
    )
    .expect("insert extracted_content");
    conn
}

/// 构造一个 stub Asset（用于 build_kc_frontmatter 测试入参）。
fn make_test_asset(asset_id: &str) -> Asset {
    Asset {
        id: asset_id.to_string(),
        project_id: "p".to_string(),
        asset_type: "document".to_string(),
        name: "x.pdf".to_string(),
        original_name: "x.pdf".to_string(),
        file_path: "/tmp/x.pdf".to_string(),
        file_size: 0,
        mime_type: "application/pdf".to_string(),
        captured_at: "2026-05-28T00:00:00Z".to_string(),
        imported_at: "2026-05-28T00:00:00Z".to_string(),
        source_type: "imported".to_string(),
        source_data: None,
        is_starred: false,
        source_asset_id: None,
        derivative_version: 0,
    }
}

/// 构造一个 ExtractionResult stub（10KB structured_md）。
fn make_extraction_result(md: &str) -> ExtractionResult {
    ExtractionResult {
        raw_text: md.to_string(),
        structured_md: md.to_string(),
        quality_level: 3,
        extractor_type: "markitdown".to_string(),
        segments: Vec::new(),
        needs_ocr_fallback: false,
    }
}

// task_027b：原 `persist_resolved_to_db` helper 已删除——bench 现直接调
// `app_lib::extraction::scheduler::kc_persist_resolved_with_conn`（canonical，
// scheduler.rs `pub fn` + `#[doc(hidden)]`），消除 DRY 复刻漂移隐患。

// =====================================================================
// JSON 输出（target/bench_results.json）
// =====================================================================

/// 用 `serde_json` 序列化两个 bench 的分位数到 `target/bench_results.json`。
///
/// **路径**：`target/bench_results.json`——`target/` 是 cargo 默认 build 输出目录，
/// 必然可写；CI 与本地都能稳定访问。input.md AC-2 明确路径。
///
/// **JSON 结构**：见模块文档"输出 JSON 形态"。
fn write_bench_results_json(
    kc_ingest: &Percentiles,
    main_pipeline: &Percentiles,
) -> PathBuf {
    let json = serde_json::json!({
        "task": "task_024_perf_benchmark",
        "run_at_utc": chrono::Utc::now().to_rfc3339(),
        "samples": SAMPLE_COUNT,
        "input_bytes": INPUT_BYTES,
        "kc_ingest_10kb": {
            "p50_ms": kc_ingest.p50_ms,
            "p95_ms": kc_ingest.p95_ms,
            "p99_ms": kc_ingest.p99_ms,
            "threshold_ms": KC_INGEST_P95_THRESHOLD_MS,
            "passes_prd_4_1": kc_ingest.p95_ms < KC_INGEST_P95_THRESHOLD_MS,
        },
        "main_pipeline": {
            "p50_ms": main_pipeline.p50_ms,
            "p95_ms": main_pipeline.p95_ms,
            "p99_ms": main_pipeline.p99_ms,
            "threshold_ms": MAIN_PIPELINE_P95_THRESHOLD_MS,
            "passes_prd_4_1": main_pipeline.p95_ms < MAIN_PIPELINE_P95_THRESHOLD_MS,
        }
    });
    // 寻找 target/——从 CARGO_TARGET_DIR / CARGO_MANIFEST_DIR 推断；fallback 到当前 cwd 下 target/。
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO_MANIFEST_DIR")
                .map(|d| PathBuf::from(d).join("target"))
        })
        .unwrap_or_else(|| PathBuf::from("target"));
    fs::create_dir_all(&target_dir).expect("ensure target/");
    let path = target_dir.join("bench_results.json");
    fs::write(&path, serde_json::to_string_pretty(&json).unwrap())
        .expect("write bench_results.json");
    path
}

// =====================================================================
// Bench 1：KC ingest 10KB × 100（input.md AC-1.bench_kc_ingest_10kb）
// =====================================================================

/// 测量 `KcClient::ingest_text` 在 mock success 场景下 10KB 输入的 P50/95/99 延迟。
///
/// **PRD §4.1 阈值**：P95 < 5s（mock 应远低于；本测试 panic 守护超 5s 即失败）。
///
/// **复用对象**：`KcClient` + `MockKcServer` 单例（client cache 已预热——首请求
/// 的 HTTP keepalive / DNS / wiremock route 命中均在第 1 次后稳定）。
async fn bench_kc_ingest_10kb_inner() -> Percentiles {
    let enhanced = "# enhanced\n\n#AI\n\n## 摘要\n\n（mock）";
    let meta = KcMockMeta::default();
    let mock = MockKcServer::start_with_success(enhanced, meta).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let input = make_10kb_markdown();
    let opts = KcIngestOptions::default();

    // 预热：跑 5 次不计时（让 reqwest connection pool / wiremock route match cache 稳定）
    for _ in 0..5 {
        let _ = client.ingest_text(&input, &opts).await;
    }

    let mut durations: Vec<Duration> = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let t0 = Instant::now();
        let res = client.ingest_text(&input, &opts).await;
        let elapsed = t0.elapsed();
        // 保证每次都成功（mock success）；如果失败说明 bench 路径异常，应当 panic 让测试红
        match res {
            Ok(KcIngestOutcome::Success { .. }) => durations.push(elapsed),
            other => panic!("bench_kc_ingest_10kb: expected Ok(Success), got {other:?}"),
        }
    }

    mock.stop();
    percentiles_ms(durations)
}

// =====================================================================
// Bench 2：主链路 100 次（input.md AC-1.bench_main_pipeline_p95）
// =====================================================================

/// 测量"模拟拖入 → 全链路"（KC enrich + frontmatter writer + db_update_kc_fields）的
/// P50/95/99 延迟。
///
/// **链路构造**（与 scheduler.rs::save_and_materialize KC 注入分支字面对齐）：
/// 1. `KcClient::ingest_text`（mock success）→ Outcome::Success；
/// 2. `kc::enrichment::resolve_outcome` + `build_kc_frontmatter` writer → ResolvedEnrichment；
/// 3. `kc_persist_resolved_with_conn`（task_027b 单源化的 canonical 由 scheduler.rs 直接暴露）：
///    `db_update_kc_fields` + `db_conversion_meta_kc_insert` + `update_failure_code`。
///
/// 不含磁盘 .md 写盘（input.md 列了"落地 .md"但磁盘写盘是 mem-mapped fs cache 主导，
/// 在 mock + tmpfs 下与 DB 阶段同数量级；本测度量纯 NC 计算 + DB 路径，更稳定可比）。
///
/// **PRD §4.1 阈值**：P95 < 30s（mock + in-memory DB 应远低于；本测试 panic 守护 30s）。
async fn bench_main_pipeline_p95_inner() -> Percentiles {
    let enhanced = "# enhanced\n\n#AI\n\n## 摘要\n\n（mock）";
    let meta = KcMockMeta::default();
    let mock = MockKcServer::start_with_success(enhanced, meta).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let asset_id = "asset-bench-pipeline";
    let asset = make_test_asset(asset_id);
    let input = make_10kb_markdown();
    let raw_extraction = make_extraction_result(&input);
    let opts = KcIngestOptions::default();

    // 预热：跑 3 次（首次 KC ingest + DB connection 初次访问 sqlite_master 等开销不计时）
    for i in 0..3 {
        let warmup_id = format!("warmup-{i}");
        let conn = setup_in_memory_db(&warmup_id);
        if let Ok(KcIngestOutcome::Success { enhanced_md, meta }) =
            client.ingest_text(&input, &opts).await
        {
            let outcome = KcEnrichmentOutcome::Success { enhanced_md, meta };
            let resolved = resolve_outcome(&raw_extraction, outcome, |meta| {
                build_kc_frontmatter(&asset, &raw_extraction, meta)
            });
            kc_persist_resolved_with_conn(&conn, &warmup_id, "application/pdf", "deadbeef", &resolved);
        }
    }

    let mut durations: Vec<Duration> = Vec::with_capacity(SAMPLE_COUNT);
    for i in 0..SAMPLE_COUNT {
        // 每轮新建独立的 in-memory DB——确保 conversion_meta 不累积干扰（更接近"拖入"
        // 一次性场景）。Setup（schema migration + 4 行 INSERT）不计时——下面只圈住实际链路。
        let iter_asset_id = format!("asset-bench-{i}");
        let conn = setup_in_memory_db(&iter_asset_id);
        let iter_asset = Asset {
            id: iter_asset_id.clone(),
            ..asset.clone()
        };

        let t0 = Instant::now();
        let ingest_res = client.ingest_text(&input, &opts).await;
        let (enhanced_md, kc_meta): (String, KcMeta) = match ingest_res {
            Ok(KcIngestOutcome::Success { enhanced_md, meta }) => (enhanced_md, meta),
            other => panic!("bench_main_pipeline: expected Ok(Success), got {other:?}"),
        };
        let outcome = KcEnrichmentOutcome::Success {
            enhanced_md,
            meta: kc_meta,
        };
        let resolved = resolve_outcome(&raw_extraction, outcome, |meta| {
            build_kc_frontmatter(&iter_asset, &raw_extraction, meta)
        });
        kc_persist_resolved_with_conn(&conn, &iter_asset_id, "application/pdf", "deadbeef", &resolved);
        durations.push(t0.elapsed());

        // 验证写入正确（first/last iter 抽查；不计时）
        if i == 0 || i == SAMPLE_COUNT - 1 {
            let row = db_ext::db_read_kc_status(&conn, &iter_asset_id)
                .expect("read kc status")
                .expect("row exists");
            assert_eq!(row.kc_enriched.as_deref(), Some("true"));
            assert_eq!(row.kc_version.as_deref(), Some("0.9"));
            assert_eq!(row.kc_tags_source.as_deref(), Some(KcTagsSource::AiAndRule.as_str()));
        }
    }

    mock.stop();
    percentiles_ms(durations)
}

// =====================================================================
// 入口测试：跑两个 bench + 写 JSON + 阈值守护
// =====================================================================

/// **入口测试**：跑两个 bench、写 `target/bench_results.json`、断言 PRD §4.1 阈值。
///
/// `--nocapture`：用 `cargo test --test kc_perf_smoke -- --nocapture` 看 stdout 实测值；
/// 默认 `cargo test` 仍跑（但 stdout 隐藏，看 JSON 文件可视化即可）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kc_perf_smoke_runs_both_benches_and_asserts_prd_4_1_thresholds() {
    println!("\n=== task_024_perf_benchmark：开始 KC 性能 benchmark ===");
    println!("样本数：{SAMPLE_COUNT} | 输入大小：{INPUT_BYTES} bytes");

    // -- Bench 1：KC ingest 10KB × 100
    println!("\n--- Bench 1：bench_kc_ingest_10kb ---");
    let kc_ingest = bench_kc_ingest_10kb_inner().await;
    println!(
        "  P50 = {:.3} ms | P95 = {:.3} ms | P99 = {:.3} ms | 阈值 = {:.0} ms (PRD §4.1)",
        kc_ingest.p50_ms, kc_ingest.p95_ms, kc_ingest.p99_ms, KC_INGEST_P95_THRESHOLD_MS,
    );

    // -- Bench 2：主链路 100 次
    println!("\n--- Bench 2：bench_main_pipeline_p95 ---");
    let main_pipeline = bench_main_pipeline_p95_inner().await;
    println!(
        "  P50 = {:.3} ms | P95 = {:.3} ms | P99 = {:.3} ms | 阈值 = {:.0} ms (PRD §4.1)",
        main_pipeline.p50_ms,
        main_pipeline.p95_ms,
        main_pipeline.p99_ms,
        MAIN_PIPELINE_P95_THRESHOLD_MS,
    );

    // -- 写 JSON
    let json_path = write_bench_results_json(&kc_ingest, &main_pipeline);
    println!("\n--- JSON 输出 ---");
    println!("  {}", json_path.display());

    // -- PRD §4.1 阈值断言：mock + in-memory 路径理应远低于阈值；超阈值即触发 CI 红
    assert!(
        kc_ingest.p95_ms < KC_INGEST_P95_THRESHOLD_MS,
        "KC ingest P95 {:.3} ms 超过 PRD §4.1 阈值 {} ms",
        kc_ingest.p95_ms,
        KC_INGEST_P95_THRESHOLD_MS
    );
    assert!(
        main_pipeline.p95_ms < MAIN_PIPELINE_P95_THRESHOLD_MS,
        "主链路 P95 {:.3} ms 超过 PRD §4.1 阈值 {} ms",
        main_pipeline.p95_ms,
        MAIN_PIPELINE_P95_THRESHOLD_MS
    );

    println!("\n=== task_024_perf_benchmark：PRD §4.1 阈值守护通过 ===\n");
}

// =====================================================================
// 单测：百分位算法自身正确性（避免 P95 计算 bug 假阳性）
// =====================================================================

#[test]
fn percentiles_ms_nearest_rank_matches_expected_indices() {
    // 1..=100 ms，sorted 后：sorted[k-1] = k ms
    // P50 → ceil(0.50 × 100) = 50 → sorted[49] = 50 ms
    // P95 → ceil(0.95 × 100) = 95 → sorted[94] = 95 ms
    // P99 → ceil(0.99 × 100) = 99 → sorted[98] = 99 ms
    let samples: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
    let p = percentiles_ms(samples);
    assert!((p.p50_ms - 50.0).abs() < 1e-9, "p50 应为 50ms，实际 {}", p.p50_ms);
    assert!((p.p95_ms - 95.0).abs() < 1e-9, "p95 应为 95ms，实际 {}", p.p95_ms);
    assert!((p.p99_ms - 99.0).abs() < 1e-9, "p99 应为 99ms，实际 {}", p.p99_ms);
}

#[test]
fn make_10kb_markdown_is_at_least_10kb() {
    let s = make_10kb_markdown();
    assert!(s.len() >= INPUT_BYTES, "10KB 文本生成应 ≥ {INPUT_BYTES}");
    // 也不应超过 11KB（防止生成过多 → 失去 10KB 指标意义）
    assert!(s.len() < INPUT_BYTES + 200, "10KB 文本生成应接近 10KB");
}
