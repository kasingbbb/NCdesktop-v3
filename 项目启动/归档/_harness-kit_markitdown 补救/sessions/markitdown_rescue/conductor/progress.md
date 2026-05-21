# Conductor Progress — markitdown_rescue

## 当前状态
STATE: ACCEPTANCE
当前 Task: 全部 11 个 task 完成，等待 PM 验收
更新时间: 2026-05-12

## 已完成 Tasks
- [x] task_001_architect — 技术方案 + 6 个 ADR + 11 个 Dev/Review task 拆分
- [x] task_002_dev_asset_model — Asset 加字段 + V5 迁移 · Reviewer PASS 4.775/5 · 0 BLOCKER 0 MAJOR · 跨 task 登记 M-1（scheduler 注释）
- [x] task_003_dev_db_asset_funcs — db/asset.rs 3 fn + 4 单测 · Reviewer PASS 4.75/5 · 0 BLOCKER 0 MAJOR · 复用 ASSET_SELECT 常量，列序一致性达成
- [x] task_004_dev_db_tag_funcs — db/tag.rs 2 fn + dropzone sync · Reviewer PASS 4.85/5 · 0 BLOCKER 0 MAJOR · R6 单点实现达成（INSERT INTO asset_tags 仅 3 处）
- [x] task_005_dev_conversion_abstraction — extraction/conversion.rs（ConversionAttempt + file_sha256 + classify_error）· Reviewer PASS 5.00/5 · 0 BLOCKER 0 MAJOR · 偏离登记 M-2（sha2 实际未在 Cargo.toml，Dev 已合理补全）
- [x] task_006_dev_conversion_meta — conversion_meta 表 V6 + CRUD（无 UNIQUE，append-only）· Reviewer PASS 5.0/5 · 0 BLOCKER 0 MAJOR · 跨 task 字段一致性 OK
- [x] task_007_dev_markitdown_enrich — MarkItDownExtractor 版本缓存 + error_class 前缀 + embedded venv 优先 + 7 测试 · Reviewer PASS 4.70/5 · 0 BLOCKER 0 MAJOR
- [x] task_008_dev_scheduler_fallback — **M-1 关闭** · scheduler 主循环 fallback 编排 · write_placeholder_md 拆分 · 决策纯函数 decide_next_step · 41 extraction 测试 PASS · Reviewer PASS 4.6/5 · 0 BLOCKER 0 MAJOR · 自主修复 3 处 mod.rs 注册缺口（与 M-1 同类，Reviewer 判定合理）
- [x] task_009_dev_get_conversion_meta_cmd — get_conversion_meta Tauri 命令 + TS 类型 + 注册 · Reviewer PASS 4.85/5 · 0 BLOCKER 0 MAJOR · 与 PM 手改前端零冲突（git diff 仅 + 行）
- [x] task_010_dev_inspector_meta — InspectorExtraction 三态展示 + extractionStore.fetchConversionMeta · Reviewer PASS 4.6/5（92/100）· 0 BLOCKER 0 MAJOR · 冲突 guard 严守，仅动 2 个白名单文件
- [x] task_011_dev_retrigger_extraction — retrigger_extraction 命令 + 前端 wiring + V7 pipeline_tasks 迁移 · Fix 一轮闭环 · 二审 PASS 90/100 · 修复了 task_008 遗留的 lib.rs manage(PipelineScheduler) 缺失 + pipeline_tasks 表 DDL 缺失
- [x] task_012_ux_review_knowledge — 知识下游审计报告 · Reviewer PASS 4.75/5 · Q1 ✅ / Q2 ❌ / Q3 ❌ / Q4 ✅ / Q5 ✅ · 关键发现：FTS 不索引正文（搜索功能与 markitdown 集成脱节）+ concept-extract-requested 事件死信（自动抽取链路未启用）

## 当前 Task 详情
Task ID: task_001_architect
描述: Architect 角色再次回放：基于宪章 v1.0 + 现有代码 review，输出技术方案与 task 清单
状态: DONE（等待 PM 确认进入 Dev 循环）
交付物路径:
  - sessions/markitdown_rescue/conductor/tasks/task_001_architect/output.md
  - sessions/markitdown_rescue/conductor/tasks/task_002_dev_asset_model/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_003_dev_db_asset_funcs/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_004_dev_db_tag_funcs/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_005_dev_conversion_abstraction/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_006_dev_conversion_meta/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_007_dev_markitdown_enrich/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_008_dev_scheduler_fallback/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_009_dev_get_conversion_meta_cmd/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_010_dev_inspector_meta/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_011_dev_retrigger_extraction/input.md
  - sessions/markitdown_rescue/conductor/tasks/task_012_ux_review_knowledge/input.md

## 待执行 Task 队列（按拓扑顺序）
- [ ] task_002_dev_asset_model           — Asset 加字段 + assets 表迁移
- [ ] task_003_dev_db_asset_funcs        — db/asset.rs 新增 3 函数
- [ ] task_004_dev_db_tag_funcs          — db/tag.rs 新增 2 函数 + dropzone 接 sync
- [ ] task_005_dev_conversion_abstraction — extraction/conversion.rs（含工具函数，不含新 trait）
- [ ] task_006_dev_conversion_meta       — conversion_meta 表 + CRUD
- [ ] task_007_dev_markitdown_enrich     — MarkItDown 适配器版本缓存 + classify_error + embedded venv 优先
- [ ] task_008_dev_scheduler_fallback    — scheduler 主循环 fallback + placeholder 拆分（**关键路径**）
- [ ] task_009_dev_get_conversion_meta_cmd — Tauri 命令
- [ ] task_010_dev_inspector_meta        — Inspector 展示三态
- [ ] task_011_dev_retrigger_extraction  — 重试命令统一
- [ ] task_012_ux_review_knowledge       — 知识下游审计（review-only）

## 并行机会
- {task_003, task_004} 可在 task_002 完成后并行
- {task_005, task_006} 可在 task_004 完成后并行
- {task_010, task_011} 可在 task_009 完成后并行

## 已知问题 / Blockers
- B1：`项目启动/NCdesktop/src-tauri/` 当前 `cargo check` 不通过——task_002~004 完成前**禁止**合并任何其他改动。
- B2：`src-tauri/` 整体在 git 中未跟踪——PM 需决定在 task_002 启动前是否先 `git add -N src-tauri/`，以便后续 task 可以分 PR 提交。

## 关键决策记录
- [2026-05-12 ADR-001] Asset 加字段（不新建联结表）
- [2026-05-12 ADR-002] derivative_version 双写 source + derivative，仅真成功路径推进
- [2026-05-12 ADR-003] Fallback 决策搬到 scheduler 主循环，不在 get_extractor_for 内
- [2026-05-12 ADR-004] conversion_meta 与 extracted_content 并列；append-only，不加唯一约束
- [2026-05-12 ADR-005] 保留 Extractor trait，不新建 Converter trait
- [2026-05-12 ADR-006] placeholder 不推进版本号、不写 extracted_content

## 状态转移日志
- [2026-05-12] STATE: INIT → ARCHITECTURE | 原因: 用户要求按 harness-kit 流程重跑 architect | 风险: 低
- [2026-05-12] STATE: ARCHITECTURE → ARCHITECTURE_DONE | 原因: output.md + 11 个 input.md + progress.md 全部产出 | 风险: 低（待 PM 确认 ADR）
- [2026-05-12] STATE: ARCHITECTURE_DONE → TASK_START_PENDING_PM_APPROVAL | 原因: PM 指示 Conductor 启动开发；task_002 为 L 级（schema 变更），按协议输出 Impact Summary 等待 PM 确认 | 风险: 中
- [2026-05-12] STATE: TASK_START_PENDING_PM_APPROVAL → DEVELOPING | 原因: PM 批准 task_002 dispatch；强调"严格按 architect 流程，不跳过 Reviewer" | 风险: 中
- [2026-05-12] STATE: DEVELOPING(task_002) → REVIEWING(task_002) | 原因: Dev 交付完成，契约验证通过 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_002) → DEVELOPING(task_003) | 原因: Reviewer PASS 4.775/5；无 BLOCKER/MAJOR；跨 task M-1 已回填到 task_003/004/008 input.md | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_003) → REVIEWING(task_003) | 原因: Dev 交付完成 0 error 4 测试通过 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_003) → DEVELOPING(task_004) | 原因: Reviewer PASS 4.75/5；M-1 调研结果证实 scheduler 只在一处声明，task_008 关闭路径清晰 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_004) → REVIEWING(task_004) | 原因: Dev 交付完成 0 error 3 测试通过；grep 自检证明 3 处 INSERT | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_004) → FAN_OUT_DECISION_PENDING_PM | 原因: Reviewer PASS 4.85/5；critical path 前 4 步全清；架构方案 §十一 允许 task_005 / task_006 并行，按 Conductor 与 PM 先前约定在此停下等 PM 决策 | 风险: 低
- [2026-05-12] STATE: FAN_OUT_DECISION_PENDING_PM → DEVELOPING(task_005) | 原因: PM 选择 B（保守顺序策略）；task_005 → task_006 串行 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_005) → REVIEWING(task_005) | 原因: Dev 交付，发现 M-2（sha2 fact error），补 Cargo.toml | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_005) → DEVELOPING(task_006) | 原因: Reviewer PASS 5.00/5；M-2 偏离判定合理；PM "no stopping" 指令生效，继续推进 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_006) → REVIEWING(task_006) | 原因: Dev 交付 0 error 4 测试通过 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_006) → DEVELOPING(task_007) | 原因: Reviewer PASS 5.0/5；UNIQUE 复核通过 ADR-004 严格落实 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_007) → REVIEWING(task_007) | 原因: Dev 交付 0 error 7 测试通过 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_007) → DEVELOPING(task_008) | 原因: Reviewer PASS 4.70/5；按 PM "no stopping" 指令绕过 L 级 PM 闸门进入关键路径 task_008（M-1 关闭点） | 风险: 中（scheduler 取消注释后首次重新参与编译）
- [2026-05-12] STATE: DEVELOPING(task_008) → REVIEWING(task_008) | 原因: Dev 完成 M-1 关闭 + fallback + placeholder 拆分；自主修复 3 处额外 mod.rs；41 测试 PASS | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_008) → DEVELOPING(task_009) | 原因: Reviewer PASS 4.6/5；M-1 真实关闭通过亲验；3 处自主修复判定合理；M-3 已登记 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_009) → REVIEWING(task_009) | 原因: Dev 交付 3 文件 0 error，零冲突 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_009) → DEVELOPING(task_010) | 原因: Reviewer PASS 4.85/5；进入前端 task_010 | 风险: 低（含与 PM 手改 31 文件的潜在冲突 guard）
- [2026-05-12] STATE: DEVELOPING(task_010) → REVIEWING(task_010) | 原因: Dev 仅动 2 个白名单文件 tsc 0 error | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_010) → DEVELOPING(task_011) | 原因: Reviewer PASS 4.6/5；冲突 guard 严守通过亲验 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_011) → REVIEWING(task_011) | 原因: Dev 交付 cargo check 0 error 3 单测过；自主修复 commands/mod.rs 注册缺口 | 风险: 中（Dev 声明 scheduler 已 manage 未亲验）
- [2026-05-12] STATE: REVIEWING(task_011) → FIX_PENDING(task_011) | 原因: Reviewer FIX 72/100；1 BLOCKER（lib.rs::setup 未 manage(PipelineScheduler) 运行时 panic）+ 1 MAJOR（pipeline_tasks 生产 migration 缺失） | 风险: 高（happy path 不可用） | 累积异常: 首次 FIX
- [2026-05-12] STATE: FIX_PENDING(task_011) → DEVELOPING(task_011-fix) | 原因: PM 指示 "继续"；dispatch Dev Fix Mode | 风险: 中
- [2026-05-12] STATE: DEVELOPING(task_011-fix) → REVIEWING(task_011-fix) | 原因: Dev Fix 完成 manage(PipelineScheduler) + V7 pipeline_tasks 迁移；根因分析填齐 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_011-fix) → DEVELOPING(task_012) | 原因: Fix 二审 PASS 90/100；BLOCKER + MAJOR 全部修复通过亲验；task_006 / task_008 测试零回归 | 风险: 低
- [2026-05-12] STATE: DEVELOPING(task_012) → REVIEWING(task_012) | 原因: Dev 审计报告交付，5 问题全答 + 数据流图 + P-1/P-2/P-3 骨架 | 风险: 低
- [2026-05-12] STATE: REVIEWING(task_012) → ACCEPTANCE | 原因: Reviewer PASS 4.75/5；独立复核 Q2 / Q3 ❌ 项通过；session 全部 11 task 完成 | 风险: 低

## 跨 session 后续工作（建议 PM 开新 session 处理）
- **P-1（高优先级）**：搜索 FTS 增加 `structured_md` 索引 + 结果按 source_asset_id 去重。**这是 markitdown 集成的"用户感知 0"问题——不修则 task_008 落地的 .md 衍生件用户搜不到内容。**
- **P-2（高优先级）**：实现 `notecapt/concept-extract-requested` 监听器，让 task_008 已经 emit 的事件真正驱动知识抽取。推荐方案 A（后端 tokio::spawn）。
- **P-3（中优先级）**：补 placeholder 写入路径的 status 防御测试，确保 `extracted_content.status` 不被 placeholder 路径误置为 'extracted'。
- **M-3 遗留**：12 个 pre-existing test failure（concepts 表缺失，V3 缺迁移）。
- **PM 手改的 31 个前端文件**：仍在工作树为 modified 状态未提交，待 PM 自行决定 commit 策略。

## 跨 Task 待办登记（Conductor 维护）
- **M-1**：`src/extraction/mod.rs:4` 当前注释了 `pub mod scheduler;`。Reviewer 在 task_002 审查时发现这导致 scheduler.rs 不参与编译。已在 task_003/004/008 input.md 末尾追加"取消注释 + 再次 cargo check"动作。task_008 完成后此项关闭。
- **M-2**：session_context.md §2 与多个 input.md 错误声明"sha2 已在 Cargo.toml"，实际只有 sha1。Dev 在 task_005 中合理补全 `sha2 = "0.10"`。task_008 取消 scheduler 注释后 scheduler.rs:560 `use sha2::{Digest, Sha256}` 因此可以编译通过，**task_008 无需再处理 sha2**。本项已在 task_005 时事实关闭，仅作记录。
- **M-3**（新登记）：pre-existing 12 个测试失败位于 `db::co_occurrence` 和 `db::knowledge`，错误为 `no such table: concepts`。源头在 `db/migration.rs:124` 注释明示"V3 基表未在源码中存在"。**不属于本 session 任何 task 的引入**，但需要后续 PR 独立处理（补 V3 migration 或对相关测试加 `#[ignore]`）。当前不影响 markitdown 集成功能。
- **M-1 已关闭**（2026-05-12 task_008）：`extraction/mod.rs:9` 真实改为 `pub mod scheduler;`，cargo check 0 error，scheduler 重新参与编译。

## 关键决策记录（追加）
- [2026-05-12] PM 手动改动了 31 个前端 UI 文件（Inspector/Sidebar/Toolbar/TagTree/AssetListView 等）。Conductor 已扫描，确认与本 session 的 backend task 零冲突；task_010 已加冲突 guard 禁止触碰 PM 手改文件。

## 下一步（PM 行动项 — 当前等待）
PM 确认下方 Impact Summary 后，Conductor 将 dispatch Dev agent 执行 task_002。
