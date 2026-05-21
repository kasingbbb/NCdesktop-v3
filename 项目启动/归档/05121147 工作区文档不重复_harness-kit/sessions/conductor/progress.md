# Conductor Progress

## 当前状态
STATE: ACCEPTANCE（P0 MVP 全部完工 + HOTFIX V11 已就绪）
当前 Task: 等待用户重启验证 V11 migration
更新时间: 2026-05-13

## 已完成 Tasks
- [x] session_context.md 填写完毕
- [x] 复杂度判定：M
- [x] Layer 1 辩论（问题定义）→ 3 层模型 + 四态矩阵 + 8 条成功标准
- [x] Layer 4 辩论（策略）→ MVP P0/P1/P2 分期 + 6 条关键技术决策
- [x] PRD v1 产出：`product/prd/workspace_unified_md_prd_v1.md`
- [x] task_001_architect — 基于 PRD v1 产出技术方案（含 8 条 ADR / 9 个 dev task 切分）
- [x] task_002_dev_m0_atomic_import — Review PASS 4.55/5（ADR-006 失败语义实现严格，无 BLOCKER/MAJOR，仅 MINOR）
- [x] task_003_dev_m1m2_list_state — Review PASS 4.65/5（list_root_assets 单查询 + 窗口函数取最近一条；compute_asset_state 纯函数；DTO 双侧对齐；2 处偏离均合理）
- [x] task_004_dev_m3_asset_id_commands — Review PASS 4.65/5（resolve_asset_pair 双向解算；rename 双写 root+derivative 不动磁盘；UTF-8 字节校验精确；前端旧 updateAsset 已下线）
- [x] task_005_dev_m4_outbound_payload — Review PASS 4.65/5（PRD §4.4 6 规则 sanitize 全覆盖；EXDEV fallback；缓存目录幂等重建；OutboundError camelCase JSON 前后端对齐；MINOR：缓存目录 helper 未公开，留 task_006 接入）
- [x] task_007_dev_m7_source_scan — Review PASS 4.75/5（SourceMissingSet 归位到 source_scan.rs；task_003 偏离 (a) 已修；RwLock+不阻塞 setup；3 单测全 PASS；MINOR：Cargo.toml 残留 notify="8" 待后续清理）
- [x] task_006_dev_m5m6_retry_delete — Review PASS 4.55/5（retry 薄包装 + V7 索引兜底 ≤1 活动态；delete_with_cascade 七类数据零孤儿 + 物理文件断言；outbound cache 锁后清；架构发现 source_asset_id 无 FK 需显式 DELETE derivative）
- [x] task_008_frontend_integration — Review PASS 4.45/5（DTO 切换彻底；4 态徽章 + data-state；mousedown 同步 kick prepareOutboundPayload；MixedStates 提前实现加分；唯一 MAJOR：useDragAssets 测试仅覆盖 1 个 OutboundError 变体，task_009 补 3 个参数化）
- [x] task_009_integration_tests — Review PASS 4.45/5（S1–S5+S7–S8 七测全过；bench best=106ms 远低 200ms warn；前端补 3 变体；揭示 outbound "新名.md.md" 缺陷→ FIX_001 已修复 4.85/5）
- [x] task_005 FIX_001 — outbound_filename_from_root：剥扩展名 + sanitize stem + 长度预算 188+3+9；s2 集成测试升级为正向断言；PRD §S2 三处一致语义=stem 一致
- [x] task_010_ux_review — **ESCALATE**：启发式 3.4/5、总体 3.2/5；2 BLOCKER + 5 MAJOR；详见 ux_review.md
- [x] task_011_ux_blocker_fix — Code Review PASS 4.68/5 + UX Round 2 PASS 4.4/5（启发式 4.0/5）；2 BLOCKER 全解除（查看原文件菜单项 + reveal_source_file Tauri 命令 / 列表行 source-missing 角标双触点）；5 MAJOR 全解除（重试 1s 防抖 / cursor: not-allowed / toast dedupe 3s / RenameAssetModal UTF-8 字节计数 / 键盘 Enter F2 Backspace Delete）；仅 1 MINOR：右键删除 window.confirm 与键盘 Delete 中文 Modal 路径不一致

## 当前 Task 详情
Task ID: 无（等待 PM 验收）
描述: P0 M0–M7 + 集成测试 + UX 修复全部完工，进入 ACCEPTANCE
状态: AWAITING_ACCEPTANCE
输入契约: 三个 task 的 input.md 均已就位

## 待执行 Task 队列
- [~] task_004_dev_m3_asset_id_commands（并行批次）
- [~] task_005_dev_m4_outbound_payload（并行批次）
- [~] task_007_dev_m7_source_scan（并行批次） — M1 折叠列表 + M2 四态聚合（list_root_assets / compute_asset_state）
- [ ] task_004_dev_m3_asset_id_commands — M3 rename / resolve_asset_pair 等命令 asset_id 化
- [ ] task_005_dev_m4_outbound_payload — M4 outbound MD payload（sanitize + hardlink/copy + 缓存目录）
- [ ] task_006_dev_m5m6_retry_delete — M5 重试入口 + M6 删除级联（root+derivative+两文件+meta+pipeline_tasks+outbound cache）
- [ ] task_007_dev_m7_source_scan — M7 启动期 source 扫描 + 内存 SourceMissingSet
- [ ] task_008_frontend_integration — 前端 AssetListView 切 DTO / 拖拽禁用 / 重试按钮 / 中文文案
- [ ] task_009_integration_tests — 集成测试覆盖 S1/S2/S3/S4/S5/S7/S8（PRD §8）
- [ ] task_010_ux_review — UX 体验审查（状态文案、键盘可达、错误提示一致性）

依赖拓扑：关键路径 task_002 → task_003 → task_006 → task_009 → task_010；task_004 / task_005 / task_007 在 task_003 完成后可并行；task_008 依赖 task_003+task_004+task_005。

## 已知问题 / Blockers
- ⚠️ 仓库层面 pre-existing 合并冲突（`git ls-files -u` 显示 `src/components/layout/*` Inspector/Sidebar/SidebarFooter/SidebarItem/Toolbar 5 文件 + `src/styles/*.css` 未合并）。与本期 P0 task_002–008 改动零交集，**不阻塞** Rust 集成测试 task_009；但会阻塞 task_010 UX 评审中"整页拖拽 + 侧栏"端到端验证。建议在 task_010 前先由用户解决该批冲突，或下一会话单独 task 处理。
- ⚠️ Cargo.toml 残留 `notify = "8"` 未使用依赖（task_007 reviewer 发现，pre-existing），建议后续清理。

## 关键决策记录
- 2026-05-13 复杂度=M：核心 UX 改动 + 中等技术不确定性 + 数据模型演进，但任务规模与安全敏感度均不高。
- 2026-05-13 Layer 1：采用 Asset / Primary Rendition / Source Material 三层模型，MVP 锁 1:1，命令以 asset_id 为唯一标识。
- 2026-05-13 Layer 4：本期零新增 migration；查询单一入口 `list_root_assets()`；outbound 采用 hardlink + copy fallback + 双 representation。
- 2026-05-13 Architect 决议：ADR-005 outbound 缓存目录 = `~/Library/Caches/NCdesktop/outbound/{asset_id}/`，幂等重建；ADR-003 状态实时派生（4 表 LEFT JOIN，硬回退阈值 200ms @ 10k）；ADR-006 M0 enqueue 失败不回滚 asset 行，状态派生为 offline，由 M5/M9 兜底。
- 2026-05-13 task_006 架构发现：`assets.source_asset_id` 在 V5 通过 `ALTER TABLE ADD COLUMN` 添加，SQLite 限制无法附加 FK 约束。删除级联必须**显式** `DELETE FROM assets WHERE id = derivative_id`，不能依赖 FK CASCADE。本期已在 `delete_with_cascade` 内实现；如未来重建 schema，可考虑迁移到带 FK 的列。

## 状态转移日志
[2026-05-13] STATE: INIT → DEBATING | 原因: session_context.md 已就绪 | 风险: 低
[2026-05-13] STATE: DEBATING → ARCHITECTURE | 原因: PRD v1 已交付，包含 Conductor 桥接摘要 | 风险: 低
[2026-05-13] STATE: ARCHITECTURE → DEV_DISPATCH | Task: task_001_architect 完成（output.md + 9 份 input.md 全部交付，8 条 ADR 覆盖 PRD §10 三争议点） | 原因: Architect 方案验收通过，可启动 dev 序列 | 风险: 低
[2026-05-13] STATE: DEV_DISPATCH → REVIEW → DEV_DISPATCH | Task: task_002 Dev 交付（commands/dropzone.rs，~250 行含测试，2 单测全 PASS）→ Reviewer 综合 4.55/5 PASS | 原因: ADR-006 失败语义实现严格，零 BLOCKER/MAJOR | 风险: 低 | 备注: 包名修正 -p app_lib → -p notecapt，已应用到后续 task
[2026-05-13] STATE: DEV_DISPATCH → REVIEW → DEV_DISPATCH | Task: task_003 Dev 交付（db/asset.rs + models/asset.rs + commands/asset.rs + src/types/workspaceAsset.ts，~600 行）→ Reviewer 综合 4.65/5 PASS | 原因: list_root_assets 单查询设计正确，compute_asset_state 纯函数化彻底，DTO 双侧对齐；2 处偏离均合理 | 风险: 低
[2026-05-13] STATE: DEV_DISPATCH | 并行批次启动 task_004 / task_005 / task_007（依赖拓扑允许三者并行） | 风险: 中（实际改为顺序执行，避免 lib.rs 等共享文件竞态）
[2026-05-13] STATE: DEV_DISPATCH → REVIEW → DEV_DISPATCH | Task: task_004 PASS 4.65/5 → task_005 PASS 4.65/5 → task_007 PASS 4.75/5 → task_006 PASS 4.55/5 → task_008 PASS 4.45/5 → task_009 PASS 4.45/5 + FIX_001 PASS 4.85/5 | 原因: P0 M0–M7 全部完成，集成测试 + bench 全过 | 风险: 低
[2026-05-13] STATE: DEV_DISPATCH → REVIEW → ESCALATE | Task: task_010 UX 审查 总体 3.2/5，2 BLOCKER + 5 MAJOR | 原因: PRD §2.2 场景 5 "查看原文件" UI 入口缺失；M7 后端 source_missing 字段前端 0 消费 | 风险: 高（核心用户旅程不可达）
[2026-05-13] STATE: ESCALATE → DEV_DISPATCH | PM 决策 A：启动 task_011_ux_blocker_fix 修 2 BLOCKER + 5 MAJOR | 风险: 低
[2026-05-13] STATE: DEV_DISPATCH → REVIEW → ACCEPTANCE | Task: task_011 Dev 完成（新增 RenameAssetModal + reveal_source_file Tauri 命令 + 6 处前端改造，31 测全过）→ Code Review PASS 4.68/5 → UX Round 2 PASS 4.4/5 | 原因: ESCALATE 解除，2 BLOCKER + 5 MAJOR 全清，P0 全部完工 | 风险: 低
[2026-05-13] HOTFIX task_012_v11_conversion_meta_repair | 用户报"no such table: conversion_meta"，sqlite 实测 user_version=10 但缺 V6 conversion_meta 表；根因：曾存在 V9/V10 migration 把 user_version 推到 10 后被删除但未携带建表；修复：新增 V11 idempotent migration 用 IF NOT EXISTS 兜底，3 测全 PASS | 风险: 低
[2026-05-13] task_013_diagnosis_auto_extract | 用户报"导入不自动提取/未真正转 MD"；诊断：链路结构无 bug（DB 实测 37 root → 33 真 MD + 8 placeholder + 4 failed），根因 = markitdown SUPPORTED 不全（image/text 走 placeholder）+ 讯飞 language=autodialect 用户密钥不支持 + 前端不区分 placeholder vs 真 MD
[2026-05-13] task_014_auto_extract_coverage (A 全包) | Dev 完成：扩 markitdown image/* + 新 text_passthrough extractor（plain/markdown/csv/json/xml）+ 讯飞 language=cn + DTO 加 extractor_type + 前端 placeholder_* 黄色徽章；cargo extraction 59/59、commands 35/35、vitest 7/7 全 PASS；待用户重启验证 | 风险: 低
