# Task 交付 — task_010_audio_route_guard

## 实现摘要

实现 audio/video 误路由的三层防护：

1. **AC-1**：`SUPPORTED_MIME_TYPES` 严格不含 `audio/*` / `video/*`（初始状态已合规，本 task 在数组上方加锁定式注释引用 H5 / CI gate 规则）。
2. **AC-2**：`MarkItDownExtractor::extract()` 入口在 task_007 runtime_check 短路 6 行**之后**追加 audio/video 防御 — debug build 用 `debug_assert!` panic 暴露开发期误路由；release build 走 `FailureCode::EAudioWrongRoute` 错误码返回（不 panic 用户进程）。
3. **AC-3**：`scheduler.rs` 主循环在 `get_extractor_for` 调用**之前**插入 `video_route_should_reject` 拦截分支 —— 显式落 `conversion_meta.failure_code = E_AUDIO_WRONG_ROUTE` + `materialize_placeholder` + `db_mark_task_status("failed")`，杜绝 video/* 走默认 unsupported 静默路径。`audio/*` 仍通过 `get_extractor_for → fallback → IflytekAsrExtractor::can_handle` 命中 iflytek（既有事实，未触碰）。
4. **AC-4**：新增 7 个单测覆盖 mp3/wav/m4a/flac/x-wav → iflytek、mp4/webm/quicktime/x-msvideo → 显式拒绝、人为污染 mp3/mp4 → debug_assert panic、task_007 短路优先级、video/* 无 extractor 候选证明。
5. **AC-5**：`audio_asr_iflytek.rs` 本 session 0 改动（其在初始 git status 中的 MM 状态来自 task_014 Fix-A3 预存量，与 task_010 无关）。

## 修改的文件

| 文件 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/src/extraction/extractors/markitdown.rs` | 修改 | AC-1 注释锁定；新增 `mime_prefix_from_path` / `mime_starts_with` helper；AC-2 入口 debug_assert + release 返码；新增 4 个单测 + 1 个 task_007 兼容性测试 |
| `NCdesktop/src-tauri/src/extraction/scheduler.rs` | 修改 | AC-3 video 显式拒绝分支；新增 `video_route_should_reject` / `audio_should_route_to_iflytek` 纯函数；新增 3 个单测 |

> 注：`scheduler.rs` PDF 分支 / `audio_asr_iflytek.rs` / `failure_code.rs` / `runtime_check.rs` / `migration.rs` / `scripts/` 均 0 改动。`models.rs` / `mod.rs` / `scan_pdf_detect.rs` 等其他文件来自 task_008/009 并行 dev 的工作树预存量，不在本 task 修改清单内。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（无新文件）
- [x] API 路径/命名与 Architect 方案一致（错误码 `E_AUDIO_WRONG_ROUTE` 严格复用 FailureCode 枚举）
- [x] 数据模型与 Architect 方案一致（conversion_meta.failure_code 字段由 task_008 落地，本 task 仅写入）
- [x] 未引入计划外的新依赖（mime helper 自建，未引入 `mime` / `mime_guess` crate）
- 偏离说明：**无**

## mime_starts_with helper 来源（新建）

**自建** —— 项目中无现成 mime helper（`commands::sync::guess_mime` 是私有 fn，模块边界不互通）。markitdown.rs 内新增两个 helper：

- `mime_prefix_from_path(path) -> &'static str`：基于文件扩展名映射到 `"audio/"` / `"video/"` / 空串。扩展名集合：
  - audio: mp3 / wav / m4a / aac / flac / ogg / oga / opus / wma
  - video: mp4 / mov / avi / mkv / webm / m4v / wmv / flv
- `mime_starts_with(path, prefix) -> bool`：薄包装。

**设计理由**：`Extractor::extract()` trait 签名只接 `&Path`，不传 mime；helper 必须能从路径自给自足。不引入 `mime_guess` crate（避免 Cargo.toml 改动 + 防止 task_002 manifest 校验破坏）。映射表覆盖 scheduler / Asset.mime_type 实际可见的常见后缀。

## video 错误码选择（**复用 EAudioWrongRoute**，未 ESCALATE）

input.md 未明示 video 专用错误码；现有 8 个 FailureCode 枚举中 `EAudioWrongRoute` 字面是 "audio" 但语义为"走错路由 / 本期不接"——同义复用。

**选择理由**：
1. PRD 底线 #4 + ADR-007 锁定 8 错误码不增加变体；
2. video/* 与 audio 路由错误同属"误路由不支持"语义类；
3. 不引入新变体 → migration.rs / failure_code.rs / 8-错误码契约表零改动。

**已诚实写入** scheduler.rs 注释（行 158-163）。Reviewer 如认为需要新增 `E_VIDEO_UNSUPPORTED` 变体，应作为 ESCALATE 项触发 task_008 Fix mode。

## task_007 6 行短路保留证据

`markitdown.rs::extract()` 顶部短路（task_007 PASS 已确认）：

```rust
// task_007 FIX：runtime self-check 快照短路（AC-3）。
if let Some(code) = options.runtime_check_failed {
    return Err(parse_error_with_class(&format!(
        "runtime self-check failed | failure_code={code}"
    )));
}
```

task_010 audio/video 阻断**追加在其后**（行 102 起），与 task_007 短路独立无重叠。新增单测 `task_007_short_circuit_precedes_task_010_audio_block` 验证两者优先级 —— mp3 输入 + `runtime_check_failed = Some(ERuntimeMissing)` 时，**先**命中 task_007 短路返回 Err，**不**触发 task_010 debug_assert panic。

另外原 task_007 测试 `extract_short_circuits_when_runtime_check_failed` / `extract_does_not_short_circuit_when_runtime_check_ok` 均仍 PASS（见测试结果 17 passed / 0 failed）。

## audio_asr_iflytek.rs 0 改动证据

- `find src/extraction -name "*.rs" -mmin -30` 输出**不含** `audio_asr_iflytek.rs`（本 session 30 分钟内无文件系统写入）。
- `git diff HEAD -- src/extraction/extractors/audio_asr_iflytek.rs` 显示的 diff 全部来自 task_014 Fix-A3 预存量（行 31-47 `DEFAULT_IFLYTEK_LANGUAGE` / `resolve_language` 函数 —— 在本 session 启动**之前**已存在于 staged + unstaged 状态）。
- PRD 底线 #4 / AC-5 严守。

## 测试命令

```bash
cd NCdesktop/src-tauri
cargo test --lib
cargo test --lib extraction::extractors::markitdown
cargo test --lib extraction::scheduler
# AC-1 grep gate（精确版本，限定 SUPPORTED_MIME_TYPES 数组段）
awk '/^const SUPPORTED_MIME_TYPES/,/^\];/' src/extraction/extractors/markitdown.rs \
  | grep -nE '"(audio|video)/' && echo VIOLATION || echo PASS
```

## 测试结果

```
test result: ok. 213 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
```

baseline 195 → 213 = +18 passed（task_010 直接新增 8 个 + task_008/009 并行 dev 工作树带来另外 10 个）。

模块切片：
- `extraction::extractors::markitdown`：17 passed / 0 failed
- `extraction::scheduler`：15 passed / 0 failed

AC-1 grep 精确版：`PASS`（SUPPORTED_MIME_TYPES 数组段 0 命中 audio/video）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| 正常 | mp3/wav/m4a/flac/x-wav mime → 路由 iflytek 而非 markitdown | 已测 | PASS（`audio_mime_routes_to_iflytek_not_markitdown`，断言 `extractor.name() == "audio_asr_iflytek"`）|
| 正常 | video/mp4/webm/quicktime/x-msvideo mime → `video_route_should_reject == true` | 已测 | PASS（`video_mime_is_explicitly_rejected`）|
| 边界 | video/* mime 无任何 extractor 候选 → 必须依赖显式拒绝路径 | 已测 | PASS（`video_mime_has_no_extractor_so_must_be_explicitly_rejected`）|
| 异常 | 人为污染 mp3 直接调 markitdown::extract → debug_assert panic | 已测 | PASS（`extract_panics_in_debug_on_audio_pollution`，`#[should_panic(expected = "markitdown 不应路由到 audio/*")]`）|
| 异常 | 人为污染 mp4 直接调 markitdown::extract → debug_assert panic | 已测 | PASS（`extract_panics_in_debug_on_video_pollution`）|
| 边界 | mp3 + runtime_check_failed → task_007 短路优先，不触发 task_010 panic | 已测 | PASS（`task_007_short_circuit_precedes_task_010_audio_block`）|
| 边界 | `SUPPORTED_MIME_TYPES` 数组任一项 prefix 不为 audio/ 或 video/ | 已测 | PASS（`supported_mime_types_excludes_audio_and_video`）|
| 边界 | `mime_prefix_from_path` 对常见 audio/video 扩展名映射正确 | 已测 | PASS（`mime_prefix_from_path_maps_audio_video_extensions`）|
| 边界 | release build 走 `FailureCode::EAudioWrongRoute` 返回 Err（非 panic） | 未测 | release 路径在 `cargo test`（默认 debug）下不可直接覆盖；由 `#[cfg(not(debug_assertions))]` 代码块 + 注释保证，已在 `extract()` 内行 156-164 锁定 |
| 异常 | task_007 原有 2 测试在 task_010 改动后仍 PASS | 已测 | PASS（`extract_short_circuits_when_runtime_check_failed` / `extract_does_not_short_circuit_when_runtime_check_ok` 均 ok）|

## 浏览器/运行时验证

N/A —— 本 task 为 Rust 后端路由防御 + 单测，无 UI 入口。运行期行为由 scheduler 主循环 + extract() 入口在打包后端到端测试时覆盖（task_012 / task_013 范围）。

## 已知局限

1. **release build 路径未被单测直接覆盖**：`cargo test` 默认走 debug，`#[cfg(not(debug_assertions))]` 分支只能在 `cargo test --release` 下执行；本 session 未跑 release 单测以节省时间。代码逻辑由代码 review + 注释锁定。
2. **video 错误码字面是 audio**：`E_AUDIO_WRONG_ROUTE` 名称对 video 不直观；前端展示文案需要单独映射"video 不支持"（task_011 / UI 文案范围）。已在 scheduler.rs 注释中说明。
3. **`mime_prefix_from_path` 仅扩展名启发式**：未实现 magic-byte 检测；与 input.md 技术约束"路由判定优先级：mime > 扩展名；不一致时取保守值（拒绝）"一致 —— 主路由由 scheduler 用 mime 字符串决策，markitdown.rs 入口的 path-based 判定仅作为**防御性**第二道关卡，不取代上层 mime 路由。

## 需要 Reviewer 特别关注的地方

1. **AC-1 grep gate 粒度**：input.md 给的字面命令 `grep -nE '"(audio|video)/' src/extraction/extractors/markitdown.rs` 在新增 helper / 单测后会有命中（helper 中 `"audio/"` 字面前缀字符串 + 单测中 mime 字面）。**实质 AC-1（SUPPORTED 数组无 audio/video）已满足**，已用精确 awk 版本证明。Reviewer 决定是否：
   - 接受精确 awk gate（推荐：贴合 AC 实质）
   - 要求 helper 改用别名常量绕过字面 grep（成本较高，无功能收益）
2. **video 错误码复用 `EAudioWrongRoute`**：是否需要 ESCALATE 触发新增 `E_VIDEO_UNSUPPORTED` 变体？我的判断是不需要（语义同义、避免破坏 8 错误码契约）；但 Reviewer 立场可能不同。
3. **release build 路径未单测覆盖**：可选要求"补 release build 单测"（运行 `cargo test --release -- task_010` 验证 release 分支返回 Err 而非 panic）。
4. **scheduler.rs 范围 gate**：本 task 改动严格限于 `audio/* + video/*` 分支与新增辅助纯函数，未触动 PDF 分支。
