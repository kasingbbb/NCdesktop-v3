use std::path::Path;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

pub struct AudioAsrExtractor;

impl Extractor for AudioAsrExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        // macOS / Windows 都启用：macOS 走本地 SFSpeechRecognizer，Windows 走 cloud_ai。
        // 其他平台构建永返 false
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            matches!(
                mime_type,
                "audio/mpeg" | "audio/mp4" | "audio/wav" | "audio/flac" | "audio/x-wav"
            )
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = mime_type;
            false
        }
    }

    fn name(&self) -> &'static str {
        "audio_asr"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            let transcription = crate::macos::asr_ffi::transcribe_audio(file_path)
                .map_err(ExtractionError::OcrError)?;

            let transcription = transcription.trim().to_string();

            if transcription.is_empty() {
                return Ok(ExtractionResult {
                    raw_text: String::new(),
                    structured_md: String::new(),
                    quality_level: 0,
                    extractor_type: "audio_asr".to_string(),
                    segments: vec![],
                    needs_ocr_fallback: false,
                });
            }

            let segments = vec![ContentSegment {
                segment_type: "asr_transcription".to_string(),
                content: transcription.clone(),
                page: None,
                confidence: None,
                bbox: None,
            }];

            Ok(ExtractionResult {
                raw_text: transcription.clone(),
                structured_md: transcription,
                quality_level: 1,
                extractor_type: "audio_asr".to_string(),
                segments,
                needs_ocr_fallback: false,
            })
        }

        #[cfg(target_os = "windows")]
        {
            // Windows 走 cloud_ai 异步分支；extract() trait 为 sync（caller 已在
            // spawn_blocking 内），用 Handle::current().block_on 桥接。
            let asr = tokio::runtime::Handle::current()
                .block_on(crate::cloud_ai::transcribe_audio(file_path))?;

            let transcription = asr.transcription.trim().to_string();

            if transcription.is_empty() {
                return Ok(ExtractionResult {
                    raw_text: String::new(),
                    structured_md: String::new(),
                    quality_level: 0,
                    extractor_type: "audio_asr".to_string(),
                    segments: vec![],
                    needs_ocr_fallback: false,
                });
            }

            let segments = vec![ContentSegment {
                segment_type: "asr_transcription".to_string(),
                content: transcription.clone(),
                page: None,
                confidence: None,
                bbox: None,
            }];

            Ok(ExtractionResult {
                raw_text: transcription.clone(),
                structured_md: transcription,
                quality_level: 1,
                extractor_type: "audio_asr".to_string(),
                segments,
                needs_ocr_fallback: false,
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = file_path;
            Err(ExtractionError::UnsupportedPlatform)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_audio_types() {
        let extractor = AudioAsrExtractor;
        // 在 macOS / Windows 上应该能处理
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            assert!(extractor.can_handle("audio/mpeg"));
            assert!(extractor.can_handle("audio/mp4"));
            assert!(extractor.can_handle("audio/wav"));
            assert!(extractor.can_handle("audio/flac"));
        }
        // 不处理非音频类型
        assert!(!extractor.can_handle("application/pdf"));
        assert!(!extractor.can_handle("image/jpeg"));
        assert!(!extractor.can_handle(""));
    }

    #[test]
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn test_can_handle_returns_false_on_unsupported() {
        let extractor = AudioAsrExtractor;
        assert!(!extractor.can_handle("audio/mpeg"));
        assert!(!extractor.can_handle("audio/mp4"));
    }
}
