# Task 001 — Architect 输入

## 目标
将 PRD `sessions/custom_classification/prd/custom_classification_prd_v1.md` 拆解为可执行的 Dev task 清单与技术方案（含 ADR），输出 `output.md`、更新 `progress.md` 的 task 队列，并为每个子 task 写 `input.md`。

## 前置条件
- [x] PRD v1 已完成并通过人类 PM 审阅（2026-05-09）
- [x] Debate 4 层结论已固化于 `sessions/custom_classification/debate/session_001/`
- [x] session_context 完整可读
- [x] 现状代码事实点已固化（见参考文件章节）

## 验收标准（AC）
1. `output.md` 含完整 ADR 列表，至少覆盖以下决策点：
   - ADR-001: `categories` 主键策略（自增 INTEGER vs 复合 (library_id, slug)）
   - ADR-002: Bug 1 启动期自愈扫描位置（`db/migration.rs` vs 独立 `db/repair.rs`）
   - ADR-003: `list_workspace_assets` 分页策略（cursor vs offset）
   - ADR-004: Prompt dry-run 在线探活实现（复用 `llm/client.rs` vs 新增 `validate_prompt`）
   - ADR-005: WorkspaceFolderStrip → WorkspaceCategorySidebar 替换策略（feature flag / 直接替换 / 双栈共存）
   - ADR-006: V10 migration 失败时三档降级的入口位置（启动期 / 首屏渲染期）
   - ADR-007: 子目录导入"本地启发式 mismatch"判定算法选择
2. Task 拆分必须严格遵守 PR-1 / PR-2 / PR-3 / PR-4 边界，**不允许**跨 PR 单 task；
3. PR-4 与 PR-1 的并行可行性必须在依赖拓扑中显式标注；
4. 每个 task 的 input.md 含：目标 / 前置条件 / AC / 技术约束 / 参考文件 / 影响范围；
5. Task 粒度自检通过（单一目标、可独立测试、≤2000 行变更、依赖清晰、AC 客观可验）；
6. 风险登记表至少包含：迁移失败、跨项目串扰、Prompt 注入、长目录性能、LLM 离线导致 dry-run 阻塞；
7. 安全考量章节必须回应 session_context §3 三条不可妥协底线。

## 技术约束（来自 session_context）
- 语言：TypeScript（前端，严格模式）+ Rust（Tauri 后端）
- 数据库：SQLite（`src-tauri/src/db/`），所有 schema 变更走 migration
- IPC 边界：所有跨进程数据走 `src/lib/tauri-commands.ts`，Tauri command 返回 `Result<T, String>`
- Prompt 默认值留在 `src-tauri/src/llm/prompts.rs`；用户覆盖层从 `settings` KV 读取，渲染时合并注入
- Zustand store：副作用集中在 action 层；组件只 dispatch / 读取
- 文件 IO：集中在 `src-tauri/src/workspace.rs`，写盘前路径合法性校验
- 自定义分类名禁字符：`/ \ : * ? " < > |`、长度上限、保留字（`__uncategorized__`、`__archived__`、`other`）

## 不可妥协底线（红线）
1. PARA 既有资产 100% 向后兼容，迁移可撤销 30 天（`categories_v9_backup`）
2. Prompt 关键占位符（如 `{content}`）缺失即禁止保存
3. 工作区映射修复不得引入跨项目串扰

## 参考文件
- PRD：`sessions/custom_classification/prd/custom_classification_prd_v1.md`
- Session Context：`sessions/custom_classification/session_context.md`
- Debate 结论：`sessions/custom_classification/debate/session_001/debate_conclusions.md`
- Debate 全程：`sessions/custom_classification/debate/session_001/debate_log.md`
- 现状代码事实点：
  - `项目启动/NCdesktop/src-tauri/src/llm/prompts.rs`（PROMPT_VERSION=1.1，3 段 Prompt 待暴露）
  - `项目启动/NCdesktop/src-tauri/src/commands/dropzone.rs`（L115-126 sanitize / L347 topics 写入 / `resolve_import_project_id`）
  - `项目启动/NCdesktop/src-tauri/src/db/migration.rs`（L646 settings KV 已存在）
  - `项目启动/NCdesktop/src-tauri/src/commands/workspace_folders.rs`（仅一级目录，需新增 `list_workspace_assets`）
  - `项目启动/NCdesktop/src/stores/uiStore.ts`（L32-33 workspaceFolderRelativePath，"__ROOT__" 哨兵）
  - `项目启动/NCdesktop/src/stores/projectStore.ts`（activeProjectId）
  - `项目启动/NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（升级目标）

## 预估影响范围
- Rust：`db/migration.rs`、`db/repair.rs`（新）、`commands/dropzone.rs`、`commands/workspace_folders.rs`、`commands/prompts.rs`（新）、`commands/categories.rs`（新）、`llm/prompts.rs`、`workspace.rs`
- TS：`lib/tauri-commands.ts`、`stores/categoryStore.ts`（新）、`stores/uiStore.ts`、`stores/promptStore.ts`（新）、`components/features/WorkspaceCategorySidebar.tsx`（新，替换 Strip）、`components/features/FolderListView.tsx` / `FolderIconView.tsx`（新）、`components/settings/CategoryEditor.tsx`（新）、`components/settings/PromptEditor.tsx`（新）
- Schema：V10 migration（categories / category_aliases / assets.category_slug + categories_v9_backup）
- 测试：导入路由 / 分类迁移 / Prompt 占位符校验

## 输出位置
- 技术方案：`sessions/conductor/tasks/task_001_architect/output.md`
- 子 task input：`sessions/conductor/tasks/task_002_*/input.md` … `task_NNN_*/input.md`
- 进度更新：`sessions/conductor/progress.md`（追加 task 队列 + 状态转移日志）

## 工作流提示
- 严格按 4-PR 边界拆分；每 PR 内部继续按"schema → command → store → component → test"分层；
- PR-2、PR-4 在拓扑中标注「可与 PR-1 并行启动」；
- 每个 task 末尾追加"Reviewer 重点关注项"（M 复杂度起，强制）。
