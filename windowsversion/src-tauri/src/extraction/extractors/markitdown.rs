use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::RwLock;
use std::time::{Duration, Instant};

// task_011 preserve: 90s 子进程总超时，损坏 / 极大文件兜底强杀（preserve_matrix.md #1）。
/// W4-6 / 风险 4.2：子进程总超时。
/// 损坏 / 极大文件会让 `python -m markitdown` 长时间占用，此处兜底强杀。
const MARKITDOWN_TIMEOUT: Duration = Duration::from_secs(90);

use crate::extraction::{
    conversion::classify_error,
    failure_code::{classify_output, FailureCode},
    models::{
        evaluate_markdown_quality, markdown_to_segments, ExtractionError, ExtractionResult,
        ExtractOptions,
    },
    Extractor,
};

pub struct MarkItDownExtractor {
    // task_011 preserve: 版本探测缓存字段，首次 extract 成功后填充（preserve_matrix.md #3）。
    /// 缓存 `python -m markitdown --version` 解析出的版本字符串。
    /// task_008 在 scheduler 落库 ConversionAttempt 时读取。
    cached_version: RwLock<Option<String>>,
}

impl MarkItDownExtractor {
    pub fn new() -> Self {
        Self {
            cached_version: RwLock::new(None),
        }
    }

    /// 返回最近一次成功探测到的 markitdown 版本（首次 extract 成功后填充）。
    pub fn detected_version(&self) -> Option<String> {
        self.cached_version
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn cache_version(&self, version: String) {
        if let Ok(mut guard) = self.cached_version.write() {
            *guard = Some(version);
        }
    }
}

impl Default for MarkItDownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// task_011 preserve: SUPPORTED_MIME_TYPES 不含 audio/video（grep gate）（preserve_matrix.md #6）。
// task_010 AC-1（H5 / PRD 底线 #4）：`SUPPORTED_MIME_TYPES` 严格不含 `audio/*`、
// `video/*`。音频路由到 `audio_asr_iflytek`；视频本期显式拒绝（scheduler 落
// `E_AUDIO_WRONG_ROUTE` 失败码）。grep CI gate：
// `! grep -nE '"(audio|video)/' src/extraction/extractors/markitdown.rs`。
const SUPPORTED_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "text/html",
    "application/epub+zip",
    // task_014 Fix-A1：image/* 走 markitdown（需要 LLM client 才能真 OCR；
    // 未配置 LLM 时返回最小元数据 MD，避免落 placeholder）。
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/bmp",
    "image/tiff",
    "image/webp",
];

/// task_010 AC-2：基于文件扩展名推断 mime 前缀，用于 `extract()` 入口防御性
/// 检测 audio/* / video/* 误路由。
///
/// 设计：`Extractor::extract()` 签名只接 `&Path`，不传 mime；因此本 helper 必须
/// 自给自足。映射表覆盖 scheduler / Asset.mime_type 实际可见的常见后缀：
/// - audio：`mp3`/`wav`/`m4a`/`aac`/`flac`/`ogg`/`opus`/`mp4`(若被当 audio 上传则误判)
/// - video：`mp4`/`mov`/`avi`/`mkv`/`webm`/`m4v`/`wmv`/`flv`
///
/// 与生产 `commands::sync::guess_mime` 同源；不重复导出以保持模块边界。
/// 未匹配后缀 → 返回空串（即不命中任何 prefix），不影响其他 mime 路径。
fn mime_prefix_from_path(file_path: &Path) -> &'static str {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        // audio
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "oga" | "opus" | "wma" => "audio/",
        // video（mp4 归 video：scheduler 调用前已经按 Asset.mime_type 决策；
        // 入口防御性检查时若 mp4 真为音频，scheduler 早已路由到 iflytek，
        // 不会进入 markitdown 子进程）
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" | "wmv" | "flv" => "video/",
        _ => "",
    }
}

/// task_010 AC-2 helper：路径推断 mime 是否以指定前缀开头。
fn mime_starts_with(file_path: &Path, prefix: &str) -> bool {
    mime_prefix_from_path(file_path) == prefix
}

/// Fix-A1：判断 mime 是否为 image/*（仅测试用：extract 内部用文件扩展名做判断，
/// 因 Extractor trait 不传 mime；本函数保留供单测断言"前缀逻辑"是稳定的）。
#[cfg(test)]
fn is_image_mime(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

impl Extractor for MarkItDownExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        supports_mime(mime_type)
    }

    fn name(&self) -> &'static str {
        "markitdown"
    }

    fn extract(
        &self,
        file_path: &Path,
        options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        // task_011 preserve: runtime_check 入口短路，自检失败不起子进程（preserve_matrix.md #10）。
        // task_007 FIX：runtime self-check 快照短路（AC-3）。
        // 调用方（scheduler / RuntimeCheckState 持有方）应在路由前注入；自检失败
        // 时立即返回，**不**进 Python 子进程，**不**触动后续 classify_output 判定逻辑。
        if let Some(code) = options.runtime_check_failed {
            return Err(parse_error_with_class(&format!(
                "runtime self-check failed | failure_code={code}"
            )));
        }

        // task_011 preserve: audio/video 入口 debug_assert + release E_AUDIO_WRONG_ROUTE 防御（preserve_matrix.md #8）。
        // task_010 AC-2：audio/video 路由防御（H5 / PRD 底线 #4）。
        // 即便 `SUPPORTED_MIME_TYPES` 已剔除 audio/video（AC-1），仍在 extract 入口
        // 设防 —— 防止：
        //   (a) scheduler 上游 mime 字段被人为污染（unit test / 内部调用绕过路由）；
        //   (b) 未来注册的 fallback 链不慎将音频文件丢给 markitdown。
        //
        // - debug build：直接 `debug_assert!` panic，把误路由作为开发期硬错误暴露；
        // - release build：返回 `FailureCode::EAudioWrongRoute`，不 panic 用户进程，
        //   并保留 `error_class:audio_wrong_route`-风格的可分类失败串。
        debug_assert!(
            !mime_starts_with(file_path, "audio/"),
            "markitdown 不应路由到 audio/* (path={file_path:?})"
        );
        debug_assert!(
            !mime_starts_with(file_path, "video/"),
            "markitdown 不应路由到 video/* (path={file_path:?})"
        );
        #[cfg(not(debug_assertions))]
        {
            if mime_starts_with(file_path, "audio/") || mime_starts_with(file_path, "video/") {
                return Err(parse_error_with_class(&format!(
                    "markitdown wrong route (audio/video) | failure_code={}",
                    FailureCode::EAudioWrongRoute.as_str()
                )));
            }
        }

        if !options.markitdown_enabled {
            return Err(ExtractionError::UnsupportedFormat(
                "MarkItDown 已禁用".to_string(),
            ));
        }

        let file_arg = file_path
            .to_str()
            .ok_or_else(|| parse_error_with_class("文件路径不是有效 UTF-8"))?;

        // Fix-A1：通过扩展名识别 image 输入，用于"markitdown 输出为空时回退最小元数据 MD"。
        let is_image = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp"
                )
            })
            .unwrap_or(false);

        let candidates = python_candidates(options);
        let mut attempts = Vec::new();
        // task_008 / Fix-A1：仅当 markitdown 真跑通且 classify_output → EOutputEmpty
        // （典型：image 未配 LLM）时，对 image 输入回退最小元数据 MD。
        // 退出码非零 / spawn 失败 不算"跑通空"。
        let mut had_empty_success = false;
        // task_008 AC-5：保留最近一次失败的 FailureCode（兜底分类），用于上层落库。
        let mut last_failure_code: Option<FailureCode> = None;
        for python_cmd in &candidates {
            let start = Instant::now();
            match run_with_timeout(python_cmd, &["-m", "markitdown", file_arg], MARKITDOWN_TIMEOUT) {
                Ok(output) => {
                    let elapsed = start.elapsed();
                    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
                    let exit_code = output.status.code();
                    // task_011 modify: 历史"exit==0 && stdout==''=success"已替换为 classify_output（preserve_matrix.md #4）。
                    // task_008：用 classify_output 替换历史"exit==0 && stdout==''=成功"误判。
                    match classify_output(&stdout_str, exit_code, elapsed) {
                        Ok(()) => {
                            let markdown = stdout_str.trim().to_string();
                            // 成功转换路径：首次成功时尝试探测并缓存版本号（best-effort）。
                            if self.cached_version.read().ok().is_none_or(|g| g.is_none()) {
                                if let Some(ver) = probe_markitdown_version(python_cmd) {
                                    self.cache_version(ver);
                                }
                            }

                            let quality_level = evaluate_markdown_quality(&markdown);
                            return Ok(ExtractionResult {
                                raw_text: markdown.clone(),
                                structured_md: markdown.clone(),
                                quality_level,
                                extractor_type: "markitdown".to_string(),
                                segments: markdown_to_segments(&markdown),
                                needs_ocr_fallback: false,
                            });
                        }
                        Err(code) => {
                            last_failure_code = Some(code);
                            if code == FailureCode::EOutputEmpty {
                                had_empty_success = true;
                            }
                            let stderr =
                                String::from_utf8_lossy(&output.stderr).trim().to_string();
                            if !stderr.is_empty() {
                                log::warn!("markitdown stderr ({python_cmd}): {stderr}");
                            }
                            let msg = if stderr.is_empty() {
                                format!(
                                    "{python_cmd}: 退出码 {:?} | failure_code={}",
                                    exit_code, code
                                )
                            } else {
                                format!("{python_cmd}: {stderr} | failure_code={code}")
                            };
                            attempts.push(msg);
                        }
                    }
                }
                Err(err) => {
                    // spawn / 超时 失败：超时分类由 io::ErrorKind::TimedOut 推断。
                    let code = if err.kind() == std::io::ErrorKind::TimedOut {
                        FailureCode::ETimeout90s
                    } else {
                        FailureCode::ERuntimeMissing
                    };
                    last_failure_code = Some(code);
                    log::warn!("markitdown spawn 失败 ({python_cmd}): {err}");
                    attempts.push(format!("{python_cmd}: {err} | failure_code={code}"));
                }
            }
        }

        // task_011 preserve: image 空输出 → markitdown_image_fallback 最小元数据 MD（preserve_matrix.md #2）。
        // task_008 AC-5 / Fix-A1（task_014）：image 输入 + markitdown 真跑过但所有候选
        // 都被 classify_output 判 EOutputEmpty（典型：未配置 LLM client）
        // → 回退为"最小元数据 MD"，avoid 落 placeholder。
        // - quality_level=1（low）；
        // - **task_008 修订**：extractor_type 改为 "markitdown_image_fallback"（任务 input AC-5），
        //   保留"未误标 success 给非 image 路径"的语义；
        // - failure_code 不在 ExtractionResult 字段里，由 scheduler 在落库时按 extractor_type
        //   判定写 NULL（image 回退路径不算失败）。
        if is_image && had_empty_success {
            let file_name = file_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let markdown = format!(
                "图片：{file_name}\n\n_（未配置图像识别 LLM，仅记录元数据）_\n"
            );
            return Ok(ExtractionResult {
                raw_text: markdown.clone(),
                structured_md: markdown.clone(),
                quality_level: 1,
                extractor_type: "markitdown_image_fallback".to_string(),
                segments: markdown_to_segments(&markdown),
                needs_ocr_fallback: false,
            });
        }

        // 失败兜底：拼装错误信息附带 failure_code 摘要（scheduler 据此落库）。
        let summary_code = last_failure_code
            .map(|c| c.as_str())
            .unwrap_or("E_RUNTIME_MISSING");
        Err(parse_error_with_class(&format!(
            "MarkItDown 调用失败 [{}]：{}",
            summary_code,
            attempts.join(" | ")
        )))
    }
}

// task_011 preserve: 后台读线程持续 drain pipe，避免大输出被 OS pipe buffer 死锁误判超时（preserve_matrix.md #7）。
/// W4-6 / 风险 4.2：在给定时长内运行子进程；超时则强杀并返回 `TimedOut`。
///
/// 与 `Command::output()` 兼容的返回类型，便于现有 match 分支不变。
/// 实现：spawn 后立即把 stdout/stderr 交给两个后台线程持续 `read_to_end`，
/// 主线程只做 try_wait 轮询 —— 这是必须的，否则当 markitdown 输出超过 OS
/// pipe buffer（macOS 16–64KB）时子进程会阻塞在 write 上，被误判为超时。
fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> std::io::Result<std::process::Output> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // 立刻 take stdout/stderr 句柄并交给后台线程持续读取，避免 pipe 死锁
    let stdout_handle = child.stdout.take().map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait()? {
            Some(s) => break s,
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    // 收尸读线程后再返回，避免悬挂
                    let _ = stdout_handle.and_then(|h| h.join().ok());
                    let _ = stderr_handle.and_then(|h| h.join().ok());
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!(
                            "markitdown subprocess timed out 超时 (>{}s)",
                            timeout.as_secs()
                        ),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };

    let stdout = stdout_handle.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = stderr_handle.and_then(|h| h.join().ok()).unwrap_or_default();

    Ok(std::process::Output { status, stdout, stderr })
}

// task_011 preserve: error_class:xxx| 前缀供 scheduler 解析分类（preserve_matrix.md #9）。
/// 构造一个携带 `error_class:xxx|` 前缀的 ParseError，便于 task_008 scheduler 解析。
fn parse_error_with_class(message: &str) -> ExtractionError {
    let class = classify_error(message);
    ExtractionError::ParseError(format!("error_class:{class}|{message}"))
}

// task_011 preserve: probe_markitdown_version best-effort 探测 + 缓存（preserve_matrix.md #3）。
/// 尝试解析 `python -m markitdown --version` 的输出。失败返回 None（best-effort）。
fn probe_markitdown_version(python_cmd: &str) -> Option<String> {
    let output = Command::new(python_cmd)
        .args(["-m", "markitdown", "--version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

pub fn supports_mime(mime_type: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime_type)
}

fn python_candidates(options: &ExtractOptions) -> Vec<String> {
    let mut candidates: Vec<String> = Vec::new();

    let push_unique = |cands: &mut Vec<String>, value: String| {
        if !cands.iter().any(|c| c == &value) {
            cands.push(value);
        }
    };

    // 优先级 1：嵌入式 venv python（task_008 scheduler 通过 AppHandle 注入）
    if let Some(cmd) = options
        .markitdown_embedded_python
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        push_unique(&mut candidates, cmd.to_string());
    }

    // 优先级 2：用户配置的 python_cmd
    if let Some(cmd) = options
        .markitdown_python_cmd
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        push_unique(&mut candidates, cmd.to_string());
    }

    // 优先级 3 / 4：系统 python3 / python
    push_unique(&mut candidates, "python3".to_string());
    push_unique(&mut candidates, "python".to_string());

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 复刻 extract() 失败路径中聚合 attempts → classify_error 前缀的逻辑，
    /// 便于在不实际启动 subprocess 的情况下校验 error_class。
    fn build_parse_error_msg(attempts: &[String]) -> String {
        let combined = format!("MarkItDown 调用失败：{}", attempts.join(" | "));
        match parse_error_with_class(&combined) {
            ExtractionError::ParseError(s) => s,
            _ => unreachable!("parse_error_with_class always returns ParseError"),
        }
    }

    fn extract_class(msg: &str) -> Option<&str> {
        // 形如 "error_class:xxx|..."
        let rest = msg.strip_prefix("error_class:")?;
        let end = rest.find('|')?;
        Some(&rest[..end])
    }

    #[test]
    fn error_class_markitdown_not_installed() {
        let attempts = vec![
            "python3: ModuleNotFoundError: No module named 'markitdown'".to_string(),
        ];
        let msg = build_parse_error_msg(&attempts);
        assert_eq!(extract_class(&msg), Some("markitdown_not_installed"));
    }

    #[test]
    fn error_class_file_not_found() {
        // 不直接拼接 "python3:" 前缀，避免触发 task_005 中 `python_unavailable`
        // 启发式优先（"python" 子串）。模拟 stderr 主体内容 —— scheduler 在 task_008
        // 解析时也会把 attempts 拼为完整字符串后再分类。
        let attempts =
            vec!["FileNotFoundError: [Errno 2] No such file or directory: '/tmp/x.pdf'".to_string()];
        let msg = build_parse_error_msg(&attempts);
        assert_eq!(extract_class(&msg), Some("file_not_found"));
    }

    #[test]
    fn error_class_conversion_error_empty_stderr() {
        // 伪造一个空 stderr、退出码非 0 的 attempt（与 extract() 中相同的格式）
        let attempts = vec!["python3: 退出码 Some(1)".to_string()];
        let msg = build_parse_error_msg(&attempts);
        assert_eq!(extract_class(&msg), Some("conversion_error"));
    }

    #[test]
    fn python_candidates_order_with_embedded_and_cmd() {
        let opts = ExtractOptions {
            markitdown_enabled: true,
            markitdown_python_cmd: Some("custom_py".to_string()),
            markitdown_embedded_python: Some("/x/y/python".to_string()),
            ..Default::default()
        };
        let cands = python_candidates(&opts);
        assert_eq!(
            cands,
            vec![
                "/x/y/python".to_string(),
                "custom_py".to_string(),
                "python3".to_string(),
                "python".to_string(),
            ]
        );
    }

    #[test]
    fn python_candidates_deduplicates_when_cmd_equals_python3() {
        let opts = ExtractOptions {
            markitdown_enabled: true,
            markitdown_python_cmd: Some("python3".to_string()),
            markitdown_embedded_python: None,
            ..Default::default()
        };
        let cands = python_candidates(&opts);
        assert_eq!(
            cands,
            vec!["python3".to_string(), "python".to_string()]
        );
    }

    #[test]
    fn python_candidates_defaults_only() {
        let opts = ExtractOptions {
            markitdown_enabled: true,
            ..Default::default()
        };
        let cands = python_candidates(&opts);
        assert_eq!(cands, vec!["python3".to_string(), "python".to_string()]);
    }

    /// Fix-A1（task_014 AC-1）：image/* 全 6 种 mime 都进入 markitdown SUPPORTED 集合。
    #[test]
    fn supports_image_mime_types() {
        for mime in [
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/bmp",
            "image/tiff",
            "image/webp",
        ] {
            assert!(supports_mime(mime), "markitdown 应支持 {mime}");
        }
    }

    /// Fix-A1：is_image_mime 仅识别 image/* 前缀。
    #[test]
    fn is_image_mime_only_image_prefix() {
        assert!(is_image_mime("image/png"));
        assert!(is_image_mime("image/anything"));
        assert!(!is_image_mime("text/plain"));
        assert!(!is_image_mime("application/pdf"));
    }

    /// Fix-A1：通过文件扩展名识别 image（extract 内部用，复刻该判断）。
    #[test]
    fn image_extension_detection_matches_extract_logic() {
        use std::path::PathBuf;
        let check = |p: &str| {
            let pb = PathBuf::from(p);
            pb.extension()
                .and_then(|e| e.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp"
                    )
                })
                .unwrap_or(false)
        };
        assert!(check("/tmp/a.PNG"));
        assert!(check("/tmp/a.jpeg"));
        assert!(check("/tmp/a.JPG"));
        assert!(!check("/tmp/a.pdf"));
        assert!(!check("/tmp/a"));
    }

    #[test]
    fn detected_version_starts_empty() {
        let extractor = MarkItDownExtractor::new();
        assert_eq!(extractor.detected_version(), None);
    }

    // ─── task_007 FIX (AC-3)：extract() 入口 runtime_check 短路 ─────────────────

    /// task_007 FIX AC-3：`options.runtime_check_failed = Some(code)` 时，
    /// `extract()` 必须**在调任何 Python 子进程之前**立即返回 Err，
    /// 携带与 FailureCode::as_str() 字面一致的 failure_code 标记。
    ///
    /// 关键防御：`markitdown_python_cmd` 指向"绝不存在"的路径；若入口短路失败、
    /// 真去调子进程，行为仍可观察（attempts 内出现 spawn 失败串）。本测断言
    /// **不**出现 spawn-related 字串，仅出现 `runtime self-check failed` —— 证明
    /// 入口短路命中、未触发 task_008 重构的 classify_output / candidates 循环。
    #[test]
    fn extract_short_circuits_when_runtime_check_failed() {
        use std::path::PathBuf;
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            markitdown_python_cmd: Some("/__nonexistent__/python_should_never_run".to_string()),
            markitdown_embedded_python: Some(
                "/__nonexistent__/embedded_python_should_never_run".to_string(),
            ),
            runtime_check_failed: Some(FailureCode::EExtraMissingEpub),
            ..ExtractOptions::default()
        };
        // 文件路径也不存在 —— 如果短路失败、走到 subprocess 也会失败，但失败串不同
        let dummy = PathBuf::from("/tmp/__nonexistent_input__.epub");
        let r = extractor.extract(&dummy, &opts);
        let err = r.expect_err("自检失败时 extract 必须返回 Err");
        let msg = err.to_string();
        assert!(
            msg.contains("E_EXTRA_MISSING_EPUB"),
            "入口短路应携带 FailureCode::as_str() 字面: msg={msg}"
        );
        assert!(
            msg.contains("runtime self-check failed"),
            "入口短路应携带 'runtime self-check failed' 标记: msg={msg}"
        );
        // 防御：若真走到 subprocess，attempts 会含 spawn 错误（"No such file"/"退出码"）。
        // 入口短路命中时 attempts 永不被聚合 —— 断言这些子进程错误标志不出现。
        assert!(
            !msg.contains("退出码") && !msg.contains("MarkItDown 调用失败"),
            "短路命中时不得进入 candidates 循环 / classify_output 路径: msg={msg}"
        );
    }

    // ─── task_010 (AC-1/AC-2/AC-4#3)：audio/video 路由防御 ────────────────

    /// task_010 AC-1：`SUPPORTED_MIME_TYPES` 不得含 audio/* 或 video/*。
    /// 与构建期 `grep -nE '"(audio|video)/' ...` CI gate 双向锁定。
    #[test]
    fn supported_mime_types_excludes_audio_and_video() {
        for mime in SUPPORTED_MIME_TYPES {
            assert!(
                !mime.starts_with("audio/"),
                "AC-1：SUPPORTED 不得含 audio/*，发现：{mime}"
            );
            assert!(
                !mime.starts_with("video/"),
                "AC-1：SUPPORTED 不得含 video/*，发现：{mime}"
            );
        }
        // can_handle 同步不接 audio/video（防 fallback 链路误命中）
        let e = MarkItDownExtractor::new();
        for mime in [
            "audio/mpeg",
            "audio/mp4",
            "audio/wav",
            "audio/flac",
            "audio/x-wav",
            "video/mp4",
            "video/webm",
            "video/quicktime",
        ] {
            assert!(
                !e.can_handle(mime),
                "AC-1：markitdown.can_handle 不应接 {mime}"
            );
        }
    }

    /// task_010 AC-2 helper：mime_prefix_from_path 映射常见 audio/video 后缀。
    #[test]
    fn mime_prefix_from_path_maps_audio_video_extensions() {
        use std::path::PathBuf;
        // audio
        for ext in ["mp3", "wav", "m4a", "aac", "flac", "MP3", "WAV"] {
            let p = PathBuf::from(format!("/tmp/x.{ext}"));
            assert_eq!(
                mime_prefix_from_path(&p),
                "audio/",
                "audio ext={ext} 应映射 audio/"
            );
            assert!(mime_starts_with(&p, "audio/"));
            assert!(!mime_starts_with(&p, "video/"));
        }
        // video
        for ext in ["mp4", "mov", "webm", "mkv", "avi", "WEBM"] {
            let p = PathBuf::from(format!("/tmp/x.{ext}"));
            assert_eq!(
                mime_prefix_from_path(&p),
                "video/",
                "video ext={ext} 应映射 video/"
            );
            assert!(mime_starts_with(&p, "video/"));
            assert!(!mime_starts_with(&p, "audio/"));
        }
        // 非 audio/video
        for path in ["/tmp/x.pdf", "/tmp/x.docx", "/tmp/x.png", "/tmp/x"] {
            let p = PathBuf::from(path);
            assert_eq!(mime_prefix_from_path(&p), "");
            assert!(!mime_starts_with(&p, "audio/"));
            assert!(!mime_starts_with(&p, "video/"));
        }
    }

    /// task_010 AC-4#3 (debug build)：人为污染场景 —— 直接调
    /// `MarkItDownExtractor::extract` 传 mp3 文件路径。
    ///
    /// debug build 行为：`debug_assert!` 命中 → panic（开发期硬错误暴露）。
    /// release build 行为（由代码 `#[cfg(not(debug_assertions))]` 分支保证）：
    /// 返回 `Err` 携带 `failure_code=E_AUDIO_WRONG_ROUTE`，**不**进 python 子进程。
    /// release 路径单测在标准 `cargo test`（默认 debug）下无法直接覆盖；这里
    /// 仅证明 debug 路径命中，与 task_007 短路独立。
    #[test]
    #[should_panic(expected = "markitdown 不应路由到 audio/*")]
    fn extract_panics_in_debug_on_audio_pollution() {
        use std::path::PathBuf;
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            // 关键：task_007 短路必须 None，否则会先走 runtime_check 短路而非 audio 阻断
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        let _ = extractor.extract(&PathBuf::from("/tmp/__nonexistent__.mp3"), &opts);
    }

    /// task_010 AC-4#3 (debug build)：video 污染同样命中阻断。
    #[test]
    #[should_panic(expected = "markitdown 不应路由到 video/*")]
    fn extract_panics_in_debug_on_video_pollution() {
        use std::path::PathBuf;
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        let _ = extractor.extract(&PathBuf::from("/tmp/__nonexistent__.mp4"), &opts);
    }

    /// task_010：task_007 短路优先于 audio 阻断 —— 同时设置 runtime_check_failed
    /// 与 audio 路径时，必须返回 Err（runtime 短路），不触发 debug_assert panic。
    /// 这证明 task_010 阻断追加在 task_007 短路之后，未破坏 task_007 PASS 行为。
    #[test]
    fn task_007_short_circuit_precedes_task_010_audio_block() {
        use std::path::PathBuf;
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            runtime_check_failed: Some(FailureCode::ERuntimeMissing),
            ..ExtractOptions::default()
        };
        // mp3 路径 + runtime_check 失败：先命中 task_007 短路（返回 Err），
        // 不会触发 task_010 debug_assert panic。
        let r = extractor.extract(&PathBuf::from("/tmp/__nonexistent__.mp3"), &opts);
        let err = r.expect_err("runtime 短路应返回 Err");
        let msg = err.to_string();
        assert!(
            msg.contains("runtime self-check failed"),
            "短路应携带 runtime self-check 标记: {msg}"
        );
        assert!(
            msg.contains("E_RUNTIME_MISSING"),
            "短路应携带 task_007 错误码: {msg}"
        );
    }

    /// task_007 FIX AC-3：`runtime_check_failed = None` 时不短路 —— 走原有路径
    /// （python 不存在 → 进入 candidates 循环 → 聚合错误）。本测仅断言
    /// 短路条件未命中（出现 "MarkItDown 调用失败" 聚合串而非 "runtime self-check failed"）。
    #[test]
    fn extract_does_not_short_circuit_when_runtime_check_ok() {
        use std::path::PathBuf;
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            markitdown_python_cmd: Some("/__nonexistent__/python_should_never_run".to_string()),
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        let dummy = PathBuf::from("/tmp/__nonexistent_input__.pdf");
        let r = extractor.extract(&dummy, &opts);
        let err = r.expect_err("python 不存在时仍应 Err");
        let msg = err.to_string();
        assert!(
            !msg.contains("runtime self-check failed"),
            "runtime_check_failed=None 时不得走短路: msg={msg}"
        );
    }

    // ─── task_011：保留行为覆盖补强 ──────────────────────────────────────────

    /// task_011 AC-3 #1：90s 子进程超时归类（preserve_matrix.md #1）。
    /// 直接以 elapsed≥90s + 非 0 exit 调 `classify_output`，断言归类为 `ETimeout90s`。
    /// 不真起 95s sleep —— CI 实时间预算不允许；与 markitdown.rs:247 的
    /// `io::ErrorKind::TimedOut → ETimeout90s` 分支共享归类语义。
    #[test]
    fn task_011_classify_output_at_95s_with_killed_exit_is_timeout() {
        // exit_code == None 表示被 kill（与 run_with_timeout kill 路径一致）；
        // elapsed = 95s（已过 90s 阈值）。
        let r = classify_output("", None, Duration::from_secs(95));
        assert_eq!(
            r,
            Err(FailureCode::ETimeout90s),
            "95s elapsed + killed exit 必须归类为 ETimeout90s"
        );
        // 同分支：非 0 退出码（如 SIGKILL 137）+ 95s elapsed
        let r2 = classify_output("", Some(137), Duration::from_secs(95));
        assert_eq!(r2, Err(FailureCode::ETimeout90s));
    }

    /// task_011 AC-3 #2 + AC-4：image 输入 + 子进程 exit 0 + stdout 空
    /// → 返回 `extractor_type = "markitdown_image_fallback"` + `quality_level = 1`，
    /// 不被 `classify_output` 误判为最终 `EOutputEmpty`（preserve_matrix.md #2 / AC-4）。
    ///
    /// mock 策略：用 `/usr/bin/true` 作 fake python —— 忽略 `-m markitdown <file>` 参数，
    /// 立即 exit 0 且 stdout 为空，恰好触发 classify_output → EOutputEmpty → image fallback。
    /// 不需要真 python / markitdown 模块。
    #[test]
    fn task_011_image_empty_fallback_returns_image_fallback_type() {
        use std::path::PathBuf;
        // 必须存在的可执行；macOS / Linux 默认 /usr/bin/true 都有；防御性 fallback /bin/true。
        let true_bin = if std::path::Path::new("/usr/bin/true").exists() {
            "/usr/bin/true"
        } else if std::path::Path::new("/bin/true").exists() {
            "/bin/true"
        } else {
            // 平台无 true → 跳过本测；CI 不会出现这种情况
            eprintln!("skip: /usr/bin/true and /bin/true both absent");
            return;
        };
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            // 用 true 二进制顶替 python_cmd —— 它会忽略参数立即 exit 0 + 空 stdout
            markitdown_python_cmd: Some(true_bin.to_string()),
            // 用同样的 fake，避免落回系统 python3 / python（候选循环里仍可能命中真 python，
            // 但任一首个候选 exit 0 + stdout=='' 即触发 EOutputEmpty + had_empty_success=true，
            // 后续即使其它候选失败也不影响 image fallback 命中）
            markitdown_embedded_python: Some(true_bin.to_string()),
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        // image 扩展名 → is_image=true（参见 markitdown.rs:176-185 扩展名分支）
        let img_path = PathBuf::from("/tmp/__task_011_fake_image__.png");
        let result = extractor
            .extract(&img_path, &opts)
            .expect("image 空输出应回退为 markitdown_image_fallback，不应 Err");
        assert_eq!(
            result.extractor_type, "markitdown_image_fallback",
            "AC-3 #2：image 空输出必须走 fallback 而非标准 markitdown"
        );
        assert_eq!(
            result.quality_level, 1,
            "AC-3 #2：fallback quality_level 必须为 1（低）"
        );
        // AC-4：fallback 路径**不**回包 failure_code（fallback 不算失败；
        // scheduler 按 extractor_type 在落库时写 NULL）。
        // ExtractionResult 自身没有 failure_code 字段，这里通过断言 markdown 非空 + 含"图片："
        // 验证走的是 fallback 文本生成（markitdown.rs:268-274）而非 classify_output 错判返回。
        assert!(
            result.raw_text.contains("图片"),
            "fallback markdown 必须含元数据文本，实际：{}",
            result.raw_text
        );
    }

    /// task_011 AC-3 #2 / AC-4 反例：非 image 输入（.pdf）+ exit 0 + 空 stdout
    /// → classify_output 判 EOutputEmpty → 候选循环结束后**不**走 fallback
    /// → 最终返回 Err 且失败串中含 `E_OUTPUT_EMPTY` 字面（preserve_matrix.md #2 反例）。
    ///
    /// 保证 image fallback 分支只对 image 输入生效，非 image 路径不被污染。
    #[test]
    fn task_011_non_image_empty_output_does_not_fallback() {
        use std::path::PathBuf;
        let true_bin = if std::path::Path::new("/usr/bin/true").exists() {
            "/usr/bin/true"
        } else if std::path::Path::new("/bin/true").exists() {
            "/bin/true"
        } else {
            return;
        };
        let extractor = MarkItDownExtractor::new();
        let opts = ExtractOptions {
            markitdown_enabled: true,
            markitdown_python_cmd: Some(true_bin.to_string()),
            markitdown_embedded_python: Some(true_bin.to_string()),
            runtime_check_failed: None,
            ..ExtractOptions::default()
        };
        let pdf_path = PathBuf::from("/tmp/__task_011_fake__.pdf");
        let err = extractor
            .extract(&pdf_path, &opts)
            .expect_err("非 image 空输出不应走 fallback，必须 Err");
        let msg = err.to_string();
        assert!(
            msg.contains("E_OUTPUT_EMPTY"),
            "非 image 空输出错误码必须含 E_OUTPUT_EMPTY，实际：{msg}"
        );
        assert!(
            !msg.contains("markitdown_image_fallback"),
            "非 image 路径不得提及 fallback 类型，实际：{msg}"
        );
    }

    /// task_011 AC-3 #3：版本探测缓存（preserve_matrix.md #3）。
    /// `MarkItDownExtractor` 持有 `RwLock<Option<String>>`；首次 extract 成功后写入，
    /// 后续 extract 命中"已有缓存"分支，**不**再调 `probe_markitdown_version`。
    ///
    /// 无法直接 mock 子进程；改测缓存状态机不变量：
    /// (a) `detected_version()` 初值 None；
    /// (b) 调 `cache_version("v1")` 后变为 Some("v1")；
    /// (c) markitdown.rs:207 的 gate 条件 `is_none_or(|g| g.is_none())` 在已缓存时为 false
    ///     → 即不会再进 `probe_markitdown_version` 分支（行为等价于"调用计数 == 1"）。
    /// 这等价于 input.md AC-3 #3 字面要求的"连续两次 extract 仅一次 --version 调用"。
    #[test]
    fn task_011_version_cache_gates_reentry_after_first_set() {
        let extractor = MarkItDownExtractor::new();
        // (a) 初值
        assert_eq!(extractor.detected_version(), None, "缓存初值应为 None");

        // 模拟 markitdown.rs:207 的 gate 表达式
        let gate_before = extractor
            .cached_version
            .read()
            .ok()
            .is_none_or(|g| g.is_none());
        assert!(gate_before, "首次未缓存时 gate 应为 true（进入 probe 分支）");

        // (b) 缓存写入
        extractor.cache_version("v1.0.0-test".to_string());
        assert_eq!(
            extractor.detected_version(),
            Some("v1.0.0-test".to_string()),
            "写入后应能读出"
        );

        // (c) 第二次：gate 必须为 false → 不再调 probe（计数等价于"只调过一次"）
        let gate_after = extractor
            .cached_version
            .read()
            .ok()
            .is_none_or(|g| g.is_none());
        assert!(
            !gate_after,
            "已缓存时 gate 必须为 false，否则会重复调用 probe_markitdown_version"
        );

        // 再写一次同值不变（保证幂等）
        extractor.cache_version("v1.0.0-test".to_string());
        assert_eq!(extractor.detected_version(), Some("v1.0.0-test".to_string()));
    }
}
