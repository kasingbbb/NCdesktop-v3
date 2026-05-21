# 技术方案 — task_001_architect

> 输入：PRD v1（custom_classification_prd_v1.md）+ session_context.md + debate_conclusions.md
> 输出：本 output.md（含 7 项 ADR + 系统架构 + 数据模型 + API 设计 + 目录结构 + 安全考量 + 风险登记 + Task 清单 + 依赖拓扑）+ 各子 task `input.md`

---

## 项目概述

NCdesktop 在 PARA 自动归类基础上，引入用户自定义分类体系、Finder 风格 WorkspaceView、可编辑 Prompt 三段；同时根治"子目录导入资产不可见"复合缺陷（schema 失配 + slug 派生不可逆 + ProjectFolderScope 未注入）。MVP 拆 4 PR，PR-1 基础设施为 PR-2/PR-3 硬前置，PR-4 独立并行。每 PR 独立可上线、可回滚、各带 feature flag。

---

## 技术选型

| 领域 | 选型 | 备注 |
|------|------|------|
| Schema 迁移 | 沿用现有 `db/migration.rs` 多版本机制 | 新增 V10；保留 `categories_v9_backup` 30 天 |
| Prompt 持久化 | 复用 V1 `settings` KV 表 | 零新表；key 命名规范见 ADR-008 |
| Tauri command 层 | 沿用现有 invoke + `tauri-commands.ts` 封装 | 新增 `commands/categories.rs`、`commands/prompts.rs` |
| 前端状态 | Zustand 新增 `categoryStore` `promptStore` | 副作用集中在 action |
| 列表渲染 | `react-virtuoso` 已在依赖中，复用 | 1k 文件目录虚拟滚动 |
| 图标视图 v1 | mime → 内置 SVG 图标映射 | 缩略图留 v2（F18） |
| dry-run 在线探活 | 复用 `llm/client.rs` 现有 chat 接口 | 短超时 5s（ADR-004） |

---

## Architecture Decision Records (ADR)

### ADR-001：`categories` 主键策略 — 选「自增 INTEGER + UNIQUE(library_id, slug)」
- **状态**：已接受
- **上下文**：分类需要稳定标识，但 slug 可能随用户改 label 而升级；UUID 增加迁移负担。
- **决策**：主键 `id INTEGER PRIMARY KEY AUTOINCREMENT`；同时建 `UNIQUE(library_id, slug)` 复合唯一索引；外键引用统一用 `category_slug`（业务键）而非 `id`，便于路径拼接与 alias 兼容。
- **被排除**：(a) 复合主键 `(library_id, slug)` — 与 alias 表外键级联复杂；(b) UUID — MVP 无跨设备同步需求。
- **后果**：renaming label 不动 id 与 slug；slug 变更（用户显式改 slug）走"新 row + alias 历史 slug"路径。

### ADR-002：启动期自愈扫描位置 — 选「独立 `db/repair.rs`」
- **状态**：已接受
- **上下文**：F2 `topics` 失配 + F3 降级启动需要在迁移成功后、第一次查询前完成自愈扫描；混入 `migration.rs` 会污染 schema 升级语义。
- **决策**：新建 `src-tauri/src/db/repair.rs`，导出 `run_post_migration_repair(conn, mode: RepairMode) -> RepairReport`；模式 `Strict` / `Lenient` / `ReadOnly` 对应三档降级。
- **被排除**：合入 `migration.rs::run_v10` — 违反单一职责。
- **后果**：`startup.rs` 调用顺序：migrate → repair → set_app_state(mode)；测试可单独覆盖 repair。

### ADR-003：`list_workspace_assets` 分页策略 — 选「cursor-based（mtime + id）」
- **状态**：已接受
- **上下文**：长目录 1k+ 文件需虚拟滚动；offset 在并发 insert 时丢失/重复。
- **决策**：cursor 编码 `(updated_at_unix:u64, id:i64)`，按 `(updated_at DESC, id DESC)` 复合索引扫描，page_size 默认 200，最大 500。
- **被排除**：offset/limit — 并发不稳。
- **后果**：必须为 `assets` 加 `idx_assets_proj_cat_updated`；返回 `next_cursor: Option<String>` (base64)。

### ADR-004：dry-run 在线探活 — 选「复用 `llm/client.rs` 现有 chat 接口」
- **状态**：已接受
- **上下文**：F15 三态需"在线"判定；新增 ping endpoint 与现有 LLM Provider 抽象重叠。
- **决策**：`commands/prompts.rs::dry_run_prompt` 内部 spawn 一个 5s 超时的小 prompt（10 tokens 输出），返回 `DryRunOutcome { online_ok: bool, schema_ok: bool, error: Option<String> }`；前端按 `online_ok && schema_ok` 决定保存可用性。
- **被排除**：新增 `validate_prompt` HTTP-style endpoint — 与项目 IPC-only 风格不符。
- **后果**：dry-run 真实消耗 token（极少）；前端显示"测试中…"避免误解。

### ADR-005：WorkspaceFolderStrip 替换策略 — 选「feature flag `workspace_view_v2` 双栈共存」
- **状态**：已接受
- **上下文**：直接替换风险大；双栈共存便于灰度。
- **决策**：保留 `WorkspaceFolderStrip` 不动；新增 `WorkspaceCategorySidebar` 与 `FolderListView` / `FolderIconView`；顶层 `WorkspaceLayout` 按 `featureFlags.workspace_view_v2` 路由。flag 默认 off，PR-3 合并后由 PM 翻 on。
- **被排除**：直接替换 — 回滚成本高。
- **后果**：两套并存约 2 个 release，flag 全量后清理 Strip。

### ADR-006：V10 迁移三档降级入口 — 选「启动期统一入口 `startup::bootstrap`」
- **状态**：已接受
- **上下文**：迁移失败 / repair 部分失败 / DB corrupt 三档，必须在首次窗口渲染前确定模式。
- **决策**：在 `tauri::Builder::setup` 中调用 `bootstrap()`，返回 `AppMode::{Normal, Degraded(Reason), ReadOnly(Reason)}`；通过 `tauri::State` 注入；前端启动时读 `get_app_mode` 命令决定 UI（横幅 / 只读屏蔽编辑入口）。
- **被排除**：首屏渲染期判定 — 迟于路径决策，已可能写脏。
- **后果**：`bootstrap` 必须 idempotent + 可单测；任何后续命令在 ReadOnly 模式下短路返回"只读模式"错误。

### ADR-007：子目录导入 mismatch 启发式 — 选「文件名 token + 现有同分类资产 tag 词袋 Jaccard」
- **状态**：已接受
- **上下文**：F5 不打断流程的提示，需轻量、本地、可解释。
- **决策**：取导入文件 base name 做中英文 token 切分（CJK n-gram + ascii word），与当前 `category_slug` 下既有资产的 tags 词袋（前 50 项）做 Jaccard 相似度；阈值 < 0.05 即触发 toast。前端无既有资产时跳过判定。
- **被排除**：调用 LLM 二次决策 — 与"跳过 LLM"原则冲突；TF-IDF — 引入新依赖收益低。
- **后果**：算法实现 ≤ 80 行；阈值可后续调参；持久化"用户对 toast 的处置"留 v2。

### ADR-008：`settings` KV 命名规范（Prompt 持久化）
- **状态**：已接受
- **上下文**：Prompt 三段共用 settings 表，需可扩展 + 易回滚。
- **决策**：key 形如 `prompt.override.{kind}.{field}`，`kind ∈ {classify, naming, tagging}`，`field ∈ {system, user, output, validated_offline, user_skipped_validation, updated_at}`；value 为 JSON 序列化字符串。
- **被排除**：单 row JSON blob — diff 困难。
- **后果**：恢复默认 = 删除对应 keys；version 升级可加 `prompt.override.{kind}.version` 字段。

---

## 系统架构

```
┌────────────────────────────── Frontend (React + Zustand) ────────────────────────────┐
│  WorkspaceLayout                                                                      │
│   ├─ flag.workspace_view_v2 ? <WorkspaceCategorySidebar/> : <WorkspaceFolderStrip/>  │
│   ├─ <FolderListView/> | <FolderIconView/>     ◀── categoryStore / assetStore        │
│   └─ <Breadcrumb/> + <EmptyImportCTA/>                                                │
│                                                                                        │
│  SettingsPanel                                                                         │
│   ├─ <CategoryManager/>                       ◀── categoryStore                       │
│   └─ <PromptEditor kind={classify|naming|tagging}/> ◀── promptStore                   │
│                                                                                        │
│  lib/tauri-commands.ts (统一封装)                                                      │
└──────────────────────┬───────────────────────────────────────────────────────────────┘
                       │  invoke / event
┌──────────────────────▼───────────────────── Tauri Backend (Rust) ────────────────────┐
│  commands/                                                                            │
│   ├─ dropzone.rs           ── ProjectFolderScope 注入；F4/F5/F6                       │
│   ├─ workspace_folders.rs  ── 既有                                                    │
│   ├─ workspace_assets.rs   ── 新：list_workspace_assets (F7)                          │
│   ├─ categories.rs         ── 新：CRUD / list / set_disabled (F11)                    │
│   └─ prompts.rs            ── 新：get/save/dry_run/reset (F12-F16)                    │
│                                                                                       │
│  db/                                                                                  │
│   ├─ migration.rs          ── 新增 V10                                                │
│   └─ repair.rs             ── 新：run_post_migration_repair (F2/F3)                   │
│                                                                                       │
│  llm/prompts.rs            ── 默认值仍在；新增 merge(default, override) 渲染层        │
│  llm/client.rs             ── 既有；dry_run_prompt 复用                               │
│  workspace.rs              ── 路径合法性 + ProjectFolderScope 谓词                    │
│  startup.rs                ── 新：bootstrap() → AppMode (ADR-006)                     │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 数据模型

### V10 新增表

```sql
-- categories：分类对象
CREATE TABLE categories (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id  TEXT NOT NULL,
  slug        TEXT NOT NULL,                    -- 路径用，[a-z0-9一-龥_-]
  label       TEXT NOT NULL,                    -- 展示用
  parent_id   INTEGER REFERENCES categories(id) CHECK (parent_id IS NULL),  -- F17
  icon        TEXT,                             -- v2 启用
  sort_order  INTEGER NOT NULL DEFAULT 0,
  is_disabled INTEGER NOT NULL DEFAULT 0,
  is_builtin  INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(library_id, slug)
);

-- category_aliases：历史 slug 映射
CREATE TABLE category_aliases (
  alias_slug  TEXT NOT NULL,
  library_id  TEXT NOT NULL,
  target_slug TEXT NOT NULL,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY(library_id, alias_slug),
  FOREIGN KEY(library_id, target_slug) REFERENCES categories(library_id, slug)
);

-- assets 表新增列
ALTER TABLE assets ADD COLUMN category_slug TEXT;
CREATE INDEX idx_assets_proj_cat_updated
  ON assets(project_id, category_slug, updated_at DESC, id DESC);

-- 备份表（30 天）
CREATE TABLE categories_v9_backup AS SELECT * FROM /* 旧投影 */;
-- 实际由 V10 在迁移前 dump 旧 PARA 状态生成，含 retention_until 列

-- 内置 PARA 种子（library_id 取当前 library_id）
INSERT INTO categories(library_id, slug, label, sort_order, is_builtin) VALUES
  (?, '1-项目', '项目', 10, 1),
  (?, '2-领域', '领域', 20, 1),
  (?, '3-资源', '资源', 30, 1),
  (?, '4-存档', '存档', 40, 1),
  (?, '__uncategorized__', '未归类', 90, 1);
```

### Prompt 持久化（settings KV）

```
key                                              | value (JSON string)
-----------------------------------------------------------------------
prompt.override.classify.user                    | "完整用户改写..."
prompt.override.classify.output                  | "输出格式段..."
prompt.override.classify.validated_offline       | "true|false"
prompt.override.classify.user_skipped_validation | "true|false"
prompt.override.classify.updated_at              | "2026-05-09T..."
prompt.override.naming.user                      | ...
prompt.override.tagging.user                     | ...
```

---

## API 设计

### 新增 Tauri commands

| 命令 | 入参 | 出参 | 备注 |
|------|------|------|------|
| `list_workspace_assets` | `{project_id, category_slug?, sub_path?, cursor?, page_size?}` | `{items: AssetView[], next_cursor: Option<String>}` | F7；走 idx_assets_proj_cat_updated |
| `list_categories` | `{library_id, include_disabled?}` | `Category[]` | F11 |
| `create_category` | `{library_id, slug, label, sort_order?}` | `Category` | F11；slug 白名单 |
| `rename_category` | `{library_id, slug, label}` | `Category` | F11；只改 label |
| `set_category_disabled` | `{library_id, slug, disabled}` | `Category` | F11 |
| `delete_category` | `{library_id, slug}` | `()` | F11；仅 builtin=0 且引用计数=0 |
| `add_category_alias` | `{library_id, alias_slug, target_slug}` | `()` | slug 升级路径 |
| `get_prompt` | `{kind}` | `{default_text, override_text?, override_meta}` | F12 |
| `save_prompt` | `{kind, field, text}` | `{validated: bool, ...}` | F12-F14 |
| `dry_run_prompt` | `{kind, draft_text, sample?}` | `DryRunOutcome` | F15；ADR-004 |
| `reset_prompt` | `{kind, field?}` | `()` | F16 |
| `get_app_mode` | `{}` | `AppMode` | ADR-006 |

### dropzone 接口扩展

`import_files` 新增可选 `workspace_folder_relative_path: Option<String>`；后端断言：若该 path 对应有效 `category_slug`（含 alias 解析）则跳过 LLM，绑定 `category_slug = 解析结果`；否则 fallback LLM 分类。

---

## 目录结构

```
项目启动/NCdesktop/
├─ src-tauri/src/
│  ├─ commands/
│  │  ├─ dropzone.rs                  (modify) F4/F5/F6
│  │  ├─ workspace_folders.rs         (modify) 接口语义收紧
│  │  ├─ workspace_assets.rs          (new)    F7
│  │  ├─ categories.rs                (new)    F11
│  │  └─ prompts.rs                   (new)    F12-F16
│  ├─ db/
│  │  ├─ migration.rs                 (modify) V10
│  │  └─ repair.rs                    (new)    F2/F3
│  ├─ llm/
│  │  ├─ prompts.rs                   (modify) merge layer
│  │  └─ client.rs                    (read)   dry-run 复用
│  ├─ workspace.rs                    (modify) F6
│  ├─ startup.rs                      (new)    bootstrap → AppMode
│  └─ lib.rs                          (modify) 注册新命令 + bootstrap
├─ src/
│  ├─ lib/tauri-commands.ts           (modify) 新命令封装
│  ├─ stores/
│  │  ├─ categoryStore.ts             (new)
│  │  ├─ promptStore.ts               (new)
│  │  └─ uiStore.ts                   (modify) AppMode + featureFlags
│  ├─ components/features/
│  │  ├─ WorkspaceCategorySidebar.tsx (new)    F8
│  │  ├─ FolderListView.tsx           (new)    F9
│  │  ├─ FolderIconView.tsx           (new)    F9 v1 占位
│  │  ├─ Breadcrumb.tsx               (new)    F10
│  │  ├─ EmptyImportCTA.tsx           (new)    F10
│  │  ├─ MismatchToast.tsx            (new)    F5
│  │  └─ WorkspaceLayout.tsx          (modify) feature flag 路由 (ADR-005)
│  └─ components/settings/
│     ├─ CategoryManager.tsx          (new)    F11
│     └─ PromptEditor.tsx             (new)    F13-F16
└─ src-tauri/tests/                    (new)    repair / sanitize / scope tests
```

---

## 安全考量

| 底线（PRD §6） | 落实点 |
|---------------|--------|
| 1. 旧资产 0 丢失 | `categories_v9_backup` 30 天；MVP 不做物理 mv，仅 DB 重定向 |
| 2. Prompt 占位符 + 输出格式段双校验 | F14 静态校验 + F15 dry-run；缺失 disable 保存 |
| 3. ProjectFolderScope 跨项目隔离 | dropzone 入口 `assert_scope(project_id, relative_path)`；写盘前再次校验 |
| 4. V10 失败不拒启 | ADR-006 三档 AppMode |
| Prompt 注入 | Rust `str::replace` 注入变量，禁用 format! / template engine |
| 路径越权 | slug 白名单 + label 不入路径 + `Path::components` 反 traversal |

---

## 风险登记表

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| V10 迁移 panic | 低 | 高 | 事务包裹；失败回滚至 V9；AppMode=ReadOnly |
| `topics` 旧数据无法解析 | 中 | 中 | 读时自愈 + 异步回填；解析失败回退 `[]` |
| 跨项目串扰 | 低 | 高 | dropzone 双重 scope 校验 + 单测 |
| Prompt 误改全失效 | 中 | 中 | 三态 dry-run + 静态校验 + 一键恢复 |
| 长目录性能 | 中 | 中 | cursor 分页 + 虚拟滚动 + 索引 |
| LLM 离线阻塞编辑 | 中 | 中 | `validated_offline=true` 旁路 |
| Finder IPC N+1 | 低（已规避） | — | DB 投影代替 read_dir |

---

## Task 清单（17 个）

> 命名约定：`task_NNN_<role>_<pr>_<scope>`

### task_001_architect — 本任务

### PR-1 基础设施（F1/F2/F3/F17，硬阻塞 PR-2/PR-3）

- **task_002_dev_pr1_schema_v10** — V10 migration：`categories` / `category_aliases` 表 + `assets.category_slug` 列 + 索引 + 备份表 + PARA 内置种子。约 350 行 Rust + SQL。
- **task_003_dev_pr1_topics_self_healing** — 新建 `db/repair.rs::run_post_migration_repair`；`ai_analyses.topics` 读时自愈 + 异步回填；`commands/dropzone.rs:347` 写入修正（裸字符串 → JSON 数组）。约 250 行。
- **task_004_dev_pr1_degraded_startup** — 新建 `startup.rs::bootstrap` → `AppMode::{Normal, Degraded, ReadOnly}`；`get_app_mode` 命令；前端横幅与只读模式屏蔽；`lib.rs` 注册顺序。约 300 行。

### PR-2 Bug 修复（F4/F5/F6，依赖 PR-1）

- **task_005_dev_pr2_scope_terms** — 后端术语重命名（`ProjectFolderRoot` / `ProjectFolderScope` 谓词），`workspace.rs::assert_scope`；不改外部接口签名（前端零感知）。约 200 行。
- **task_006_dev_pr2_subdir_direct_import** — `dropzone::import_files` 接收 `workspace_folder_relative_path`；解析 `category_slug`（含 alias）→ 命中则跳过 LLM 路径决策（保留 AI 摘要/标签后台并行）；前端 dropzone 透传当前 view slug；feature flag `subdir_direct_import`。约 350 行。
- **task_007_dev_pr2_mismatch_toast** — 本地启发式（ADR-007 Jaccard）；新增 `MismatchToast` 组件；不阻塞导入。约 200 行。

### PR-3 视图层（F7-F11，依赖 PR-1）

- **task_008_dev_pr3_list_workspace_assets** — 后端 `commands/workspace_assets.rs::list_workspace_assets`（cursor 分页 ADR-003）+ 索引就绪 + 单测。约 300 行。
- **task_009_dev_pr3_sidebar** — `WorkspaceCategorySidebar`（基于 Strip 升级为纵向，复用 ~60%）；`categoryStore`；feature flag `workspace_view_v2` 双栈共存（ADR-005）。约 350 行。
- **task_010_dev_pr3_list_view** — `FolderListView`（react-virtuoso）+ `FolderIconView` v1（mime → SVG 图标）；列：图标 / 名称 / 分类 / 标签 / 大小 / 修改时间。约 400 行。
- **task_011_dev_pr3_breadcrumb_empty** — `Breadcrumb` + `EmptyImportCTA`（绑定当前 slug，跳过 LLM）；与 PR-2 F4 联动。约 200 行。
- **task_012_dev_pr3_category_manager** — `CategoryManager` 平铺 CRUD + `commands/categories.rs`；删除仅在 builtin=0 且引用计数=0 显式可见。约 450 行。

### PR-4 Prompt 编辑（F12-F16，独立可并行 PR-1）

- **task_013_dev_pr4_prompts_commands** — `commands/prompts.rs` 四命令；`llm/prompts.rs` merge 层；`settings` KV 命名规范（ADR-008）；恢复默认。约 400 行。
- **task_014_dev_pr4_editor_ui** — `PromptEditor`（三段：system 锁/user/output 锁）+ 占位符 chip + 静态校验"必含变量集合 ⊆ 已用变量集合"；红色下划线未识别 `{xxx}`。约 450 行。
- **task_015_dev_pr4_dry_run_three_state** — F15 三态：在线必过 / 离线 schema-only + `validated_offline=true` / 用户跳过 + 二次确认 + `user_skipped_validation=true`；ADR-004 dry-run 实现。约 350 行。
- **task_016_dev_pr4_reset_default** — 三段独立"恢复默认"图标 + 全局"全部恢复"二次确认；与 KV 删除联动。约 150 行。

### 终验

- **task_017_ux_review** — UX Evaluator 审查 Finder 视图 + 设置面板；产出 scorecard。

---

## Task 依赖拓扑

```
task_001 (本) ────┐
                  ├─ PR-1 ─ task_002 ─ task_003 ─ task_004 ─┐
                  │                                          │
                  │                                          ├─ PR-2 ─ task_005 ─ task_006 ─ task_007 ─┐
                  │                                          │                                         │
                  │                                          └─ PR-3 ─ task_008 ─ task_009 ─ task_010 ─┤
                  │                                                              ↘ task_011             │
                  │                                                              ↘ task_012             │
                  │                                                                                     │
                  └─ PR-4 (并行) ── task_013 ─ task_014 ─ task_015 ─ task_016 ───────────────────────┤
                                                                                                       ↓
                                                                                                  task_017
```

**并行机会**：
- PR-1 完成后，PR-2 与 PR-3 可并行（不同 Dev / 不同 worktree）
- PR-4 自 PR-1 起即可并行（不依赖 schema V10，仅依赖既有 settings KV）
- PR-3 内部 task_009 / task_010 / task_011 / task_012 可在 task_008 完成后两两并行

**关键路径**：task_002 → task_003 → task_004 → (task_006 ∥ task_009) → task_010 → task_017
（PR-4 路径 task_013→task_014→task_015→task_016 可与之并行进行，不影响关键路径）

---

## Task 粒度自检（全部通过）

| Task | 单一目标 | 独立可测 | ≤2000 行 | 依赖清晰 | AC 客观 |
|------|---------|---------|---------|---------|---------|
| 002–016 | ✓ | ✓（每 task 自带测试） | ✓（最大 ~450 行） | ✓ | ✓ |

---

## 移交说明

- 每个 task 的 `input.md` 已按交接契约 §2 生成于 `sessions/conductor/tasks/task_NNN_*/input.md`
- Conductor 在派发 Dev 前应：(a) 校验前置 task DONE；(b) 复杂度 M 起在 input.md 末尾追加 Reviewer 重点关注项
- PR-2/PR-3 启动前必须有 PR-1 三个 task 全部 DONE 的 progress.md 证据；否则停在 ESCALATE
- PR-4 可在 task_004 完成后立即启动（仅依赖 V1 settings 表）

---

**Architect 签字**：task_001_architect 完成；交付物 = 本 output.md + 16 份 input.md + progress.md 队列更新。
