# Task 输入 — task_001_architect

## 目标

为 NCdesktop **用户自定义 Prompt 功能** 设计完整技术方案，并拆分出可执行的 Dev Task 清单（每个 Dev task 一份 `input.md`）。

## 前置条件

- 依赖 task：无（流水线首个 task）
- 必须先存在的文件/接口：NCdesktop 现有代码（前端 React/TS + 后端 Tauri/Rust + SQLite）

## 验收标准（Acceptance Criteria）

1. **AC-1（现状勘察）** — `output.md` 中必须先有"现状勘察"章节，列出：
   - 现有的内置 Prompt 在代码中的存放位置（搜索 `tagging`/`para`/`concept`/`aggregation` 等关键词）
   - 前端 `src/stores/promptStore.ts` 当前职责
   - LLM 调用链中 Prompt 的注入点（哪个函数/文件读取并发送 system prompt）
   - 现有的 SQLite migration 机制与 settings 持久化机制（参考 `src/stores/settingsStore.ts` 与 `src-tauri/src/db/`）
   - 现有的设置面板组件结构（参考 `src/components/features/SettingsPanel.tsx`）
2. **AC-2（技术方案）** — `output.md` 至少包含 Architect prompt 要求的全部章节：项目概述 / 技术选型 / ADR / 系统架构 / 数据模型 / API 设计 / 目录结构 / 安全考量 / 风险登记表 / Task 清单 / Task 依赖拓扑
3. **AC-3（ADR 覆盖）** — 至少为以下决策点产出 ADR：
   - ADR-001: 内置 Prompt 的存放方式（代码常量 vs 资源文件）与 fallback 机制
   - ADR-002: 用户自定义 Prompt 的 SQLite 表结构（采用 PRD § 4 的 schema，或如有更优方案需说明）
   - ADR-003: 输出格式约束的实现位置（PRD R1 风险）—— 在 prompt 后注入、独立 system message、还是后置 schema 校验
   - ADR-004: token 长度校验的执行时机（PRD R2 风险）—— 保存时、调用前、或两者
   - ADR-005: 前端编辑面板的状态管理路径（扩展现有 `promptStore.ts` 还是新建 store）
4. **AC-4（Task 拆分）** — Task 清单需满足：
   - 每个 task 单一目标，可独立验证
   - 标注依赖与可并行项
   - 每个 task 写出 `tasks/task_00N_xxx/input.md`，字段符合 handoff_contracts.md § 2「Architect → Dev Task 输入契约」
   - 至少包含：① 数据层（SQLite 表 + Tauri command CRUD）② 后端 LLM 调用链注入点改造 ③ 前端 store/类型 ④ 前端 UI（设置面板新增"Prompt 自定义"面板与四个折叠子项 + 编辑框 + 保存/恢复按钮）⑤ 输出格式校验层 & token 校验 ⑥ 端到端测试 ⑦ UX 评审
5. **AC-5（风险闭环）** — PRD 桥接摘要中 R1/R2/R3 三项风险必须在风险登记表中出现，并指向具体 task 与 ADR
6. **AC-6（progress.md 同步）** — 完成后在 `progress.md` 的"待执行 Task 队列"中写入完整 task 清单，并标注依赖拓扑

## 技术约束

来源：`sessions/custom_prompt_v1/session_context.md` § 2、§ 3、§ 5

- **技术栈**：Rust (Tauri 2.x backend) + TypeScript (React/Next.js 前端) + SQLite（本地优先）
- **不可妥协底线**（已记入 progress.md "不可妥协的技术底线"）：
  1. 内置 Prompt 始终作为 fallback 存在
  2. 用户自定义 Prompt 必须持久化到本地 SQLite
  3. 隐私优先：不上传云端
  4. 一键恢复默认
  5. 内置 Prompt 升级不覆盖用户自定义
- **代码规范**：
  - Prompt 模板使用结构化格式（参考 PRD：直接编辑全文 + 输出格式约束独立）
  - 用户自定义部分与系统内置部分必须有明确分层边界
  - LLM 调用时合并后的 Prompt 需有 token 长度校验
  - 所有 Prompt 操作需支持撤销/回退（一键恢复默认即满足）

## 参考文件

**必读（现状勘察用）**：
- `sessions/custom_prompt_v1/prd/custom_prompt_prd_v1.md` — PRD（v1.1）
- `sessions/custom_prompt_v1/session_context.md` — 项目上下文
- `sessions/custom_prompt_v1/conductor/progress.md` — 当前 Conductor 状态（含三项风险摘要）
- `core/handoff_contracts.md` — Architect → Dev Task 输入契约（§ 2）

**NCdesktop 代码（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/promptStore.ts` — 现有 Prompt store
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/settingsStore.ts` — 设置 store（持久化范式参考）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/SettingsPanel.tsx` — 设置面板（新功能挂载点）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/types/settings.ts` — 设置类型定义
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/llm/` — LLM 调用链（Prompt 注入点）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/` — SQLite & migration
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/` — Tauri commands

**搜索关键词（用 grep 在 src-tauri/src/llm/ 下找内置 Prompt 文本）**：`tag`、`para`、`concept`、`aggregat`、`system_prompt`、`PROMPT`

## 预估影响范围（由 Architect 在 output.md 中精确化）

- **新建文件**：SQLite migration、`user_custom_prompt` 相关 Rust 模块、前端 PromptCustomizationPanel 组件、相关类型
- **修改文件**：`promptStore.ts`（或新建 store）、`SettingsPanel.tsx`、LLM 调用链中读取 system prompt 的位置、`settings.ts` 类型
- **必须保留兼容**：现有内置 Prompt 调用路径不能因引入数据库读取而变慢/破坏；migration 必须向前兼容（旧用户首次启动表为空）

## 产出物

1. `sessions/custom_prompt_v1/conductor/tasks/task_001_architect/output.md` — 完整技术方案（按 Architect prompt 模板）
2. `sessions/custom_prompt_v1/conductor/tasks/task_00N_xxx/input.md` — 每个 Dev/Test task 的输入文档（N=2,3,...）
3. 更新 `sessions/custom_prompt_v1/conductor/progress.md` 的"待执行 Task 队列"与"状态转移日志"

## 给 Architect 的特别提示

- **不要凭空假设代码结构** —— PRD § 4 的 schema 是一个起点；如果在现状勘察中发现现有 store/migration 模式有更优做法，请在 ADR 里说明。
- **PRD R1（输出格式异常）是最关键风险** —— 必须在 ADR-003 给出明确的实施位置，并在 task 拆分中分配独立的实现 task。
- 现有 `promptStore.ts` 已存在 —— 必须先读懂它的现职责，再决定扩展 vs 新建。
- 复杂度 L → 完整 Conductor + Architecture Guard。task 清单末尾必须保留 `task_NNN_architecture_guard` 与 `task_NNN_ux_review` 节点。
