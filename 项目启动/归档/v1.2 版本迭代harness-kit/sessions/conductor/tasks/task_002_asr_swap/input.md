# Task 输入 — task_002_asr_swap

## 目标

将 extraction 调度器中的 ASR extractor 从 `AudioAsrExtractor`（macOS SFSpeechRecognizer FFI）替换为 `IflytekAsrExtractor`（科大讯飞 WebAPI），并将讯飞 API 凭据注入 AppState。

## 前置条件

- 依赖 task：**task_001_iflytek_client 必须已完成**（`audio_asr_iflytek.rs` 存在且 `cargo check` 通过）
- 必须先存在的文件：
  - `src-tauri/src/extraction/extractors/audio_asr_iflytek.rs`
  - `src-tauri/src/extraction/extractors/mod.rs`
  - `src-tauri/src/lib.rs`（AppState 定义处）

## 验收标准（Acceptance Criteria）

1. **AC-1**：`extraction/extractors/mod.rs` 中的 extractor 列表，`IflytekAsrExtractor` 替换 `AudioAsrExtractor`（音频 mime 类型命中讯飞 extractor）
2. **AC-2**：`IflytekAsrExtractor` 实例化时所需的 APPID/APIKey/APISecret 从 AppState 读取，不在 mod.rs 或任何前端可见文件中出现明文
3. **AC-3**：原有 `AudioAsrExtractor` 代码保留不删除（仅停止注册），便于回滚
4. **AC-4**：`cargo check` 通过，无编译警告（unused import 等）
5. **AC-5**：在 `lib.rs` 初始化 AppState 时，讯飞凭据已正确设置（可以是编译期常量或从 settings 读取，明确选择一种）

## 技术约束

- 修改范围：`mod.rs`（extractor 注册）、`lib.rs`（AppState 凭据注入）
- 不修改 extraction scheduler 逻辑、badge 状态机、数据库写入路径
- 凭据注入方式推荐：在 AppState 新增 `iflytek_config: IflytekConfig` 字段，`IflytekConfig { appid, api_key, api_secret }` 从环境变量或编译期常量读取

## 参考文件

- `src-tauri/src/extraction/extractors/mod.rs` — 当前 extractor 注册列表
- `src-tauri/src/extraction/extractors/audio_asr.rs` — 旧 extractor（保留不删）
- `src-tauri/src/lib.rs` — AppState 定义和初始化
- task_001 的 `audio_asr_iflytek.rs`

## 预估影响范围

- 修改文件：
  - `src-tauri/src/extraction/extractors/mod.rs`
  - `src-tauri/src/lib.rs`
- 不新建文件

## Conductor 复杂度评估

**S 级**（纯接线工作，逻辑在 task_001 已实现）
→ Reviewer 关注点：凭据是否真的只在 Rust 层，注册顺序是否正确
