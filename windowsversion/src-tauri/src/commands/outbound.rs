//! Outbound payload 投递命令（task_005_dev_m4_outbound_payload）。
//!
//! 提供 [`prepare_outbound_payload`]：把多选 done 态资产的 canonical markdown
//! rendition 投影到系统缓存目录下的稳定文件名，供 dropzone 启动 NSFilenamesPboardType
//! 拖出。落盘走 `fs::hard_link` → 跨卷 `fs::copy` fallback，缓存目录每次调用前
//! 幂等重建（ADR-005）。
//!
//! ## 错误模型
//! 错误以 [`OutboundError`] 序列化为 JSON 字符串返回（Tauri 命令 `Err(String)`
//! 通道），前端 `tauri-commands.ts` 解析后据此 toast / 禁用 startDrag。
//!
//! ## 范围
//! 本任务**不**实现 NSStringPboardType 双 representation —— 按 ADR-008 留作
//! Phase 1 末 spike；当前依靠文件名 `.md` 后缀让 ChatGPT / Claude 桌面端识别。

use crate::db::{self, Database};
use crate::models::AssetState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tauri::State;

/// 单条 outbound 投影结果，序列化给前端用于 NSFilenamesPboardType 启动。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutboundEntry {
    pub asset_id: String,
    /// 缓存目录内 `.md` 文件的绝对路径
    pub path: String,
    /// sanitize 后的文件名（不含目录前缀），用于 UI 调试展示
    pub display_name: String,
}

/// 结构化错误：序列化为 JSON 字符串后通过 `Result<_, String>` 上抛。
///
/// `kind` 字段固定为蛇形枚举名，便于前端做联合类型判断；其它字段按需附带上下文。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OutboundError {
    /// 入参 asset_ids 为空
    #[serde(rename = "emptyInput")]
    EmptyInput { message: String },
    /// 单选场景下唯一资产不是 done 态
    #[serde(rename = "stateNotDone")]
    StateNotDone {
        #[serde(rename = "assetId")]
        asset_id: String,
        state: String,
        message: String,
    },
    /// 多选场景下存在 ≥1 非 done 态资产
    #[serde(rename = "mixedStates")]
    MixedStates {
        offending: Vec<String>,
        message: String,
    },
    /// 资产存在但 canonical markdown rendition 在 DB 或磁盘上缺失
    #[serde(rename = "renditionMissing")]
    RenditionMissing {
        #[serde(rename = "assetId")]
        asset_id: String,
        message: String,
    },
    /// 资产 id 在数据库中不存在
    #[serde(rename = "assetNotFound")]
    AssetNotFound {
        #[serde(rename = "assetId")]
        asset_id: String,
        message: String,
    },
    /// 文件系统 IO 失败（hardlink / copy / mkdir / remove_dir_all）
    #[serde(rename = "ioFailed")]
    IoFailed {
        #[serde(rename = "assetId")]
        asset_id: Option<String>,
        detail: String,
        message: String,
    },
}

impl OutboundError {
    fn to_json(&self) -> String {
        // serde_json 序列化对 OutboundError 不会失败（字段都是基础类型）；
        // 极端兜底返回纯文本，避免双重 Result。
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                "{{\"kind\":\"ioFailed\",\"message\":\"序列化错误失败\",\"detail\":\"{self:?}\"}}"
            )
        })
    }
}

/// outbound 缓存的子目录前缀：`{cache_dir}/NCdesktop/outbound/{asset_id}/`。
const CACHE_SUBDIR: &str = "NCdesktop/outbound";

/// 最终落盘文件名上限：200 字节（含 `.md` 后缀与可能的 `_<asset_id8>` 截断后缀）。
///
/// 与 PRD §4.4 对齐；macOS HFS+/APFS 单段上限 255 字节，留出余量。
const MAX_FILENAME_BYTES: usize = 200;

/// `.md` 扩展名长度（含点）。
const MD_EXT_LEN: usize = 3; // ".md"

/// 截断标记后缀长度：`_` + asset_id 前 8 位 = 9 字节。
const TRUNC_SUFFIX_LEN: usize = 9;

/// stem-only sanitize 的最大字节数（保留 `.md` 与可能的截断后缀）。
const MAX_STEM_BYTES: usize = MAX_FILENAME_BYTES - MD_EXT_LEN - TRUNC_SUFFIX_LEN; // 188

/// Windows 保留字（无论扩展名），不区分大小写。
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 按 PRD §4.4 清洗 display_name → 磁盘安全的 outbound 文件名（不含 `.md` 后缀）。
///
/// 规则（按顺序）：
/// 1. `/` 与 `\` → `_`；
/// 2. 删除 U+0000–U+001F 与 U+007F；
/// 3. 保留 CJK / emoji / 其余 Unicode；
/// 4. 截断到 UTF-8 ≤ 200 字节并对齐字符边界；
/// 5. 若截断发生，追加 `_<asset_id 前 8 位>`；
/// 6. 末尾的 `.` / 空格连续序列 → 追加 `_`；
/// 7. 若 stem 大写匹配 Windows 保留字（CON/PRN/AUX/NUL/COMn/LPTn）→ 追加 `_`；
/// 8. 空 stem 兜底为 `_<asset_id 前 8 位>`，避免 dropzone 拿到空文件名。
pub fn sanitize_outbound_filename(display_name: &str, asset_id: &str) -> String {
    // 1 + 2 + 3：单遍替换 / 过滤
    let mut buf = String::with_capacity(display_name.len());
    for ch in display_name.chars() {
        match ch {
            '/' | '\\' => buf.push('_'),
            c if (c as u32) <= 0x1F || (c as u32) == 0x7F => {
                // 删除控制字符（不写入）
            }
            c => buf.push(c),
        }
    }

    // 4 + 5：UTF-8 字节截断 + 字符边界对齐
    let id8: String = asset_id.chars().take(8).collect();
    let truncated;
    if buf.len() > MAX_STEM_BYTES {
        // 找到 ≤ MAX_STEM_BYTES 的最大字符边界
        let mut cut = MAX_STEM_BYTES;
        while cut > 0 && !buf.is_char_boundary(cut) {
            cut -= 1;
        }
        buf.truncate(cut);
        truncated = true;
    } else {
        truncated = false;
    }
    if truncated {
        buf.push('_');
        buf.push_str(&id8);
    }

    // 6：尾随 `.` 或空格 → 追加 `_`
    let trailing_bad = buf.chars().last().map(|c| c == '.' || c == ' ').unwrap_or(false);
    if trailing_bad {
        buf.push('_');
    }

    // 7：Windows 保留字（大小写无关）→ 追加 `_`
    let upper = buf.to_ascii_uppercase();
    if WINDOWS_RESERVED.iter().any(|w| *w == upper.as_str()) {
        buf.push('_');
    }

    // 8：空兜底
    if buf.is_empty() {
        buf.push('_');
        buf.push_str(&id8);
    }

    buf
}

/// 从 root display_name（如 "新名.pdf" / "音频笔记.m4a" / "笔记"）派生 outbound
/// 落盘文件名（固定 `.md` 扩展名）。
///
/// 流程（修复 task_005 / task_009 暴露的 "新名.md.md" 缺陷）：
/// 1. 取最后一个 `.` 切分 stem 与原扩展名（首位 `.` 不算分隔，避免 ".env" → ""）；
/// 2. stem 走 PRD §4.4 sanitize（[`sanitize_outbound_filename`]）；
/// 3. 拼接 `stem + ".md"`，最终文件名 UTF-8 ≤ [`MAX_FILENAME_BYTES`] 字节。
///
/// **不要把 derivative.name（已含 `.md`）传进来**；调用方应传 root.name。
pub fn outbound_filename_from_root(root_name: &str, asset_id: &str) -> String {
    // 1) 剥离原扩展名：与 commands::asset::derivative_name_from_root 同源（task_004），
    //    但 sanitize 顺序倒过来 —— 先按 `.` 切，再 sanitize stem（避免 sanitize 把
    //    `.pdf` 中的合法字符乱动；同时确保 `.` 之后部分不进入 stem）。
    let stem_raw = match root_name.rfind('.') {
        Some(idx) if idx > 0 => &root_name[..idx],
        _ => root_name,
    };
    let stem = sanitize_outbound_filename(stem_raw, asset_id);
    format!("{stem}.md")
}

/// 计算单条 asset 的 outbound 缓存目录：`{cache}/NCdesktop/outbound/{asset_id}/`。
fn outbound_dir_for(cache_root: &Path, asset_id: &str) -> PathBuf {
    cache_root.join(CACHE_SUBDIR).join(asset_id)
}

/// 对外的纯路径解算助手（task_006 M6 删除级联引用）：根据 outbound root +
/// `asset_id` 拼出该 asset 的 outbound 缓存目录。
///
/// **outbound root 选型（2026-05-17 修订）**：从 `dirs_next::cache_dir()`
/// （`~/Library/Caches/`）改到 [`std::env::temp_dir()`]（macOS 上是
/// `/var/folders/.../T/`）。
///
/// 原因：macOS Finder 对 `~/Library/Caches/` 子树启用"hidden / system dir"
/// 沙盒过滤 —— `startDrag` 即便启动了 NSDraggingSession，目标 path 在
/// Caches 下时 Finder 直接拒收，用户连鼠标的"+"图标都看不到。
/// 用 user 自己手动从 Finder 拖 cache 文件到桌面也是同样被拒（验证于
/// 2026-05-17 release 拖拽诊断）。
///
/// `std::env::temp_dir()` 满足两个核心条件：
/// - **user-accessible**：Finder 能正常拖出
/// - **临时语义一致**：OS 重启后清理，与原 cache 语义对齐
///
/// **无 IO**：不创建 / 不删除，仅 path-only 拼接，供 [`commands::asset::delete_asset`]
/// 触发 `fs::remove_dir_all` 时复用，避免 M4 与 M6 之间出现路径口径漂移。
pub fn outbound_cache_dir_for(asset_id: &str) -> Option<PathBuf> {
    Some(outbound_dir_for(&outbound_root_dir(), asset_id))
}

/// outbound 落盘根目录。见 [`outbound_cache_dir_for`] 注释中关于
/// `cache_dir → temp_dir` 切换的设计依据。
fn outbound_root_dir() -> PathBuf {
    std::env::temp_dir()
}

/// 幂等清理 + 重建单条 asset 的 outbound 缓存目录。
///
/// 抽出成纯函数便于单测（不依赖 dirs_next 的全局缓存根）。
fn reset_outbound_dir(dir: &Path) -> io::Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::create_dir_all(dir)
}

/// 把 rendition 投影到 cache_path：先 hardlink，若跨卷则 copy fallback。
///
/// 与 [`crate::commands::dropzone::try_rename_or_copy_remove`] 同源模式：
/// 仅在 `ErrorKind::CrossesDevices` 时降级为 copy；其它错误（权限 / 磁盘满）直接抛。
fn link_or_copy_rendition(rendition: &Path, cache_path: &Path) -> io::Result<()> {
    match fs::hard_link(rendition, cache_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(rendition, cache_path).map(|_| ())
        }
        Err(e) => Err(e),
    }
}

/// 单 asset 的状态 + rendition 视图（从 `list_root_assets` 过滤而来）。
#[derive(Debug, Clone)]
struct AssetStateInput {
    asset_id: String,
    display_name: String,
    state: AssetState,
    rendition_path: Option<String>,
}

/// 校验所有目标 asset 必须为 done 态（AC-5）：
/// - 单选非 done → `StateNotDone`；
/// - 多选混合（≥1 非 done）→ `MixedStates`（offending 收齐全部非 done id）。
fn classify_state(inputs: &[AssetStateInput]) -> Result<(), OutboundError> {
    let offending: Vec<String> = inputs
        .iter()
        .filter(|x| x.state != AssetState::Done)
        .map(|x| x.asset_id.clone())
        .collect();

    if offending.is_empty() {
        return Ok(());
    }

    if inputs.len() == 1 {
        let only = &inputs[0];
        let state_label = serde_json::to_value(only.state)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", only.state).to_lowercase());
        return Err(OutboundError::StateNotDone {
            asset_id: only.asset_id.clone(),
            state: state_label,
            message: "该素材尚未完成转换，无法拖出".to_string(),
        });
    }

    Err(OutboundError::MixedStates {
        message: format!(
            "选中的 {} 个素材中有 {} 个尚未完成转换，请仅选择已完成项",
            inputs.len(),
            offending.len()
        ),
        offending,
    })
}

/// Tauri 命令：为多选 asset 准备 outbound .md 投影文件。
///
/// 详见模块文档；错误统一序列化为 JSON 字符串。
#[tauri::command]
pub async fn prepare_outbound_payload(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
) -> Result<Vec<OutboundEntry>, String> {
    log::info!(
        "[outbound] prepare_outbound_payload 调用：{} 个 asset_id（前 3 个：{:?}）",
        asset_ids.len(),
        asset_ids.iter().take(3).collect::<Vec<_>>(),
    );
    if asset_ids.is_empty() {
        return Err(OutboundError::EmptyInput {
            message: "未选择任何素材".to_string(),
        }
        .to_json());
    }

    // 1. 拿 join 行：通过 root asset 找到 project_id，再调 list_root_assets。
    //    多选若跨 project 同样支持（按 project_id 分组查询）。
    let inputs = collect_state_inputs(&database, &asset_ids).map_err(|e| {
        log::warn!("[outbound] collect_state_inputs 失败：{}", e.to_json());
        e.to_json()
    })?;

    // 2. 状态校验（AC-5：单选 / 多选 / 顺序与 input.md 对齐）
    classify_state(&inputs).map_err(|e| {
        log::warn!("[outbound] classify_state 拒绝：{}", e.to_json());
        e.to_json()
    })?;

    // 3. rendition 存在性（DB + 磁盘），任何一条缺失 → RenditionMissing
    for input in &inputs {
        match input.rendition_path.as_deref() {
            Some(p) if Path::new(p).exists() => {}
            _ => {
                return Err(OutboundError::RenditionMissing {
                    asset_id: input.asset_id.clone(),
                    message: "Markdown 衍生件文件丢失，请重试或重新转换".to_string(),
                }
                .to_json());
            }
        }
    }

    // 4. outbound 落盘根：用 std::env::temp_dir() 而非 dirs_next::cache_dir()
    //    （见 outbound_cache_dir_for 注释：macOS Finder 拒绝从 ~/Library/Caches 拖文件）。
    //    temp_dir 在 macOS 是 /var/folders/.../T/，user-accessible，Finder 能正常拖。
    let cache_root = outbound_root_dir();

    // 5. 投影到缓存目录
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let dir = outbound_dir_for(&cache_root, &input.asset_id);
        reset_outbound_dir(&dir).map_err(|e| {
            OutboundError::IoFailed {
                asset_id: Some(input.asset_id.clone()),
                detail: e.to_string(),
                message: "准备 outbound 缓存目录失败".to_string(),
            }
            .to_json()
        })?;

        // 修复（task_005/009）：input.display_name 是 root.name（如 "新名.pdf"）。
        // 必须先剥离原扩展名再 sanitize stem，最后拼 `.md`；否则会产出 "新名.pdf.md"
        // 或 "新名.md.md"（违反 PRD §S2 三处一致硬约束）。
        let file_name = outbound_filename_from_root(&input.display_name, &input.asset_id);
        let cache_path = dir.join(&file_name);

        let rendition_path = input
            .rendition_path
            .as_deref()
            .expect("rendition_path 已在上方校验非 None");
        link_or_copy_rendition(Path::new(rendition_path), &cache_path).map_err(|e| {
            OutboundError::IoFailed {
                asset_id: Some(input.asset_id.clone()),
                detail: e.to_string(),
                message: "落盘 outbound .md 失败".to_string(),
            }
            .to_json()
        })?;

        out.push(OutboundEntry {
            asset_id: input.asset_id,
            path: cache_path.to_string_lossy().to_string(),
            display_name: file_name,
        });
    }

    log::info!(
        "[outbound] prepare_outbound_payload 成功：{} 个 entry（首条 path={}）",
        out.len(),
        out.first().map(|e| e.path.as_str()).unwrap_or("<empty>"),
    );
    Ok(out)
}

/// 从 DB 解算每个 asset_id 的 state + rendition_path（不做磁盘 IO）。
///
/// 思路：通过 `resolve_asset_pair` 拿到 root.project_id，按 project 分组调
/// `list_root_assets` 拿到 join 行；命令层用 `Path::exists()` 派生 state，
/// 严格遵守 ADR-003 / 硬约束（不在 commands/ 拼 SQL）。
fn collect_state_inputs(
    database: &Database,
    asset_ids: &[String],
) -> Result<Vec<AssetStateInput>, OutboundError> {
    let conn = database.conn.lock().map_err(|e| OutboundError::IoFailed {
        asset_id: None,
        detail: format!("数据库锁获取失败: {e}"),
        message: "内部错误，请重试".to_string(),
    })?;

    // 第一步：把每个 asset_id 解算回 root（asset_id 可能是 derivative.id）。
    // 同时记录 project_id 用于批量 list_root_assets。
    let mut root_ids: Vec<String> = Vec::with_capacity(asset_ids.len());
    let mut project_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for aid in asset_ids {
        let (root, _deriv) = db::asset::resolve_asset_pair(&conn, aid).map_err(|_e| {
            OutboundError::AssetNotFound {
                asset_id: aid.clone(),
                message: "素材不存在".to_string(),
            }
        })?;
        project_ids.insert(root.project_id.clone());
        root_ids.push(root.id);
    }

    // 第二步：按 project 拉 join 行，建 root_id → (asset, join) 索引。
    let mut index: std::collections::HashMap<String, (crate::models::Asset, db::asset::AssetListJoinRow)> =
        std::collections::HashMap::new();
    for pid in &project_ids {
        let rows = db::asset::list_root_assets(&conn, pid).map_err(|e| OutboundError::IoFailed {
            asset_id: None,
            detail: e,
            message: "读取工作区状态失败".to_string(),
        })?;
        for (asset, join) in rows {
            index.insert(asset.id.clone(), (asset, join));
        }
    }

    // 第三步：按调用方传入顺序生成 AssetStateInput
    let mut out = Vec::with_capacity(root_ids.len());
    for root_id in root_ids {
        let (asset, join) = index.get(&root_id).ok_or_else(|| OutboundError::AssetNotFound {
            asset_id: root_id.clone(),
            message: "素材不存在".to_string(),
        })?;
        let rendition_path = join.rendition_path.clone();
        let rendition_exists = rendition_path
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
        out.push(AssetStateInput {
            asset_id: asset.id.clone(),
            display_name: asset.name.clone(),
            state,
            rendition_path,
        });
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ---------- AC-2：sanitize 规则 6 分支 ----------

    #[test]
    fn sanitize_replaces_slash_and_backslash() {
        let got = sanitize_outbound_filename("a/b\\c", "01234567abcd");
        assert_eq!(got, "a_b_c");
    }

    #[test]
    fn sanitize_strips_control_chars_and_del() {
        let raw = format!("hi{}{}{}there", '\u{0001}', '\u{001F}', '\u{007F}');
        let got = sanitize_outbound_filename(&raw, "01234567abcd");
        assert_eq!(got, "hithere");
    }

    #[test]
    fn sanitize_preserves_cjk_and_emoji() {
        let got = sanitize_outbound_filename("会议纪要 📝 Q3", "01234567abcd");
        assert_eq!(got, "会议纪要 📝 Q3");
    }

    #[test]
    fn sanitize_windows_reserved_appends_underscore() {
        let got_con = sanitize_outbound_filename("CON", "deadbeefcafe");
        assert_eq!(got_con, "CON_");
        // 小写也命中
        let got_lpt = sanitize_outbound_filename("lpt3", "deadbeefcafe");
        assert_eq!(got_lpt, "lpt3_");
    }

    #[test]
    fn sanitize_trailing_dot_or_space_appends_underscore() {
        let dotted = sanitize_outbound_filename("note.", "deadbeefcafe");
        assert_eq!(dotted, "note._");
        let spaced = sanitize_outbound_filename("note ", "deadbeefcafe");
        assert_eq!(spaced, "note _");
    }

    #[test]
    fn sanitize_truncates_long_utf8_and_appends_asset_id_suffix() {
        // 用 3 字节字符 "好"，重复 100 次 = 300 字节，超过 200 字节上限
        let raw: String = "好".repeat(100);
        let aid = "0123456789abcdef";
        let got = sanitize_outbound_filename(&raw, aid);
        // 后缀必须包含 _01234567（asset_id 前 8 位）
        assert!(got.ends_with("_01234567"), "实际值: {got}");
        // 截断后总字节数应 ≤ 200 + "_01234567"(9) = 209
        assert!(got.len() <= MAX_STEM_BYTES + 1 + 8, "实际字节数: {}", got.len());
        // 主体只包含 "好"，无半个字符
        let stem = got.trim_end_matches("_01234567");
        assert!(stem.chars().all(|c| c == '好'));
    }

    // ---------- FIX (task_005/009)：outbound_filename_from_root ----------

    #[test]
    fn outbound_filename_strips_original_ext_and_appends_md() {
        // 关键回归：root.name = "新名.pdf" → "新名.md"（不再是 "新名.pdf.md"）
        assert_eq!(
            outbound_filename_from_root("新名.pdf", "01234567abcd"),
            "新名.md"
        );
        assert_eq!(
            outbound_filename_from_root("音频笔记.m4a", "01234567abcd"),
            "音频笔记.md"
        );
        // 双扩展：只剥最后一个
        assert_eq!(
            outbound_filename_from_root("archive.tar.gz", "01234567abcd"),
            "archive.tar.md"
        );
    }

    #[test]
    fn outbound_filename_handles_no_ext() {
        assert_eq!(
            outbound_filename_from_root("笔记", "01234567abcd"),
            "笔记.md"
        );
        // 首位 `.` 不算扩展名分隔（".env" 不应退化为空 stem）
        assert_eq!(
            outbound_filename_from_root(".env", "01234567abcd"),
            ".env.md"
        );
    }

    #[test]
    fn outbound_filename_truncates_long_stem_with_asset_id_suffix() {
        // 用 3 字节字符 "好"，重复 100 次 = 300 字节，远超 stem 上限
        let raw: String = format!("{}.pdf", "好".repeat(100));
        let aid = "0123456789abcdef";
        let got = outbound_filename_from_root(&raw, aid);
        // 必须以 `_<id8>.md` 结尾
        assert!(got.ends_with("_01234567.md"), "实际值: {got}");
        // 总字节数 ≤ MAX_FILENAME_BYTES（200）
        assert!(
            got.len() <= MAX_FILENAME_BYTES,
            "文件名超过 200 字节: {} 字节",
            got.len()
        );
        // 主体 stem 仅含 "好"（截断必须对齐字符边界）
        let body = got
            .trim_end_matches(".md")
            .trim_end_matches("_01234567");
        assert!(body.chars().all(|c| c == '好'), "实际 body: {body}");
    }

    #[test]
    fn outbound_filename_sanitizes_slash_in_stem() {
        // root.name 含 `/` → sanitize 走 PRD §4.4 规则替换为 `_`
        assert_eq!(
            outbound_filename_from_root("a/b.pdf", "01234567abcd"),
            "a_b.md"
        );
    }

    // ---------- AC-5：状态校验（纯函数 classify_state） ----------

    fn input(id: &str, state: AssetState) -> AssetStateInput {
        AssetStateInput {
            asset_id: id.to_string(),
            display_name: id.to_string(),
            state,
            rendition_path: Some(format!("/tmp/{id}.md")),
        }
    }

    #[test]
    fn classify_state_single_non_done_returns_state_not_done() {
        let inputs = vec![input("a1", AssetState::Converting)];
        match classify_state(&inputs) {
            Err(OutboundError::StateNotDone { asset_id, state, .. }) => {
                assert_eq!(asset_id, "a1");
                assert_eq!(state, "converting");
            }
            other => panic!("期望 StateNotDone, 实际 {other:?}"),
        }
    }

    #[test]
    fn classify_state_mixed_returns_mixed_states_with_offending() {
        let inputs = vec![
            input("a1", AssetState::Done),
            input("a2", AssetState::Failed),
            input("a3", AssetState::Done),
            input("a4", AssetState::Offline),
        ];
        match classify_state(&inputs) {
            Err(OutboundError::MixedStates { offending, .. }) => {
                assert_eq!(offending, vec!["a2".to_string(), "a4".to_string()]);
            }
            other => panic!("期望 MixedStates, 实际 {other:?}"),
        }
    }

    #[test]
    fn classify_state_all_done_passes() {
        let inputs = vec![
            input("a1", AssetState::Done),
            input("a2", AssetState::Done),
        ];
        assert!(classify_state(&inputs).is_ok());
    }

    // ---------- AC-3 / AC-4：缓存目录幂等 + hardlink ----------

    #[test]
    fn reset_outbound_dir_is_idempotent_and_empties_existing_files() {
        let tmp = tempfile::tempdir().expect("创建临时目录失败");
        let dir = tmp.path().join("asset_xyz");
        // 第一次创建并写入污染文件
        reset_outbound_dir(&dir).expect("初次创建失败");
        fs::write(dir.join("stale.md"), b"old").expect("写入失败");
        assert!(dir.join("stale.md").exists());
        // 再次调用 → 旧文件应被清空
        reset_outbound_dir(&dir).expect("再次创建失败");
        assert!(dir.exists());
        assert!(!dir.join("stale.md").exists());
    }

    #[test]
    fn link_or_copy_rendition_happy_path_creates_file_with_same_content() {
        let tmp = tempfile::tempdir().expect("创建临时目录失败");
        let src = tmp.path().join("rendition.md");
        fs::write(&src, b"# hello\noutbound").expect("写入 source 失败");
        let dst_dir = tmp.path().join("out");
        fs::create_dir_all(&dst_dir).expect("创建目标目录失败");
        let dst = dst_dir.join("nice.md");

        link_or_copy_rendition(&src, &dst).expect("hardlink/copy 失败");
        let content = fs::read(&dst).expect("读取目标失败");
        assert_eq!(content, b"# hello\noutbound");
    }

    // ---------- AC-1：OutboundError JSON 序列化 ----------

    #[test]
    fn outbound_error_serializes_to_camel_case_json() {
        let err = OutboundError::StateNotDone {
            asset_id: "a1".to_string(),
            state: "converting".to_string(),
            message: "尚未完成".to_string(),
        };
        let s = err.to_json();
        assert!(s.contains("\"kind\":\"stateNotDone\""), "got: {s}");
        assert!(s.contains("\"assetId\":\"a1\""), "got: {s}");
    }
}
