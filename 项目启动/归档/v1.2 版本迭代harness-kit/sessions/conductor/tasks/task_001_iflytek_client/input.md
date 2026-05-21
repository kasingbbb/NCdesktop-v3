# Task 输入 — task_001_iflytek_client

## 目标

在 `src-tauri/src/extraction/extractors/` 下新建 `audio_asr_iflytek.rs`，实现科大讯飞非实时语音转写 WebAPI 的完整调用流程：鉴权 → 提交任务 → 异步轮询 → 返回纯文本结果。

## 前置条件

- 依赖 task：无（独立实现）
- 必须先存在的文件：
  - `src-tauri/Cargo.toml`（需添加 `hmac`、`base64` crate）
  - `src-tauri/src/extraction/extractors/mod.rs`（注册新 extractor 时需修改）

## 验收标准（Acceptance Criteria）

1. **AC-1**：`IflytekAsrExtractor` 实现 `Extractor` trait，`can_handle` 对 `audio/mpeg`、`audio/mp4`、`audio/wav`、`audio/flac`、`audio/x-wav` 返回 true
2. **AC-2**：HMAC-SHA256 鉴权头生成正确（可通过单元测试验证签名格式符合讯飞文档）
3. **AC-3**：`extract()` 方法异步完成：提交文件 → 获取 taskId → 轮询（间隔 10s，最大 180 次即 30 分钟）→ 返回拼接纯文本
4. **AC-4**：API 返回错误（401/超时/网络失败）时，返回 `Err(ExtractionError::OcrError(描述))`，不 panic
5. **AC-5**：APPID/APIKey/APISecret 通过函数参数或 AppState 传入，不在文件中硬编码（代码里用常量占位符即可，真实凭据从外部注入）
6. **AC-6**：Cargo.toml 新增 `hmac`、`base64` 依赖，`cargo check` 通过

## 技术约束

- HTTP 客户端：使用现有 `reqwest 0.12`，开启 `rustls-tls` feature（已有）
- 加密：`sha2 0.10`（已有）+ 新增 `hmac = "0.12"`、`base64 = "0.22"`（标准 crate，稳定版本）
- 异步：`tokio::time::sleep` 实现轮询，不阻塞线程
- 错误类型：沿用 `ExtractionError::OcrError(String)`（见 `extraction/models.rs`）
- 输出格式：`ExtractionResult` 的 `raw_text` 和 `structured_md` 均填充转录纯文本（与现有 `audio_asr.rs` 行为一致）

## 讯飞 API 关键信息

- 接口地址：`https://office-api-ist-dx.iflyaisol.com`
- 鉴权方式：HMAC-SHA256（参考讯飞官方文档 "非实时转写 WebAPI" 鉴权章节）
  - 签名字符串：`host: <host>\ndate: <RFC1123 date>\nPOST <path> HTTP/1.1`
  - Authorization 格式：`api_key="<APIKey>",algorithm="hmac-sha256",headers="host date request-line",signature="<base64(hmac)>"`
- 音频编码：将文件字节直接 base64 编码（standard encoding），填入请求体 audio 字段
- 支持格式：mp3、m4a、wav、flac（encoding 字段对应填写）
- 轮询：提交后获取 taskId，GET 状态接口直到 `orderState == 4`（完成）或 `orderState == -1`（失败）

## 参考文件

- `src-tauri/src/extraction/extractors/audio_asr.rs` — 现有 ASR extractor，参考 trait 实现结构
- `src-tauri/src/extraction/models.rs` — ExtractionResult、ExtractionError、Extractor trait 定义
- `src-tauri/src/extraction/extractors/mod.rs` — extractor 注册方式
- `src-tauri/Cargo.toml` — 现有依赖列表

## 预估影响范围

- 新建文件：`src-tauri/src/extraction/extractors/audio_asr_iflytek.rs`
- 修改文件：`src-tauri/Cargo.toml`（添加 hmac、base64 依赖）
- 暂不修改：`mod.rs` 注册（由 task_002 完成）

## Conductor 复杂度评估

**M 级**（涉及新外部 API、新 crate 依赖、异步轮询逻辑）
→ Reviewer 重点关注：HMAC 签名正确性、凭据不泄露、轮询超时处理
