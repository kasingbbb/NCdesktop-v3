# Task 输入 — task_010_audio_route_guard

## 目标
确保 markitdown `SUPPORTED_MIME_TYPES` 不含 `audio/*` / `video/*`，且 `MarkItDownExtractor::extract()` 入口 `assert` 阻断；scheduler 单测覆盖"音频文件不会被路由进 markitdown"。

## 前置条件
- 依赖 task：task_008（错误码已落地）
- 必须先存在的文件/接口：`FailureCode::EAudioWrongRoute`

## 验收标准（Acceptance Criteria）
1. AC-1：`extractors/markitdown.rs:54` 的 `SUPPORTED_MIME_TYPES` **不得**含 `audio/*` / `video/*`（grep 检查作为 CI gate）。
2. AC-2：`MarkItDownExtractor::extract()` 入口添加：
   ```rust
   debug_assert!(!mime_starts_with(file_path, "audio/"));
   debug_assert!(!mime_starts_with(file_path, "video/"));
   ```
   并在 release build 改为返回 `FailureCode::EAudioWrongRoute`（不 panic 用户进程）。
3. AC-3：`scheduler` 路由层确保 `audio/*` → `audio_asr_iflytek::extract`、`video/*` → 显式拒绝（本期不支持 video）。
4. AC-4：单测：
   - 模拟 mp3 / wav / m4a → 路由到 iflytek 而非 markitdown；
   - 模拟人为污染（直接调用 `MarkItDownExtractor::extract` 传 mp3）→ 返回 `EAudioWrongRoute` 错误码。
5. AC-5：严禁修改 `audio_asr_iflytek.rs` 业务逻辑（PRD 底线 #4）；本 task 只动 markitdown.rs 与 scheduler.rs。

## 技术约束
- 路由判定优先级：mime > 扩展名；不一致时取保守值（拒绝）。
- 不得静默吃掉 audio 路由错误：必须落 `conversion_meta.failure_code`。

## 参考文件
- `src-tauri/src/extraction/extractors/markitdown.rs:54-69`
- `src-tauri/src/extraction/extractors/audio_asr_iflytek.rs`（仅参考，不动）
- PRD §3.1 F5、底线 #4

## 预估影响范围
- 修改：`src-tauri/src/extraction/extractors/markitdown.rs`、`src-tauri/src/extraction/scheduler.rs`
- 新增测试：`src-tauri/src/extraction/scheduler.rs` 末尾 `#[cfg(test)]`
