# Session Context — 工作区文件夹管理（悬浮窗导入页面）

---

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop · 悬浮窗导入页面新增工作区文件夹管理
- **一句话描述**：在「项目 → 悬浮窗导入」页面把当前只读的工作区 chip 条升级为可写的 Finder 列表风组件，支持新建/重命名/删除/拖拽移动素材到文件夹。
- **项目类型**：Desktop App（Tauri）
- **复杂度等级**：L（4 层完整 Debate）
  - 理由：UI 改造 + 文件系统写操作 + DB 事务一致性 + 系统回收站调用 + 路径越界安全约束，多维度都涉及风险点。

---

## 2. 技术上下文 [必填]

- **主语言**：TypeScript（前端）+ Rust（后端 Tauri）
- **框架/运行时**：Tauri + React 19 + Zustand
- **数据库**：SQLite
- **关键外部依赖**：`trash` crate（待引入，移到系统回收站）、lucide-react（图标）
- **现有代码库**：改造现有代码
- **目标部署环境**：本地（macOS 优先；Windows/Linux 删除命令本期返回未支持错误）

---

## 3. 关键约束 [必填]

- **安全性要求**：高 — 工作区写操作必须做路径越界保护（canonicalize 后必须仍在 `project_workspace_dir(project_id)` 下），`ai_organized` 严禁应用内写入。
- **性能要求**：中 — 单素材拖拽即时响应；rename 同事务批量更新 asset 路径不应阻塞 UI 超过 200ms（中等量级）。
- **用户体验要求**：中 — 视觉向 macOS Finder「列表视图」靠拢，但不做像素级复刻；交互需顺手（inline 编辑、Enter 提交、Esc 取消、拖拽高亮）。
- **可维护性要求**：中 — 沿用现有 `WorkspaceFolderEntry.kind` 枚举，前后端命名约定统一（前端 camelCase、后端 snake_case）。
- **不可妥协的底线**：
  1. `ai_organized` 在前后端各拦一次，禁止任何应用内写操作（重命名/删除/写入/拖入）。
  2. 所有写命令必须 `canonicalize()` 后校验仍在项目工作区根目录下，构造 `../../etc` 类越界请求必须拒绝。
  3. 删除走系统回收站，禁止 `fs::remove_dir_all` 硬删。
  4. rename / move 必须在同一 SQL 事务中更新所有受影响 `assets.file_path`，并通知前端刷新。
  5. 命名校验：禁含 `/ \ :`、禁以 `.` 开头、禁与同级同名、禁用保留字 `organized`。前后端各校验一次，**后端为权威**。
  6. 跨平台路径正斜杠规范化沿用现有约定（`replace(/\\/g, "/")`）。

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 25% | 4 个 P0 动作必须按验收标准全过 |
| 安全性 | 25% | 路径越界、`ai_organized` 保护、回收站 |
| 代码质量 | 15% | 沿用现有约定，不引入无关重构 |
| 测试覆盖 | 20% | Rust 单测覆盖越界/保留字/受保护写；至少 1 个集成测试覆盖 rename 后 DB 同步 |
| 架构一致性 | 10% | 命令注册到 `invoke_handler!`、前端在 `tauri-commands.ts` 加 camelCase 封装 |
| 可维护性 | 5% | — |

> 总和 100%。安全性和测试权重提高，因为本期核心是「写操作首次落地」。

---

## 5. 领域特定代码规范

```
- TS：React 19 函数组件，状态用 Zustand。新增组件放 src/components/features/。
- Tauri 命令：所有新命令必须注册到 src-tauri/src/lib.rs 的 invoke_handler!。
- 前端 IPC 封装：在 src/lib/tauri-commands.ts 加 camelCase 包装函数。
- 路径：跨平台用正斜杠归一（replace(/\\\\/g, "/")）；伪根目录 relativePath = "__ROOT__"。
- 错误处理：后端命令统一返回 Result<T, String>；用户可读的中文错误信息。
- 不要顺手改无关代码；commit 用中文 Conventional Commits（参考 chore(fixtures): …）。
```

---

## 6. 领域特定审查重点

```
- 写命令是否 canonicalize() 后再校验在 project_workspace_dir 下。
- ai_organized kind 的写操作是否在前后端两层都被拦截。
- rename / move 是否在同一 SQL 事务中完成「物理 rename + DB 批量 file_path 前缀替换」。
- 删除是否调用 trash crate（macOS），未走 fs::remove_dir_all。
- 命名校验：保留字 organized、`/ \ :`、`.` 开头、同级同名。
- Windows/Linux 平台未实现的命令是否返回明确错误（与现有 reveal 命令对齐）。
- 拖拽：drop target 高亮是否仅 2px 内描边而非整行反色；drop 到 ai_organized 是否被前端阻止 + toast。
- inline 编辑：Enter 提交 / Esc 取消 / 失焦提交 三态是否齐全。
```

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：
  Tauri IPC 设计、Rust 文件系统安全（canonicalize、路径越界）、SQLite 事务、HTML5 拖拽 API、React 受控组件 inline 编辑、macOS Finder 列表视图交互范式、`trash` crate / `osascript`。
- **Reviewer 应重点关注的风险域**：
  路径越界（symlink 攻击、`..` 注入）、跨设备 rename 失败（EXDEV）、SQL 前缀替换误伤（前缀冲突如 `参考` vs `参考资料`）、并发写竞态、删除目录非空时的回收站行为、Tauri v2 拖拽事件支持差异、深色模式色值漏配。

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`product/prd/workspace_folder_mgmt_prd_v1.md`
- **源码路径**：项目实际位于 `NCdesktop/`（与本 harness-kit 同级）；本 session 仅产出 PRD/进度文档，不直接落码到 `product/src/`。
- **Session 记录路径**：`sessions/workspace_folder_mgmt/`
- **Debate 记录**：`sessions/workspace_folder_mgmt/debate/session_001/{debate_log.md, debate_conclusions.md}`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`（后续 Conductor 阶段填充）

---

## 9. 辩题概述

- **核心辩题**：在「ai_organized 受保护、路径越界必须拒绝、删除必须走回收站」三条硬约束下，如何用最小代价把「悬浮窗导入」页面从只读 chip 升级为可写 Finder 列表，并保证 rename/move 的 DB 事务一致性？
- **辩论偏好**：
  - 重点辩论层：问题定义 + 差距分析 + 策略（理想态稍轻）
  - 最关心的维度：安全（路径越界 + 受保护目录） + 体验（inline 编辑 + 拖拽反馈）+ 数据一致性（事务）

---

## 10. 用户原始需求（Verbatim 注入，作为辩论的事实基础）

> 来源：PM 在启动会话时的需求陈述。下文为原文，Debate 各角色必须以此为事实基础，不得虚构需求。

### 背景

NCdesktop（Tauri + React 19 + Zustand + SQLite）当前的"项目 → 悬浮窗导入"页面只能把工作区文件夹当筛选 chip 用，无法做任何写操作。本期补齐 **新建 / 重命名 / 删除 / 拖拽移动素材** 这四个核心动作；视觉与交互向 macOS Finder「列表视图」靠拢，但**不做像素级复刻**——风格对、操作顺手即可。

工作区物理路径：`~/Downloads/NoteCaptWorkPlace/<projectId>/`
子目录种类（沿用现有 `WorkspaceFolderEntry.kind`）：
- `organized/<AI 归类名>/` → `ai_organized`（**受保护，不可写**）
- 根目录散文件 → `root_import`（伪 relativePath `__ROOT__`）
- 用户自建根级文件夹 → `root`

### 必读现有代码

- 页面容器：`src/components/features/AssetListView.tsx`
- 待替换的 chip 条：`src/components/features/WorkspaceFolderStrip.tsx`
- IPC 封装：`src/lib/tauri-commands.ts`
- UI 状态：`src/stores/uiStore.ts`（`workspaceFolderRelativePath`）
- 类型：`src/types/workspace.ts`
- 后端命令：`src-tauri/src/commands/workspace_folders.rs`
- 后端工具：`src-tauri/src/workspace.rs`
- 命令注册：`src-tauri/src/lib.rs` 的 `invoke_handler!`

### P0｜必做（本期核心交付）

#### F1 新建文件夹
- 入口：列表上方工具栏 `+ 新建文件夹` 按钮；可选键盘 `⌘⇧N`。
- 行为：在列表末尾插入一行空白可编辑项，默认名 `未命名文件夹`，名称全选进入 inline 编辑。Enter 提交，Esc 取消，失焦提交。
- 仅在项目根级新建（不嵌套）。
- 成功后该新文件夹自动选中。

#### F2 重命名
- 入口：右键菜单"重命名" / 选中后按 `Enter` / 工具栏按钮。
- 行为：行内 inline 编辑，全选当前名，Enter 提交 / Esc 取消。
- 仅 `root` 文件夹可重命名；`ai_organized` 与 `__ROOT__` 入口灰显或不显。
- 后端必须同事务更新数据库中所有 `file_path` 前缀匹配旧路径的 asset 记录。

#### F3 删除（移到系统回收站）
- 入口：右键菜单"移到废纸篓" / 选中后按 `⌘⌫` / 工具栏按钮。
- 行为：弹二次确认。若文件夹非空，确认文案明示数量：`"该文件夹包含 N 个素材，一同移到废纸篓？"`
- 仅 `root` 文件夹可删除。
- macOS 实现：用 `trash` crate（推荐）或 `osascript tell application "Finder" to delete`；禁止 `fs::remove_dir_all` 硬删。
- Windows/Linux 本期返回 `Err("当前平台不支持")`，与现有 `reveal` 命令对齐。

#### F4 拖拽素材移动到文件夹
- drag source：AssetListView 右栏"工作区"中的素材卡片。
- drop target：文件夹列表的行（`root` 与 `__ROOT__`）。
- drag enter 时目标行高亮（2px 内描边 `var(--border-active)`，不要整行反色）。
- drop 触发 `move_asset_to_workspace_folder`。
- 不允许 drop 到 `ai_organized` 行；前端阻止 + toast 提示 "AI 归类目录受保护，不可手动移入"。
- 后端：物理 `fs::rename`（同卷）；数据库同事务更新 `assets.file_path`。
- 本期只做单素材拖拽；多选拖拽留二期。

### P1｜应做（让上面 4 个动作有像样的载体）— 见原始需求文档（视图 / 交互 / 工具栏三块）。

### P2｜可选（不做不扣分）— 见原始需求文档。

### 后端新增 Tauri 命令签名建议

```rust
pub fn create_workspace_folder(project_id: String, name: String) -> Result<WorkspaceFolderEntry, String>;
pub fn rename_workspace_folder(project_id: String, relative_path: String, new_name: String) -> Result<WorkspaceFolderEntry, String>;
pub fn delete_workspace_folder(project_id: String, relative_path: String, confirm_non_empty: bool) -> Result<(), String>;
pub fn move_asset_to_workspace_folder(asset_id: String, target_relative_path: String) -> Result<Asset, String>;
```

### 验收标准（P0 必须全过）

1. 工具栏 `+` 或 `⌘⇧N` → 列表末尾插入空白可编辑行 → 输入"参考资料" Enter → 物理目录 `~/Downloads/NoteCaptWorkPlace/<projectId>/参考资料/` 出现，列表刷新，该行被选中。
2. 选中 `root` 行按 Enter → 行内编辑全选 → 改名 Enter → 物理目录改名 + DB 中所有 `file_path` 前缀匹配旧路径的 asset 同步更新 + 素材列表实时重渲染。
3. 选中含 2 个 asset 的 `root` 行按 ⌘⌫ → 弹确认含"包含 2 个素材"→ 确认后 macOS 废纸篓中可见该目录，DB 中两个 asset 按 `delete_asset` 现有语义处理。
4. 把素材卡片拖到目标 `root` 行 → 目标行 2px 内描边 → 松手 → 文件物理移动、DB 更新、筛选切到目标后该素材可见，原位置不可见。
5. 对 `organized/1-项目` 行尝试重命名 / 删除 / 拖入素材 → 全部被前端阻止或后端拒绝；后端被直接调用也必须返回错误。
6. 构造 `relative_path = "../../etc"` 之类越界请求 → 后端拒绝（Rust 单测覆盖）。

### 测试

- `pnpm test` 全绿。新增组件测试覆盖：行 inline 编辑提交/取消、双击进入筛选。
- `cargo test --manifest-path src-tauri/Cargo.toml` 全绿。新增 Rust 单测：路径越界拒绝、保留字 `organized` 拒绝、`ai_organized` 写操作拒绝。
- 至少一个集成测试覆盖"rename 后 DB `file_path` 前缀同步"。

### 交付

- 单个 PR，commit 用中文 Conventional Commits（参考 `chore(fixtures): …`）。
- PR 描述贴一张新列表截图 + 一段 10 秒 GIF：演示新建 → 重命名 → 拖入素材 → 删除四连。
- 不要顺手改无关代码。
- **本期只需完成 P0 需求**。
