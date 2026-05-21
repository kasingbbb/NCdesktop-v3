# Debate Log — custom_classification / session_001

- 主题：自定义分类 + Finder 视图 + 工作区映射 Bug 修复
- 复杂度：L（4 层完整 Debate）
- 角色：Host（主持/路由）、Proposer、Reviewer
- 日期：2026-05-09
- 关联 PRD：`sessions/custom_classification/prd/custom_classification_prd_v1.md`

---

## Layer 1 — 问题定义（3 轮）

### Round 1
**Proposer H1**：三个需求是「同一对象（分类体系）的三个切面」——Bug 1 是当前 PARA 实现的边界缺陷；Finder 视图是 PARA 树的可视化；自定义分类是 PARA 的可扩展点。统一以「分类对象 = 显示名 + slug + 父子关系 + 资产集合」建模。

**Reviewer 反驳 [L2]**：H1 把三件事都收敛到「分类对象」，掩盖了 Bug 1 的真因可能不在分类层而在「导入路由上下文」（活动 projectId + 当前工作区子路径）。要求 Proposer 给出 Bug 1 的根因假设链，而非直接套用模型。

**Proposer H2 修订**：Bug 1 根因假设——`resolve_import_project_id` 只读 `active_project_id`，未消费前端 `workspaceFolderRelativePath`，导致导入到子目录时仍按"项目根 + LLM 分类"路径写入，从而在子项目工作区视图中"消失"。

**Reviewer [L1]**：接受 H2 但补两条待证假设：
- H3：是否还存在 `topics` 字段 schema 与代码 contract 不一致的次生缺陷？
- H4：`sanitize_path_segment` 对中文/特殊字符的处理是否会让 `category` 落盘后与 DB 记录不一致？

### Round 2
**Proposer 调查后回应**：
- H3 **成立**：`ai_analyses.topics` 在 V1 schema 是 TEXT default `'[]'`（JSON 数组语义），但 `commands/dropzone.rs:347` 写入 `topics: r.category.clone()`（裸字符串）。这是写入侧违反列契约。
- H4 **初判成立**：怀疑 `sanitize_path_segment` 把中文压成空串触发 `other` 兜底。

**Reviewer [L3]**：H4 初判**错误**。Rust 标准库 `char::is_alphanumeric()` 对 CJK 字符返回 `true`（按 Unicode Alphabetic 属性）。给出反例：`"项目"` 通过 sanitize 不会被压缩，问题不在中文。要求重做 H4。

### Round 3
**Proposer H4'**（重写）：真正的失败模式是
1. **LLM 输出含路径分隔符或空格**：例如返回 `"项目 / 领域"`，sanitize 剥掉 `/` 与空格 → 拼成 `项目领域`，与任何已存在分类目录都不匹配；
2. **`take(48)` 截断**：长 category 被截断后产生新 slug，等同于"幽灵分类"；
3. 叠加 H3 `topics` schema 失配 → 工作区视图按 JSON 数组解析失败 → UI 看不到。

**Reviewer 接受 [L0]**：Bug 1 改判为**复合缺陷**（导入上下文丢失 + topics 字段失配 + display↔slug 不可逆），不是单点 mapping。要求术语收敛。

### 收敛产出
- 引入三术语：
  - **ProjectFolderRoot**：磁盘路径概念 `<project>/organized/<category_slug>/`
  - **ProjectFolderScope**：导入上下文谓词（active_project_id + workspaceFolderRelativePath）
  - **WorkspaceView**：UI 概念，按 DB 投影渲染
- Bug 1 根因正式版：ProjectFolderScope 未注入到 import 路由 + topics schema 失配 + slug 不可逆三者叠加。

---

## Layer 2 — 理想态

**Proposer**：三件事的统一理想态：
1. **分类对象模型**：DB 为权威源，slug 稳定 + 显示名可变；用户自定义与内置 PARA 同模；
2. **WorkspaceView**：按 DB 投影、不直接 `read_dir`；Finder 风格在 UI 层负责，IPC 仅暴露聚合后的列表 + 子路径分页；
3. **Prompt 编辑器三态**：
   - 在线 LLM 可达：dry-run **必须通过**才能保存；
   - 离线/未配置：仅做 schema-level 占位符校验，置 `validated_offline=true` 标记；
   - 用户显式跳过：二次确认弹窗 + 标 `user_skipped_validation=true`。

**Reviewer [L2]**：要求把"分类作用域"明确——是 library 级还是 project 级？

**Proposer**：MVP 锁 library 级（一套 categories 跨所有项目共用）；project 级 override 留 v2。

**Reviewer 接受**。

### 收敛产出
- 分类作用域：library 级
- WorkspaceView 数据源：DB 投影（不走 FS read_dir）
- Prompt 编辑：三态 dry-run 容灾

---

## Layer 3 — 差距分析

**Proposer**：
- Schema：新增 `categories(id, library_id, slug, label, parent_id, icon, sort_order, is_disabled, is_builtin)` + `category_aliases(slug, alias_label, library_id)`；`assets` 增 `category_slug` 列。
- 命令：新增 `list_workspace_assets(project_id, category_slug, sub_path?)`；新增 `commands/prompts.rs` 4 个命令（list/get/save/restore_default + dry_run）。
- 复用：`settings` KV 表已存在（V1 migration line 646），Prompt 覆盖层零新表。
- 前端：升级 `WorkspaceFolderStrip` → `WorkspaceCategorySidebar`（约 60% 复用）。

**Reviewer [L2]**：
- 反驳"UUID 化 categories 表"——用 slug+label 二元组即可，UUID 引入额外迁移成本无 MVP 收益。
- 要求 Bug 1 修复必须独立 PR 且不依赖 schema V10。

**Proposer 调整**：
- 接受 slug+label 二元组 + aliases 表，UUID 升级路径写入 v2 备忘；
- Bug 1（PR-2）只动 `resolve_import_project_id` + `dropzone.rs:347` topics 写入修正 + 启动期一次 `topics` 自愈扫描，不依赖 V10。

**Reviewer 接受**。

### 收敛产出
- Schema 决定：lightweight pair（slug+label）+ aliases，V10 migration 带 `categories_v9_backup` 30 天保留
- Prompt 持久化：复用 `settings` KV，零新 schema
- Bug 1 与 schema 解耦，可独立 PR

---

## Layer 4 — 策略（MVP 取舍）

**Proposer 初稿**：单 PR 合并三件事，节省 review 成本。

**Reviewer [L3]**：单 PR 不现实——三个变更耦合 schema、命令面、UI、Prompt 安全四个域，review 风险与回滚粒度不可控。强制拆分。

**Proposer 修订**：4-PR 拆分：
- **PR-1 基础设施**：V10 migration + `categories` / `category_aliases` 表 + `assets.category_slug` + 内置 PARA 种子；
- **PR-2 Bug 修复**：ProjectFolderScope 注入 + topics 写入修正 + 启动期自愈；可独立交付；
- **PR-3 视图层**：`list_workspace_assets` + `WorkspaceCategorySidebar` + Finder 列表/图标/面包屑；依赖 PR-1；
- **PR-4 Prompt 编辑器**：`commands/prompts.rs` + 设置面板三段编辑器 + 占位符校验 + 三态 dry-run + restore-default；**可与 PR-1 并行**（仅依赖 settings 表）。

**Reviewer 追加风险点**：
- V10 迁移失败的降级路径？→ 三档：成功 / 部分失败归 `__uncategorized__` + 横幅 / DB 损坏只读安全模式。
- 子目录导入时是否还要调 LLM？→ 不调；当前视图 slug 直接绑定，本地启发式 mismatch 时弹 toast 让用户确认。

**Reviewer 接受 [L0]**。

### 收敛产出
- 4-PR MVP 拆分（PR-2/PR-4 可与 PR-1 并行）
- 子目录导入跳过 LLM，分类直接绑定当前视图 slug
- 三档降级启动方案

---

## Argument Tracker

| ID | 假设 | 状态 | 终判 |
|----|------|------|------|
| H1 | 三需求合并到「分类对象」单一模型 | 部分接受 | 用作建模骨架，不掩盖 Bug 真因 |
| H2 | Bug 1 根因 = `resolve_import_project_id` 缺 workspaceFolderRelativePath | 成立 | 入 PR-2 |
| H3 | `topics` 字段 schema 与写入代码失配 | 成立 | 入 PR-2 自愈扫描 |
| H4 | `sanitize_path_segment` 把中文压成空串 | 否决 | CJK 走 Unicode Alphabetic |
| H4' | display↔slug 不可逆 + LLM 输出含分隔符 | 成立 | 入 PR-1（slug 稳定化）+ PR-2（兜底） |
| H5 | 单 PR 合并三件事 | 否决 | 强制拆 4 PR |
| H6 | 自定义分类用 UUID 三元组 | 否决 | 用 slug+label + aliases |
| H7 | dry-run 必须在线通过 | 部分接受 | 改三态容灾 |

---

## Trade-off Matrix

| 维度 \ 选项 | 单 PR 合并 | 4-PR 拆分 | 备注 |
|-------------|-----------|-----------|------|
| Review 成本 | 高 | 中 | 拆分胜 |
| 回滚粒度 | 不可控 | 单 PR 级 | 拆分胜 |
| 上线节奏 | 一锤子 | Bug-fix 可先发 | 拆分胜 |
| 集成风险 | 集中 | 分散可控 | 拆分胜 |
| 工作量 | 略低 | 略高 | 单 PR 微胜 |

| 维度 \ 选项 | UUID 三元组 | slug+label 二元组 + aliases |
|-------------|-------------|----------------------------|
| 重命名稳定性 | 强 | 强（slug 不变） |
| 迁移成本 | 高 | 低 |
| 跨项目同步 | 容易 | MVP 不需要 |
| **MVP 取舍** | 留 v2 | 选定 |

---

## 终判（4 层）
- L1：Bug 1 = 复合缺陷（scope 注入 + topics schema + slug 不可逆）
- L2：library 级分类作用域 + DB 权威 list + Prompt 三态
- L3：lightweight pair schema + Prompt 复用 settings KV + Bug 修与 schema 解耦
- L4：4-PR 拆分（PR-2/PR-4 与 PR-1 并行），三档降级启动
