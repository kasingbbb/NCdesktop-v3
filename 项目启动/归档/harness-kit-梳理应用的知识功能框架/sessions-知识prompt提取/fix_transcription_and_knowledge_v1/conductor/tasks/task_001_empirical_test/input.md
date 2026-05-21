# Task 输入 — task_001_empirical_test

## 目标
实测 PRD v1 中所有 H 级探索性测试用例，产出真实通过/失败矩阵，并明确 F-11 后端 stub 实际实现状态。

## 前置条件
- 依赖 task：无
- 必须先存在的文件/接口：当前 main 分支可正常 build & run

## 验收标准
1. 产出 `output.md`，包含一张矩阵：每个 H 级用例（W-02/04/05/06/10/11/13, V-01/02, T-01/02, K-01/02/03, I-01/02/03, S-01, Q-01/02/03/04/05, E-02 + X-01）的【实际行为 / 预期行为 / PASS·FAIL / 证据（截图或代码引用）】
2. F-11 三个命令（`synthesize_viewpoints` / `generate_extensions` / `concept_relations` 写入路径）的实现状态明确标注：实现 / stub / 部分实现，附 grep 证据
3. 基于实测结果重排修复优先级（如有偏差）
4. 给 Architect 列出"必须先回答的设计问题"清单

## 技术约束
- 不修改源码，仅观察
- 数据库查询通过 `sqlite3` CLI 或 Tauri devtools
- 测试样本文件放 `sessions/fix_transcription_and_knowledge_v1/test_fixtures/`

## 参考文件
- `src-tauri/src/extraction/scheduler.rs:455` (`source_asset_should_materialize`)
- `src-tauri/src/extraction/scheduler.rs:479` (`materialize_md`)
- `src-tauri/src/commands/knowledge.rs` (所有概念命令)
- `src-tauri/src/workspace.rs`
- `KNOWLEDGE_DESIGN_CHARTER.md`
- `notecapt 知识进化功能迭代宪章v1.0.md`

## 预估影响范围
- 新建文件：output.md + test_fixtures/ 目录
- 修改文件：仅 progress.md（状态推进）
