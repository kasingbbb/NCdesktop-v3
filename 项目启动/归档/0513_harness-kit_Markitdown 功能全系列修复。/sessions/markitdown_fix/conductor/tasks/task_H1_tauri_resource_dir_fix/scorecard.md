# Review Scorecard — task_H1_tauri_resource_dir_fix

## 审查思考过程

1. **Task 意图**：修复 `cargo tauri dev` 模式下 `app.path().resource_dir()` 返回 `target/<profile>/` 而非 `src-tauri/resources/`，导致 `runtime-manifest.json` ENOENT、所有 PDF/PNG 转录被 `E_RUNTIME_MISSING` 短路的根因。手段：(a) `tauri.conf.json bundle.resources` 加 manifest 让 prod build 自动拷贝；(b) `runtime_check.rs` 加 `#[cfg(debug_assertions)]` 的 dev fallback 到 `CARGO_MANIFEST_DIR/resources/`。
2. **AC 检查结果**：AC-1 ✅ / AC-2 ✅ / AC-3 ✅（runtime_check.rs 内 log 等价覆盖，input.md 明确允许） / AC-4 ✅（3 新单测均通） / AC-5 ⏳ PENDING-USER-MACHINE（合理） / AC-6 ✅
3. **关键发现**：
   - `select_runtime_paths` 纯函数抽象到位 — 签名 `(&Path, Option<&Path>) → (PathBuf, PathBuf, bool)` 完全消除 `AppHandle` 依赖，单测注入双路径毫不费力。
   - `#[cfg(debug_assertions)]` 配合 `env!("CARGO_MANIFEST_DIR")` 注入选择 — release 编译时 `dev_fallback=None`，编译器消除分支；release 模式 `cargo test --lib --release` 全 13 测通过验证 dead-code 消除无副作用。
   - 红线零越界：tauri.conf 仅加 `runtime-manifest.json`（4KB），未把 `markitdown-venv/` (363M) 塞进 bundle.resources。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~4 + AC-6 全部满足；AC-5 由用户机实测落地合理；dev fallback 路径决策逻辑无副作用、可串到 verify_with_paths 走通 7 imports；prod 路径解析逻辑零退步 |
| 安全性 | 25% | 5 | 无新增依赖；fallback 仅 debug_assertions 启用，prod 编译期消除无注入风险；fallback 候选路径仍走 `is_file()` 检查不 panic；未触红线文件（audio_asr/failure_code/scheduler/markitdown/sync）。 |
| 代码质量 | 15% | 5 | `select_runtime_paths` 纯函数职责单一、文档注释充分；`verify_runtime_manifest` 流程清晰；log 字面同时含 resource_dir/manifest_path/venv_python 三要素，定位 1 行 log 可达 |
| 测试覆盖 | 15% | 5 | 3 新单测覆盖 fallback 命中 / 双缺 / None 边界三种关键场景；现有 10 测保留不破；端到端跑 verify_with_paths 验证 fallback 路径上的产物可用（不仅"选对"）；全量 229/0 PASS |
| 架构一致性 | 10% | 5 | runtime_check.rs 内部新增 private helper 不污染对外接口；RuntimeManifest schema 不变；tauri.conf 路径位置不变；未引入计划外依赖 |
| 可维护性 | 10% | 5 | dev/prod cfg 分支注释含"prod 不 fallback 避免误覆盖 bug"的设计 rationale；select_runtime_paths 文档列出三条决策规则；未来若新增 fallback 候选目录，扩展点清晰 |

**综合分：5.0/5**（加权 = 5×0.25 + 5×0.25 + 5×0.15 + 5×0.15 + 5×0.10 + 5×0.10 = 5.00）

## 总体判断

- [x] **PASS**（AC-5 PENDING-USER-MACHINE 合理；dev 工作机不能跑全 Tauri 启动链是事实约束）

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **`select_runtime_paths` 命名**：当前签名将 venv_python 子路径硬编码为 `markitdown-venv/bin/python`；若未来 venv 目录名变更需双处改动。可选改进：抽出常量 `MARKITDOWN_VENV_PYTHON_REL = "markitdown-venv/bin/python"`。**非阻塞**。
2. **dev fallback 命中后未对 venv_python 做 `is_file()` 验证**：`select_runtime_paths` 只检查 manifest 存在就切换 fallback，venv 是否存在由下游 `verify_with_paths` 探测。逻辑正确（错误码统一为 `ERuntimeMissing`），但 log 链条上 fallback 命中时若 venv 还是缺则会先打 "dev fallback ..."、再打"venv python 不存在"两行 — 可读但不是单 1 行 log 定位。**非阻塞**。
3. **AC-3 用 runtime_check.rs 内 log 等价覆盖 lib.rs**：input.md 原文允许该简化，dev output.md 第 177 行有明确偏离说明。审查认可。**非阻塞**。

## 红线核查

| 红线项 | 状态 |
|--------|------|
| 修改 `build-macos-dmg.sh` | ✅ 未触（git diff 干净） |
| 修改 `prepare-embedded-*.sh` | ✅ 未触 |
| 修改 `sign-bundle.sh` / `notarize.sh` / `vm-smoke.sh` | ✅ 未触 |
| 修改 `audio_asr_iflytek.rs` (PRD 底线#4) | ✅ 未触 |
| 修改 `failure_code.rs` | ✅ 未触 |
| 修改 `scheduler.rs` | ✅ 未触 |
| 修改 `extractors/markitdown.rs` | ✅ 未触 |
| 修改 `commands/sync.rs guess_mime` (task_H2 范围) | ✅ 未触 |
| tauri.conf bundle.resources 加入 `python/` 或 `markitdown-venv/` | ✅ 未触（仅加 runtime-manifest.json 4KB） |
| 引入新依赖 | ✅ 未触（cargo check 无新 crate） |
| prod 启用 dev fallback 绕过 bug | ✅ 未触（`#[cfg(debug_assertions)]` 严格隔离，release build 13 测通过验证） |
| 破坏 verify_with_paths 探针逻辑 | ✅ 未触（task_007 PASS 核心保留） |

**红线全过：YES**

注：`git status` 显示 `lib.rs / sync.rs / scheduler.rs / extractors/markitdown.rs / audio_asr_iflytek.rs` 等带 `MM` 标记，属并行 task（H2、baseline）的工作树状态，**与 task_H1 范围无关**。`git diff -- runtime_check.rs tauri.conf.json` 输出仅显示 tauri.conf 的 3 行 +（manifest 项）；runtime_check.rs 是新文件（`??`）。dev output.md 第 26 行已主动声明此情况。

## 4 关注点结论

1. **`env!("CARGO_MANIFEST_DIR")` 编译期常量在他人机器**：dev output.md 关注点 #1 已说明 — 每个开发者本机编译都拿到本机绝对路径，`cargo run` 时正确；分发的 dev 构建若拷到第三机器跑，fallback 路径不存在时 `is_file()` 静默返回 false 走默认路径 → 双都缺 → `ERuntimeMissing`（不 panic）。**安全。**
2. **dev fallback 触发后 log 内容**：`log::info!` 字面同时打印 `resource_dir` / `manifest_path` / `manifest_path.parent()`（即 fallback dir） / `venv_python`。一行 log 即可对比 resource_dir → fallback dir 两侧路径。**充分。**
3. **`select_runtime_paths` 纯函数设计合理性**：签名 `(&Path, Option<&Path>) → (PathBuf, PathBuf, bool)` 完全消除 `AppHandle` 依赖，3 个单测均无需 mock 任何 Tauri 类型。不是过度抽象 — 是把"路径决策"从"运行时状态"中剥离的标准重构。**合理。**
4. **prod 编译 `#[cfg(debug_assertions)]` 剥离验证**：`cargo test --lib --release` 全 13 测通过（无 `dev_fallback unused` warning，编译器正确消除）。`select_paths_no_fallback_when_dev_fallback_is_none` 单测专门验证 prod 等价语义。**确认 prod 二进制不含 fallback 代码。**

## cargo test 实测数字

```
test result: ok. 229 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.96s
```

dev 报 229 ✅ 与实测匹配。release 模式 13/13（filtered out 216 是非 runtime_check 路径，正常）。

## env! 路径推断的安全性判定

**安全。** 三层保护：
- `is_file()` 检查 — fallback 路径不存在不会 panic，静默回到默认路径；
- `#[cfg(debug_assertions)]` — release 编译不注入；
- 双都缺时统一 `ERuntimeMissing` — 错误码语义清晰，UI 层显示"运行时缺失"。

## select_runtime_paths 纯函数设计合理性

**合理。** 该函数职责单一（路径决策无 IO 外的副作用）、签名极简（两个路径输入 + 一个三元组输出）、可单测（不需 Tauri AppHandle mock）、可扩展（未来加候选目录只需扩 fallback 链）。非过度抽象。

## scorecard 落盘

YES — 写入 `…/task_H1_tauri_resource_dir_fix/scorecard.md`
