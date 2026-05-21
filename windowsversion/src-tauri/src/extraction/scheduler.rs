use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;
use tauri::{AppHandle, Emitter, Manager};
use crate::db::Database;
use crate::db::extraction as db_ext;
use crate::db::conversion_meta::{self as db_conv_meta, ConversionMetaRow};
use crate::extraction::conversion::{classify_error, file_sha256};
use crate::extraction::extractors::{
    get_extractor_for, get_fallback_extractor_for_excluding,
};
use crate::extraction::failure_code::FailureCode;
use crate::extraction::models::{ExtractOptions, ExtractionError, ExtractionResult};
use crate::extraction::runtime_check::RuntimeCheckState;
use uuid::Uuid;

const SETTING_MARKITDOWN_ENABLED: &str = "markitdownEnabled";
const SETTING_MARKITDOWN_PYTHON_CMD: &str = "markitdownPythonCmd";
/// task_014 Fix-A3：讯飞 ASR language 覆盖；默认走 audio_asr_iflytek::DEFAULT_IFLYTEK_LANGUAGE ("cn")。
const SETTING_IFLYTEK_LANGUAGE: &str = "iflytekLanguage";

pub struct PipelineScheduler {
    is_running: Arc<TokioMutex<bool>>,
}

impl PipelineScheduler {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(TokioMutex::new(false)),
        }
    }

    /// 单个素材入队
    pub fn enqueue(app: &AppHandle, asset_id: &str) -> Result<String, String> {
        let db = app.state::<Database>();
        let conn = db.conn.lock().map_err(|e| format!("DB 锁失败: {e}"))?;

        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        if db_ext::get_extracted_content(&conn, asset_id)?.is_none() {
            db_ext::insert_extracted_content(&conn, &db_ext::ExtractedContentRow {
                id: Uuid::new_v4().to_string(),
                asset_id: asset_id.to_string(),
                status: "pending".to_string(),
                error_message: None,
                retry_count: 0,
                raw_text: None,
                structured_md: None,
                quality_level: 0,
                extractor_type: String::new(),
                segments_json: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            })?;
        }

        let task = db_ext::PipelineTaskRow {
            id: task_id.clone(),
            asset_id: asset_id.to_string(),
            task_type: "extract".to_string(),
            status: "queued".to_string(),
            retry_count: 0,
            max_retries: 3,
            error_message: None,
            priority: 100,
            batch_id: None,
            created_at: now,
            started_at: None,
            completed_at: None,
        };

        match db_ext::insert_pipeline_task(&conn, &task) {
            Ok(_) => {},
            Err(e) if e.contains("UNIQUE constraint") => {
                return Ok("already_queued".to_string());
            },
            Err(e) => return Err(e),
        }

        Ok(task_id)
    }

    /// 批量入队
    pub fn enqueue_batch(app: &AppHandle, asset_ids: &[String]) -> Result<String, String> {
        let batch_id = Uuid::new_v4().to_string();
        for asset_id in asset_ids {
            Self::enqueue(app, asset_id)?;
        }
        Ok(batch_id)
    }

    /// 启动后台执行循环（幂等：已在运行时直接返回）
    pub fn start(&self, app: AppHandle) {
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            // 幂等检查：已有调度循环时直接退出
            {
                let mut guard = is_running.lock().await;
                if *guard {
                    return;
                }
                *guard = true;
            }

            loop {
                // ─── 1. 取下一个待处理任务（sync 辅助函数，不跨 await 持有 MutexGuard）
                let next_task = match db_get_next_task(&app) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("调度器：{e}，退出调度循环");
                        break;
                    }
                };

                let Some(task) = next_task else {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    let has_tasks = match db_has_queued_tasks(&app) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("调度器：{e}，退出调度循环");
                            break;
                        }
                    };

                    if !has_tasks {
                        break;
                    }
                    continue;
                };

                // ─── 2. 标记任务为运行中
                db_mark_task_running(&app, &task.id, &task.asset_id);

                let _ = app.emit("extraction:progress", serde_json::json!({
                    "assetId": task.asset_id,
                    "status": "extracting",
                    "message": "正在提取..."
                }));

                // ─── 3. 取素材信息
                let asset_info = db_get_asset(&app, &task.asset_id);

                let Some(asset) = asset_info else {
                    db_mark_task_status(&app, &task.id, &task.asset_id, "failed", "素材不存在");
                    continue;
                };

                // ─── 4. 查找合适的提取器
                let options = db_get_extract_options(&app).unwrap_or_default();

                // task_010 AC-3：video/* 显式拒绝（本期不支持视频提取）。
                // 与 audio/* 不同：audio/* 由 `get_extractor_for` 通过 fallback
                // 链路命中 `audio_asr_iflytek`（IflytekAsrExtractor.can_handle）；
                // video/* 当前**没有任何 extractor**，若不显式拦截会被默认 "unsupported"
                // 路径吞掉、不写 `conversion_meta.failure_code` —— 违反技术约束
                // "不得静默吃掉路由错误"。
                //
                // 实现：复用 `FailureCode::EAudioWrongRoute`（同语义 "走错路由 / 本期
                // 不接"，PRD 底线 #4 锁定 8 错误码不增加变体）。落 conversion_meta
                // 失败码 + materialize_placeholder + db_mark_task_status('failed')。
                if video_route_should_reject(&asset.mime_type) {
                    let code = FailureCode::EAudioWrongRoute;
                    let source_hash = file_sha256(Path::new(&asset.file_path))
                        .unwrap_or_else(|_| String::new());
                    write_conversion_meta(
                        &app,
                        &asset.id,
                        "video_reject",
                        &asset.mime_type,
                        &source_hash,
                        0,
                        false,
                        Some(code.as_str()),
                    );
                    update_conversion_meta_failure_code(&app, &asset.id, Some(code));
                    let reason = format!(
                        "video/* not supported (mime={}, failure_code={})",
                        asset.mime_type,
                        code
                    );
                    db_mark_task_status(&app, &task.id, &task.asset_id, "failed", &reason);
                    let _ = app.emit(
                        "extraction:failed",
                        serde_json::json!({
                            "assetId": task.asset_id,
                            "errorMessage": reason,
                            "failureCode": code.as_str(),
                        }),
                    );
                    if source_asset_should_materialize(&asset) {
                        materialize_placeholder(&app, &asset, code.as_str(), &reason);
                    }
                    continue;
                }

                let extractor = get_extractor_for(&asset.mime_type, &options);
                let Some(extractor) = extractor else {
                    db_mark_task_status(&app, &task.id, &task.asset_id, "unsupported", "");
                    if source_asset_should_materialize(&asset) {
                        if source_asset_is_markdown(&asset) {
                            materialize_source_markdown(&app, &asset);
                        } else {
                            materialize_placeholder(
                                &app,
                                &asset,
                                "unsupported",
                                &format!("无可用提取器（mime: {}）", asset.mime_type),
                            );
                        }
                    }
                    continue;
                };

                // ─── 5. 执行提取（ADR-003：primary → fallback → placeholder 三级编排）
                let primary_name = extractor.name().to_string();

                // task_007 FIX (AC-3)：markitdown 路由前读 RuntimeCheckState 快照；
                // 自检失败时不进 Python 子进程，直接写 conversion_meta.failure_code
                // + 落 placeholder + 标记任务失败，跳过本任务。
                if let Some(code) = runtime_check_short_circuit(&primary_name, &options) {
                    let source_hash = file_sha256(Path::new(&asset.file_path))
                        .unwrap_or_else(|_| String::new());
                    write_conversion_meta(
                        &app,
                        &asset.id,
                        &primary_name,
                        &asset.mime_type,
                        &source_hash,
                        0,
                        false,
                        Some(code.as_str()),
                    );
                    update_conversion_meta_failure_code(&app, &asset.id, Some(code));
                    let reason =
                        format!("runtime self-check failed (failure_code={code})");
                    db_mark_task_status(
                        &app,
                        &task.id,
                        &task.asset_id,
                        "failed",
                        &reason,
                    );
                    let _ = app.emit(
                        "extraction:failed",
                        serde_json::json!({
                            "assetId": task.asset_id,
                            "errorMessage": reason,
                            "failureCode": code.as_str(),
                        }),
                    );
                    if source_asset_should_materialize(&asset) {
                        materialize_placeholder(&app, &asset, code.as_str(), &reason);
                    }
                    continue;
                }

                // task_009 (AC-3)：进入 markitdown 子进程前，对 `application/pdf` 做
                // 结构性嗅探（XObject + Font 引用判定），扫描型 PDF 直接短路写
                // `conversion_meta.failure_code = EScanPdfUnsupported` 并产出 placeholder，
                // 不再调用 markitdown（H6：禁启发式 / 禁"运行后看 stdout 长度"）。
                //
                // 仅当 primary 为 markitdown 时启用：text-passthrough 等其他 extractor
                // 不消费扫描件；fallback (pdf_text) 由 task_010 / 后续 OCR 接入。
                //
                // 决策语义：
                // - Ok(true)  → 短路 + EScanPdfUnsupported（与 runtime_check 短路语义一致）
                // - Ok(false) → 走常规 markitdown 路径
                // - Err(e)    → 按 ParseError 处理（不"猜测"成 scan）：log warn 后 fall-through，
                //               让 markitdown 自尝试；其失败仍走 task_008 失效四元分类。
                if primary_name == "markitdown" && asset.mime_type == "application/pdf" {
                    match scan_pdf_route_decision(Path::new(&asset.file_path)) {
                        ScanPdfDecision::ShortCircuit => {
                            let code = FailureCode::EScanPdfUnsupported;
                            let source_hash = file_sha256(Path::new(&asset.file_path))
                                .unwrap_or_else(|_| String::new());
                            write_conversion_meta(
                                &app,
                                &asset.id,
                                &primary_name,
                                &asset.mime_type,
                                &source_hash,
                                0,
                                false,
                                Some(code.as_str()),
                            );
                            update_conversion_meta_failure_code(&app, &asset.id, Some(code));
                            let reason = format!(
                                "scan pdf detected pre-markitdown (failure_code={code})"
                            );
                            db_mark_task_status(
                                &app,
                                &task.id,
                                &task.asset_id,
                                "failed",
                                &reason,
                            );
                            let _ = app.emit(
                                "extraction:failed",
                                serde_json::json!({
                                    "assetId": task.asset_id,
                                    "errorMessage": reason,
                                    "failureCode": code.as_str(),
                                }),
                            );
                            if source_asset_should_materialize(&asset) {
                                materialize_placeholder(&app, &asset, code.as_str(), &reason);
                            }
                            continue;
                        }
                        ScanPdfDecision::FallThrough => {
                            // 非扫描 / 解析失败 → 走常规 markitdown 路径
                        }
                    }
                }

                let primary_attempt =
                    run_extractor_blocking(extractor, &asset.file_path, &options).await;

                // 计算源文件哈希（任一路径写 conversion_meta 都用同一份；失败仅 warn）
                let source_hash = file_sha256(Path::new(&asset.file_path)).unwrap_or_else(|e| {
                    log::warn!("调度器：计算 file_sha256 失败 {}: {}", asset.file_path, e);
                    String::new()
                });

                let mime_for_meta = asset.mime_type.clone();
                let mut primary_attempt_class: Option<String> = None;
                let primary_step = match &primary_attempt {
                    Ok(r) if extraction_is_usable(r) => Step::PrimarySuccess,
                    Ok(_) => Step::PrimaryEmpty,
                    Err(e) => {
                        let class = extract_error_class(e);
                        primary_attempt_class = Some(class.to_string());
                        Step::PrimaryError
                    }
                };

                match primary_step {
                    Step::PrimarySuccess => {
                        // 真成功路径：写 extracted_content + materialize_md + conversion_meta
                        // 这里 primary_attempt 一定是 Ok（见 primary_step 决策），但仍用
                        // 模式匹配避免 unwrap/expect。
                        let r = match primary_attempt {
                            Ok(r) => r,
                            Err(_) => unreachable!("PrimarySuccess decided from Ok arm"),
                        };
                        save_and_materialize(&app, &asset, &task, &r);
                        write_conversion_meta(
                            &app,
                            &asset.id,
                            &primary_name,
                            &mime_for_meta,
                            &source_hash,
                            r.quality_level,
                            false,
                            None,
                        );
                    }
                    Step::PrimaryEmpty | Step::PrimaryError => {
                        // 登记 primary 失败/空 一行 conversion_meta
                        let primary_err_class = primary_attempt_class
                            .clone()
                            .unwrap_or_else(|| "empty_output".to_string());
                        write_conversion_meta(
                            &app,
                            &asset.id,
                            &primary_name,
                            &mime_for_meta,
                            &source_hash,
                            0,
                            false,
                            Some(&primary_err_class),
                        );

                        // 尝试 fallback（排除 primary 名称防止死循环）
                        let fb = get_fallback_extractor_for_excluding(
                            &asset.mime_type,
                            &primary_name,
                        );
                        let mut fallback_done = false;
                        if let Some(fb_extractor) = fb {
                            let fb_name = fb_extractor.name().to_string();
                            let _ = app.emit("extraction:progress", serde_json::json!({
                                "assetId": task.asset_id,
                                "status": "extracting",
                                "fallbackUsed": true,
                                "message": format!("{primary_name} 失败，回退到 {fb_name}..."),
                            }));
                            let fb_attempt = run_extractor_blocking(
                                fb_extractor,
                                &asset.file_path,
                                &options,
                            )
                            .await;
                            match fb_attempt {
                                Ok(r) if extraction_is_usable(&r) => {
                                    save_and_materialize(&app, &asset, &task, &r);
                                    write_conversion_meta(
                                        &app,
                                        &asset.id,
                                        &fb_name,
                                        &mime_for_meta,
                                        &source_hash,
                                        r.quality_level,
                                        true,
                                        None,
                                    );
                                    fallback_done = true;
                                }
                                Ok(_) => {
                                    // fallback 也空
                                    write_conversion_meta(
                                        &app,
                                        &asset.id,
                                        &fb_name,
                                        &mime_for_meta,
                                        &source_hash,
                                        0,
                                        true,
                                        Some("empty_output"),
                                    );
                                }
                                Err(fb_err) => {
                                    let fb_class = extract_error_class(&fb_err);
                                    write_conversion_meta(
                                        &app,
                                        &asset.id,
                                        &fb_name,
                                        &mime_for_meta,
                                        &source_hash,
                                        0,
                                        true,
                                        Some(fb_class),
                                    );
                                }
                            }
                        }

                        if !fallback_done {
                            // 都失败 → placeholder（不推进 derivative_version）
                            let error_msg = match &primary_attempt {
                                Ok(_) => "提取成功但结构化内容为空".to_string(),
                                Err(e) => e.to_string(),
                            };
                            // 把真实失败原因打到日志，避免下游只看到 code=conversion_error
                            // 排查不到根因。primary_name / asset_id 一并带上方便定位。
                            log::warn!(
                                "调度器：提取失败 asset={} primary={} reason={}",
                                asset.id,
                                primary_name,
                                error_msg
                            );
                            let is_terminal = task.retry_count + 1 >= task.max_retries;
                            db_handle_task_error(
                                &app, &task.id, &task.asset_id,
                                task.retry_count, task.max_retries,
                                &error_msg,
                            );
                            let _ = app.emit("extraction:failed", serde_json::json!({
                                "assetId": task.asset_id,
                                "errorMessage": error_msg,
                                "retryCount": task.retry_count + 1,
                            }));
                            if is_terminal && source_asset_should_materialize(&asset) {
                                if source_asset_is_markdown(&asset) {
                                    materialize_source_markdown(&app, &asset);
                                } else {
                                    let code = primary_attempt_class
                                        .as_deref()
                                        .unwrap_or("extract_failed");
                                    materialize_placeholder(&app, &asset, code, &error_msg);
                                }
                            }
                        }
                    }
                }
            }

            // 退出循环时重置运行标志，以便下次调用 start() 能重新启动
            let mut guard = is_running.lock().await;
            *guard = false;
        });
    }

    /// 启动恢复：重置 running 状态的任务为 queued
    pub fn recover(app: &AppHandle) -> Result<u64, String> {
        let db = app.state::<Database>();
        let conn = db.conn.lock().map_err(|e| format!("DB 锁失败: {e}"))?;
        db_ext::reset_running_tasks(&conn)
    }
}

// ─── 同步 DB 辅助函数（不跨 await，MutexGuard 不需要 Send）────────────────────

fn db_get_next_task(app: &AppHandle) -> Result<Option<db_ext::PipelineTaskRow>, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("DB 锁失败（取任务）: {e}"))?;
    Ok(db_ext::get_queued_tasks(&conn, 1)
        .unwrap_or_default()
        .into_iter()
        .next())
}

fn db_has_queued_tasks(app: &AppHandle) -> Result<bool, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("DB 锁失败（统计）: {e}"))?;
    let stats = db_ext::get_pipeline_stats(&conn).unwrap_or_else(|_| db_ext::PipelineStats {
        queued: 0, running: 0, completed: 0, failed: 0, cancelled: 0,
    });
    Ok(stats.queued > 0)
}

fn db_mark_task_running(app: &AppHandle, task_id: &str, asset_id: &str) {
    let db = app.state::<Database>();
    if let Ok(conn) = db.conn.lock() {
        let _ = db_ext::update_task_status(&conn, task_id, "running", None);
        let _ = db_ext::update_extraction_status(&conn, asset_id, "extracting", None);
    };
}

fn db_get_asset(app: &AppHandle, asset_id: &str) -> Option<crate::models::Asset> {
    let db = app.state::<Database>();
    // 存入变量使临时值（Result<MutexGuard, _>）在此处析构，早于 db 析构
    let result = match db.conn.lock() {
        Ok(conn) => crate::db::asset::get_by_id(&conn, asset_id).unwrap_or(None),
        Err(e) => {
            log::error!("调度器：DB 锁失败（取素材）: {e}");
            None
        }
    };
    result
}

fn db_get_extract_options(app: &AppHandle) -> Result<ExtractOptions, String> {
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|e| format!("DB 锁失败（读取提取配置）: {e}"))?;

    let markitdown_enabled = crate::db::settings::get(&conn, SETTING_MARKITDOWN_ENABLED)?
        .map(|v| {
            let trimmed = v.trim().trim_matches('"').to_ascii_lowercase();
            !matches!(trimmed.as_str(), "false" | "0" | "off")
        })
        .unwrap_or(true);

    let markitdown_python_cmd = crate::db::settings::get(&conn, SETTING_MARKITDOWN_PYTHON_CMD)?
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| detect_embedded_markitdown_python(app));

    // task_014 Fix-A3：iflytekLanguage setting → ExtractOptions.iflytek_language。
    // None / 空 由 extractor 端兜底为 DEFAULT_IFLYTEK_LANGUAGE ("cn")。
    let iflytek_language = crate::db::settings::get(&conn, SETTING_IFLYTEK_LANGUAGE)?
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty());

    // task_007 FIX (AC-3)：注入 runtime 自检快照。失败时由调用方在 markitdown
    // 路由分支前 short-circuit；markitdown::extract() 入口亦读此字段防御性短路。
    let runtime_check_failed = runtime_check_snapshot_err(app);

    Ok(ExtractOptions {
        markitdown_enabled,
        markitdown_python_cmd,
        iflytek_language,
        runtime_check_failed,
        ..ExtractOptions::default()
    })
}

/// task_007 FIX：读取 `RuntimeCheckState` 快照中的 FailureCode（若失败）。
/// 缺失 manage（极端 dev 路径）视为通过 —— 不引入额外失败风险。
fn runtime_check_snapshot_err(app: &AppHandle) -> Option<FailureCode> {
    app.try_state::<RuntimeCheckState>()
        .and_then(|state| state.snapshot().err())
}

// ─── task_010 (AC-3)：audio/video 路由判定（纯函数，便于单测） ─────────────

/// task_010 AC-3：判断给定 mime 是否应被 video 路由分支显式拒绝。
///
/// 行为：mime 以 `video/` 开头 → true；其他（含 audio/* / application/* / text/*）→ false。
/// **不**消费扩展名（路由判定优先级：mime > 扩展名；上层 scheduler 主循环已用 mime）。
pub(crate) fn video_route_should_reject(mime_type: &str) -> bool {
    mime_type.starts_with("video/")
}

/// task_010 AC-3：判断给定 mime 是否应路由到 audio_asr_iflytek（而非 markitdown）。
///
/// 行为：mime 以 `audio/` 开头 且属 iflytek `can_handle` 集合（mp3/mp4/wav/flac/x-wav）
/// → true；其他 audio/* 子类型（如 `audio/x-m4a`）当前 iflytek 未声明 can_handle，
/// 仍会走 fallback 链路但落到 "unsupported"。这里采纳 input.md 技术约束"保守值（拒绝）"
/// 不在本 task 内扩展 iflytek can_handle 列表（PRD 底线 #4：不动 audio_asr_iflytek.rs）。
///
/// 当前仅用于 `#[cfg(test)]` 路径断言；主循环不直接消费此判定（已由
/// `get_extractor_for` → fallback 链路命中 iflytek 实现）。
#[allow(dead_code)]
pub(crate) fn audio_should_route_to_iflytek(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "audio/mpeg" | "audio/mp4" | "audio/wav" | "audio/flac" | "audio/x-wav"
    )
}

/// task_007 FIX AC-3：**纯函数** —— 判断 markitdown 路由分支是否应短路。
/// 输入 `extractor_name`（来自 `Extractor::name()`）+ `ExtractOptions`，
/// 返回 `Some(FailureCode)` → 短路（不进 python 子进程，由调用方写 conversion_meta + 落库失败）；
/// 返回 `None` → 走常规路径。
///
/// 仅作用于 `extractor_name == "markitdown"`；其他 extractor 不受 runtime_manifest
/// 自检影响（task_007 范围仅 markitdown 路径；音频/PDF 等 fallback 由各自 task 处理）。
fn runtime_check_short_circuit(
    extractor_name: &str,
    options: &ExtractOptions,
) -> Option<FailureCode> {
    if extractor_name != "markitdown" {
        return None;
    }
    options.runtime_check_failed
}

/// task_009：扫描型 PDF 路由决策（纯函数版 IO，可单测）。
///
/// 输入：原始 PDF 文件路径。
/// 输出：
/// - `ShortCircuit` → 嗅探判定为扫描型 PDF（`Resources.XObject` 仅含 Image 且无 Font）；
///   调用方必须短路写 `EScanPdfUnsupported` + placeholder，**不**调用 markitdown。
/// - `FallThrough`  → 非扫描 / 解析失败 / 加密 PDF / 无 page tree；
///   调用方走常规 markitdown 路径（解析失败属"ParseError 处理"，不"猜测"为扫描）。
///
/// **严格分支映射**（与 ADR-006 / input.md AC-3 一致）：
/// - `is_scan_pdf == Ok(true)`  → `ShortCircuit`
/// - `is_scan_pdf == Ok(false)` → `FallThrough`
/// - `is_scan_pdf == Err(_)`    → `FallThrough`（log warn；让 markitdown 自尝试）
fn scan_pdf_route_decision(path: &Path) -> ScanPdfDecision {
    match crate::extraction::scan_pdf_detect::is_scan_pdf(path) {
        Ok(true) => ScanPdfDecision::ShortCircuit,
        Ok(false) => ScanPdfDecision::FallThrough,
        Err(e) => {
            log::warn!(
                "扫描 PDF 嗅探失败 {}: {} —— 按 ParseError 处理，继续走 markitdown",
                path.display(),
                e
            );
            ScanPdfDecision::FallThrough
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanPdfDecision {
    /// 扫描型 PDF：写 `EScanPdfUnsupported` + placeholder + 跳过 markitdown
    ShortCircuit,
    /// 非扫描 / 嗅探失败：走常规 markitdown 路径
    FallThrough,
}

/// task_007 FIX：将刚插入的 conversion_meta 行的 `failure_code` 字段写为指定码
/// （或 NULL）。失败仅 warn，与 `write_conversion_meta` 一致。
fn update_conversion_meta_failure_code(
    app: &AppHandle,
    source_asset_id: &str,
    code: Option<FailureCode>,
) {
    let db = app.state::<Database>();
    let conn = match db.conn.lock() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("更新 conversion_meta.failure_code：DB 锁失败: {e}");
            return;
        }
    };
    if let Err(e) = db_conv_meta::update_failure_code(&conn, source_asset_id, code) {
        log::warn!(
            "更新 conversion_meta.failure_code 失败 (source={source_asset_id}): {e}"
        );
    }
}

fn db_mark_task_status(app: &AppHandle, task_id: &str, asset_id: &str, status: &str, reason: &str) {
    let db = app.state::<Database>();
    if let Ok(conn) = db.conn.lock() {
        let msg = if reason.is_empty() { None } else { Some(reason) };
        if status == "unsupported" {
            let _ = db_ext::update_task_status(&conn, task_id, "completed", None);
            let _ = db_ext::update_extraction_status(&conn, asset_id, "unsupported", None);
        } else {
            let _ = db_ext::update_task_status(&conn, task_id, status, msg);
            let _ = db_ext::update_extraction_status(&conn, asset_id, status, msg);
        }
    };
}

fn db_save_extraction_result(
    app: &AppHandle,
    asset_id: &str,
    task_id: &str,
    raw_text: &str,
    structured_md: &str,
    quality_level: i32,
    extractor_type: &str,
    segments_json: Option<&str>,
) {
    let db = app.state::<Database>();
    if let Ok(conn) = db.conn.lock() {
        let _ = db_ext::update_extraction_result(
            &conn, asset_id, raw_text, structured_md,
            quality_level, extractor_type, segments_json,
        );
        let _ = db_ext::update_task_status(&conn, task_id, "completed", None);
    } else {
        log::error!("调度器：DB 锁失败（写提取结果）: 素材 {asset_id}");
    };
}

fn db_handle_task_error(
    app: &AppHandle,
    task_id: &str,
    asset_id: &str,
    retry_count: i32,
    max_retries: i32,
    error_msg: &str,
) {
    let db = app.state::<Database>();
    if let Ok(conn) = db.conn.lock() {
        let _ = db_ext::update_task_status(&conn, task_id, "failed", Some(error_msg));
        if retry_count + 1 < max_retries {
            let _ = db_ext::update_task_status(&conn, task_id, "queued", Some(error_msg));
        } else {
            let _ = db_ext::update_extraction_status(&conn, asset_id, "failed", Some(error_msg));
        }
    };
}

fn source_asset_should_materialize(asset: &crate::models::Asset) -> bool {
    // E1 F-1: 所有原件（非衍生）都应在工作区产出 .md 邻居
    asset.source_asset_id.is_none()
}

fn source_asset_is_markdown(asset: &crate::models::Asset) -> bool {
    asset.asset_type == "markdown" || asset.mime_type == "text/markdown"
}

fn build_frontmatter(
    source_id: &str,
    version: i32,
    extractor_type: &str,
    quality_level: i32,
) -> String {
    let now = chrono::Utc::now().to_rfc3339();
    format!(
        "---\nsource_asset_id: {}\nderivative_version: {}\nextracted_at: {}\nextractor_type: {}\nquality_level: {}\n---\n\n",
        source_id, version, now, extractor_type, quality_level
    )
}

fn archive_existing_version(
    workspace_dir: &Path,
    source_id: &str,
    version: i32,
    old_path: &str,
) {
    let versions_dir = workspace_dir.join("_versions").join(source_id);
    if let Err(e) = std::fs::create_dir_all(&versions_dir) {
        log::warn!(
            "物化 MD：创建版本目录失败 {}: {}",
            versions_dir.display(),
            e
        );
        return;
    }
    let archive_path = versions_dir.join(format!("v{}.md", version));
    if let Err(e) = std::fs::copy(old_path, &archive_path) {
        log::warn!(
            "物化 MD：归档旧版本失败 {} -> {}: {}",
            old_path,
            archive_path.display(),
            e
        );
    }
}

/// 计算**内存字符串**的 SHA-256（hex 小写），用于 `extracted_content.content_hash`
/// 的内容指纹（markdown 正文）。
///
/// AC-6（task_008）：scheduler 不再持有"另一套"文件哈希实现 —— 对**文件路径**统一使用
/// `crate::extraction::conversion::file_sha256`。本函数只服务于"已在内存中的字符串"
/// 这一窄场景，与 `file_sha256` 算法一致（同样 `sha2::Sha256` + hex 小写），但接口
/// 不同（路径 vs 字节序列），故保留为薄 wrapper 而非复制 file_sha256。
fn compute_sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn detect_embedded_markitdown_python(app: &AppHandle) -> Option<String> {
    let resource_dir = app.path().resource_dir().ok()?;
    let candidates = [
        resource_dir.join("markitdown-venv/bin/python"),
        resource_dir.join("markitdown-venv/bin/python3"),
        resource_dir.join("python/bin/python3"),
        resource_dir.join("python/bin/python"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
}

// ─────────────────────────────────────────────────────────────────────────────

/// 共享派生 MD 写盘逻辑（E1 F-1/F-2 + E2 F-3/F-4 + E3 F-6）：
/// - 注入 YAML frontmatter
/// - 若已有派生 .md，将旧版本归档到 `_versions/<source_asset_id>/v{N}.md`
/// - 写入 DB 并更新 source/derivative 的 derivative_version 与 content_hash
/// - 失败时仅 warn，不影响原件提取主流程
fn write_derivative_md(
    app: &AppHandle,
    source_asset: &crate::models::Asset,
    md_body: &str,
    quality_level: i32,
    extractor_type: &str,
) {
    let workspace_dir = match crate::workspace::ensure_project_workspace(&source_asset.project_id) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("物化 MD：无法创建工作区目录: {e}");
            return;
        }
    };

    let stem_raw = Path::new(&source_asset.name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("content");
    let stem = crate::utils::safe_name::sanitize_stem(stem_raw);
    let md_display_name = format!("{stem}.md");

    let next_version = source_asset.derivative_version + 1;
    let frontmatter =
        build_frontmatter(&source_asset.id, next_version, extractor_type, quality_level);
    let final_content = format!("{frontmatter}{md_body}");
    let hash = compute_sha256(md_body);
    let now = chrono::Utc::now().to_rfc3339();
    let file_size = final_content.len() as i64;

    let db = app.state::<Database>();
    let conn = match db.conn.lock() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("物化 MD：DB 锁失败: {e}");
            return;
        }
    };

    let existing = crate::db::asset::find_markdown_derivative(&conn, &source_asset.id)
        .ok()
        .flatten();

    let (derived_id, target_path, is_new) = if let Some(existing) = existing.as_ref() {
        archive_existing_version(
            &workspace_dir,
            &source_asset.id,
            source_asset.derivative_version,
            &existing.file_path,
        );
        (
            existing.id.clone(),
            std::path::PathBuf::from(&existing.file_path),
            false,
        )
    } else {
        let new_id = Uuid::new_v4().to_string();
        let file_name = format!("{new_id}_{stem}.md");
        (new_id, workspace_dir.join(file_name), true)
    };

    if let Err(e) = std::fs::write(&target_path, &final_content) {
        log::warn!("物化 MD：写出文件失败 {}: {e}", target_path.display());
        return;
    }

    if is_new {
        let derived_asset = crate::models::Asset {
            id: derived_id.clone(),
            project_id: source_asset.project_id.clone(),
            asset_type: "markdown".to_string(),
            name: md_display_name.clone(),
            original_name: md_display_name.clone(),
            file_path: target_path.to_string_lossy().to_string(),
            file_size,
            mime_type: "text/markdown".to_string(),
            captured_at: now.clone(),
            imported_at: now.clone(),
            source_type: "converted_from".to_string(),
            source_data: Some(source_asset.id.clone()),
            is_starred: false,
            source_asset_id: Some(source_asset.id.clone()),
            derivative_version: next_version,
        };
        if let Err(e) = crate::db::asset::insert(&conn, &derived_asset) {
            log::warn!("物化 MD：写入衍生 Asset 失败: {e}");
            let _ = std::fs::remove_file(&target_path);
            return;
        }
    } else {
        if let Err(e) = crate::db::asset::update_markdown_derivative(
            &conn,
            &derived_id,
            &md_display_name,
            file_size,
            &now,
        ) {
            log::warn!("物化 MD：更新衍生 Asset 失败 {}: {}", derived_id, e);
            return;
        }
    }

    // 版本号推进
    let _ = crate::db::asset::set_derivative_version(&conn, &derived_id, next_version);
    let _ = crate::db::asset::set_derivative_version(&conn, &source_asset.id, next_version);

    if let Err(e) =
        crate::db::tag::propagate_tags_to_derivative(&conn, &source_asset.id, &derived_id)
    {
        log::warn!(
            "物化 MD：继承标签失败 {} -> {}: {}",
            source_asset.id,
            derived_id,
            e
        );
    }

    let segments_json =
        serde_json::to_string(&crate::extraction::models::markdown_to_segments(md_body)).ok();
    if let Err(e) = crate::db::extraction::upsert_extraction_result(
        &conn,
        &derived_id,
        md_body,
        md_body,
        quality_level,
        extractor_type,
        segments_json.as_deref(),
    ) {
        log::warn!("物化 MD：更新衍生提取内容失败 {}: {}", derived_id, e);
    }

    // content_hash：源件 + 衍生件都写，供 F-8 增量抽取判重
    let _ = crate::db::extraction::set_content_hash(&conn, &derived_id, &hash);
    let _ = crate::db::extraction::set_content_hash(&conn, &source_asset.id, &hash);

    let _ = app.emit(
        "notecapt/asset-converted",
        serde_json::json!({
            "sourceAssetId": source_asset.id,
            "derivedAssetId": derived_id,
            "projectId": source_asset.project_id,
            "derivativeVersion": next_version,
        }),
    );

    // E4 F-7: 物化成功后通知前端/后台触发 library 级增量概念抽取
    // MVP 采用事件驱动：前端监听 `notecapt/concept-extract-requested` 调用
    // `extract_concepts_for_library(force=false)`，F-8 的去重日志确保不会
    // 无限触发重复抽取。
    if let Ok(Some(project)) = crate::db::project::get_by_id(&conn, &source_asset.project_id) {
        let _ = app.emit(
            "notecapt/concept-extract-requested",
            serde_json::json!({
                "libraryId": project.library_id,
                "triggerAssetId": source_asset.id,
                "triggerDerivedAssetId": derived_id,
            }),
        );
    }

    log::info!(
        "物化 MD v{} 完成: {} -> {} ({})",
        next_version,
        source_asset.id,
        derived_id,
        target_path.display()
    );
}

/// 成功路径：抽取结果已落库，将 structured_md 物化到工作区
fn materialize_md(
    app: &AppHandle,
    source_asset: &crate::models::Asset,
    md_body: &str,
    quality_level: i32,
    extractor_type: &str,
) {
    write_derivative_md(app, source_asset, md_body, quality_level, extractor_type);
}

/// 失败/不支持/空白路径：产出占位 .md，保证"每个原件都有工作区 .md 邻居"。
///
/// **ADR-006（task_008）**：placeholder 路径**不复用** `write_derivative_md`，
/// 改走单独的 `write_placeholder_md`：
/// - **不**调用 `set_derivative_version`（不推进版本号）
/// - **不**归档旧版本
/// - **不**写 `extracted_content`（避免 status=extracted 让真转换被跳过——R3）
fn materialize_placeholder(
    app: &AppHandle,
    source_asset: &crate::models::Asset,
    failure_code: &str,
    reason: &str,
) {
    let body = format!(
        "# {name}\n\n> 此为 NoteCapt 自动生成的工作区占位 Markdown：原件暂时无法抽取为结构化 Markdown。\n\n- **失败代码**: `{code}`\n- **原因**: {reason}\n- **原始文件**: `{path}`\n- **MIME**: `{mime}`\n\n> 你可以手动编辑补充笔记。后续再次抽取成功时，当前内容会被该次成功的 markdown 直接覆盖（不归档）。\n",
        name = source_asset.name,
        code = failure_code,
        reason = reason,
        path = source_asset.file_path,
        mime = source_asset.mime_type,
    );
    write_placeholder_md(app, source_asset, &body, failure_code, reason);
}

/// task_008 ADR-006：placeholder 专用写盘路径。
///
/// 与 `write_derivative_md` 的差异（严格 R3）：
/// - **不**推进 `derivative_version`（source/derivative 双侧均保持原值）
/// - **不**归档旧版本（若已有 derivative 文件，直接覆盖；不进 _versions/）
/// - **不**写 `extracted_content.status='extracted'`（保留为 failed 状态，
///   见调用方写入的 `update_extraction_status('failed', ...)`），让"日后真转换
///   成功"时 status 转 extracted、derivative_version 0→1 的链路不会被跳过。
/// - 文件名仍用 `<derived_id>_<stem>.md`（首次创建）或覆盖现有 derivative 文件，
///   保持"每个原件 ↔ 唯一 derivative"的不变量（ADR-001）。
/// - 仅 emit `notecapt/asset-placeholder` 事件供前端区分三态。
fn write_placeholder_md(
    app: &AppHandle,
    source_asset: &crate::models::Asset,
    md_body: &str,
    failure_code: &str,
    reason: &str,
) {
    let workspace_dir =
        match crate::workspace::ensure_project_workspace(&source_asset.project_id) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("写 placeholder：无法创建工作区目录: {e}");
                return;
            }
        };

    let stem_raw = Path::new(&source_asset.name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("content");
    let stem = crate::utils::safe_name::sanitize_stem(stem_raw);
    let md_display_name = format!("{stem}.md");

    // placeholder 的 frontmatter：版本号保持 source_asset.derivative_version
    // （**不**推进），quality_level=0，extractor_type 前缀 placeholder_。
    let frontmatter = build_frontmatter(
        &source_asset.id,
        source_asset.derivative_version,
        &format!("placeholder_{failure_code}"),
        0,
    );
    let final_content = format!("{frontmatter}{md_body}");
    let now = chrono::Utc::now().to_rfc3339();
    let file_size = final_content.len() as i64;

    let db = app.state::<Database>();
    let conn = match db.conn.lock() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("写 placeholder：DB 锁失败: {e}");
            return;
        }
    };

    let existing = crate::db::asset::find_markdown_derivative(&conn, &source_asset.id)
        .ok()
        .flatten();

    let (derived_id, target_path, is_new) = if let Some(existing) = existing.as_ref() {
        // 已有 derivative → 直接覆盖，不归档（placeholder 不进 _versions/）
        (
            existing.id.clone(),
            std::path::PathBuf::from(&existing.file_path),
            false,
        )
    } else {
        let new_id = Uuid::new_v4().to_string();
        let file_name = format!("{new_id}_{stem}.md");
        (new_id, workspace_dir.join(file_name), true)
    };

    if let Err(e) = std::fs::write(&target_path, &final_content) {
        log::warn!("写 placeholder：写出文件失败 {}: {e}", target_path.display());
        return;
    }

    if is_new {
        let derived_asset = crate::models::Asset {
            id: derived_id.clone(),
            project_id: source_asset.project_id.clone(),
            asset_type: "markdown".to_string(),
            name: md_display_name.clone(),
            original_name: md_display_name.clone(),
            file_path: target_path.to_string_lossy().to_string(),
            file_size,
            mime_type: "text/markdown".to_string(),
            captured_at: now.clone(),
            imported_at: now.clone(),
            source_type: "converted_from".to_string(),
            source_data: Some(source_asset.id.clone()),
            is_starred: false,
            source_asset_id: Some(source_asset.id.clone()),
            // placeholder 不推进版本号：与 source 保持一致（通常 0）
            derivative_version: source_asset.derivative_version,
        };
        if let Err(e) = crate::db::asset::insert(&conn, &derived_asset) {
            log::warn!("写 placeholder：写入衍生 Asset 失败: {e}");
            let _ = std::fs::remove_file(&target_path);
            return;
        }
    } else if let Err(e) = crate::db::asset::update_markdown_derivative(
        &conn,
        &derived_id,
        &md_display_name,
        file_size,
        &now,
    ) {
        log::warn!("写 placeholder：更新衍生 Asset 失败 {}: {}", derived_id, e);
        return;
    }

    // 标签继承仍要做（占位也是 derivative）
    if let Err(e) =
        crate::db::tag::propagate_tags_to_derivative(&conn, &source_asset.id, &derived_id)
    {
        log::warn!(
            "写 placeholder：继承标签失败 {} -> {}: {}",
            source_asset.id,
            derived_id,
            e
        );
    }

    // 关键：**不**调用 set_derivative_version，**不**调用 upsert_extraction_result。
    // 仅 emit 一个区分三态的事件。
    let _ = app.emit(
        "notecapt/asset-placeholder",
        serde_json::json!({
            "sourceAssetId": source_asset.id,
            "derivedAssetId": derived_id,
            "projectId": source_asset.project_id,
            "failureCode": failure_code,
            "reason": reason,
        }),
    );

    log::info!(
        "写 placeholder 完成（不推进版本号）: {} -> {} ({}, code={})",
        source_asset.id,
        derived_id,
        target_path.display(),
        failure_code,
    );
}

/// .md 原件路径：读取源文件正文 → 注入 frontmatter → 写工作区副本
fn materialize_source_markdown(app: &AppHandle, source_asset: &crate::models::Asset) {
    let body = match std::fs::read_to_string(&source_asset.file_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "物化源 MD：读取失败 {}: {e}",
                source_asset.file_path
            );
            materialize_placeholder(
                app,
                source_asset,
                "read_failed",
                &format!("读取源文件失败: {e}"),
            );
            return;
        }
    };
    let quality = crate::extraction::models::evaluate_markdown_quality(&body);
    write_derivative_md(app, source_asset, &body, quality, "source_markdown");
}

// ─── task_008：fallback 编排辅助 ─────────────────────────────────────────────

/// 主循环的三态决策标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Primary 返回 Ok 且 `extraction_is_usable(r) == true`
    PrimarySuccess,
    /// Primary 返回 Ok 但内容不可用（quality_level==0 或 structured_md 为空）
    PrimaryEmpty,
    /// Primary 返回 Err
    PrimaryError,
}

/// 判断一次抽取结果是否"可用"（用于走真成功路径）：
/// `quality_level > 0` 且 `structured_md` 非空。
fn extraction_is_usable(r: &ExtractionResult) -> bool {
    r.quality_level > 0 && !r.structured_md.is_empty()
}

/// 在 `spawn_blocking` 中运行单个 Extractor，统一处理 JoinError：
/// 如果线程 panic 则映射为 `ExtractionError::ParseError`（前缀 `error_class:conversion_error|`
/// 由 task_007 约定，但 panic 走 conversion_error 兜底）。
async fn run_extractor_blocking(
    extractor: Box<dyn crate::extraction::Extractor>,
    file_path: &str,
    options: &ExtractOptions,
) -> Result<ExtractionResult, ExtractionError> {
    let path = file_path.to_string();
    let opts = options.clone();
    let started = Instant::now();
    let join = tokio::task::spawn_blocking(move || {
        extractor.extract(Path::new(&path), &opts)
    })
    .await;
    let _elapsed = started.elapsed();
    match join {
        Ok(res) => res,
        Err(e) => Err(ExtractionError::ParseError(format!(
            "error_class:conversion_error|提取任务 panic: {e}"
        ))),
    }
}

/// 从 `ExtractionError` 提取稳定的 error_class。
///
/// 约定（task_007）：`ExtractionError::ParseError` 字符串以 `error_class:xxx|...` 开头
/// 时，解析 xxx；否则使用 `conversion::classify_error` 兜底子串匹配。
fn extract_error_class(e: &ExtractionError) -> &'static str {
    let s = e.to_string();
    if let Some(class) = parse_error_class_prefix(&s) {
        // 把动态字符串映射回静态字符串集合（覆盖 classify_error 输出全集 + 兜底）
        return map_to_static_class(class);
    }
    classify_error(&s)
}

/// 解析 `error_class:xxx|...` 前缀，返回 xxx；失败返回 None。
fn parse_error_class_prefix(msg: &str) -> Option<&str> {
    // ExtractionError::Display 形如 "解析错误: error_class:xxx|..."
    // 兼容裸字符串和带前缀两种
    let rest = msg.strip_prefix("error_class:")
        .or_else(|| msg.find("error_class:").map(|i| &msg[i + "error_class:".len()..]))?;
    let end = rest.find('|')?;
    Some(&rest[..end])
}

/// 将动态字符串归并到 classify_error 的 8 个静态枚举之一。
fn map_to_static_class(class: &str) -> &'static str {
    match class {
        "file_not_found" => "file_not_found",
        "permission_denied" => "permission_denied",
        "unsupported_format" => "unsupported_format",
        "markitdown_not_installed" => "markitdown_not_installed",
        "python_unavailable" => "python_unavailable",
        "empty_output" => "empty_output",
        "timeout" => "timeout",
        _ => "conversion_error",
    }
}

/// 成功路径共用：写 extracted_content + emit completed + materialize_md / source_md。
fn save_and_materialize(
    app: &AppHandle,
    asset: &crate::models::Asset,
    task: &db_ext::PipelineTaskRow,
    r: &ExtractionResult,
) {
    let segments_json = serde_json::to_string(&r.segments).ok();
    db_save_extraction_result(
        app,
        &task.asset_id,
        &task.id,
        &r.raw_text,
        &r.structured_md,
        r.quality_level,
        &r.extractor_type,
        segments_json.as_deref(),
    );
    let _ = app.emit(
        "extraction:completed",
        serde_json::json!({
            "assetId": task.asset_id,
            "qualityLevel": r.quality_level,
            "extractorType": r.extractor_type,
        }),
    );
    if source_asset_should_materialize(asset) {
        if source_asset_is_markdown(asset) {
            materialize_source_markdown(app, asset);
        } else {
            materialize_md(
                app,
                asset,
                &r.structured_md,
                r.quality_level,
                &r.extractor_type,
            );
        }
    }
}

/// 写一行 `conversion_meta`。失败仅 `warn`，不影响主流程（task_008 硬约束）。
fn write_conversion_meta(
    app: &AppHandle,
    source_asset_id: &str,
    converter_name: &str,
    source_mime: &str,
    source_hash: &str,
    quality_level: i32,
    fallback_used: bool,
    error_class: Option<&str>,
) {
    let row = ConversionMetaRow {
        id: Uuid::new_v4().to_string(),
        source_asset_id: source_asset_id.to_string(),
        derived_asset_id: None,
        converter_name: converter_name.to_string(),
        converter_version: String::new(),
        source_mime: source_mime.to_string(),
        source_hash: source_hash.to_string(),
        quality_level,
        fallback_used,
        error_class: error_class.map(|s| s.to_string()),
        conversion_ms: None,
        converted_at: chrono::Utc::now().to_rfc3339(),
    };

    let db = app.state::<Database>();
    let conn = match db.conn.lock() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("写 conversion_meta：DB 锁失败: {e}");
            return;
        }
    };
    if let Err(e) = db_conv_meta::insert(&conn, &row) {
        log::warn!(
            "写 conversion_meta 失败（source={} converter={}）: {}",
            source_asset_id,
            converter_name,
            e
        );
    }
}

/// **纯函数版本**的 fallback 决策（不依赖 AppHandle / 数据库 / IO），
/// 用于单测覆盖 AC-4 的 5 个场景决策路径。主循环的 `match primary_step { ... }`
/// 用相同语义。
///
/// `#[allow(dead_code)]`：仅在 `#[cfg(test)]` 中被消费；主循环为减少分配采用
/// 内联的 `Step` 枚举走真实路径（IO + DB + emit）。两边保持决策语义一致——
/// 见 tests 模块的 5 个场景测试。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextStep {
    /// primary 直接成功 → materialize_md, conversion_meta(primary, fallback=false)
    UsePrimary,
    /// primary 失败/空 → fallback 成功 → materialize_md, conversion_meta(fallback, fallback=true)
    /// 同时附带 primary 的一行失败 conversion_meta（meta 写两次）
    UseFallback,
    /// primary 失败/空，且 fallback 也失败/空/不可用 → placeholder + 两行 conversion_meta
    Placeholder,
}

/// 纯函数：基于 primary 和（可选）fallback 的两次抽取结果，决策下一步。
///
/// `fallback_result == None` 表示不存在 fallback 候选（已被排除或 mime 无候选）。
/// 此情况下 primary 失败/空 → 直接 Placeholder。
#[allow(dead_code)]
fn decide_next_step(
    primary_result: &Result<ExtractionResult, ExtractionError>,
    fallback_result: Option<&Result<ExtractionResult, ExtractionError>>,
) -> NextStep {
    if let Ok(p) = primary_result {
        if extraction_is_usable(p) {
            return NextStep::UsePrimary;
        }
    }
    match fallback_result {
        Some(Ok(f)) if extraction_is_usable(f) => NextStep::UseFallback,
        _ => NextStep::Placeholder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::models::ContentSegment;

    fn ok_result(quality: i32, md: &str) -> Result<ExtractionResult, ExtractionError> {
        Ok(ExtractionResult {
            raw_text: md.to_string(),
            structured_md: md.to_string(),
            quality_level: quality,
            extractor_type: "test".to_string(),
            segments: Vec::<ContentSegment>::new(),
            needs_ocr_fallback: false,
        })
    }
    fn err_result(msg: &str) -> Result<ExtractionResult, ExtractionError> {
        Err(ExtractionError::ParseError(msg.to_string()))
    }

    /// AC-4 T-1：primary 成功 → UsePrimary（fallback 不会被调用，fallback_result=None）
    #[test]
    fn t1_primary_success_uses_primary() {
        let p = ok_result(2, "# hello\n");
        let step = decide_next_step(&p, None);
        assert_eq!(step, NextStep::UsePrimary);
    }

    /// AC-4 T-2：primary Err / fallback 成功 → UseFallback
    #[test]
    fn t2_primary_err_fallback_success_uses_fallback() {
        let p = err_result("error_class:markitdown_not_installed|...");
        let f = ok_result(1, "fallback body");
        let step = decide_next_step(&p, Some(&f));
        assert_eq!(step, NextStep::UseFallback);
    }

    /// AC-4 T-3：primary Err / fallback Err → Placeholder
    #[test]
    fn t3_both_err_uses_placeholder() {
        let p = err_result("error_class:timeout|...");
        let f = err_result("pdf parse failed");
        let step = decide_next_step(&p, Some(&f));
        assert_eq!(step, NextStep::Placeholder);
    }

    /// AC-4 T-4：placeholder 已存在的语义→该场景重跑后 primary 成功 → UsePrimary
    /// （证明真成功不会被 placeholder 历史污染：决策只看本次结果）
    #[test]
    fn t4_after_placeholder_primary_success_overrides() {
        let p = ok_result(2, "# real content\n");
        let step = decide_next_step(&p, None);
        assert_eq!(step, NextStep::UsePrimary);
    }

    /// AC-4 T-5：primary 成功（重复执行）→ UsePrimary（幂等：决策不依赖历史）
    #[test]
    fn t5_idempotent_repeat_primary_success() {
        let p = ok_result(3, "# stable\n");
        let step1 = decide_next_step(&p, None);
        let step2 = decide_next_step(&p, None);
        assert_eq!(step1, NextStep::UsePrimary);
        assert_eq!(step2, NextStep::UsePrimary);
    }

    /// 额外：primary Ok 但 quality_level==0 → 视为 PrimaryEmpty；fallback 成功 → UseFallback
    #[test]
    fn primary_ok_empty_then_fallback_success() {
        let p = ok_result(0, "");
        let f = ok_result(2, "real");
        let step = decide_next_step(&p, Some(&f));
        assert_eq!(step, NextStep::UseFallback);
    }

    /// 额外：primary Ok 但 structured_md 为空 → PrimaryEmpty；无 fallback 候选 → Placeholder
    #[test]
    fn primary_ok_empty_no_fallback_candidate_uses_placeholder() {
        let p = ok_result(2, "");
        let step = decide_next_step(&p, None);
        assert_eq!(step, NextStep::Placeholder);
    }

    /// AC-4 / task_007 约定：error_class:xxx| 前缀解析
    #[test]
    fn parse_error_class_prefix_strips_prefix() {
        assert_eq!(
            parse_error_class_prefix("error_class:timeout|foo bar"),
            Some("timeout")
        );
        // ExtractionError::Display 加前缀 "解析错误: " 也能被识别
        assert_eq!(
            parse_error_class_prefix("解析错误: error_class:markitdown_not_installed|x"),
            Some("markitdown_not_installed")
        );
        assert_eq!(parse_error_class_prefix("plain error no prefix"), None);
    }

    /// AC-4：extract_error_class 在 ParseError 带前缀时直接解析；无前缀走 classify_error 兜底
    #[test]
    fn extract_error_class_prefers_prefix_then_falls_back() {
        let e1 =
            ExtractionError::ParseError("error_class:timeout|did not finish".to_string());
        assert_eq!(extract_error_class(&e1), "timeout");

        let e2 = ExtractionError::ParseError("subprocess timed out after 60s".to_string());
        assert_eq!(extract_error_class(&e2), "timeout");

        let e3 = ExtractionError::ParseError("some odd failure".to_string());
        assert_eq!(extract_error_class(&e3), "conversion_error");

        // 未知 class → 兜底
        let e4 = ExtractionError::ParseError("error_class:weirdo|x".to_string());
        assert_eq!(extract_error_class(&e4), "conversion_error");
    }

    // ─── task_007 FIX (AC-3)：runtime_check 路由短路 ─────────────────────────

    /// task_007 FIX AC-3：markitdown 路由 + runtime_check 失败 → 返回 Some(code) 短路。
    /// **不调子进程**：本测仅断言纯函数决策；scheduler 主循环据此 `continue;`
    /// 而**不**调用 `run_extractor_blocking`。
    #[test]
    fn runtime_check_short_circuits_markitdown_on_failure() {
        let opts = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: Some(FailureCode::ERuntimeMissing),
            ..ExtractOptions::default()
        };
        assert_eq!(
            runtime_check_short_circuit("markitdown", &opts),
            Some(FailureCode::ERuntimeMissing),
            "markitdown 路由 + 自检失败 → 必须短路"
        );

        let opts_epub = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: Some(FailureCode::EExtraMissingEpub),
            ..ExtractOptions::default()
        };
        assert_eq!(
            runtime_check_short_circuit("markitdown", &opts_epub),
            Some(FailureCode::EExtraMissingEpub),
            "EPUB extras 缺失 → 同样短路携带 EExtraMissingEpub"
        );
    }

    /// task_007 FIX AC-3：自检成功 / 非 markitdown 路由 → 不短路，走常规路径。
    #[test]
    fn runtime_check_does_not_short_circuit_on_pass_or_non_markitdown() {
        // (a) 自检通过 → 不短路（无论 extractor）
        let opts_ok = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        assert_eq!(runtime_check_short_circuit("markitdown", &opts_ok), None);

        // (b) 自检失败但 extractor != markitdown → 不短路（fallback / 文本直通不依赖 python venv）
        let opts_fail = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: Some(FailureCode::ERuntimeMissing),
            ..ExtractOptions::default()
        };
        assert_eq!(runtime_check_short_circuit("pdf_text", &opts_fail), None);
        assert_eq!(runtime_check_short_circuit("text_passthrough", &opts_fail), None);
        assert_eq!(runtime_check_short_circuit("audio_asr_iflytek", &opts_fail), None);
    }

    // ─── task_010 (AC-3/AC-4)：audio/video 路由 ─────────────────────────────

    /// task_010 AC-4#1：mp3 / wav / m4a / mp4(audio) / flac mime 都路由到 iflytek，
    /// 而非 markitdown。基于 `get_extractor_for` 真实链路（`audio_asr_iflytek::name()`
    /// 与 `markitdown::name()` 字面对比）。
    #[test]
    fn audio_mime_routes_to_iflytek_not_markitdown() {
        use crate::extraction::extractors::get_extractor_for;
        let opts = ExtractOptions {
            markitdown_enabled: true,
            ..ExtractOptions::default()
        };
        for mime in ["audio/mpeg", "audio/wav", "audio/mp4", "audio/flac", "audio/x-wav"] {
            let extractor = get_extractor_for(mime, &opts)
                .unwrap_or_else(|| panic!("audio mime={mime} 应有 extractor"));
            assert_eq!(
                extractor.name(),
                "audio_asr_iflytek",
                "AC-3：{mime} 必须路由到 iflytek，实际：{}",
                extractor.name()
            );
            assert!(
                audio_should_route_to_iflytek(mime),
                "audio_should_route_to_iflytek({mime}) 应为 true"
            );
        }
    }

    /// task_010 AC-4#2：video/* mime 在主循环被 `video_route_should_reject` 拦截，
    /// 显式拒绝（不依赖 fallback / unsupported 静默路径）。
    #[test]
    fn video_mime_is_explicitly_rejected() {
        for mime in ["video/mp4", "video/webm", "video/quicktime", "video/x-msvideo"] {
            assert!(
                video_route_should_reject(mime),
                "AC-3：{mime} 应被 video 路由拒绝"
            );
        }
        // 非 video/* mime 不应被拒绝
        for mime in [
            "audio/mpeg",
            "application/pdf",
            "image/png",
            "text/html",
            "text/plain",
        ] {
            assert!(
                !video_route_should_reject(mime),
                "{mime} 不该被 video 拒绝分支拦截"
            );
        }
    }

    /// task_010 AC-3：video/* 不存在合法 extractor 候选（即使 markitdown_enabled）。
    /// 没有 task_010 显式拒绝路径，调度器会落 "unsupported" 默默吞错；这里证明
    /// `get_extractor_for` 对 video/* 返回 None，**强制**主循环走 `video_route_should_reject`
    /// 拦截分支才能写 failure_code。
    #[test]
    fn video_mime_has_no_extractor_so_must_be_explicitly_rejected() {
        use crate::extraction::extractors::get_extractor_for;
        let opts = ExtractOptions {
            markitdown_enabled: true,
            ..ExtractOptions::default()
        };
        for mime in ["video/mp4", "video/webm", "video/quicktime"] {
            assert!(
                get_extractor_for(mime, &opts).is_none(),
                "{mime} 必须没有 extractor 候选 → 否则 task_010 video 拒绝路径会被绕过"
            );
        }
    }

    // ─── task_009 (AC-3)：PDF scan route decision ─────────────────────────

    /// task_009 AC-3：损坏/不存在 PDF → FallThrough（按 ParseError 处理，
    /// 让 markitdown 自尝试；不"猜测"成 scan）。
    #[test]
    fn scan_pdf_route_decision_falls_through_on_parse_err() {
        // 不存在路径 → is_scan_pdf 必然 Err → 决策必须 FallThrough
        let p = Path::new("/nonexistent/path/__never_exists__.pdf");
        assert_eq!(scan_pdf_route_decision(p), ScanPdfDecision::FallThrough);
    }

    /// task_009 AC-3：非 PDF 字节流 → Err → FallThrough（不短路）
    #[test]
    fn scan_pdf_route_decision_falls_through_on_corrupted_bytes() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"this is not a pdf").unwrap();
        assert_eq!(
            scan_pdf_route_decision(tmp.path()),
            ScanPdfDecision::FallThrough,
            "解析失败必须 FallThrough，不可猜测为 scan"
        );
    }

    /// AC-2：extraction_is_usable 在 quality==0 或 md 空时返回 false
    #[test]
    fn extraction_is_usable_rejects_empty_or_zero_quality() {
        let r1 = ExtractionResult {
            raw_text: "".into(),
            structured_md: "real".into(),
            quality_level: 0,
            extractor_type: "x".into(),
            segments: vec![],
            needs_ocr_fallback: false,
        };
        assert!(!extraction_is_usable(&r1));

        let r2 = ExtractionResult {
            raw_text: "".into(),
            structured_md: "".into(),
            quality_level: 2,
            extractor_type: "x".into(),
            segments: vec![],
            needs_ocr_fallback: false,
        };
        assert!(!extraction_is_usable(&r2));

        let r3 = ExtractionResult {
            raw_text: "".into(),
            structured_md: "ok".into(),
            quality_level: 1,
            extractor_type: "x".into(),
            segments: vec![],
            needs_ocr_fallback: false,
        };
        assert!(extraction_is_usable(&r3));
    }
}
