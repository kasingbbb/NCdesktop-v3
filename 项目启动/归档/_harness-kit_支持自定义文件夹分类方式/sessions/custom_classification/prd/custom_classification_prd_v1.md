# PRD v1 — 自定义文件夹分类方式与 WorkspaceView 修复

> **Session**：custom_classification
> **复杂度**：L（4 层完整 Debate）
> **产出日期**：2026-05-09
> **基线 commit**：67076bd（main）

---

## 1. 项目概述

NCdesktop（NoteCapt）当前具备**文件转录、智能重命名、自动打标签、AI 归类（PARA）**四项核心能力。本次迭代解决三类问题：

1. **修复**：用户在已分类子目录内导入文件后，工作区视图不显示对应资产（Bug 1）。
2. **新增**：将工作区从单层 strip 升级为 Finder 风格的目录浏览视图。
3. **赋能**：允许用户在设置中自定义分类体系，并直接编辑命名 / 分类 / 打标签三段 Prompt。

**经 Layer 1 实证**，Bug 1 的真因是 **schema 失配 + slug 派生不可逆**的复合缺陷（`ai_analyses.topics` 字段类型为 JSON 数组但被写入裸字符串；`sanitize_path_segment` 对含 `/` 的 LLM 输出产生显示↔目录不可逆映射），并非简单的"映射没对上"。本 PRD 在修复 Bug 的同时一并落实自定义分类的数据模型与 Prompt 编辑的容灾设计。

---

## 2. 用户与核心场景

### 2.1 目标用户

本地工作流的研究生 / 独立开发者 / 知识工作者；已使用 PARA 但有个性化分类需求；macOS 重度用户（Finder 心智模型熟悉）。

### 2.2 核心场景

**场景 A — 子目录定向导入**
用户在 P/Q3-论文/refs 子目录视图，拖入 PDF；期望文件落到 `refs/` 当前视图，**不被 LLM 重新分配到其他目录**。

**场景 B — 自定义"课程"分类**
研究生用户在设置 → "工作区分类" 卡片新建 `课程`（slug 自动派生 `课程`，可手改为 `course`）；启用后，导入新讲义可直接路由到该分类，旧 PARA 资产保持原位可见。

**场景 C — Prompt 个性化**
用户改命名 Prompt 为 `YYYY-MM-DD_主题` 风格；保存时系统强制校验（占位符存在 + 输出格式段未破坏 + 在线 dry-run 通过 / 离线 schema-only），下次分类任务即生效。

**场景 D — 视图切换 / 子目录浏览**
用户在 WorkspaceView 中以列表 / 图标视图查看资产，双击进入子目录，面包屑可回退；空目录显示"导入到此分类"按钮（绑定当前 slug，不走 LLM）。

---

## 3. 功能需求（带优先级）

| ID | 功能 | 优先级 | PR |
|---|---|---|---|
| F1 | V10 schema 迁移：`categories` 表 + `category_aliases` 表 + `assets.category_slug` 列 | P0 | PR-1 |
| F2 | `ai_analyses.topics` 读时自愈（解析失败回退 `[]` 并异步写回 sanitize 值）；全表回填脚本 | P0 | PR-1 |
| F3 | 降级启动三档：全成功静默升级 / 部分失败归入 `__uncategorized__` + 首屏 banner / DB 损坏进入只读安全模式 | P0 | PR-1 |
| F4 | 子目录视图导入跳过 LLM 路径决策，直接 `category_slug = 当前视图 slug`；AI 摘要 / 标签后台并行 | P0 | PR-2 |
| F5 | 本地启发式 mismatch toast（不打断流程，提供"点此重选"） | P0 | PR-2 |
| F6 | 后端术语重命名：`ProjectFolderRoot` / `ProjectFolderScope` 谓词；`workspace.rs` 接口语义收紧 | P0 | PR-2 |
| F7 | `list_workspace_assets(project_id, category_slug, sub_path?)` Tauri command（DB 权威 + fs 元数据 + 读时自愈） | P0 | PR-3 |
| F8 | WorkspaceCategorySidebar（基于现有 WorkspaceFolderStrip 升级为纵向） | P0 | PR-3 |
| F9 | 列表视图（图标 / 名称 / 分类 / 标签 / 大小 / 修改时间，按 mtime 倒序）；图标视图 v1 占位（mime 类型图标） | P0 | PR-3 |
| F10 | 面包屑三段（Library > Project > WorkspaceView/...）+ 空目录态"导入到此分类"按钮 | P0 | PR-3 |
| F11 | CategoryManager 平铺 CRUD（新增 / 重命名 / 启停 / 删除）— 删除仅在 builtin=0 且引用计数=0 才显式可见 | P0 | PR-3 |
| F12 | `commands/prompts.rs` 四 command：`get_prompt` / `save_prompt` / `dry_run_prompt` / `reset_prompt`；library 级覆盖；持久化用 V1 已有 `settings` KV 表 | P0 | PR-4 |
| F13 | 三段 Prompt 编辑器（system/user/output，system + output 默认锁，"我知道风险"解锁） | P0 | PR-4 |
| F14 | 占位符 chip 侧栏 + 编辑区未识别 `{xxx}` 红色下划线 + 静态校验"必含变量集合 ⊆ 已用变量集合" | P0 | PR-4 |
| F15 | dry-run 三态容灾：在线必过 / 离线 schema-only + `validated_offline=true` / 用户主动跳过二次确认 | P0 | PR-4 |
| F16 | 三段独立"恢复默认"图标 + 全局"全部恢复默认"二次确认 | P0 | PR-4 |
| F17 | `parent_id` 字段保留 schema 但 MVP 不暴露 UI（CHECK 约束 `IS NULL`） | P1 | PR-1 |
| F18 | 图标视图缩略图（PDF 首页 / 视频首帧 / 图片 thumbnail） | P2 | v2 |
| F19 | parent_id 启用、icon 选择器、拖拽排序 | P2 | v2 |
| F20 | project 级 Prompt 覆盖、Monaco 编辑器、Prompt 版本历史 | P2 | v2 |
| F21 | FS↔DB 双向自愈、Finder IPC 性能优化 | P2 | v2（独立 issue `#finder-ipc-batching`） |

---

## 4. 非功能需求

### 4.1 安全性
- **路径越权**：`slug` 严格白名单 `[a-z0-9一-龥_-]`（含 CJK 表意字符），禁止 `../` 与路径分隔符；`label` 仅展示用，永不参与路径拼接。
- **Prompt 注入**：用户输入的 Prompt 在 Rust 侧用 `str::replace` 注入变量（非 format!/模板引擎），杜绝二次求值；保存前必过占位符与输出格式段静态校验。
- **DB 备份**：V10 迁移保留 `categories_v9_backup` 表 30 天，可手动回滚。

### 4.2 性能
- WorkspaceView 含 1000 文件目录首屏 < 300ms（`list_workspace_assets` 走 DB 索引 + 客户端虚拟滚动）。
- 切换分类筛选 < 50ms（前端 in-memory 过滤）。
- dry-run 在线模式超时 5s 即降级为离线 schema-only。

### 4.3 可用性
- 子目录导入后，资产经 Tauri event `workspace:asset-changed` 推送，前端 invalidate query，**1 秒内可见**，无需手动刷新。
- 离线 / 无 LLM 配置情况下 Prompt 编辑器不阻塞，标记 `validated_offline=true`，下次联网首次执行前再做真实 dry-run。

### 4.4 可维护性
- 内置默认 Prompt 仍在 `src-tauri/src/llm/prompts.rs`（const）；用户覆盖层从 `settings` KV 表读取，渲染时合并；升级内置 Prompt 不影响用户改动。

---

## 5. 技术约束

来自 `session_context.md` §5：
- TypeScript 严格模式；跨进程数据走 `src/lib/tauri-commands.ts` 统一封装。
- Zustand store 副作用集中在 action，组件只 dispatch / 读取。
- Rust：Tauri command 必须 `Result<T, String>`；错误信息中文友好。
- 文件 IO 集中于 `src-tauri/src/workspace.rs`（重命名后语义收紧）；磁盘写入前路径合法性校验。
- Prompt 默认值 + 用户覆盖层分离。

来自 Layer 1 实证：
- `settings` KV 表已存在于 V1 migration（migration.rs:646）→ Prompt 持久化**零 schema 改动**。
- `commands/workspace_folders.rs::list_project_workspace_folders` 已存在但只列一级目录无文件 → 需新增 `list_workspace_assets`。
- `WorkspaceFolderStrip.tsx` 现为横向单层 → 升级为 `WorkspaceCategorySidebar`，复用度 ~60%。

---

## 6. 不可妥协的技术底线（来自 Layer 1）

1. **数据安全底线**：自定义分类生效后，已有 PARA 分类资产必须保持可用；任何物理 mv 必须事务化（tempdir + rename）+ 失败回滚。MVP 默认采用懒保留策略，物理 mv 仅高级路径并要求二次确认。
2. **Prompt 契约底线**：占位符（如 `{content}`、`{filename}`）+ 输出格式段（JSON schema 描述）双重校验；缺失即 disable 保存。
3. **跨项目隔离底线**：工作区映射修复不得引入跨项目串扰；`ProjectFolderScope` 谓词在 dropzone 入口断言。
4. **降级启动底线**：V10 迁移失败不得让 App 拒启；最坏进入只读安全模式 + 导出按钮。

---

## 7. 分期计划

### MVP（本次迭代，4 PR 拆分）

| PR | 范围 | 依赖 | 灰度 |
|---|---|---|---|
| **PR-1 基础设施** | F1, F2, F3, F17 | — | 后端开关：可关闭 V10 启动 |
| **PR-2 Bug 修复** | F4, F5, F6 | PR-1 | feature flag `subdir_direct_import` |
| **PR-3 视图层** | F7, F8, F9, F10, F11 | PR-1 | feature flag `workspace_view_v2` |
| **PR-4 Prompt 编辑** | F12-F16 | 独立可并行 | feature flag `custom_prompts` |

每个 PR 独立可上线、可回滚。

### v2 / v3
- F18 图标视图缩略图
- F19 parent_id 启用 + icon 选择器 + 拖拽排序
- F20 project 级 Prompt + Monaco + 版本历史
- F21 FS↔DB 双向自愈 + Finder IPC 批量化

---

## 8. 风险登记（Layer 3）

| 风险 | 等级 | 缓解 |
|---|---|---|
| 历史数据回填 sanitize 不可逆 | 🔴 高·技术 | dry-run + 备份表 30 天 + 部分失败软降级（归入 `__uncategorized__`） |
| Prompt 误改全项目分类失效 | 🔴 高·产品 | dry-run 三态容灾 + 静态校验 + 一键恢复 + system/output 段二次确认门 |
| Prompt 注入 / 路径越权 | 🟡 中·安全 | slug 严格白名单 + label 不参与路径 + Rust 侧 `str::replace` |
| DB↔FS 一致性偏差 | 🟡 中·技术 | DB 为单一权威源；MVP 仅 DB→FS 单向自愈，FS→DB v2 |
| 空类别堆积 | 🟢 低·产品 | sidebar 折叠"已停用"分组 |

---

## 9. 论证追踪表（最终）

| 论点 | 状态 | 备注 |
|---|---|---|
| Bug 1 根因 = 单纯忽略 workspaceFolderRelativePath | ❌ 已推翻 | 实证后改写为复合缺陷 |
| Bug 1 真因：topics schema 失配 + sanitize 不可逆 | ✅ 已验证 | F2 + F4 解决 |
| 数据模型为单 active project + 子路径 filter | ✅ 已验证 | F6 改名 ProjectFolderScope |
| 自定义分类 = slug+label 二元组 + alias 表 | ✅ 已验证 | F1 |
| Folder View v1 只读（无应用内拖拽改名删除） | ✅ 已验证 | F8-F11 |
| Prompt 编辑 = 占位符 + 输出格式段双校验 + 三态容灾 | ✅ 已验证 | F13-F16 |
| 子目录导入跳过 LLM 路径决策 | ✅ 已验证 | F4 + F5 mismatch toast 兜底 |
| MVP 单 PR | ❌ 已推翻 | 拆 4 PR |
| Finder IPC 性能 | ⏸️ 搁置 | 独立 issue P2 |
| parent_id 启用 | ⏸️ 搁置 | schema 保留 + CHECK 约束，UI v2 |
| UUID 化 category | ⏸️ 搁置 | v2 升级路径 |

---

## 10. 当前 Prompt 全文清单（用户编辑参考）

> 以下为 `src-tauri/src/llm/prompts.rs` 中**将暴露给用户编辑**的三段 Prompt 默认值。Prompt v1.1。

### 10.1 命名 Prompt（`suggestedFileName` 段，嵌入 classify_prompt 内）

```
偏项目/任务：倾向"强动词 + 具象对象/目标 + 关键时间或版本"，如：设计2024Q3官网重构版、招聘前端工程师_05月。
偏领域/资源：「核心责任或兴趣点 + 可选材料类型」，如：健康管理_年度体检汇总、建筑学参考_立面集。
通用文件/素材：极简可检索，可用下划线连接要素，如：会议纪要_XX项目_240510、竞品分析_幻灯片草案。
去掉无意义装饰词，保留可搜索关键词；不要使用路径分隔符或非法文件名字符。
```

**占位符**：`{content}`（必填）

### 10.2 分类 Prompt（`classify_prompt`）

完整内容见 `src-tauri/src/llm/prompts.rs:26-76`，核心 PARA 路由 + 策略过滤 + JSON schema 约束。

**占位符**：`{content}`（必填）
**输出格式段**：
```
- 只输出一段合法 JSON 文本
- 不要使用 markdown 代码块
- 不要在 JSON 前后追加任何解释性句子
- JSON 含：category、tags、confidence(0-1)、language、suggestedFileName
```

### 10.3 打标签 Prompt（`tags` 段，嵌入 classify_prompt 内）

```
tags：3～5 个，短词，偏行动与归宿（如「Q3交付」「会议纪要」「竞品」），避免空洞学科名与纯格式词堆砌。
```

**占位符**：`{content}`（必填）

> **MVP 实现说明**：三段 Prompt 在 v1 仍以"嵌入 classify_prompt"形式存在，但前端编辑器将其拆为三 textarea 渲染（标签段、命名段、整体 system/user/output），保存时再合并回完整 Prompt。这避免了拆分 prompts.rs 的大动作，同时给用户三个独立编辑入口。

---

## Conductor 桥接摘要

> 遵守 `harness-kit/core/handoff_contracts.md` §1 的格式要求。

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|------|--------|-------------|----------------------|
| V10 schema + 回填 + 降级启动 | P0 | 兼容旧 PARA 数据 | 不可拒启；备份表 30 天 |
| 子目录直接归类 + ProjectFolderScope | P0 | 场景 A | 不再调用 LLM 决定路径；mismatch 软提示 |
| WorkspaceCategorySidebar + 列表视图 + 面包屑 | P0 | 场景 D | DB 为权威源，避开 Finder IPC 性能问题 |
| CategoryManager 平铺 CRUD | P0 | 场景 B | 删除仅在 builtin=0 且引用计数=0 显式 |
| Prompt 三段编辑器 + dry-run 三态容灾 | P0 | 场景 C | system/output 锁；离线降级；无 LLM 不阻塞 |

### 不可妥协的技术底线

1. 自定义分类生效后旧资产 0 丢失；任何物理 mv 事务化 + 二次确认
2. Prompt 占位符 + 输出格式段双重校验，缺失即 disable 保存
3. ProjectFolderScope 谓词在 dropzone 入口断言，杜绝跨项目串扰
4. V10 迁移失败不得拒启 App，最坏进入只读安全模式

### 已识别的高风险项

| 风险 | 来源 | 状态 | 缓解策略 |
|------|------|------|----------|
| 历史数据回填 sanitize 不可逆 | Round 3 | 已识别 | dry-run + 备份表 + 部分失败归入 `__uncategorized__` |
| Prompt 误改导致分类失效 | Round 2 L2-D | 已识别 | dry-run 三态 + 静态校验 + 恢复默认 + system/output 锁 |
| Prompt 注入 / 路径越权 | Round 1 | 已识别 | slug 白名单 + label 不入路径 + str::replace |
| DB↔FS 一致性 | Round 4 | 已识别 | MVP 单向 DB→FS；FS→DB v2 |
| Finder IPC 性能 | Round 1 L2-E | 已搁置 | 独立 issue `#finder-ipc-batching`，P2 |
| `topics` 字段 JSON 失配 | Round 3 实证 | 已识别 | F2 读时自愈 + 全表回填 |

### MVP 边界声明

**做什么（4 PR）**：
- PR-1 基础设施（schema + 回填 + 降级启动 + topics 自愈）
- PR-2 Bug 修复（子目录直接归类 + ProjectFolderScope + mismatch toast）
- PR-3 视图层（WorkspaceCategorySidebar + 列表视图 + 面包屑 + CategoryManager 平铺 CRUD + 空目录态）
- PR-4 Prompt 编辑（三段编辑器 + dry-run 三态 + 占位符校验 + 恢复默认）

**不做什么（v2 及以后）**：
- 应用内拖拽移动 / 改名 / 删除文件 — 数据安全风险高，先用"在 Finder 中显示"逃生口
- 图标视图缩略图（PDF 首页 / 视频首帧）— 依赖 v5 抽取产物，单独迭代
- parent_id 树形启用 + icon 选择器 + 拖拽排序 — schema 已留空间，UI v2
- project 级 Prompt 覆盖 / Monaco 编辑器 / 版本历史 — 范围蔓延
- FS↔DB 双向自愈 — MVP 仅单向
- Finder IPC 批量化性能优化 — 独立 issue P2
- UUID 化 category schema — v2 升级路径

### Debate 中未达成共识的争议

无。所有争议均在 Layer 1 / 2 / 3 闭环。

**Architect 在设计 task 拆分时需特别注意**：
1. PR-1 是阻塞 PR-2 / PR-3 的硬依赖，不可并行实现，但 PR-4 完全独立可并行。
2. F4 子目录直接归类的实现需要前端 dropzone 上传时同时传递 `workspaceFolderRelativePath`，后端断言"该 path 对应有效 category_slug"才跳过 LLM；否则 fallback 到 LLM 分类（保留向后兼容）。
3. F15 dry-run 三态需要后端先暴露"LLM 健康检查"endpoint 或前端通过 `llmStore` 状态推断在线/离线。

---

**PRD 版本**：v1.0
**下一步**：Conductor 读取本 PRD + session_context.md，进入 ARCHITECTURE 状态，启动 Architect 拆 task。
