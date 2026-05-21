use crate::db::knowledge::{
    delete_concept as db_delete_concept, delete_extensions_for_concept, delete_viewpoints_for_concept,
    get_concept_detail as db_get_concept_detail, get_concepts_with_stats,
    insert_case, insert_concept, insert_extension, insert_viewpoint,
    update_concept as db_update_concept,
    Concept, ConceptCase, ConceptDetail, ConceptExtension, ConceptViewpoint, ConceptWithStats,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use crate::llm::prompt_runtime::{
    assemble_messages_for_aggregation, assemble_messages_for_concept, inspect_messages_for_log,
    AggregationVars, ConceptVars,
};
use futures_util::stream::{self, StreamExt};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State};

// ─────────────────────────────────────────────────────────────────────────────
// task_perf_01_backend 性能优化常量
// ─────────────────────────────────────────────────────────────────────────────

/// 概念抽取并发度（PRD 决策；不做配置化）。
/// 调高需先评估 LLM 提供商 RPM / TPM 上限与 SQLite 写锁争用。
const CONCEPT_EXTRACTION_CONCURRENCY: usize = 4;

/// 单文档喂给 concept LLM 的最大字节数。
/// 诊断报告：平均 62KB / 最大 970KB content 大部分是冗余 OCR/转写文本；
/// 8 KiB 截断后单次 LLM 输入 token 从 ~15K 降到 ~2K，单文档延迟 58s → ~15s。
const CONCEPT_CONTENT_MAX_BYTES: usize = 8192;

/// 截断标记（中文，温和）—— 截断发生时追加到 user message 末尾，
/// 告知 LLM 后文已省略，避免它对"残缺末尾"做奇怪推断。
const CONCEPT_TRUNCATION_NOTE: &str =
    "\n\n[Note: content truncated to 8 KiB for performance; first chunk shown above.]";

// ─────────────────────────────────────────────────────────────────────────────
// 进度结构体
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionProgress {
    pub total_assets: usize,
    pub processed: usize,
    pub concepts_found: usize,
    pub status: String, // "running" | "completed" | "error"
}

// ─────────────────────────────────────────────────────────────────────────────
// 同步 CRUD commands
// ─────────────────────────────────────────────────────────────────────────────

/// 获取知识库概念列表（含统计）
#[tauri::command]
pub fn get_concepts(
    db: State<'_, Database>,
    library_id: String,
) -> Result<Vec<ConceptWithStats>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    get_concepts_with_stats(&conn, &library_id)
}

/// 获取单个概念完整详情（观点 + 案例 + 拓展）
#[tauri::command]
pub fn get_concept_detail(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Option<ConceptDetail>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db_get_concept_detail(&conn, &concept_id)
}

/// 更新概念名称或定义（标记 user_edited）
#[tauri::command]
pub fn update_concept(
    db: State<'_, Database>,
    concept_id: String,
    name: Option<String>,
    definition: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db_update_concept(&conn, &concept_id, name.as_deref(), definition.as_deref())
}

/// 删除概念（级联删除观点/案例/拓展）
#[tauri::command]
pub fn delete_concept(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db_delete_concept(&conn, &concept_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// 异步：概念提取（后台任务，通过 Tauri event 推进度）
// ─────────────────────────────────────────────────────────────────────────────

/// 扫描知识库所有素材，对每个素材调用 LLM 提取概念（task_perf_01 性能改造）。
///
/// 改造要点：
/// - **并发**：4 路 `buffer_unordered`，对应 `CONCEPT_EXTRACTION_CONCURRENCY`；
///   单文档约 ~58s 串行变 ~22s 平均并发，content 截断到 8 KiB 后再降到 ~7-10min 全量。
/// - **增量扫描**：`force_full=false` 时 SELECT WHERE `concept_extracted_at IS NULL`，
///   `force_full=true` 时先 UPDATE 重置全库标记再全量扫描（用户 escape hatch）。
///   `assets.concept_extracted_at` 由 V16 迁移落地。
/// - **错误隔离**：单文档 chat_completion / parse 失败仅 `log::error!`，不终止 batch；
///   失败文档不写 `concept_extracted_at`（下次增量自动重试）。
/// - **进度**：`processed` / `concepts_found` 用 `AtomicUsize` 并发安全；
///   每文档完成（成功/失败/跳过）均 emit `notecapt/concept-extraction-progress`。
///
/// IPC 名 `start_concept_extraction`（前端 task_perf_02 期望签名）。
/// 旧 IPC 名 `extract_concepts_for_library` 保留为 thin wrapper（见同文件）。
#[tauri::command]
pub async fn start_concept_extraction(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    library_id: String,
    force_full: bool,
) -> Result<ExtractionProgress, String> {
    // 1. 读取 LLM 配置
    let client = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        LLMClient::from_db_or_env(&conn)?
    };

    // 2. force_full 时先重置整个 library 的 concept_extracted_at 标记（escape hatch）
    if force_full {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        reset_library_concept_extracted_at(&conn, &library_id)?;
    }

    // 3. 查询需要处理的素材：force_full=false 仅查 concept_extracted_at IS NULL
    let assets = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        fetch_library_assets_for_extraction(&conn, &library_id, force_full)?
    };

    let total = assets.len();
    // 并发安全计数器：buffer_unordered 闭包内 fetch_add 后用 Relaxed 读最新值 emit。
    // emit_progress 走 app.emit（事件总线，本身线程安全），无需额外同步。
    let processed = Arc::new(AtomicUsize::new(0));
    let concepts_found = Arc::new(AtomicUsize::new(0));
    let skipped_incremental = Arc::new(AtomicUsize::new(0));

    emit_progress(&app, &library_id, total, 0, 0, "running");

    // 4. 预加载已存在的概念（含 user_edited 标记，用于 F-9）和 F-8 已处理 (asset, hash) 集合。
    //    并发闭包内读取这两个快照（不变快照），写入 concepts 表时用 INSERT OR IGNORE +
    //    重新 SELECT id 解决"两个并发闭包同时插入同名 concept"的竞争（concepts.UNIQUE(library_id, name)）。
    let existing_concepts = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        get_concepts_with_stats(&conn, &library_id)?
            .into_iter()
            .map(|c| (c.name.clone(), (c.id.clone(), c.user_edited)))
            .collect::<std::collections::HashMap<_, _>>()
    };
    let logged_pairs = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        crate::db::concepts_extraction_log::fetch_logged_pairs(&conn, &library_id)?
    };

    // 5. 并发主循环：buffer_unordered(4)
    // 闭包通过 `db: &State<Database>` 借用（State<'_, Database> 在 async fn 主帧中存活），
    // 闭包返回 Result<(asset_id, concepts_inserted), String>。Err 仅 log，不抛 ?。
    let stream_results = stream::iter(assets.into_iter())
        .map(|(asset_id, project_name, asset_name, content_snippet, content_hash)| {
            // 把闭包内需要的可 clone 数据各 clone 一份；&db 与 &client / &app 走 borrow 即可。
            let library_id = library_id.clone();
            let existing_concepts = &existing_concepts;
            let logged_pairs = &logged_pairs;
            let processed = Arc::clone(&processed);
            let concepts_found = Arc::clone(&concepts_found);
            let skipped_incremental = Arc::clone(&skipped_incremental);
            let client = &client;
            let db = &db;
            let app = &app;

            async move {
                // ─── 5.1 空内容跳过（不算失败，仅推进 processed）───
                if content_snippet.trim().is_empty() {
                    let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    let cf = concepts_found.load(Ordering::Relaxed);
                    emit_progress(app, &library_id, total, p, cf, "running");
                    return Ok::<_, String>(());
                }

                // ─── 5.2 F-8 增量去重（force_full=false 才生效）───
                if !force_full {
                    if let Some(hash) = content_hash.as_ref() {
                        if logged_pairs.contains(&(asset_id.clone(), hash.clone())) {
                            skipped_incremental.fetch_add(1, Ordering::Relaxed);
                            let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                            let cf = concepts_found.load(Ordering::Relaxed);
                            emit_progress(app, &library_id, total, p, cf, "running");
                            return Ok(());
                        }
                    }
                }

                // ─── 5.3 content 截断到 8 KiB（byte-safe UTF-8 边界）+ 拼 prompt ───
                let (truncated_content, did_truncate) =
                    truncate_content_for_concept(&content_snippet, CONCEPT_CONTENT_MAX_BYTES);

                let assembled = {
                    let conn = match db.conn.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!(
                                "concept extraction db lock failed for asset {asset_id}: {e}"
                            );
                            let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                            let cf = concepts_found.load(Ordering::Relaxed);
                            emit_progress(app, &library_id, total, p, cf, "running");
                            return Ok(());
                        }
                    };
                    let msgs = assemble_messages_for_concept(
                        &conn,
                        ConceptVars {
                            asset_name: asset_name.clone(),
                            project_name: project_name.clone(),
                            content: truncated_content,
                        },
                    );
                    match msgs {
                        Ok(mut m) => {
                            // 截断发生时在最后一条 user message 末尾追加 truncated note
                            if did_truncate {
                                if let Some(user_msg) =
                                    m.iter_mut().rev().find(|msg| msg.role == "user")
                                {
                                    user_msg.content.push_str(CONCEPT_TRUNCATION_NOTE);
                                }
                            }
                            let ctx = inspect_messages_for_log(&conn, "concept", &m);
                            Some((m, ctx))
                        }
                        Err(e) => {
                            log::warn!(
                                "概念抽取 messages 组装失败，跳过素材 {}: {}",
                                asset_id, e
                            );
                            None
                        }
                    }
                }; // 释放 DB 锁

                let Some((messages, log_ctx)) = assembled else {
                    let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    let cf = concepts_found.load(Ordering::Relaxed);
                    emit_progress(app, &library_id, total, p, cf, "running");
                    return Ok(());
                };

                log::info!(
                    "LLM call: module={} bytes={} user_overridden={}",
                    log_ctx.module,
                    log_ctx.total_bytes,
                    log_ctx.user_overridden
                );

                // ─── 5.4 LLM 调用（最耗时；锁已释放，4 路真正并发）───
                let response = match chat_completion(client, messages).await {
                    Ok(r) => r,
                    Err(e) => {
                        // 错误隔离：仅 log，不抛；processed 推进但 conceptsFound 不变；
                        // 不写 concept_extracted_at，下次增量自动重试。
                        log::error!(
                            "concept extraction failed for asset {asset_id}: {e}"
                        );
                        let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                        let cf = concepts_found.load(Ordering::Relaxed);
                        emit_progress(app, &library_id, total, p, cf, "running");
                        return Ok(());
                    }
                };

                let extracted = match parse_extracted_concepts(&response) {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!(
                            "concept extraction parse failed for asset {asset_id}: {e}"
                        );
                        let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                        let cf = concepts_found.load(Ordering::Relaxed);
                        emit_progress(app, &library_id, total, p, cf, "running");
                        return Ok(());
                    }
                };

                // ─── 5.5 写入 concepts / cases / 标记 concept_extracted_at（短作用域抢锁）───
                let mut local_concepts_count = 0usize;
                {
                    let conn = match db.conn.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!(
                                "concept extraction db write lock failed for asset {asset_id}: {e}"
                            );
                            let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                            let cf = concepts_found.load(Ordering::Relaxed);
                            emit_progress(app, &library_id, total, p, cf, "running");
                            return Ok(());
                        }
                    };
                    let now = chrono::Utc::now().to_rfc3339();

                    for ec in extracted {
                        // 并发竞争解决：先查快照，命中则走 append；未命中尝试 INSERT OR IGNORE，
                        // 然后用 (library_id, name) 重新 SELECT id（保证另一并发闭包先插入时仍能拿到 id）。
                        let concept_id: String = if let Some((existing_id, _ue)) =
                            existing_concepts.get(&ec.name)
                        {
                            // F-9: user_edited 概念仅追加 source_asset_id + cases，
                            // 绝不覆写 name/definition
                            if let Err(e) = append_source_asset(&conn, existing_id, &asset_id) {
                                log::warn!(
                                    "append_source_asset 失败 asset={asset_id} concept={existing_id}: {e}"
                                );
                            }
                            existing_id.clone()
                        } else {
                            let new_id = uuid::Uuid::new_v4().to_string();
                            let c = Concept {
                                id: new_id.clone(),
                                library_id: library_id.clone(),
                                name: ec.name.clone(),
                                aliases: ec.aliases.clone(),
                                definition: Some(ec.definition.clone()),
                                source_asset_ids: vec![asset_id.clone()],
                                source_project_ids: vec![],
                                user_edited: false,
                                created_at: now.clone(),
                                updated_at: now.clone(),
                            };
                            // insert_concept 用 INSERT OR IGNORE；如另一并发闭包已抢先插入同名 concept，
                            // 此处静默忽略，再重查实际 id 走 append 路径。
                            let _ = insert_concept(&conn, &c);
                            match conn.query_row(
                                "SELECT id FROM concepts WHERE library_id = ?1 AND name = ?2",
                                params![library_id, ec.name],
                                |r| r.get::<_, String>(0),
                            ) {
                                Ok(id) => {
                                    if id != new_id {
                                        // 另一闭包先插入了，补一次 source_asset 追加
                                        if let Err(e) =
                                            append_source_asset(&conn, &id, &asset_id)
                                        {
                                            log::warn!(
                                                "append_source_asset 兜底失败 asset={asset_id}: {e}"
                                            );
                                        }
                                    }
                                    id
                                }
                                Err(e) => {
                                    log::error!(
                                        "concept 查询失败 lib={library_id} name={}: {e}",
                                        ec.name
                                    );
                                    continue;
                                }
                            }
                        };

                        // 插入案例摘录（重复时忽略）
                        for excerpt in &ec.excerpts {
                            let case = ConceptCase {
                                id: uuid::Uuid::new_v4().to_string(),
                                concept_id: concept_id.clone(),
                                title: format!("{} — {}", project_name, asset_name),
                                excerpt: excerpt.clone(),
                                source_asset_id: Some(asset_id.clone()),
                                source_location: None,
                                relevance_note: None,
                            };
                            let _ = insert_case(&conn, &case);
                        }

                        local_concepts_count += 1;
                    }

                    // F-8 旧日志（保留，content_hash 维度）+ V16 新标记（asset_id 维度）双写
                    if let Some(hash) = content_hash.as_ref() {
                        let _ = crate::db::concepts_extraction_log::insert(
                            &conn,
                            &library_id,
                            &asset_id,
                            hash,
                        );
                    }
                    if let Err(e) = mark_asset_concept_extracted(&conn, &asset_id) {
                        log::warn!(
                            "mark concept_extracted_at 失败 asset={asset_id}: {e}"
                        );
                    }
                } // 释放 DB 锁

                // ─── 5.6 推进 processed / concepts_found 并 emit ───
                concepts_found.fetch_add(local_concepts_count, Ordering::Relaxed);
                let p = processed.fetch_add(1, Ordering::Relaxed) + 1;
                let cf = concepts_found.load(Ordering::Relaxed);
                emit_progress(app, &library_id, total, p, cf, "running");
                Ok(())
            }
        })
        .buffer_unordered(CONCEPT_EXTRACTION_CONCURRENCY);

    // 驱动 stream 跑完所有并发任务（错误已在闭包内 log 吞掉，这里不会拿到 Err）
    stream_results
        .for_each(|_| async {})
        .await;

    let skipped = skipped_incremental.load(Ordering::Relaxed);
    if skipped > 0 {
        log::info!(
            "F-8 增量抽取：库 {} 跳过 {} 个已处理素材（force_full=false）",
            library_id, skipped
        );
    }

    // 概念提取完成后，先同步触发共现关系计算（纯 SQLite，无 LLM，耗时可接受）
    // 必须在发送 concept-extraction-done 事件之前完成并释放连接锁，
    // 确保前端收到事件时 concept_relations 数据已就绪。
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        match crate::db::co_occurrence::compute_co_occurrence(&conn, &library_id) {
            Ok(n) => log::info!("共现关系计算完成，新增/更新 {n} 条关系"),
            Err(e) => log::warn!("共现关系计算失败（不影响提取结果）: {e}"),
        }
    }

    // 共现计算完成并释放连接锁后，再发送完成事件
    let final_concepts_found = concepts_found.load(Ordering::Relaxed);
    let final_processed = processed.load(Ordering::Relaxed);
    let _ = app.emit(
        "notecapt/concept-extraction-done",
        serde_json::json!({ "libraryId": library_id, "conceptCount": final_concepts_found }),
    );

    Ok(ExtractionProgress {
        total_assets: total,
        processed: final_processed,
        concepts_found: final_concepts_found,
        status: "completed".to_string(),
    })
}

/// 兼容旧 IPC 名 `extract_concepts_for_library`（参数 `force`）。
///
/// 前端 task_perf_02 完成后会切换到 `start_concept_extraction(library_id, force_full)`；
/// 在切换前/混合发布期间，本 wrapper 让旧调用方仍能工作。
/// 语义：旧 `force=true` 等价新 `force_full=true`（强制全量重扫）。
#[tauri::command]
pub async fn extract_concepts_for_library(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    library_id: String,
    force: bool,
) -> Result<ExtractionProgress, String> {
    start_concept_extraction(db, app, library_id, force).await
}

// ─────────────────────────────────────────────────────────────────────────────
// 异步：观点聚合
// ─────────────────────────────────────────────────────────────────────────────

/// 对指定概念，收集所有来源素材的相关段落，调用 LLM 生成多视角观点
#[tauri::command]
pub async fn synthesize_viewpoints(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Vec<ConceptViewpoint>, String> {
    let (client, concept, cases) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let detail = db_get_concept_detail(&conn, &concept_id)?
            .ok_or_else(|| format!("概念不存在: {concept_id}"))?;
        let cases = detail.cases.clone();
        (client, detail.concept.clone(), cases)
    };

    if cases.is_empty() {
        return Ok(vec![]);
    }

    // task_004 AC-3 改造：通过 prompt_runtime::assemble_messages_for_aggregation 组装
    // messages，启用用户自定义 aggregation Prompt 与输出格式守卫。cases 序列化逻辑
    // 由独立的 build_cases_block helper 处理（与 build_synthesis_prompt 行为一致）。
    let cases_block = build_cases_block(&cases);
    let (messages, log_ctx) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let msgs = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: concept.name.clone(),
                definition: concept.definition.clone(),
                cases_block,
            },
        )?;
        let ctx = inspect_messages_for_log(&conn, "aggregation", &msgs);
        (msgs, ctx)
    };

    log::info!(
        "LLM call: module={} bytes={} user_overridden={}",
        log_ctx.module,
        log_ctx.total_bytes,
        log_ctx.user_overridden
    );

    let response = chat_completion(&client, messages).await?;
    let viewpoints = parse_synthesized_viewpoints(&response, &concept_id)?;

    // 先清空旧观点，再写入新的
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        delete_viewpoints_for_concept(&conn, &concept_id)?;
        for vp in &viewpoints {
            insert_viewpoint(&conn, vp)?;
        }
    }

    Ok(viewpoints)
}

// ─────────────────────────────────────────────────────────────────────────────
// 异步：知识拓展生成
// ─────────────────────────────────────────────────────────────────────────────

/// 对指定概念，调用 LLM 生成上下游知识拓展
#[tauri::command]
pub async fn generate_extensions(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Vec<ConceptExtension>, String> {
    let (client, concept) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let detail = db_get_concept_detail(&conn, &concept_id)?
            .ok_or_else(|| format!("概念不存在: {concept_id}"))?;
        (client, detail.concept.clone())
    };

    let prompt = format!(
        "# Knowledge Extension Request\n\n\
        Concept: {}\nDefinition: {}\n\n\
        Generate upstream prerequisites (3 concepts) and downstream applications (3 concepts) \
        for this academic concept.\n\n\
        Return JSON array:\n\
        [{{\"direction\":\"upstream\"|\"downstream\",\"name\":\"...\",\"description\":\"...\",\"relationship\":\"...\"}}]\n\n\
        Only return the JSON array, no other text.",
        concept.name,
        concept.definition.as_deref().unwrap_or("N/A")
    );

    let messages = vec![
        ChatMessage { role: "system".to_string(), content: "You are a knowledge graph engine. Return only valid JSON.".to_string() },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let response = chat_completion(&client, messages).await?;
    let extensions = parse_extensions(&response, &concept_id)?;

    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        delete_extensions_for_concept(&conn, &concept_id)?;
        for ext in &extensions {
            insert_extension(&conn, ext)?;
        }
    }

    Ok(extensions)
}

// ─────────────────────────────────────────────────────────────────────────────
// 内部工具
// ─────────────────────────────────────────────────────────────────────────────

fn emit_progress(
    app: &tauri::AppHandle,
    library_id: &str,
    total: usize,
    processed: usize,
    found: usize,
    status: &str,
) {
    let _ = app.emit(
        "notecapt/concept-extraction-progress",
        serde_json::json!({
            "libraryId": library_id,
            "totalAssets": total,
            "processed": processed,
            "conceptsFound": found,
            "status": status,
        }),
    );
}

/// 从 library → projects → assets 取得需要处理的素材列表（**旧函数**：全量，无增量过滤）。
///
/// 保留以防 fix 期间需要回退诊断；当前主路径走 `fetch_library_assets_for_extraction`。
/// 返回 (asset_id, project_name, asset_name, content_snippet, content_hash_opt)
#[allow(dead_code)]
fn fetch_library_assets(
    conn: &rusqlite::Connection,
    library_id: &str,
) -> Result<Vec<(String, String, String, String, Option<String>)>, String> {
    fetch_library_assets_for_extraction(conn, library_id, true)
}

/// task_perf_01 AC-5：支持增量过滤的素材查询。
///
/// - `force_full=true`：返回 library 内所有 assets（同 fetch_library_assets 旧行为）；
///   调用方需在此之前调用 `reset_library_concept_extracted_at` 重置标记。
/// - `force_full=false`：仅返回 `a.concept_extracted_at IS NULL` 的 assets（未处理或上次失败）。
///
/// 返回 (asset_id, project_name, asset_name, content_snippet, content_hash_opt)。
/// content 字段沿用既有 COALESCE 链路（md_ec → ec → ai.summary → a.name），
/// 保证零行为差异于既有 task_004 LLM 调用契约。
fn fetch_library_assets_for_extraction(
    conn: &rusqlite::Connection,
    library_id: &str,
    force_full: bool,
) -> Result<Vec<(String, String, String, String, Option<String>)>, String> {
    let incremental_filter = if force_full {
        ""
    } else {
        " AND a.concept_extracted_at IS NULL"
    };
    let sql = format!(
        "SELECT a.id, p.name, a.name,
                COALESCE(md_ec.structured_md, md_ec.raw_text, ec.structured_md, ec.raw_text, ai.summary, a.name) as content,
                COALESCE(md_ec.content_hash, ec.content_hash) as content_hash
         FROM assets a
         INNER JOIN projects p ON p.id = a.project_id AND p.library_id = ?1
         LEFT JOIN assets md ON md.id = (
             SELECT id FROM assets
             WHERE source_asset_id = a.id AND asset_type = 'markdown'
             ORDER BY imported_at DESC
             LIMIT 1
         )
         LEFT JOIN extracted_content md_ec ON md_ec.asset_id = md.id AND md_ec.status = 'extracted'
         LEFT JOIN extracted_content ec ON ec.asset_id = a.id AND ec.status = 'extracted'
         LEFT JOIN ai_analyses ai ON ai.asset_id = a.id
         WHERE (a.source_asset_id IS NULL OR a.asset_type != 'markdown'){incremental_filter}
         ORDER BY a.imported_at DESC",
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("查询素材失败: {e}"))?;

    let rows: Result<Vec<_>, _> = stmt
        .query_map(params![library_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| format!("遍历素材失败: {e}"))?
        .collect();

    rows.map_err(|e| format!("读取素材行失败: {e}"))
}

/// `force_full=true` 时清空整个 library 内 assets 的 concept_extracted_at 标记，
/// 让后续 SELECT 命中全部素材（escape hatch 真相来源）。
fn reset_library_concept_extracted_at(
    conn: &rusqlite::Connection,
    library_id: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets
            SET concept_extracted_at = NULL
          WHERE project_id IN (SELECT id FROM projects WHERE library_id = ?1)",
        params![library_id],
    )
    .map_err(|e| format!("重置 concept_extracted_at 失败: {e}"))?;
    Ok(())
}

/// 单文档抽取成功后标记 `assets.concept_extracted_at = datetime('now')`。
/// 失败文档不调用此函数，下次增量自动重试。
fn mark_asset_concept_extracted(
    conn: &rusqlite::Connection,
    asset_id: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET concept_extracted_at = datetime('now') WHERE id = ?1",
        params![asset_id],
    )
    .map_err(|e| format!("写入 concept_extracted_at 失败: {e}"))?;
    Ok(())
}

/// task_perf_01 AC-3：byte-safe UTF-8 边界截断到指定字节数。
///
/// 返回 `(截断后字符串, 是否真截断)`。
/// - 输入 `content.len() <= max_bytes`：原样返回 + truncated=false。
/// - 输入 `content.len() > max_bytes`：从头取最长不超过 max_bytes 字节且不切碎多字节
///   UTF-8 字符的前缀，返回 + truncated=true。
///
/// 不会在 ASCII / Unicode 字符中间切断。中文一字符 3 字节，emoji 通常 4 字节，
/// `char_indices()` 给出每个字符的起始字节位置，take_while 取到最后一个完全装下的字符尾。
fn truncate_content_for_concept(content: &str, max_bytes: usize) -> (String, bool) {
    if content.len() <= max_bytes {
        return (content.to_string(), false);
    }
    // 找出最后一个字符末尾 ≤ max_bytes 的切点（含该字符整体）
    let cut = content
        .char_indices()
        .take_while(|(byte_start, ch)| byte_start + ch.len_utf8() <= max_bytes)
        .map(|(byte_start, ch)| byte_start + ch.len_utf8())
        .last()
        .unwrap_or(0);
    (content[..cut].to_string(), true)
}



fn append_source_asset(
    conn: &rusqlite::Connection,
    concept_id: &str,
    asset_id: &str,
) -> Result<(), String> {
    let current: Option<String> = conn
        .query_row(
            "SELECT source_asset_ids FROM concepts WHERE id = ?1",
            params![concept_id],
            |r| r.get(0),
        )
        .ok();

    let mut ids: Vec<String> = current
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if !ids.contains(&asset_id.to_string()) {
        ids.push(asset_id.to_string());
        let json = serde_json::to_string(&ids).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE concepts SET source_asset_ids = ?2, updated_at = ?3 WHERE id = ?1",
            params![concept_id, json, now],
        )
        .map_err(|e| format!("追加素材 ID 失败: {e}"))?;
    }
    Ok(())
}

// ─── Prompt 构建 ─────────────────────────────────────────────────────────────

/// 把 cases 渲染为多段 "### Context i: title\nexcerpt\n\n" 文本块。
///
/// 摘自原 `build_synthesis_prompt` 内循环段，task_004 AC-3 拆出供
/// `assemble_messages_for_aggregation` 通过 `AggregationVars.cases_block` 注入。
/// 与原行为字符级一致（含尾随 `\n\n`），用户自定义 `aggregation` Prompt 中
/// `{cases}` 占位符会被替换为本函数输出。
fn build_cases_block(cases: &[ConceptCase]) -> String {
    let mut s = String::new();
    for (i, case) in cases.iter().enumerate() {
        s.push_str(&format!(
            "### Context {}: {}\n{}\n\n",
            i + 1,
            case.title,
            case.excerpt
        ));
    }
    s
}

/// task_004 AC-2 改造后已弃用：保留只为防止潜在的外部 import 失败，
/// 实际调用方已切到 `prompt_runtime::assemble_messages_for_concept`。
/// 若未来彻底移除，请同步删除上游 `prompt_runtime::CONCEPT_DEFAULT` 的
/// "逐字摘抄自本函数"注释。
#[allow(dead_code)]
#[deprecated(
    note = "task_004 已切换到 prompt_runtime::assemble_messages_for_concept；本函数仅保留用于回退/参考"
)]
fn build_extraction_prompt(asset_name: &str, project_name: &str, content: &str) -> String {
    format!(
        "# Document Analysis Request\n\n\
        ## Document\n\
        Title: {asset_name}\n\
        Project/Course: {project_name}\n\
        Content:\n---\n{content}\n---\n\n\
        ## Task\n\
        Extract all significant academic concepts from this document. For each concept:\n\
        1. name: The canonical English term\n\
        2. aliases: Alternative names (including translations if bilingual)\n\
        3. definition: A one-sentence definition as used in this context\n\
        4. excerpts: 1-2 direct quotes from the document that discuss this concept\n\n\
        Return as JSON array:\n\
        [{{\"name\":\"...\",\"aliases\":[\"...\"],\"definition\":\"...\",\"excerpts\":[\"...\"]}}]\n\n\
        Rules:\n\
        - Only extract substantive concepts (not generic terms like \"example\" or \"chapter\")\n\
        - Prefer established academic terminology\n\
        - Include 3-10 concepts per document\n\
        - Return only the JSON array, no other text."
    )
}

/// task_004 AC-3 改造后已弃用：保留只为防止潜在的外部 import 失败，
/// 实际调用方已切到 `prompt_runtime::assemble_messages_for_aggregation` +
/// `build_cases_block`。
#[allow(dead_code)]
#[deprecated(
    note = "task_004 已切换到 prompt_runtime::assemble_messages_for_aggregation；本函数仅保留用于回退/参考"
)]
fn build_synthesis_prompt(name: &str, definition: Option<&str>, cases: &[ConceptCase]) -> String {
    let mut s = format!(
        "# Viewpoint Synthesis Request\n\n\
        ## Concept: {name}\nDefinition: {}\n\n\
        ## Appearances across student's documents:\n\n",
        definition.unwrap_or("N/A")
    );
    s.push_str(&build_cases_block(cases));
    s.push_str(
        "## Task\n\
        For each context, synthesize a viewpoint:\n\
        1. perspective: e.g. \"Economic perspective\" or \"Psychological lens\"\n\
        2. summary: 2-3 sentences explaining how this concept is understood in this context\n\
        3. sourceContext: Which course/document this perspective comes from\n\n\
        Return as JSON array:\n\
        [{{\"perspective\":\"...\",\"summary\":\"...\",\"sourceContext\":\"...\"}}]\n\n\
        Return only the JSON array, no other text.",
    );
    s
}

// ─── JSON 解析 ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ExtractedConcept {
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    definition: String,
    #[serde(default)]
    excerpts: Vec<String>,
}

fn parse_extracted_concepts(json: &str) -> Result<Vec<ExtractedConcept>, String> {
    // 提取 JSON 数组（LLM 有时会包裹额外文本）
    let start = json.find('[').unwrap_or(0);
    let end = json.rfind(']').map(|i| i + 1).unwrap_or(json.len());
    serde_json::from_str::<Vec<ExtractedConcept>>(&json[start..end])
        .map_err(|e| format!("解析概念 JSON 失败: {e}"))
}

#[derive(Deserialize)]
struct SynthesizedViewpoint {
    perspective: String,
    summary: String,
    #[serde(rename = "sourceContext", default)]
    source_context: String,
}

fn parse_synthesized_viewpoints(
    json: &str,
    concept_id: &str,
) -> Result<Vec<ConceptViewpoint>, String> {
    let start = json.find('[').unwrap_or(0);
    let end = json.rfind(']').map(|i| i + 1).unwrap_or(json.len());
    let raw: Vec<SynthesizedViewpoint> =
        serde_json::from_str(&json[start..end]).map_err(|e| format!("解析观点 JSON 失败: {e}"))?;

    let now = chrono::Utc::now().to_rfc3339();
    Ok(raw
        .into_iter()
        .map(|v| ConceptViewpoint {
            id: uuid::Uuid::new_v4().to_string(),
            concept_id: concept_id.to_string(),
            perspective: v.perspective,
            summary: v.summary,
            source_context: Some(v.source_context).filter(|s| !s.is_empty()),
            source_asset_id: None,
            generated_at: now.clone(),
        })
        .collect())
}

#[derive(Deserialize)]
struct ExtensionItem {
    direction: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    relationship: String,
}

fn parse_extensions(json: &str, concept_id: &str) -> Result<Vec<ConceptExtension>, String> {
    let start = json.find('[').unwrap_or(0);
    let end = json.rfind(']').map(|i| i + 1).unwrap_or(json.len());
    let raw: Vec<ExtensionItem> =
        serde_json::from_str(&json[start..end]).map_err(|e| format!("解析拓展 JSON 失败: {e}"))?;

    Ok(raw
        .into_iter()
        .map(|e| ConceptExtension {
            id: uuid::Uuid::new_v4().to_string(),
            concept_id: concept_id.to_string(),
            direction: e.direction,
            name: e.name,
            description: Some(e.description).filter(|s| !s.is_empty()),
            relationship: Some(e.relationship).filter(|s| !s.is_empty()),
        })
        .collect())
}

// ─────────────────────────────────────────────────────────────────────────────
// 共现关系计算 Command
// ─────────────────────────────────────────────────────────────────────────────

/// 计算知识库内所有概念的共现关系（无 LLM 调用，纯 SQLite 计算）
///
/// 两两配对检查 source_asset_ids 交集，有交集则写入 concept_relations 表
/// （relation_type = "co_occurrence"，概念对方向：concept_a_id < concept_b_id 字典序）。
/// 返回新增/更新的关系记录数。
#[tauri::command]
pub fn knowledge_compute_co_occurrence(
    db: State<'_, Database>,
    library_id: String,
) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    crate::db::co_occurrence::compute_co_occurrence(&conn, &library_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use crate::db::user_prompt as db_user_prompt;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    /// 模拟 chat.rs::merge_system_messages 的逻辑——把所有 system 按顺序用 `\n\n`
    /// 合并成单条字符串。AC-8 字面回归断言通过本函数模拟"传给 Anthropic 的 system 字段"
    /// 的最终形态，验证 task_003 v2 "LLM 行为零差异"承诺。
    fn merged_system_field(messages: &[ChatMessage]) -> String {
        messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    // ---- AC-8：concept / aggregation 系统字段字面回归（task_003 选项 A 复刻）----

    /// AC-8：concept 调用产生的 messages 经 chat.rs 合并后，
    /// system 字段必须**逐字包含** "knowledge extraction engine"
    /// （来自 CONCEPT_SYSTEM_ADDON，逐字摘抄自原 knowledge.rs:147）。
    #[test]
    fn ac8_concept_system_field_literally_contains_knowledge_extraction_engine() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "操作系统.pdf".into(),
                project_name: "CS-101".into(),
                content: "进程是程序的一次执行...".into(),
            },
        )
        .unwrap();
        let system_field = merged_system_field(&messages);
        assert!(
            system_field.contains("knowledge extraction engine"),
            "AC-8: concept system 字段必须含 'knowledge extraction engine' 字面；实际: {system_field}"
        );
    }

    /// AC-8：aggregation 调用产生的 messages 经合并后，
    /// system 字段必须**逐字包含** "knowledge synthesis engine"
    /// （来自 AGGREGATION_SYSTEM_ADDON，逐字摘抄自原 knowledge.rs:276）。
    #[test]
    fn ac8_aggregation_system_field_literally_contains_knowledge_synthesis_engine() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: "认知偏差".into(),
                definition: Some("一种系统性偏差".into()),
                cases_block: String::new(),
            },
        )
        .unwrap();
        let system_field = merged_system_field(&messages);
        assert!(
            system_field.contains("knowledge synthesis engine"),
            "AC-8: aggregation system 字段必须含 'knowledge synthesis engine' 字面；实际: {system_field}"
        );
    }

    /// AC-8 补：concept 的自定义模板生效后，system 字段仍含字面 addon。
    #[test]
    fn ac8_concept_custom_template_still_injects_system_addon() {
        let conn = fresh_conn();
        db_user_prompt::upsert(
            &conn,
            "concept",
            "我的自定义抽取指令：{content}",
        )
        .unwrap();
        let messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "a".into(),
                project_name: "p".into(),
                content: "TEST_X".into(),
            },
        )
        .unwrap();
        let system_field = merged_system_field(&messages);
        // 自定义模板不影响 system_addon
        assert!(system_field.contains("knowledge extraction engine"));
        // 自定义内容确实进入了 user body
        assert!(messages[2].content.contains("我的自定义抽取指令"));
        assert!(messages[2].content.contains("TEST_X"));
    }

    // ---- AC-5：LlmCallContext.user_overridden 状态机 ----

    /// AC-5：未自定义任何 prompt 时，三种 module 的 inspect 都应返回
    /// user_overridden=false。
    #[test]
    fn ac5_inspect_returns_user_overridden_false_when_no_custom_prompt() {
        let conn = fresh_conn();
        let m_concept = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "a".into(),
                project_name: "p".into(),
                content: "c".into(),
            },
        )
        .unwrap();
        let ctx_concept = inspect_messages_for_log(&conn, "concept", &m_concept);
        assert_eq!(ctx_concept.module, "concept");
        assert!(!ctx_concept.user_overridden);
        assert!(ctx_concept.total_bytes > 0);

        let m_agg = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: "X".into(),
                definition: None,
                cases_block: "".into(),
            },
        )
        .unwrap();
        let ctx_agg = inspect_messages_for_log(&conn, "aggregation", &m_agg);
        assert!(!ctx_agg.user_overridden);
    }

    /// AC-5：保存 concept 自定义后，inspect 应返回 user_overridden=true。
    #[test]
    fn ac5_inspect_returns_user_overridden_true_when_concept_custom() {
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "concept", "自定义 concept {content}").unwrap();
        let m = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "a".into(),
                project_name: "p".into(),
                content: "c".into(),
            },
        )
        .unwrap();
        let ctx = inspect_messages_for_log(&conn, "concept", &m);
        assert!(
            ctx.user_overridden,
            "保存自定义 concept 后 user_overridden 应为 true"
        );
    }

    /// AC-5：保存 aggregation 自定义后，inspect 应返回 user_overridden=true。
    #[test]
    fn ac5_inspect_returns_user_overridden_true_when_aggregation_custom() {
        let conn = fresh_conn();
        db_user_prompt::upsert(
            &conn,
            "aggregation",
            "自定义 aggregation {concept_name}",
        )
        .unwrap();
        let m = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: "X".into(),
                definition: None,
                cases_block: "".into(),
            },
        )
        .unwrap();
        let ctx = inspect_messages_for_log(&conn, "aggregation", &m);
        assert!(ctx.user_overridden);
    }

    // ---- build_cases_block helper ----

    /// build_cases_block 应按 i+1 编号渲染多个 case，与原 build_synthesis_prompt
    /// 循环段字面一致。
    #[test]
    fn build_cases_block_renders_indexed_contexts() {
        let cases = vec![
            ConceptCase {
                id: "1".into(),
                concept_id: "c".into(),
                title: "AAA".into(),
                excerpt: "alpha".into(),
                source_asset_id: None,
                source_location: None,
                relevance_note: None,
            },
            ConceptCase {
                id: "2".into(),
                concept_id: "c".into(),
                title: "BBB".into(),
                excerpt: "beta".into(),
                source_asset_id: None,
                source_location: None,
                relevance_note: None,
            },
        ];
        let block = build_cases_block(&cases);
        assert!(block.contains("### Context 1: AAA\nalpha"));
        assert!(block.contains("### Context 2: BBB\nbeta"));
        // 尾随 "\n\n"（与原 build_synthesis_prompt 字面一致）
        assert!(block.ends_with("\n\n"));
    }

    /// 空 cases 应渲染为空串。
    #[test]
    fn build_cases_block_empty_cases_yields_empty_string() {
        let block = build_cases_block(&[]);
        assert!(block.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // task_perf_01_backend AC-3：truncate_content_for_concept byte-safe 截断
    // ─────────────────────────────────────────────────────────────────────────

    /// 输入 < CONCEPT_CONTENT_MAX_BYTES：原样返回，truncated=false。
    #[test]
    fn truncate_short_content_returns_original_and_false() {
        let content = "hello world";
        let (out, did) = truncate_content_for_concept(content, CONCEPT_CONTENT_MAX_BYTES);
        assert_eq!(out, "hello world");
        assert!(!did, "短内容不应被标记截断");
    }

    /// 输入 > 8 KiB：返回字节长度 ≤ 8192 + truncated=true。
    #[test]
    fn truncate_long_content_bounded_by_max_bytes_and_true() {
        let content: String = "a".repeat(20_000); // ASCII 20K
        let (out, did) = truncate_content_for_concept(&content, CONCEPT_CONTENT_MAX_BYTES);
        assert!(did, "超长内容必须标记截断");
        assert!(
            out.len() <= CONCEPT_CONTENT_MAX_BYTES,
            "截断后字节长度必须 ≤ {} 实际 {}",
            CONCEPT_CONTENT_MAX_BYTES,
            out.len()
        );
        assert_eq!(out.len(), CONCEPT_CONTENT_MAX_BYTES, "ASCII 应该刚好打满 8192");
    }

    /// 中文 / emoji 多字节 UTF-8 边界不切坏：构造一段刚好让"完整字符末尾"跨越 max_bytes 的输入，
    /// 验证返回字符串仍是合法 UTF-8（rust String 类型本身保证；额外断言长度 < max_bytes 而非 = max_bytes
    /// 因为不能切碎多字节字符，会留 1-2 字节空白）。
    #[test]
    fn truncate_respects_utf8_char_boundary_for_cjk() {
        // 中文一字符 3 字节；2735 个汉字 = 8205 字节，恰好越过 8192 边界
        let content: String = "中".repeat(2735);
        assert!(content.len() > CONCEPT_CONTENT_MAX_BYTES);

        let (out, did) = truncate_content_for_concept(&content, CONCEPT_CONTENT_MAX_BYTES);
        assert!(did, "超长 CJK 内容必须标记截断");
        // 1. 字节长度必须 ≤ 8192
        assert!(
            out.len() <= CONCEPT_CONTENT_MAX_BYTES,
            "字节长度 {} 必须 ≤ {}",
            out.len(),
            CONCEPT_CONTENT_MAX_BYTES
        );
        // 2. 字节长度必须是 3 的倍数（每个中文 3 字节，未切碎）
        assert_eq!(out.len() % 3, 0, "CJK 截断后字节长度应为 3 倍数（无半字符）");
        // 3. 8192 % 3 = 2，所以理论上 8190 字节最大（2730 个完整字符）
        assert_eq!(out.len(), 8190, "8192 / 3 = 2730 个完整汉字 × 3 = 8190 字节");
        // 4. 内容应全部为"中"字符
        assert!(out.chars().all(|c| c == '中'));
    }

    /// emoji 4 字节边界：构造刚好让 emoji 末尾跨越 max_bytes 的输入。
    #[test]
    fn truncate_respects_utf8_char_boundary_for_emoji() {
        // 🎉 是 4 字节 emoji；2049 个 = 8196 字节，越过 8192
        let content: String = "🎉".repeat(2049);
        assert!(content.len() > CONCEPT_CONTENT_MAX_BYTES);

        let (out, did) = truncate_content_for_concept(&content, CONCEPT_CONTENT_MAX_BYTES);
        assert!(did);
        assert!(out.len() <= CONCEPT_CONTENT_MAX_BYTES);
        // 每个 emoji 4 字节，8192 / 4 = 2048 个完整 emoji × 4 = 8192 字节（刚好打满）
        assert_eq!(out.len() % 4, 0);
        assert_eq!(out.len(), 8192);
    }

    /// 空字符串：边界 case，不应 panic。
    #[test]
    fn truncate_empty_content_returns_empty_and_false() {
        let (out, did) = truncate_content_for_concept("", CONCEPT_CONTENT_MAX_BYTES);
        assert_eq!(out, "");
        assert!(!did);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // task_perf_01_backend AC-5：增量扫描 fetch_library_assets_for_extraction
    // ─────────────────────────────────────────────────────────────────────────

    /// 测试 helper：建一个最小的 library + project + 三个 assets。
    /// 返回 (lib_id, [(asset_id, name)]) 供测试使用。
    fn seed_library_with_assets(conn: &Connection, n_assets: usize) -> (String, Vec<String>) {
        let lib_id = "lib_test".to_string();
        let proj_id = "proj_test".to_string();
        conn.execute_batch(&format!(
            "INSERT INTO libraries (id, name, root_path) VALUES ('{}', 'lib', '/tmp/lib');
             INSERT INTO projects (id, library_id, name) VALUES ('{}', '{}', 'p');",
            lib_id, proj_id, lib_id
        ))
        .unwrap();

        let mut asset_ids = Vec::with_capacity(n_assets);
        for i in 0..n_assets {
            let aid = format!("asset_{}", i);
            conn.execute(
                "INSERT INTO assets (id, project_id, asset_type, name, file_path)
                 VALUES (?1, ?2, 'document', ?3, ?4)",
                rusqlite::params![
                    aid,
                    proj_id,
                    format!("doc_{}.pdf", i),
                    format!("/tmp/doc_{}.pdf", i)
                ],
            )
            .unwrap();
            asset_ids.push(aid);
        }
        (lib_id, asset_ids)
    }

    /// AC-5-①：增量 mode（force_full=false）只返回 concept_extracted_at IS NULL 的素材。
    #[test]
    fn fetch_assets_incremental_skips_already_processed() {
        let conn = fresh_conn();
        let (lib, assets) = seed_library_with_assets(&conn, 3);

        // 标记 asset_0 / asset_2 为已处理
        for aid in &[&assets[0], &assets[2]] {
            conn.execute(
                "UPDATE assets SET concept_extracted_at = ?1 WHERE id = ?2",
                rusqlite::params!["2026-05-16T01:00:00Z", aid],
            )
            .unwrap();
        }

        let incremental = fetch_library_assets_for_extraction(&conn, &lib, false).unwrap();
        assert_eq!(incremental.len(), 1, "增量应只返回未处理的 1 个素材");
        assert_eq!(incremental[0].0, assets[1], "应返回 asset_1");

        let full = fetch_library_assets_for_extraction(&conn, &lib, true).unwrap();
        assert_eq!(full.len(), 3, "force_full=true 应返回全部 3 个素材");
    }

    /// AC-5-②：reset_library_concept_extracted_at 清空标记后，
    /// 增量查询返回全量（force_full=true 的实际效果）。
    #[test]
    fn reset_library_concept_extracted_at_makes_all_pending() {
        let conn = fresh_conn();
        let (lib, assets) = seed_library_with_assets(&conn, 3);

        // 全部标记已处理
        for aid in &assets {
            conn.execute(
                "UPDATE assets SET concept_extracted_at = ?1 WHERE id = ?2",
                rusqlite::params!["2026-05-16T01:00:00Z", aid],
            )
            .unwrap();
        }
        let before = fetch_library_assets_for_extraction(&conn, &lib, false).unwrap();
        assert_eq!(before.len(), 0);

        reset_library_concept_extracted_at(&conn, &lib).unwrap();

        let after = fetch_library_assets_for_extraction(&conn, &lib, false).unwrap();
        assert_eq!(after.len(), 3, "reset 后增量查询应返回全部");
    }

    /// AC-5-③：mark_asset_concept_extracted 写入 NOT NULL，下次增量自动跳过。
    /// 与 AC-4 错误隔离配合：失败素材的 concept_extracted_at 保持 NULL，下次增量会重试。
    #[test]
    fn mark_asset_concept_extracted_sets_timestamp_and_skips_next_incremental() {
        let conn = fresh_conn();
        let (lib, assets) = seed_library_with_assets(&conn, 2);

        // 模拟"asset_0 成功 → mark；asset_1 失败 → 不 mark"
        mark_asset_concept_extracted(&conn, &assets[0]).unwrap();

        let ts: Option<String> = conn
            .query_row(
                "SELECT concept_extracted_at FROM assets WHERE id = ?1",
                rusqlite::params![assets[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ts.is_some(), "asset_0 应有 concept_extracted_at 时间戳");

        let ts_null: Option<String> = conn
            .query_row(
                "SELECT concept_extracted_at FROM assets WHERE id = ?1",
                rusqlite::params![assets[1]],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ts_null.is_none(), "失败的 asset_1 应保持 NULL（下次增量重试）");

        let pending = fetch_library_assets_for_extraction(&conn, &lib, false).unwrap();
        assert_eq!(pending.len(), 1, "增量查询应只剩 asset_1（失败者）");
        assert_eq!(pending[0].0, assets[1]);
    }

    /// AC-6 并发安全：AtomicUsize 计数器在并发 fetch_add 后总和等于操作次数。
    /// （buffer_unordered 本身的并发正确性由 futures-util 保证；
    /// 这里验证我们对 Atomic 的使用模式 — fetch_add(1) + load 取最新值用于 emit — 正确。）
    ///
    /// 用 std::thread 模拟并发；与生产代码用的是同一个 AtomicUsize Ordering::Relaxed 语义。
    #[test]
    fn atomic_counter_concurrent_increments_yield_correct_total() {
        let processed = Arc::new(AtomicUsize::new(0));
        let total_tasks = 100;

        let handles: Vec<_> = (0..total_tasks)
            .map(|_| {
                let p = Arc::clone(&processed);
                std::thread::spawn(move || {
                    // 模拟"完成一个文档" — fetch_add 返回旧值
                    p.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            processed.load(Ordering::Relaxed),
            total_tasks,
            "100 个并发 fetch_add 后总和应等于 100"
        );
    }

    /// AC-2 + AC-4 + AC-6 端到端：用 tauri::async_runtime::block_on 跑一个
    /// `stream::iter().map().buffer_unordered(4)` mini pipeline，模拟"4 个任务，
    /// 其中第 2 个失败"。验证：① 4 个任务都被驱动 ② 失败的不增加 concepts_found
    /// ③ processed 计数 == 4。
    #[test]
    fn buffer_unordered_with_simulated_failures_isolates_errors() {
        let processed = Arc::new(AtomicUsize::new(0));
        let concepts_found = Arc::new(AtomicUsize::new(0));
        let total = 4usize;

        tauri::async_runtime::block_on(async {
            let processed = Arc::clone(&processed);
            let concepts_found = Arc::clone(&concepts_found);
            stream::iter(0..total)
                .map(|i| {
                    let processed = Arc::clone(&processed);
                    let concepts_found = Arc::clone(&concepts_found);
                    async move {
                        if i == 1 {
                            // 失败路径：仅 log（错误隔离），processed 推进，concepts_found 不变
                            log::error!("simulated failure for asset {}", i);
                        } else {
                            // 成功路径：每个产出 3 个 concept
                            concepts_found.fetch_add(3, Ordering::Relaxed);
                        }
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .buffer_unordered(CONCEPT_EXTRACTION_CONCURRENCY)
                .for_each(|_| async {})
                .await;
        });
        assert_eq!(
            processed.load(Ordering::Relaxed),
            total,
            "4 个任务均应推进 processed（含失败者）"
        );
        assert_eq!(
            concepts_found.load(Ordering::Relaxed),
            3 * 3,
            "3 个成功任务 × 3 concept = 9（失败者贡献 0）"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // task_perf_01_backend：buffer_unordered + truncate + 错误隔离 端到端验证
    // （集成测试用 mocked LLM 通过的话需注入 client；本地仅验证可编译路径，
    //  完整 LLM mock 走 user_prompt_e2e。这里聚焦闭包逻辑组成正确性。）
    // ─────────────────────────────────────────────────────────────────────────

    /// 验证 truncate + note 拼接：截断时 user message 末尾追加 truncation note；
    /// 不截断时不追加。
    #[test]
    fn truncated_content_appends_note_to_user_message() {
        let conn = fresh_conn();
        let long_content = "a".repeat(20_000);
        let (truncated, did) =
            truncate_content_for_concept(&long_content, CONCEPT_CONTENT_MAX_BYTES);
        assert!(did);

        // 模拟主流程：assemble + 条件追加 note
        let mut messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "doc.pdf".into(),
                project_name: "p".into(),
                content: truncated,
            },
        )
        .unwrap();
        if did {
            if let Some(user_msg) = messages.iter_mut().rev().find(|m| m.role == "user") {
                user_msg.content.push_str(CONCEPT_TRUNCATION_NOTE);
            }
        }
        let user_msg = messages.iter().rev().find(|m| m.role == "user").unwrap();
        assert!(
            user_msg.content.contains("truncated to 8 KiB"),
            "user message 必须含 truncation note"
        );
    }

    /// 不截断路径：短内容时不追加 note。
    #[test]
    fn short_content_does_not_append_truncation_note() {
        let conn = fresh_conn();
        let short_content = "hello";
        let (out, did) = truncate_content_for_concept(short_content, CONCEPT_CONTENT_MAX_BYTES);
        assert!(!did);

        let mut messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "doc.pdf".into(),
                project_name: "p".into(),
                content: out,
            },
        )
        .unwrap();
        if did {
            if let Some(user_msg) = messages.iter_mut().rev().find(|m| m.role == "user") {
                user_msg.content.push_str(CONCEPT_TRUNCATION_NOTE);
            }
        }
        let user_msg = messages.iter().rev().find(|m| m.role == "user").unwrap();
        assert!(
            !user_msg.content.contains("truncated to 8 KiB"),
            "短内容路径不应追加 truncation note"
        );
    }
}
