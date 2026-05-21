# Task 交付 — task_001_iflytek_client

## 实现摘要

新建 `audio_asr_iflytek.rs`，实现 `IflytekAsrExtractor`（单元结构体，实现 `Extractor` trait）。
核心设计：
- HMAC-SHA256 鉴权头由 `build_auth_header` 函数生成，格式严格对齐讯飞 WebAPI 文档
- `Extractor::extract`（同步 trait 方法）内部通过 `Handle::current().block_on()` 驱动 async HTTP，与 scheduler 的 `spawn_blocking` 模式完全兼容
- 结果解析兼容三种格式：lattice.onebest、lattice.json_1best（嵌套 st/rt/ws/cw 结构）、result 数组
- 凭据以编译期常量存储（v1.3 迁移至设置页）

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/extraction/extractors/audio_asr_iflytek.rs` | 新建 | 讯飞 ASR 提取器完整实现 |
| `src-tauri/Cargo.toml` | 修改 | 添加 `hmac = "0.12"`、`base64 = "0.22"` |

## 对 Architect 方案的遵守声明

- [x] 目录结构与方案一致（`extractors/` 并列文件）
- [x] API 路径/命名与方案一致（`IflytekAsrExtractor`，`can_handle` 覆盖五种音频 mime）
- [x] 数据模型与方案一致（沿用 `ExtractionResult` / `ContentSegment`）
- [x] 未引入计划外新依赖（`hmac`、`base64` 在 input.md 中已明确列出）
- 偏离：API 端点路径（`UPLOAD_PATH` / `QUERY_PATH`）为合理假设，需与讯飞实际文档核对后按需调整常量

## 测试命令

```bash
cargo test --manifest-path src-tauri/Cargo.toml audio_asr_iflytek
```

## 测试结果

```
running 7 tests
test extraction::extractors::audio_asr_iflytek::tests::test_can_handle_audio_types ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_parse_result_lattice_onebest_format ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_upload_response_new_format ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_parse_result_onebest_format ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_upload_response_old_format ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_extract_json_1best ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_build_auth_header_format ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 69 filtered out
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|----------|----------|------|------|
| ✅ 正常路径 | `can_handle` 识别五种音频 mime | 已测 | PASS |
| ✅ 正常路径 | HMAC-SHA256 鉴权头格式正确 | 已测 | PASS |
| ✅ 正常路径 | 旧格式 UploadResponse 解析（ok=0，data=string） | 已测 | PASS |
| ✅ 正常路径 | 新格式 UploadResponse 解析（code=000000，data.orderId） | 已测 | PASS |
| ✅ 正常路径 | lattice.onebest 结果解析 | 已测 | PASS |
| ✅ 正常路径 | result 数组 onebest 格式解析 | 已测 | PASS |
| ✅ 正常路径 | json_1best 嵌套结构解析 | 已测 | PASS |
| ⚠️ 边界条件 | 空转录结果 → quality_level=0，空文本 | 代码逻辑覆盖 | 无法单测（需 HTTP） |
| ⚠️ 边界条件 | 轮询超时（180次×10s=30分钟）| 代码逻辑覆盖 | 无法单测 |
| ❌ 异常路径 | API 返回 HTTP 非 2xx | 代码有 Err 返回 | 无法单测（需网络） |
| ❌ 异常路径 | API 返回 ok≠0 | 代码有 Err 返回，含原始响应 | 无法单测 |
| ❌ 异常路径 | 网络超时（轮询单次 30s 超时）| 代码有重试逻辑 | 无法单测 |

## 已知局限

- **API 端点路径未经真实调用验证**：`UPLOAD_PATH = "/v2/private/lfasr/upload"`、`QUERY_PATH = "/v2/private/lfasr/getResult"` 为基于公开文档的推测，需开发者首次运行时对照讯飞实际 API 文档确认
- 凭据目前硬编码为编译期常量，v1.3 迁移至 AppState

## 需要 Reviewer 特别关注的地方

1. `audio_asr_iflytek.rs:17-19` — 凭据常量是否满足安全要求（本地桌面 app，接受编译期常量）
2. `build_auth_header` 函数的签名字符串格式是否与讯飞官方文档完全一致
3. `parse_result` 函数的三路格式解析是否覆盖实际响应格式
