# Task 输入 — task_002_architect

## 目标
基于 task_001 实测结果，设计 E1-E6 技术方案，并回答 task_001 output.md §"给 Architect 必须先回答的设计问题清单" 的 10 个问题。

## 前置条件
- 依赖 task：task_001_empirical_test（DONE）
- 必须先存在的文件/接口：task_001 output.md

## 验收标准
1. 产出 `output.md`，对 Q1-Q10 全部给出明确答案
2. 为 E1-E6 每个模块产出实施子任务清单（task_003-008 的 input.md 可由此派生）
3. 明确影响的文件路径 + DB migration 版本号
4. 明确 backward-compat 策略（已有存量数据如何迁移）

## 技术约束
- 不引入新的外部 crate，优先复用现有（sha2、walkdir、chrono、serde_yaml 可新增 if needed）
- DB migration 必须 ≥ V9，无 destructive drop
- 保持 `session_context.md` 不可妥协底线（不覆盖 user_edited / 不删旧派生 / 每原件有邻居）

## 参考文件
- `task_001_empirical_test/output.md`
- `src-tauri/src/extraction/scheduler.rs:455-651`
- `src-tauri/src/commands/knowledge.rs:83-212`
- `src-tauri/src/db/migration.rs`

## 预估影响范围
- 新建文件：本 task 的 output.md；task_003-008 的 input.md
- 修改文件：仅 progress.md
