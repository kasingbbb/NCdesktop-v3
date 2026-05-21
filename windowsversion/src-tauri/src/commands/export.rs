use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{self, Database};
use crate::models::AIAnalysisRow;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportOptions {
    pub project_id: String,
    pub include_transcription: bool,
    pub include_ocr: bool,
    pub include_ai_summary: bool,
    pub include_tags: bool,
    pub include_notes: bool,
    pub include_timeline: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub markdown: String,
    pub word_count: usize,
    pub section_count: usize,
}

/// 将项目数据组装为结构化 Markdown
#[tauri::command]
pub async fn export_project_markdown(
    db: State<'_, Database>,
    options: ExportOptions,
) -> Result<ExportResult, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    let project = db::project::get_by_id(&conn, &options.project_id)?
        .ok_or_else(|| "项目不存在".to_string())?;

    let timeline = db::timeline::get_timeline_by_project(&conn, &options.project_id)?;
    #[allow(deprecated)] // 导出场景需要完整 asset 列（含 derivative），与工作区视图不同
    let assets = db::asset::get_by_project(&conn, &options.project_id).unwrap_or_default();
    let notes = db::note::get_by_project(&conn, &options.project_id).unwrap_or_default();
    let tags = db::tag::get_all(&conn).unwrap_or_default();

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", project.name));
    md.push_str(&format!(
        "> 导出时间：{}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    ));

    if options.include_timeline {
        if let Some(ref t) = timeline {
            md.push_str("## 时间轴信息\n\n");
            md.push_str(&format!("- 时长：{}\n", format_duration(t.duration)));
            md.push_str(&format!("- 录音时间：{} — {}\n\n", t.start_time, t.end_time));
        }
    }

    if options.include_transcription {
        md.push_str("## 音频转录\n\n");
        if let Some(ref t) = timeline {
            let audio_tracks =
                db::timeline::get_audio_tracks_by_timeline(&conn, &t.id).unwrap_or_default();
            if audio_tracks.is_empty() {
                md.push_str("> 暂无音频轨道\n\n");
            } else {
                // 当前版本转录分段存储为 JSON（segments_json），后续可升级为逐段时间戳输出
                for track in &audio_tracks {
                    let tr = db::timeline::get_transcription_by_audio(&conn, &track.id).ok().flatten();
                    md.push_str(&format!("### {}\n\n", track.file_name));
                    if let Some(tr) = tr {
                        md.push_str(&format!("```json\n{}\n```\n\n", tr.segments_json));
                    } else {
                        md.push_str("> 暂无转录\n\n");
                    }
                }
            }
        } else {
            md.push_str("> 暂无时间轴\n\n");
        }
    }

    if options.include_ocr || options.include_ai_summary {
        md.push_str("## 素材分析\n\n");
        if assets.is_empty() {
            md.push_str("> 暂无素材\n\n");
        } else {
            for a in &assets {
                md.push_str(&format!("### {}\n\n", a.name));
                let analysis: Option<AIAnalysisRow> =
                    db::asset::get_analysis(&conn, &a.id).ok().flatten();

                if let Some(ai) = analysis {
                    if options.include_ai_summary && !ai.summary.trim().is_empty() {
                        md.push_str(&format!("> **AI 摘要**：{}\n\n", ai.summary));
                    }
                    if options.include_ocr {
                        if let Some(ocr) = ai.ocr_text {
                            if !ocr.trim().is_empty() {
                                md.push_str("**OCR 文本**：\n\n");
                                md.push_str(&ocr);
                                md.push_str("\n\n");
                            }
                        }
                    }
                    if options.include_tags && !ai.suggested_tags.trim().is_empty() {
                        md.push_str(&format!("**建议标签**：`{}`\n\n", ai.suggested_tags));
                    }
                } else {
                    md.push_str("> 暂无 AI 分析\n\n");
                }
            }
        }
    }

    if options.include_tags {
        md.push_str("## 标签\n\n");
        if tags.is_empty() {
            md.push_str("> 暂无标签\n\n");
        } else {
            let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
            md.push_str(&format!("`{}`\n\n", tag_names.join("` `")));
        }
    }

    if options.include_notes {
        md.push_str("## 笔记\n\n");
        if notes.is_empty() {
            md.push_str("> 暂无笔记\n\n");
        } else {
            for n in &notes {
                md.push_str(&format!("### {}\n\n{}\n\n", n.created_at, n.content));
            }
        }
    }

    md.push_str("---\n\n*由 NoteCapt 自动导出*\n");

    Ok(ExportResult {
        markdown: md.clone(),
        word_count: md.chars().filter(|c| !c.is_whitespace()).count(),
        section_count: count_sections(&md),
    })
}

/// 将 Markdown 复制到系统剪贴板
#[tauri::command]
pub async fn copy_to_clipboard(
    app: tauri::AppHandle,
    text: String,
) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("clipboard-copy", &text)
        .map_err(|e: tauri::Error| e.to_string())?;
    Ok(())
}

fn format_duration(seconds: f64) -> String {
    let h = (seconds / 3600.0).floor() as u32;
    let m = ((seconds % 3600.0) / 60.0).floor() as u32;
    let s = (seconds % 60.0).floor() as u32;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

fn count_sections(markdown: &str) -> usize {
    markdown.lines().filter(|l| l.starts_with("## ")).count()
}
