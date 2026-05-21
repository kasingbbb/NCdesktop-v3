//! 转换尝试数据结构与工具
//!
//! 提供 `ConversionAttempt` 纯数据结构、文件 SHA-256 流式哈希、
//! 以及 stderr / 错误字符串归类工具。**不**引入新 trait（见 ADR-005）。

use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 单次转换尝试的元信息记录（纯数据 + serde）
///
/// 设计要点：
/// - 不持有业务行为；调用方负责构造与持久化
/// - serde 字段名采用 camelCase 以兼容前端/JSON 约定
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionAttempt {
    /// 转换器名称（如 "markitdown"、"pdf-text"）
    pub converter_name: String,
    /// 转换器版本（语义版本或 commit 短哈希）
    pub converter_version: String,
    /// 源文件 MIME 类型
    pub source_mime: String,
    /// 源文件 SHA-256（hex 小写）
    pub source_hash: String,
    /// 质量等级（0=失败 / 1=基础 / 2=结构化 / 3=高保真，由调用方定义）
    pub quality_level: i32,
    /// 是否使用了 fallback 路径
    pub fallback_used: bool,
    /// 错误归类（None 表示成功）
    pub error_class: Option<String>,
    /// 转换耗时（毫秒）
    pub conversion_ms: u64,
    /// 转换完成时间（RFC3339 字符串）
    pub converted_at: String,
}

/// 流式计算文件的 SHA-256 哈希，输出 hex 小写。
///
/// 以 8KB 块循环读取，避免一次性载入大文件。对同一字节序列的结果
/// 与 `scheduler::compute_sha256` 一致（同一 sha2 实现 + 同一 hex 小写格式化）。
pub fn file_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 将 stderr / 错误字符串归类为稳定的 error_class。
///
/// 匹配规则：大小写不敏感的子串包含。**顺序敏感** —— 更具体的规则在前。
/// 例如 `markitdown_not_installed` 必须先于 `python_unavailable` 检测，
/// 因为两者都可能包含 "No module" 之类的特征。
pub fn classify_error(stderr_or_err: &str) -> &'static str {
    let s = stderr_or_err.to_lowercase();

    // 1. 文件不存在（仅在不涉及 python 时归此类，python 缺失另行处理）
    if (s.contains("filenotfounderror") || s.contains("no such file"))
        && !s.contains("python")
    {
        return "file_not_found";
    }

    // 2. 权限拒绝
    if s.contains("permissionerror") || s.contains("permission denied") {
        return "permission_denied";
    }

    // 3. 不支持的格式
    if s.contains("unsupportedformatexception") || s.contains("not supported") {
        return "unsupported_format";
    }

    // 4. markitdown 模块未安装（更具体，先于 python 缺失）
    if s.contains("modulenotfounderror") || s.contains("no module named") {
        return "markitdown_not_installed";
    }

    // 5. python 命令不可用
    if s.contains("python: command not found")
        || s.contains("command not found: python")
        || (s.contains("no such file or directory") && s.contains("python"))
    {
        return "python_unavailable";
    }

    // 6. 输出为空
    if s.contains("输出为空") || s.contains("empty output") || s.contains("stdout was empty") {
        return "empty_output";
    }

    // 7. 超时
    if s.contains("timed out") || s.contains("timeout") {
        return "timeout";
    }

    // 8. 兜底
    "conversion_error"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// AC-4：file_sha256 对 "hello world" 字节序列的哈希应与公开值一致
    #[test]
    fn file_sha256_matches_known_vector() {
        let dir = std::env::temp_dir();
        let path = dir.join("ncdesktop_conversion_sha256_test.txt");
        {
            let mut f = std::fs::File::create(&path).expect("create temp file");
            f.write_all(b"hello world").expect("write");
        }
        let hash = file_sha256(&path).expect("hash file");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// AC-4：file_sha256 对大于 8KB 的输入也应正确（覆盖多块循环）
    #[test]
    fn file_sha256_handles_multi_block() {
        let dir = std::env::temp_dir();
        let path = dir.join("ncdesktop_conversion_sha256_big.bin");
        // 写入 20000 字节的 0x41 ('A')
        let data = vec![0x41u8; 20000];
        {
            let mut f = std::fs::File::create(&path).expect("create temp file");
            f.write_all(&data).expect("write");
        }
        let hash = file_sha256(&path).expect("hash");
        // 用同一库直接算一遍作为对照（确保流式与一次性结果一致）
        let mut h = Sha256::new();
        h.update(&data);
        let expected = format!("{:x}", h.finalize());
        assert_eq!(hash, expected);
        let _ = std::fs::remove_file(&path);
    }

    /// AC-4：classify_error 对 8 个典型 stderr 片段全部正确归类
    #[test]
    fn classify_error_covers_eight_classes() {
        assert_eq!(
            classify_error("FileNotFoundError: [Errno 2] No such file or directory: '/tmp/x'"),
            "file_not_found"
        );
        assert_eq!(
            classify_error("PermissionError: [Errno 13] Permission denied"),
            "permission_denied"
        );
        assert_eq!(
            classify_error("UnsupportedFormatException: format not supported"),
            "unsupported_format"
        );
        assert_eq!(
            classify_error("ModuleNotFoundError: No module named 'markitdown'"),
            "markitdown_not_installed"
        );
        assert_eq!(
            classify_error("zsh: python: command not found"),
            "python_unavailable"
        );
        assert_eq!(
            classify_error("Markitdown 转换失败：输出为空"),
            "empty_output"
        );
        assert_eq!(
            classify_error("subprocess timed out after 60s"),
            "timeout"
        );
        assert_eq!(
            classify_error("some random failure"),
            "conversion_error"
        );
    }

    /// AC-4：markitdown_not_installed 必须先于 python_unavailable 匹配
    #[test]
    fn classify_error_priority_markitdown_over_python() {
        // 同时含 "No module" 和 "python" 时应归 markitdown_not_installed
        let s = "ModuleNotFoundError: No module named 'markitdown' (python subprocess)";
        assert_eq!(classify_error(s), "markitdown_not_installed");
    }

    /// AC-1：ConversionAttempt 序列化为 camelCase（converter_name → converterName）
    #[test]
    fn conversion_attempt_serializes_camel_case() {
        let attempt = ConversionAttempt {
            converter_name: "markitdown".to_string(),
            converter_version: "0.0.1".to_string(),
            source_mime: "application/pdf".to_string(),
            source_hash: "abc123".to_string(),
            quality_level: 2,
            fallback_used: false,
            error_class: None,
            conversion_ms: 1234,
            converted_at: "2026-05-12T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&attempt).expect("serialize");
        assert!(json.contains("\"converterName\":\"markitdown\""));
        assert!(json.contains("\"converterVersion\":\"0.0.1\""));
        assert!(json.contains("\"sourceMime\":\"application/pdf\""));
        assert!(json.contains("\"sourceHash\":\"abc123\""));
        assert!(json.contains("\"qualityLevel\":2"));
        assert!(json.contains("\"fallbackUsed\":false"));
        assert!(json.contains("\"errorClass\":null"));
        assert!(json.contains("\"conversionMs\":1234"));
        assert!(json.contains("\"convertedAt\":\"2026-05-12T10:00:00Z\""));
        // 确保没有 snake_case 字段漏出
        assert!(!json.contains("converter_name"));
    }

    /// AC-1：ConversionAttempt 可往返反序列化
    #[test]
    fn conversion_attempt_roundtrip() {
        let attempt = ConversionAttempt {
            converter_name: "pdf-text".to_string(),
            converter_version: "1.0".to_string(),
            source_mime: "application/pdf".to_string(),
            source_hash: "deadbeef".to_string(),
            quality_level: 1,
            fallback_used: true,
            error_class: Some("timeout".to_string()),
            conversion_ms: 60000,
            converted_at: "2026-05-12T11:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&attempt).expect("serialize");
        let back: ConversionAttempt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.converter_name, "pdf-text");
        assert_eq!(back.fallback_used, true);
        assert_eq!(back.error_class.as_deref(), Some("timeout"));
        assert_eq!(back.conversion_ms, 60000);
    }
}
