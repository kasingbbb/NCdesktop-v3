# Review Scorecard — task_007_runtime_manifest_self_check

## 审查思考过程

### 1. Task 意图复述
在应用启动阶段实现一次性 `runtime-manifest.json` 自检：读 manifest → 逐项 `python -c "import X"` → 任一失败抛 `E_RUNTIME_MISSING` / `E_EXTRA_MISSING_<X>` → 结果缓存到 `AppState`，后续 `markitdown::extract` 与 `scheduler` 路由前读缓存短路，从而禁用所有转录入口（不是静默失败）。

### 2. AC 检查结果

| AC | 状态 | 关键证据 |
|----|------|----------|
| AC-1 函数签名 + RuntimeManifest 结构 | ✅ PASS | `runtime_check.rs:85` 签名 `fn verify_runtime_manifest(app: &AppHandle) -> Result<RuntimeManifest, FailureCode>` 字符级对齐；`RuntimeManifest` 8 字段与 task_002 实际产物对齐 |
| AC-2 manifest 解析 + 10s 子进程 + 硬编码映射 | ✅ PASS | `load_manifest()` 两个 `map_err → ERuntimeMissing`；`IMPORT_PROBE_TIMEOUT=10s` 硬截止 + `child.kill()` 兜底；`probe_import()` stderr 采集 + `log::warn!`；ebooklib → `EExtraMissingEpub` 映射 + 单测 |
| AC-3 lib.rs setup 一次性调用 + AppState 缓存 | ⚠️ **部分 FAIL** | `lib.rs:58-73` setup hook 调用 1 次 + `app.manage(RuntimeCheckState::new(...))` 缓存 ✅；但 `markitdown::extract` 与 `scheduler` 路由前**无任何**读缓存短路（grep 验证：两文件零 `RuntimeCheckState` / `runtime_check` 引用） |
| AC-4 UI 横幅 + 一键复制诊断 | N/A | 后端 dev 实例 scope 外，基础设施 `RuntimeCheckState::snapshot()` 已暴露；建议拆出独立前端 task |
| AC-5 3 场景单测 | ✅ PASS | 10 单测全过；`missing_ebooklib_returns_extra_missing_epub` / `manifest_missing_returns_runtime_missing` / `all_seven_imports_ok_returns_full_manifest` 字面对应 AC-5 三场景 |
| AC-6 log::info! runtime_id + 耗时 | ✅ PASS | `runtime_check.rs:130-135` `log::info!("[runtime_check] OK runtime_id={} imports={} elapsed_ms={}", ...)`；未写敏感路径 |

### 3. 关键发现

- **核心缺口（AC-3 调用方接入）**：Dev 自报"勿动 markitdown.rs"红线过紧，**未做**「`markitdown::extract` 与 scheduler 路由前读缓存短路」。Conductor 已先行裁决：input.md 「预估影响范围」字面列出 `scheduler.rs`、AC-3 字面要求"路由前读缓存"，因此 1-2 行短路接入属于 task_007 字面授权区间。当前实现仅暴露缓存接口、未接入调用方，AC-3 字面要求未达成。
- **map_import_failure 保守归类合理**：task_008 `FailureCode` 枚举只有 `EExtraMissingEpub` 一项 extras 缺失码，其余 6 项归 `ERuntimeMissing` 是 task_007 不擅扩 task_008 枚举的正确选择；input.md AC-2 字面用"如"举例 ebooklib，可解读为示范。这是合理的、self-consistent 的实现。
- **预存在 2 fail 已消除**：跑 `cargo test --lib` 现在 **181 passed; 0 failed**（Conductor 已同步 V13 断言修订）。
- **红线全部 PASS**：未触动 `audio_asr_iflytek.rs` / `scripts/*` / `verify-venv-shim.sh` / `failure_code.rs` 业务逻辑 / `markitdown.rs::classify_output` / `db/migration.rs` (task_007 commit 不包含) / `db/conversion_meta.rs`；自检失败也不降级到系统 python3（H1 守住）。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 35% | 4 | AC-1/2/5/6 全过；AC-3 缓存基础设施完整但**调用方短路缺失**，扣 1 分。缓存暴露的接口（`RuntimeCheckState::snapshot()`）正确、可消费 |
| 安全性 | 10% | 5 | H1 严守（venv 缺失即 fail，绝不降级 PATH）；子进程 10s 硬超时 + `child.kill()`；stderr 采集 + `log::warn!` 完备；日志无敏感路径（仅 runtime_id 字符串 + 计数） |
| 代码质量 | 10% | 5 | 模块结构清晰（`verify_runtime_manifest` 入口 + `verify_with_paths` 可测内核）；映射表硬编码且单测穷举；注释贴 PR 字面 AC；mutex poison 容错路径合理 |
| 测试覆盖 | 25% | 5 | 10 单测覆盖：完整结构 / ebooklib 失败 / 非 ebooklib 失败 / manifest 不存在 / JSON 错 / 字段缺 / venv 缺 / 映射表 8 项 / snapshot Ok+Err。CI 友好（`make_fake_python` shell 脚本驱动，无需真实 PBS Python） |
| 架构一致性 | 10% | 5 | 严格对齐 ADR-010：模块路径 `extraction/runtime_check.rs`、签名、缓存到 `AppState`、不每次重复探测；manifest 字段集对齐 task_002 实际产物（含 E-2 裁决 `docx→mammoth`） |
| 可维护性 | 10% | 5 | `map_import_failure()` 注释明示扩展点（"未来若枚举扩展（EExtraMissingPdf/...），仅改本表即可"）；`verify_with_paths()` 内核函数对测试和调试都友好 |

**综合分：4.65/5**（加权：0.35×4 + 0.10×5 + 0.10×5 + 0.25×5 + 0.10×5 + 0.10×5 = 1.40 + 0.50 + 0.50 + 1.25 + 0.50 + 0.50 = **4.65**）

---

## 总体判断

- [ ] PASS
- [x] **FIX**（综合分 4.65/5；AC-3 字面要求未达成，但缺口范围明确、修复成本极低、不涉及业务逻辑变更）
- [ ] BLOCKER

判决理由：
- 基础设施（self-check 核心 + AppState 缓存 + 10 单测全过）已工业级；
- 但 AC-3 字面要求「`markitdown::extract` 与 scheduler 路由前读缓存」未做，凭借 Conductor 裁决（input.md 已字面授权 scheduler.rs 修改 + 1-2 行 markitdown 入口短路），这属于 task_007 必须自完成的接入，**不可下放到 task_009/010/011**——否则 UI banner 与"禁用转录入口"无后端配合，违背 PRD §4.3 + ADR-010 "scheduler 消费自检结果"。
- 修复范围极窄（≤30 行 + 2 单测），无架构改动；可在 1 轮内完成。

---

## 问题列表

### BLOCKER
无。

### MAJOR

#### M-1：AC-3 调用方短路接入缺失
- **问题**：`scheduler` 路由分发处 与 `markitdown::extract` 入口未读 `RuntimeCheckState::snapshot()`，自检失败时仍会走完整子进程链。AC-3 字面"markitdown::extract 与 scheduler 路由前读缓存，失败时直接返回错误码不走子进程"未达成。
- **代码位置**：
  - `src-tauri/src/extraction/scheduler.rs`：在 primary 路由分发到具体 extractor 前的入口（PipelineScheduler 主循环或 dispatch 函数）
  - `src-tauri/src/extraction/extractors/markitdown.rs`：`MarkItDownExtractor::extract()` 函数顶部（**仅在入口加 1-2 行**，禁止改动 `classify_output` 等核心判定逻辑——红线）
- **修复方向**：
  1. scheduler 路由分发前（route guard 之后、spawn extractor 之前），获取 `app.state::<RuntimeCheckState>().snapshot()`：
     - 若 `Err(code)` → 直接将该 task 落库为 failed + `failure_code = code.as_str()`，**不**进入子进程；
     - 若 `Ok(_)` → 走原有逻辑。
  2. markitdown.rs `extract()` 入口（在任何 `Command::new(python)` 之前）：若 self-check 结果未注入或为 Err，立即返回 `ExtractionError`（携带 FailureCode）。**仅 1-2 行入口短路，不允许触及 `classify_output` 与子进程业务逻辑**。
- **验证标准**：
  1. `grep -n "RuntimeCheckState\|runtime_check" src-tauri/src/extraction/scheduler.rs src-tauri/src/extraction/extractors/markitdown.rs` 至少各 1 处命中；
  2. 新增至少 2 个单测（或集成测试）：(a) scheduler 在 self-check 失败时不 spawn 子进程、直接落 failed；(b) markitdown.rs 入口在 self-check 失败时立即返回 FailureCode；
  3. `cargo test --lib` 仍 0 fail；
  4. 不增加新依赖，不修改 `classify_output` / `map_import_failure` / `failure_code.rs` / `audio_asr_iflytek.rs` 业务逻辑。

### MINOR

#### m-1：AC-4 UI 横幅边界
- **问题**：input.md AC-4 + "预估影响范围"列出 `src/App.tsx` 是任务范围一部分，但本任务为后端 dev 实例 scope。
- **建议**：Conductor 单独切一个前端 task（`task_007b_runtime_banner_ui` 或纳入 task_012），消费 `RuntimeCheckState` Tauri command 暴露的快照即可。**本任务**不要求 dev 在 fix 中做前端工作。

#### m-2：10s 超时分支无独立运行时单测
- **问题**：dev 报告中标注"构造 sleep-10s 会让 CI 单测耗时翻倍"。
- **建议**：可补一个 `#[ignore]` 慢测（`cargo test --ignored` 跑），或不补——已实现的 `Instant::now() >= deadline + child.kill()` 守卫逻辑由 code review 兜底足够。**非必需**。

#### m-3：`extras_extra` 字段类型与 ADR-010 §4 示例不一致
- **问题**：ADR-010 §4 示例为数组，task_002 实际产物为 object。Dev 用 `serde_json::Value` 兼容已是正确选择，但 ADR-010 文档与实现存在文字偏差。
- **建议**：作为 task_002 残留偏离记录在案，无需在 task_007 处理。

---

## 给 Dev 的修复指引

### 问题清单（按优先级排序）

#### MAJOR
1. **AC-3 短路接入**（参见上文 M-1）
   - **代码位置**：`scheduler.rs` 路由分发入口 + `extractors/markitdown.rs::extract()` 顶部
   - **修复方向**：在两处入口读 `app.state::<RuntimeCheckState>().snapshot()`，`Err(code)` 时**不 spawn 子进程**、立即返回错误（scheduler 落 failed + failure_code，markitdown 返 `ExtractionError`）
   - **验证标准**：见 M-1 4 项验证

### 修复范围约束
- **只修 M-1 一项**，不要连带重构 scheduler/markitdown.rs 其他部分。
- markitdown.rs 入口短路 **仅允许 1-2 行**（在子进程 spawn 前），**严禁**触及 `classify_output` / `map_extractor_error` / Python 调用本体（task_008 业务逻辑红线）。
- scheduler.rs 可在路由分发入口处加 1 个新函数或代码段（≤20 行）。
- **不修改** `failure_code.rs` / `audio_asr_iflytek.rs` / `db/migration.rs` / `db/conversion_meta.rs` / 任何 `scripts/*` / `build.rs`。
- 修复完成后：
  1. 跑 `cargo test --lib extraction::runtime_check`（10 测仍 0 fail）；
  2. 跑 `cargo test --lib`（仍 181 pass, 0 fail，或新增短路单测后 ≥181 pass）；
  3. 新增至少 2 单测验证短路行为（scheduler 端 + markitdown 端各 1）；
  4. 输出更新的 output.md 简短追加"FIX 轮次：M-1 短路接入"段。

---

## R2 复审追加（FIX 第 1 轮 — AC-3 短路接入）

- **复审日期**：2026-05-13
- **R2 判决**：**PASS**

### 验证标准 4 项逐条结论

| # | 验证标准 | 实测结果 |
|---|---------|----------|
| A | grep 双文件命中 ≥1 | **PASS** — `scheduler.rs` 22 命中（use / 主循环短路 / db_get_extract_options 注入 / 2 helper / 2 单测），`markitdown.rs` 9 命中（入口 6 行短路 + 2 单测） |
| B | 新增 ≥2 单测 (实加 4) | **PASS** — 4 测全 PASS：`scheduler::tests::runtime_check_short_circuits_markitdown_on_failure` / `runtime_check_does_not_short_circuit_on_pass_or_non_markitdown` / `markitdown::tests::extract_short_circuits_when_runtime_check_failed` / `extract_does_not_short_circuit_when_runtime_check_ok` |
| C | cargo test --lib 0 fail | **PASS** — 实测 **195 passed; 0 failed; 0 ignored**（baseline R1 = 181，+14：本 FIX 4 测 + 其他 task V14 等增量 10 测） |
| D | 不触业务核心 | **PASS** — `runtime_check.rs` git diff 0 行（完全未触动）；`classify_output` / `python_candidates` / `map_extractor_error` 函数本体未改（短路仅在 `extract()` 前 6 行）；`failure_code.rs` / `audio_asr_iflytek.rs` / `db/migration.rs` / `db/conversion_meta.rs` 在本 FIX 范围内均未改（工作树其他 dirty 是 task_014 累积，与本 FIX 无关） |

### 红线检查

- 未改 `classify_output` / `map_import_failure` / `failure_code.rs` 业务逻辑 → **PASS**
- 未改 `audio_asr_iflytek.rs`（PRD 底线 #4，本 FIX 范围内无 diff）→ **PASS**
- 未改 `runtime_check.rs`（R1 PASS 已交付）→ **PASS**
- 未触 `task_004~006 scripts` / `task_000 区脱敏` / `task_003 verify-venv-shim.sh` / `db/migration.rs` / `db/conversion_meta.rs` → **PASS**
- `markitdown::extract` 内**未**重复探测 manifest（仅消费 `options.runtime_check_failed`，由 scheduler 在路由前从 `RuntimeCheckState` 注入）→ **PASS**
- cargo test --lib 195 ≥ 195 baseline，0 fail → **PASS**

### 综合分调整

- R1 综合分：4.65/5（功能正确性 4 分，因 AC-3 调用方短路缺失扣 1 分）
- R2 FIX 闭合后：功能正确性 4 → 5（AC-3 字面要求达成：scheduler 路由前 + markitdown::extract 入口前各 1 处短路，失败时不进子进程；4 单测覆盖 PASS + 反例边界）
- **R2 综合分：5.00/5**（加权：0.35×5 + 0.10×5 + 0.10×5 + 0.25×5 + 0.10×5 + 0.10×5 = 5.00）

### 关键工程亮点（FIX 轮）

1. **设计纯函数 `runtime_check_short_circuit(name, options)` 单测友好**：决策不依赖 AppHandle / DB / Tokio runtime，可直接断言 markitdown 路由+失败 → Some(code)，pdf_text/text_passthrough/audio_asr_iflytek+失败 → None（fallback 链不受 markitdown-venv 自检阻断，语义正确）。
2. **`try_state::<RuntimeCheckState>()` 兜底**：若 AppState 未 manage（如单测路径），返回 None → `runtime_check_failed = None` → 不影响测试。
3. **markitdown 入口短路 = 防御性双锚**：即使 scheduler 未注入（未来命令直调 extractor），自检失败状态仍正确阻断子进程。
4. **scheduler 短路路径写双锚 `error_class` + `failure_code`**：向后兼容 task_007 历史消费方（前端可能读 error_class），同时落 task_008 显式 failure_code 列。

### 最终判决

**PASS** — MAJOR-1 (AC-3 短路接入) 完整闭合；A/B/C/D 四项验证全 PASS；红线零命中；task_007 进入 **PASS 终态**。

---

## 自检清单
- [x] 逐条 AC 检查（AC-1 至 AC-6 + 红线全列表）
- [x] 检查了 session_context.md 领域审查重点（嵌入 Python rpath / runtime-manifest 一致性 / 子进程超时 + stderr / 音频路由）
- [x] M-1 给出具体代码位置 + 修复方向 + 4 项验证标准
- [x] 评分诚实（基础设施 5 分但 AC-3 未达成，功能正确性 4 分）
- [x] 修复指引清晰（≤30 行改动 + 2 单测 + 4 项验证）
