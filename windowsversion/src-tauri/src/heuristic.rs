//! PR-2 task_007: 子目录直接归类后的本地启发式 mismatch 判定
//!
//! 算法（ADR-007）：
//! - 输入：导入文件名（base name）、目标 category_slug 下既有资产 tags 词袋
//! - 中英文 token：CJK 2-gram + ASCII word
//! - Jaccard 相似度 < 0.05 → 触发 toast
//! - 既有资产为空 → 跳过判定（返回 1.0）

use std::collections::HashSet;

const MISMATCH_THRESHOLD: f32 = 0.05;

/// 中英文混合 token 切分
/// - ASCII word：连续字母数字下划线
/// - CJK：每相邻 2 字符一个 bigram
pub fn tokenize(s: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let lower = s.to_lowercase();
    // ASCII word
    let mut buf = String::new();
    let mut cjk_buf: Vec<char> = Vec::new();

    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            buf.push(ch);
            // flush CJK
            flush_cjk(&mut cjk_buf, &mut out);
        } else if is_cjk(ch) {
            cjk_buf.push(ch);
            // flush ASCII
            if !buf.is_empty() {
                let t = buf.trim_matches('_').to_string();
                if !t.is_empty() {
                    out.insert(t);
                }
                buf.clear();
            }
        } else {
            // separator
            if !buf.is_empty() {
                let t = buf.trim_matches('_').to_string();
                if !t.is_empty() {
                    out.insert(t);
                }
                buf.clear();
            }
            flush_cjk(&mut cjk_buf, &mut out);
        }
    }
    if !buf.is_empty() {
        let trimmed = buf.trim_matches('_').to_string();
        if !trimmed.is_empty() {
            out.insert(trimmed);
        }
    }
    flush_cjk(&mut cjk_buf, &mut out);
    out
}

fn flush_cjk(buf: &mut Vec<char>, out: &mut HashSet<String>) {
    if buf.is_empty() {
        return;
    }
    if buf.len() == 1 {
        out.insert(buf[0].to_string());
    } else {
        for w in buf.windows(2) {
            out.insert(w.iter().collect());
        }
    }
    buf.clear();
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF)
}

/// Jaccard 相似度
pub fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// 计算 mismatch 分数；返回 Some(score) 表示触发提示，None 表示 OK 或无样本可判
pub fn compute_mismatch(file_name: &str, sibling_tags: &[String]) -> Option<f32> {
    if sibling_tags.is_empty() {
        return None;
    }
    let file_tokens = tokenize(file_name);
    if file_tokens.is_empty() {
        return None;
    }
    let mut sib_tokens = HashSet::new();
    for t in sibling_tags {
        sib_tokens.extend(tokenize(t));
    }
    if sib_tokens.is_empty() {
        return None;
    }
    let score = jaccard(&file_tokens, &sib_tokens);
    if score < MISMATCH_THRESHOLD {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_chinese_bigrams() {
        let toks = tokenize("Q3交付计划");
        assert!(toks.contains("q3"));
        assert!(toks.contains("交付"));
        assert!(toks.contains("付计"));
        assert!(toks.contains("计划"));
    }

    #[test]
    fn tokenize_english_words() {
        let toks = tokenize("meeting_notes_2026");
        assert!(toks.contains("meeting_notes_2026"));
    }

    #[test]
    fn tokenize_mixed() {
        let toks = tokenize("会议纪要_meeting");
        assert!(toks.contains("会议"));
        assert!(toks.contains("议纪"));
        assert!(toks.contains("纪要"));
        assert!(toks.contains("meeting"));
    }

    #[test]
    fn mismatch_no_siblings() {
        assert_eq!(compute_mismatch("anything.pdf", &[]), None);
    }

    #[test]
    fn mismatch_high_similarity_returns_none() {
        let sib = vec!["Q3交付".into(), "交付计划".into()];
        let r = compute_mismatch("Q3交付方案.pdf", &sib);
        assert!(r.is_none(), "高相似度不该触发，实际 {:?}", r);
    }

    #[test]
    fn mismatch_low_similarity_triggers() {
        let sib = vec!["健康".into(), "运动".into()];
        let r = compute_mismatch("Rust编程入门.pdf", &sib);
        assert!(r.is_some(), "低相似度应触发");
    }
}
