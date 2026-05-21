//! task_008：8 类失败错误码 + `classify_output` 失效四元判定函数。
//!
//! - 序列化为 `SCREAMING_SNAKE_CASE` 字符串（forward compat：DB 列、IPC 字段均为 TEXT）；
//! - 取代历史的 "exit==0 && stdout==''" 判成功逻辑；
//! - 判定顺序固定：runtime/timeout → empty → gibberish → no_structure → Ok。
//!
//! 与 ADR-007 / Debate Layer 2 共识一致。

use std::fmt;
use std::time::Duration;

/// `classify_output` 中"非 0 退出 → 超时"边界（90s 与 markitdown.rs 超时常量同步）。
const TIMEOUT_THRESHOLD: Duration = Duration::from_secs(90);

/// 可打印字符占比阈值：< 50% 判 gibberish。
const PRINTABLE_RATIO_THRESHOLD: f32 = 0.5;

/// 8 类失败错误码（与 Debate Layer 2 共识、task_001_architect §三 ADR-007 字符级一致）。
///
/// 序列化策略：
/// - `as_str()` / `Display` / DB / IPC 一律 `SCREAMING_SNAKE_CASE`（如 `"E_RUNTIME_MISSING"`）；
/// - **不**实现 `serde::Serialize`，避免 derived 序列化形态（如 PascalCase）混入；
///   调用方落库 / 上报均显式经 `as_str()`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    /// 嵌入运行时缺失（python / venv / markitdown module 不可用）。
    ERuntimeMissing,
    /// extras 中 ebooklib 未装（epub 路径专属）。
    EExtraMissingEpub,
    /// 扫描型 pdf 进入 markitdown 路由（应由 task_009 防呆，markitdown 不支持 OCR）。
    EScanPdfUnsupported,
    /// 音频文件误进 markitdown 路径（应由 task_010 入口 assert 拦截）。
    EAudioWrongRoute,
    /// 子进程退出 0 但 stdout trim 后为空（替换历史"空字符串=成功"误判）。
    EOutputEmpty,
    /// stdout 可打印字符占比 < 50%（控制符 / 二进制噪声 / 非 UTF-8 残留）。
    EOutputGibberish,
    /// stdout 非空且 UTF-8 干净，但既无 `#` 开头 markdown 标题也无段落（非空连续文本行）。
    EOutputNoStructure,
    /// 子进程被 90s 总超时强杀。
    ETimeout90s,
}

impl FailureCode {
    /// 返回 `SCREAMING_SNAKE_CASE` 字面串（DB 与 IPC 唯一形态）。
    pub fn as_str(&self) -> &'static str {
        match self {
            FailureCode::ERuntimeMissing => "E_RUNTIME_MISSING",
            FailureCode::EExtraMissingEpub => "E_EXTRA_MISSING_EPUB",
            FailureCode::EScanPdfUnsupported => "E_SCAN_PDF_UNSUPPORTED",
            FailureCode::EAudioWrongRoute => "E_AUDIO_WRONG_ROUTE",
            FailureCode::EOutputEmpty => "E_OUTPUT_EMPTY",
            FailureCode::EOutputGibberish => "E_OUTPUT_GIBBERISH",
            FailureCode::EOutputNoStructure => "E_OUTPUT_NO_STRUCTURE",
            FailureCode::ETimeout90s => "E_TIMEOUT_90S",
        }
    }
}

impl fmt::Display for FailureCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 失效四元判定（AC-4，按 input.md 字面顺序）：
///
/// 1. exit_code 非 0：elapsed ≥ 90s → `ETimeout90s`，否则 `ERuntimeMissing`（默认运行时错误）；
/// 2. exit_code == Some(0) 且 `stdout.trim().is_empty()` → `EOutputEmpty`；
/// 3. 可打印字符占比 < 50% → `EOutputGibberish`
///    （stdout 已是 `&str`，自然为 UTF-8；这里只判可打印比；
///     "非 UTF-8"在调用方 `from_utf8_lossy` 时已表现为大量 U+FFFD 替换字符 —— 仍归此分支）；
/// 4. 既无 markdown 标题（行首 `#`）又无段落（任一非空连续文本行） → `EOutputNoStructure`；
/// 5. 否则 `Ok(())`。
///
/// **设计说明**：本函数只做"输出形态"判定，不做错误分类语义猜测；运行时缺失 / extras 缺失 /
/// 扫描 pdf / 音频路由错误由调用方（scheduler / markitdown.rs 起 spawn 前）显式赋码，
/// 本函数仅在拿到 subprocess output 后兜底分类。
pub fn classify_output(
    stdout: &str,
    exit_code: Option<i32>,
    elapsed: Duration,
) -> Result<(), FailureCode> {
    // (1) 非 0 退出
    match exit_code {
        Some(0) => {}
        _ => {
            if elapsed >= TIMEOUT_THRESHOLD {
                return Err(FailureCode::ETimeout90s);
            }
            return Err(FailureCode::ERuntimeMissing);
        }
    }

    // (2) exit==0 且 stdout trim 后为空
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(FailureCode::EOutputEmpty);
    }

    // (3) 可打印字符占比 < 50%
    //   - `\n` / `\t` 视为可打印；
    //   - 其余 `char::is_control()` 视为不可打印；
    //   - U+FFFD（lossy UTF-8 替换符）属"普通"字符不被 control 命中，但若大量出现
    //     通常伴随其他 control / 二进制残留，仍能落入比例分支。
    let total: usize = stdout.chars().count();
    if total > 0 {
        let printable: usize = stdout
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .count();
        let ratio = printable as f32 / total as f32;
        if ratio < PRINTABLE_RATIO_THRESHOLD {
            return Err(FailureCode::EOutputGibberish);
        }
    }

    // (4) 既无 markdown 标题又无段落
    //   - "markdown 标题"：任一行 trim_start 后以 `#` 开头；
    //   - "段落"：任一非空连续文本行（这里就直接判：是否存在至少一行 trim 后非空且不全为
    //     标点 / 空白）—— 由于 (2) 已排除全空，剩余只需判"是否完全不存在可读非空行"。
    let mut has_heading = false;
    let mut has_paragraph = false;
    for line in stdout.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            has_heading = true;
        }
        // 任一行 trim 后非空（且不是纯 markdown 控制字符如单独 `#`）视为段落候选。
        // 这里宽松：要求至少含一个字母 / 数字 / CJK 字符。
        if t.chars().any(|c| c.is_alphanumeric()) {
            has_paragraph = true;
        }
        if has_heading && has_paragraph {
            break;
        }
    }
    if !has_heading && !has_paragraph {
        return Err(FailureCode::EOutputNoStructure);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AC-1：as_str / Display ---

    #[test]
    fn as_str_returns_screaming_snake_case() {
        assert_eq!(FailureCode::ERuntimeMissing.as_str(), "E_RUNTIME_MISSING");
        assert_eq!(FailureCode::EExtraMissingEpub.as_str(), "E_EXTRA_MISSING_EPUB");
        assert_eq!(FailureCode::EScanPdfUnsupported.as_str(), "E_SCAN_PDF_UNSUPPORTED");
        assert_eq!(FailureCode::EAudioWrongRoute.as_str(), "E_AUDIO_WRONG_ROUTE");
        assert_eq!(FailureCode::EOutputEmpty.as_str(), "E_OUTPUT_EMPTY");
        assert_eq!(FailureCode::EOutputGibberish.as_str(), "E_OUTPUT_GIBBERISH");
        assert_eq!(FailureCode::EOutputNoStructure.as_str(), "E_OUTPUT_NO_STRUCTURE");
        assert_eq!(FailureCode::ETimeout90s.as_str(), "E_TIMEOUT_90S");
    }

    #[test]
    fn display_matches_as_str() {
        let code = FailureCode::EOutputEmpty;
        assert_eq!(format!("{code}"), "E_OUTPUT_EMPTY");
        assert_eq!(format!("{}", FailureCode::ETimeout90s), "E_TIMEOUT_90S");
    }

    // --- AC-4：classify_output 每条分支 ---

    #[test]
    fn classify_nonzero_exit_under_90s_is_runtime_missing() {
        let r = classify_output("anything", Some(1), Duration::from_secs(3));
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    #[test]
    fn classify_nonzero_exit_at_90s_is_timeout() {
        // 边界：刚到 90s
        let r = classify_output("", Some(137), Duration::from_secs(90));
        assert_eq!(r, Err(FailureCode::ETimeout90s));
    }

    #[test]
    fn classify_none_exit_at_120s_is_timeout() {
        // exit_code == None（被 kill）+ 已超过阈值
        let r = classify_output("", None, Duration::from_secs(120));
        assert_eq!(r, Err(FailureCode::ETimeout90s));
    }

    #[test]
    fn classify_exit0_empty_stdout_is_output_empty() {
        let r = classify_output("", Some(0), Duration::from_secs(1));
        assert_eq!(r, Err(FailureCode::EOutputEmpty));
    }

    #[test]
    fn classify_exit0_whitespace_only_is_output_empty() {
        // trim 后为空 也算 empty
        let r = classify_output("   \n\t  \n", Some(0), Duration::from_secs(1));
        assert_eq!(r, Err(FailureCode::EOutputEmpty));
    }

    #[test]
    fn classify_pure_control_chars_is_gibberish() {
        // 全是 NUL / 控制字符 → 可打印占比 0%
        let s: String = (0u8..10).map(|b| b as char).collect();
        // 注：(0..10) 含 \t (0x09) 与 \n (0x0a)；剩余 8 个为 control。
        // total=10, printable=2 (\t,\n) → 0.2 < 0.5 → gibberish
        let r = classify_output(&s, Some(0), Duration::from_secs(1));
        assert_eq!(r, Err(FailureCode::EOutputGibberish));
    }

    #[test]
    fn classify_exactly_50_percent_printable_is_not_gibberish() {
        // 边界：占比 == 0.5（严格 <0.5 才判 gibberish）→ 不该判 gibberish。
        // 不过下游会被 (4) 判 NoStructure（因没字母/数字）。
        // 构造："a\x01" → total=2, printable=1（a），ratio=0.5
        let r = classify_output("a\x01", Some(0), Duration::from_secs(1));
        // 0.5 不 < 0.5 → 跳过 (3)；进入 (4)：有 `a` 是 alphanumeric → 段落存在 → Ok
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn classify_lossy_replacement_dominant_is_gibberish() {
        // 模拟 from_utf8_lossy 大量 U+FFFD + control mix（实际比例 < 0.5）。
        let mut s = String::new();
        for _ in 0..3 {
            s.push('\u{FFFD}'); // 视为可打印（非 control）
        }
        for _ in 0..10 {
            s.push('\x01'); // control
        }
        // total=13, printable=3, ratio≈0.23 → gibberish
        let r = classify_output(&s, Some(0), Duration::from_secs(1));
        assert_eq!(r, Err(FailureCode::EOutputGibberish));
    }

    #[test]
    fn classify_no_heading_no_paragraph_is_no_structure() {
        // 只含标点/符号，无字母数字，无 `#` 标题
        let r = classify_output("---\n***\n", Some(0), Duration::from_secs(1));
        assert_eq!(r, Err(FailureCode::EOutputNoStructure));
    }

    #[test]
    fn classify_heading_only_is_ok() {
        let r = classify_output("# Title\n", Some(0), Duration::from_secs(1));
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn classify_paragraph_only_is_ok() {
        let r = classify_output("Hello world.\n这是一段中文。", Some(0), Duration::from_secs(1));
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn classify_full_markdown_is_ok() {
        let md = "# 报告\n\n这是 markitdown 的输出段落。\n\n## 小节\n\n更多内容。";
        let r = classify_output(md, Some(0), Duration::from_secs(1));
        assert_eq!(r, Ok(()));
    }
}
