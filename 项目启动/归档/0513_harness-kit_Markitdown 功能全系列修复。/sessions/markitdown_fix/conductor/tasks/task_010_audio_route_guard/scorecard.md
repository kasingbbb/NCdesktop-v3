# Review Scorecard — task_010_audio_route_guard

## 审查思考过程

1. **Task 意图**：在三个防线上阻止 audio/video 文件误路由进入 markitdown：
   (a) `SUPPORTED_MIME_TYPES` 字面排除 audio/video（CI gate）；
   (b) `MarkItDownExtractor::extract()` 入口在 task_007 6 行 runtime_check 短路**之后**追加 audio/video 防御 —— debug build `debug_assert!` panic，release build 返 `FailureCode::EAudioWrongRoute`；
   (c) scheduler 主循环新增 `video_route_should_reject` 拦截 + 写 `conversion_meta.failure_code`，audio/* 走原 fallback→iflytek 链不动。
2. **AC 检查结果**：
   - AC-1 ✅（awk 限定数组段 grep 0 命中；CI gate 命令固化在源码注释）
   - AC-2 ✅（debug_assert + release `#[cfg(not(debug_assertions))]` 双分支；位于 task_007 短路之后）
   - AC-3 ✅（`video_route_should_reject` 在 `get_extractor_for` 之前；写 conversion_meta + materialize_placeholder + emit failed）
   - AC-4 ✅（8 测全过；优先级测试 `task_007_short_circuit_precedes_task_010_audio_block` 明确锁定 task_007 短路先于 task_010 阻断）
   - AC-5 ✅（`audio_asr_iflytek.rs` git diff 全部为 task_014 Fix-A3 预存量；本 session 30 分钟内 mtime 不命中）
3. **关键发现**：
   - 实现层次清晰：scheduler 用 mime 判定（主路由）、markitdown.rs 用扩展名判定（防御性二道关），与 input.md 技术约束"mime > 扩展名；保守值"一致。
   - cargo test --lib 实测 **215 passed / 0 failed**（baseline 195 → +20，含并行 task_008/009/014 dev 工作树测试）。
   - markitdown 模块 17 测 / scheduler 模块 17 测全过。
   - `runtime_check` 在 markitdown.rs 命中 16 次（远高于 baseline 期待 9 次），task_007 6 行短路完整保留并独立于 task_010 阻断。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|---|---|---|---|
| 功能正确性 | 25% | 5 | 三层防御全部落地；8 单测覆盖正常/边界/异常/优先级 |
| 安全性 | 25% | 5 | 不静默吃错（显式写 failure_code）；release build 不 panic 用户进程；debug build 暴露开发期硬错误 |
| 代码质量 | 15% | 4 | helper 命名清晰，注释充分；mime_prefix_from_path 与 sync::guess_mime 重复（可接受，跨模块边界自给自足） |
| 测试覆盖 | 15% | 4 | debug 路径 8 测齐全；release 分支由 `#[cfg(not(debug_assertions))]` 锁定但单测未直跑（dev 主动声明，可接受） |
| 架构一致性 | 10% | 5 | 复用 `EAudioWrongRoute` 不扩 8 错误码契约；未引入新 crate；PDF 分支零触动 |
| 可维护性 | 10% | 5 | helper 注释含设计理由 + CI gate 字面；优先级测试自带回归保护 |

**综合分：4.7/5**（加权 = 0.25×5 + 0.25×5 + 0.15×4 + 0.15×4 + 0.10×5 + 0.10×5 = 4.7）

## 总体判断

- [x] **PASS**

## Dev 主动声明的 3 关注点判定

1. **AC-1 grep gate 字面命令粗暴匹配** → **PASS**
   - 接受 awk 限定数组段证明（`awk '/^const SUPPORTED_MIME_TYPES/,/^\];/' ... | grep -nE '"(audio|video)/'` → `PASS`）。
   - 实质 AC 满足：`SUPPORTED_MIME_TYPES` 数组 0 命中 audio/video；helper / 单测中的字面属于实现细节。
   - 注释中已固化精确 gate 命令；未来 CI 直接调用此版本即可。

2. **video 错误码复用 `EAudioWrongRoute`** → **PASS**
   - 8 错误码契约 + ADR-007 + PRD 底线 #4 锁定不增枚举变体；ESCALATE 会触发 task_008 范围回退。
   - 语义同义："走错路由 / 本期不接"，scheduler 注释已诚实记录复用决策（line 162-163）。
   - 已知局限 #2 提示前端文案需单独映射"video 不支持"——属 task_011/UI 范围，不阻塞本 task。

3. **release build 路径未单测直接覆盖** → **PASS**（MINOR 建议而非阻塞）
   - debug_assert! 在 dev/CI 默认 debug build 下提供完整防御；release 分支由 `#[cfg(not(debug_assertions))]` 静态包含。
   - 建议（不强制）：未来加 `cargo test --release -- task_010` CI job，验证 release 分支返回 Err 而非 panic。

## 红线检查

| 红线 | 状态 |
|---|---|
| 修改 `audio_asr_iflytek.rs` | ✅ 未命中（diff 全部为 task_014 Fix-A3 预存量） |
| 修改 `runtime_check.rs` / `failure_code.rs` | ✅ 未命中 |
| 破坏 task_007 6 行短路 | ✅ 未命中（短路完整保留在 line 129-136，task_010 追加在 line 138 之后；优先级测试锁定） |
| 修改 scheduler.rs PDF 分支 | ✅ 未命中（PDF 段由并行 task_009 dev 添加，task_010 仅新增 video reject 与 helper） |
| 修改 task_004~006 scripts/ | ✅ 未命中 |
| 修改 db/migration.rs / db/conversion_meta.rs / db/asset.rs | ✅ 未命中 |
| 修改 task_000 区脱敏 | ✅ 未命中 |
| 修改 task_003 verify-venv-shim.sh | ✅ 未命中 |
| 引入新依赖（mime / mime_guess） | ✅ 未命中（self-contained helper） |
| 静默吃掉 audio 路由错误 | ✅ 未命中（显式 write_conversion_meta + update_conversion_meta_failure_code + materialize_placeholder） |
| cargo test --lib 退步（< 195） | ✅ 未命中（215 passed） |

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **release build 路径单测未直接覆盖**
   - **代码位置**：`markitdown.rs:155-163`
   - **修复方向**（可选 / 不阻塞本次 PASS）：未来增加 CI job `cargo test --release -- task_010`，添加专门的 release-only 测试断言返 `Err` 携 `E_AUDIO_WRONG_ROUTE`。
   - **验证标准**：release build 下 `extract_returns_err_in_release_on_audio_pollution` 测试通过。

2. **`mime_prefix_from_path` 与 `commands::sync::guess_mime` 重复**
   - **代码位置**：`markitdown.rs:86-101`
   - **修复方向**（可选）：未来抽出 `utils::mime::guess_prefix` 共享 helper，移除两处副本。
   - **验证标准**：两个调用方共用同一 fn 且单测覆盖。

3. **`audio_should_route_to_iflytek` 标 `#[allow(dead_code)]`**
   - **代码位置**：`scheduler.rs:591`
   - **修复方向**（可选）：要么删除该 helper（已由 `get_extractor_for` 路径覆盖），要么在主循环中作为防御性断言使用。
   - **验证标准**：删除 `#[allow(dead_code)]` 属性后编译无 warning。

## cargo test --lib 实测数字

- 全量：**215 passed / 0 failed**（baseline 195 → +20，含并行 task_008/009/014 dev 工作树）
- `extraction::extractors::markitdown` 切片：**17 passed / 0 failed**
- `extraction::scheduler` 切片：**17 passed / 0 failed**

## 给 Dev 的修复指引

**PASS — 无需修复。**

3 个 MINOR 项均为可选优化建议，不影响本 task 收敛。如有时间且不打破并行 task 范围，可在后续 task 中处理。
