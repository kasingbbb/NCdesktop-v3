//! 文件名安全化：替换非法字符、压缩连续空格、截断长度。
//! 保留 CJK / emoji / 下划线 / 连字符。

const MAX_STEM_LEN: usize = 120;

/// 清洗 stem，得到磁盘安全文件名片段（不含扩展名）。
pub fn sanitize_stem(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        let replaced = match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        };
        if replaced == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            out.push(' ');
        } else {
            prev_space = false;
            out.push(replaced);
        }
    }
    let trimmed = out.trim().trim_matches('.').to_string();
    let cleaned = if trimmed.is_empty() { "untitled".to_string() } else { trimmed };
    truncate_chars(&cleaned, MAX_STEM_LEN)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_illegal_chars() {
        assert_eq!(sanitize_stem("a/b:c*d?e"), "a_b_c_d_e");
    }

    #[test]
    fn collapses_multiple_spaces() {
        assert_eq!(sanitize_stem("hello   world"), "hello world");
    }

    #[test]
    fn strips_control_bytes() {
        let raw = format!("{}{}note", '\u{0001}', '\u{0002}');
        assert_eq!(sanitize_stem(&raw), "__note");
    }

    #[test]
    fn preserves_cjk_and_emoji() {
        let raw = "我的 笔记 📝";
        assert_eq!(sanitize_stem(raw), "我的 笔记 📝");
    }

    #[test]
    fn empty_becomes_untitled() {
        assert_eq!(sanitize_stem("   "), "untitled");
        assert_eq!(sanitize_stem(""), "untitled");
    }

    #[test]
    fn truncates_long_names() {
        let raw = "a".repeat(200);
        let got = sanitize_stem(&raw);
        assert_eq!(got.chars().count(), MAX_STEM_LEN);
    }
}
