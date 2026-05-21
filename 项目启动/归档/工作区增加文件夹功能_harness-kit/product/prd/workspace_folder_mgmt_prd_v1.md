# PRD v1 — NCdesktop · 悬浮窗导入页面新增工作区文件夹管理

> 来源：`sessions/workspace_folder_mgmt/` Debate session_001（4 层 Host 主持）
> 复杂度：L｜目标 MVP：P0 必做四件套全部上线
> 本文档末尾附 **Conductor 桥接摘要**（handoff_contracts §1 必填项）

---

## 1. 项目概述

把 NCdesktop（Tauri + React 19 + Zustand + SQLite）「项目 → 悬浮窗导入」页面当前**只读** chip 工作区文件夹条，升级为可写的 Finder 列表风组件 `WorkspaceFolderListView`，落地以下四件素材整理动作：

- **F1 新建** 用户自建根级文件夹
- **F2 重命名** 用户文件夹（含 DB 中所有受影响 `assets.file_path` 同事务前缀替换）
- **F3 删除** 用户文件夹（移到 macOS 系统回收站，禁硬删；Win/Linux 返回未支持）
- **F4 拖拽** 单素材从右栏拖入目标行，物理 `fs::rename` + 数据库同事务更新

视觉/交互风格向 macOS Finder「列表视图」靠拢，但**不做像素级复刻**。

---

## 2. 用户定义与核心场景

**核心用户**：单人创作者/研究者，熟悉 macOS Finder 范式，中等熟练度，不依赖 CLI。

**端到端使用场景**：
> 用户批量导入 30 个网页剪藏 → 打开悬浮窗导入页 → 工具栏点 `+ 新建文件夹` → 输入"参考资料" Enter → 从右栏把 5 张截图逐张拖入「参考资料」行（每次行内 2px 内描边反馈）→ 想把误归入的一张图还原回根 → 把它从「参考资料」拖回 `__ROOT__` 行（双向合法）→ 选中"草稿"旧目录按 ⌘⌫ → 二次确认含「该文件夹包含 0 个素材，一同移到废纸篓？」→ 确认进废纸篓。**全程不离开悬浮窗**。

---

## 3. 功能需求（带优先级）

### P0｜本期必交付

| ID | 功能 | 关键行为 |
|---|---|---|
| F1 | 新建文件夹 | 工具栏 `+ 新建文件夹` 按钮触发；列表末尾插入空白可编辑幽灵行，默认 `未命名文件夹` 名称全选进 inline 编辑；Enter/blur 同步乐观提交，Esc 取消；成功后该行自动选中；**仅在项目根级新建**。失败时保留编辑态 + 红框 + 行内 error；切走需"放弃新建『xxx』？"二次确认 modal。 |
| F2 | 重命名 | 三入口：右键菜单"重命名" / 选中按 `Enter` / 工具栏"重命名"按钮；行内 inline 编辑全选当前名；Enter/blur 提交、Esc 取消；同步乐观 + `pendingRenameIds` selection 冻结；后端事务内更新所有受影响 `assets.file_path` 前缀；**仅 `root` 可重命名**，`ai_organized` 灰显并提示受保护，`__ROOT__` 隐藏该项。 |
| F3 | 删除（移到回收站） | 三入口：右键"移到废纸篓" / `⌘⌫` / 工具栏"移到废纸篓"按钮；二次确认 modal，非空时文案 `"该文件夹包含 N 个素材，一同移到废纸篓？"`；macOS 走 `trash` crate，删除后 `path.exists()` 复检；事务内 recount 比对 `expected_count`，不一致返回 `E_FOLDER_DIRTY` 让前端重新确认；Win/Linux 返回 `E_PLATFORM_UNSUPPORTED`；**仅 `root` 可删除**。 |
| F4 | 拖拽素材到文件夹 | drag source = 右栏素材卡片；drop target = `root` 与 `__ROOT__`（**双向合法**）；dragenter 时目标行 inset 2px 描边 `var(--accent-emphasis)`；drop 到 `ai_organized` 前端阻止 + toast `"AI 归类目录受保护，不可手动移入"`；drop 到正在编辑的行禁止图标 + toast；后端：物理 `fs::rename`（同卷），EXDEV 时走 copy-first 两阶段；DB 同事务更新 `assets.file_path`。本期单素材，多选 P2。 |
| 工具栏 | 36px 三按钮 | `+ 新建文件夹`（永激活）、`重命名`（仅 root 激活）、`移到废纸篓`（仅 root 激活）；面包屑/列显隐/视图切换全不做。 |
| 右键菜单 | 三 kind 形态 | `root`：重命名 / 移到废纸篓 / — / 在 Finder 中显示；`ai_organized`：仅"在 Finder 中显示"+ 其余项灰显 + tooltip；`__ROOT__`：仅"在 Finder 中显示"（重命名/删除不显示，**非灰显**）；空白处：新建文件夹。 |
| 列表视图 | 表格式排版 | 列头一行（浅灰背景 + 1px 底线 + 13px 半粗）+ 数据行；列：①名称（弹性宽 + 文件夹图标 16px）②项目数（前端 assetStore 聚合，右对齐）③修改时间（`MM/DD HH:mm`，右对齐）；行高 ~24px；选中行 `var(--border-active)` 背景+文字反白；hover `rgba(0,0,0,0.04)` / 深色 `rgba(255,255,255,0.06)`；`ai_organized` 行图标右下 ✨ 小角标（lucide `Sparkles` 8px）；无斑马纹/无行分隔线/无复杂阴影；列宽固定。 |

### P1｜紧时可砍

- `⌘⇧N` 新建快捷键（spec 标"可选"）
- 深色模式 hover/disabled 灰阶精修（drop 高亮已 P0）

### P2｜明确不做（附原因）

- 嵌套子文件夹（spec 明示根级）
- 多选 / 多素材拖拽（spec 明示二期）
- Windows/Linux 删除实现（与现有 reveal 命令对齐）
- 列宽拖 / 列排序 / 列显隐 / 视图切换 / 面包屑（spec 明示 P2）
- 像素级 Finder 复刻、动效精修
- 多语言（项目无 i18n 框架）
- 撤销栈
- 跨进程并发锁（单进程多窗口足够）

---

## 4. 非功能需求

### 4.1 安全（HIGH）

1. **路径越界保护**：所有写命令必须经 `validate_and_canonicalize(project_id, relative_path)`，`canonicalize()` 后 `starts_with(workspace_root_canonical)` 必须为真；拒绝包含 `..`、绝对路径、symlink 越界的输入。Rust 单测覆盖。
2. **`ai_organized` 双层拦截**：前端 disable + 后端 handler 入口拒绝（即使绕过 UI 直接调用也必须 `Err(E_PROTECTED_KIND)`）。
3. **handler 入口统一权限判定**：所有键盘/工具栏/菜单 handler 第一行 `if (selection.kind !== 'root') return;`，不依赖 UI disabled。
4. **命名校验后端权威**：禁含 `/ \ :`、禁以 `.` 开头、禁与同级同名、禁用保留字 `organized`；前端同步校验仅做即时反馈。

### 4.2 数据一致性（HIGH）

1. **rename/move 同事务**：物理 `fs::rename` + DB 前缀替换在同一 SQL 事务内完成；前缀替换 SQL：
   ```sql
   UPDATE assets
   SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1)
   WHERE file_path = :old_prefix
      OR file_path LIKE :old_prefix || '/%' ESCAPE '\';
   ```
   `:old_prefix` 必须带尾 `/`；`:old_prefix` 中 `\ % _` 预转义。
2. **EXDEV 跨卷 copy-first 两阶段**：`copy_dir(src→dst_tmp) → fsync → rename(tmp→final) → BEGIN/UPDATE DB/COMMIT → COMMIT 后 remove src`。remove 失败仅记 `cleanup_pending` 日志，启动期扫描清理。**最坏=磁盘多占用，绝不丢数据/不产孤儿**。
3. **删除 TOCTOU**：`delete_workspace_folder` 入参带 `expected_count`，后端事务内重 count，不一致返回 `E_FOLDER_DIRTY{old, now}` 让前端重新确认；trash 后扫描残留物理文件，对每个落入回收站的 asset 调用现有 `delete_asset` 语义同步 DB。
4. **写通道锁**：进程内 `Mutex<ProjectId>` 串行化 `{create_workspace_folder, rename_workspace_folder, delete_workspace_folder, move_asset_to_workspace_folder, import}`；read & 缩略图除外。
5. **NFC 自愈**：启动期扫描工作区目录，对 NFD 字节文件 `rename(B→nfc(B))` 一次性归一；`assets.file_path` 始终存 NFC。

### 4.3 错误模型

```ts
type IpcError = {
  code: 'E_NAME_INVALID' | 'E_NAME_DUP' | 'E_NAME_RESERVED'
      | 'E_PATH_ESCAPE' | 'E_PROTECTED_KIND' | 'E_NOT_FOUND'
      | 'E_CROSS_DEVICE' | 'E_PLATFORM_UNSUPPORTED'
      | 'E_TRASH_FAILED' | 'E_FOLDER_DIRTY' | 'E_INTERNAL';
  message: string;       // 仅日志/上报
  details?: Record<string, unknown>;
}
```
- Tauri invoke 边界以 `serde_json::to_string(&IpcError)` 序列化为 error string，前端 `JSON.parse` 还原。
- 前端文案表 `errorMessages[code](details)` 是用户可见文案的唯一来源；后端 `message` 不展示。

### 4.4 性能

- inline 编辑/选中/hover 响应 < 16ms（单帧）。
- rename / move 单次同卷操作 ≤ 200ms（中等量级）。
- EXDEV copy-first 跨卷大目录可降级（>1GB 接受 toast「正在跨卷迁移」）。

### 4.5 可用性

- inline 编辑 3 态完整：Enter / Esc / blur。
- 拖拽 4 阶段反馈完整：enter/over/leave/drop。
- F4 drop indicator 在浅/深色模式均可见（用 `var(--accent-emphasis)` token）。

---

## 5. 技术约束（来自 session_context）

- 前端：TS + React 19 + Zustand；新组件放 `src/components/features/`；不新增 store。
- 后端：Rust + Tauri；所有新命令注册到 `src-tauri/src/lib.rs` `invoke_handler!`；统一 `Result<T, IpcError>`。
- 路径：跨平台正斜杠归一（`replace(/\\/g, "/")`）；`__ROOT__` 仅 UI/IPC sentinel，DB 存空相对路径（根文件存裸文件名）；`assets` 写路径处加 `debug_assert!(!path.contains("__ROOT__"))`。
- 命令命名：前端 camelCase 包装在 `src/lib/tauri-commands.ts`，后端 snake_case。
- 新依赖：`trash`、`unicode-normalization`。
- Commit：中文 Conventional Commits（如 `feat(workspace): 新增工作区文件夹管理`）。
- 不顺手改无关代码。

### 5.1 新增 Tauri 命令签名（最终）

```rust
#[tauri::command]
pub fn create_workspace_folder(project_id: String, name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;

#[tauri::command]
pub fn rename_workspace_folder(project_id: String, relative_path: String, new_name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;

#[tauri::command]
pub fn delete_workspace_folder(
    project_id: String,
    relative_path: String,
    confirm_non_empty: bool,
    expected_count: u32,
) -> Result<DeleteReport /* { trashed: u32 } */, IpcError>;

#[tauri::command]
pub fn move_asset_to_workspace_folder(asset_id: String, target_relative_path: String)
    -> Result<Asset, IpcError>;

#[tauri::command]
pub fn count_folder_assets(project_id: String, relative_path: String)
    -> Result<u32, IpcError>;
```

### 5.2 前端状态（修改 `src/stores/uiStore.ts`）

```ts
workspaceFolderRelativePath: string | null   // 既有，selection
editingFolderPath: string | null              // 新增，编辑态
pendingNewFolder: boolean                     // 新增，F1 幽灵行存在标识
pendingRenameIds: Set<string>                 // 新增，selection 冻结集
dragOverPath: string | null                   // 新增，drop 候选态
```

---

## 6. 测试要求

### 6.1 Rust 单测（`cargo test --manifest-path src-tauri/Cargo.toml` 全绿）

- 路径越界：`../../etc`、`/etc/passwd`、symlink 指向 `/tmp` 三例均返 `E_PATH_ESCAPE`。
- 保留字 `organized` create/rename 返 `E_NAME_RESERVED`。
- `ai_organized` 四类写（rename/delete/create 子目录/move 入）均返 `E_PROTECTED_KIND`。
- SQL 前缀边界：建 `100%off` 与 `100` 两目录，rename `100→200`，断言 `100%off` 下 asset 路径未变。
- NFC 归一：NFD `"参考"` vs NFC `"参考"` 视为同名，第二次 create 返 `E_NAME_DUP`。

### 6.2 Rust 集成测试

- **`test_rename_db_path_sync`**：rename 后 DB 受影响行数 = 物理子树文件数。
- **`test_round_trip_root_to_folder_to_root`**：素材 `__ROOT__` → folder → `__ROOT__` 一轮，物理位置与 DB `file_path` 双向一致。
- **`test_exdev_two_phase`**：mock EXDEV，验证 copy-first 顺序与回滚。
- **`test_delete_dirty_recount`**：confirm 后并发往目录塞素材 → 后端返 `E_FOLDER_DIRTY`。

### 6.3 前端单测（`pnpm test` 全绿）

- 行 inline 编辑 Enter 提交 / Esc 取消 / blur 提交三态。
- 幽灵行失败保留编辑态 + 切走二次确认 modal。
- selection 冻结：`pendingRenameIds` 非空时点击其他行无响应。
- drop 到 `ai_organized` 触发 toast 且不发 IPC。
- drop 到正在编辑行禁止 + toast。
- 双击 root 行触发筛选切换。

### 6.4 手动验收（PR 描述附截图 + 10s GIF）

按用户原 spec §验收 1-6 全过：
1. `+` 或 `⌘⇧N` → 新建"参考资料"→ 物理目录出现+列表刷新+自动选中。
2. 选中 root 行 Enter → 改名 → 物理改名 + DB asset 同步 + 素材列表实时重渲染。
3. 含 2 asset 的 root 行 `⌘⌫` → 确认"包含 2 个素材"→ 废纸篓可见 + DB 按 `delete_asset` 现有语义。
4. 拖素材到 root 行 → 2px 描边 → 文件物理移动 + DB 更新 + 筛选切换后可见。
5. 对 `organized/1-项目` 行重命名/删除/拖入 → 前端阻止或后端拒绝；后端直接调用也返 `E_PROTECTED_KIND`。
6. `relative_path = "../../etc"` 越界请求 → 后端拒绝（单测覆盖）。

---

## 7. 分期计划（Task 拆分，8 task）

| # | Task | 依赖 | 关键交付 |
|---|---|---|---|
| T0 | 契约冻结 | — | `contracts.md`：IpcError shape、5 命令签名、`__ROOT__` 编解码、错误码枚举表 |
| T1 | 后端工具层 | T0 | `trash`+`unicode-normalization` 依赖；`validate_and_canonicalize`、`nfc_normalize`、`resolve_relative_path`；写通道 `Mutex`；`IpcError` enum；启动期 NFC 自愈 hook |
| T2 | 前端 IPC 封装（与 T1 并行） | T0 | `tauri-commands.ts` 5 个 camelCase；`IpcError` TS 类型；`errorMessages` 文案表 |
| T3 | 4 个写命令实现（与 T4 并行） | T1 | 4 命令注册 + handler 入口判定 + Rust 单测全套 |
| T4 | `count_folder_assets` + 状态 + 集成测试（与 T3 并行） | T1 | read 命令；uiStore 5 字段；2 个集成测试 |
| T5a | 列表骨架 | T2 | `WorkspaceFolderListView`：列表渲染 + 选中态 + 工具栏三按钮 + 右键菜单 + 三 kind 灰显 + 键盘 handler 入口判定 |
| T5b | inline 编辑状态机 | T5a | `mode: 'idle'\|'creating'\|'renaming'` + 三态 + 幽灵行 + selection 冻结 + 失败保留态 + 切走二次确认 modal + 组件单测 |
| T6 | F4 拖拽 + 集成测试 + GIF | T3,T4,T5b | drop 高亮 2px（深色 token）+ `__ROOT__` 双向 + ai_organized toast；rename/delete 集成测试；PR 截图+10sGIF |

---

## 8. 交付门槛 Checklist（PR Ready）

- [ ] 4 写命令 + 1 read 命令注册到 `invoke_handler!`
- [ ] `tauri-commands.ts` camelCase 封装齐全，错误用 IpcError JSON 协议
- [ ] 所有写命令入口 `validate_and_canonicalize` + ai_organized 拦截
- [ ] 写通道 `Mutex<ProjectId>` 覆盖 5 命令 + import
- [ ] `trash` + `unicode-normalization` 加入 `Cargo.toml`；Win/Linux 删除返 `E_PLATFORM_UNSUPPORTED`
- [ ] rename/move 在事务内完成 + EXDEV copy-first 两阶段实现
- [ ] 删除 `expected_count` 不一致返 `E_FOLDER_DIRTY`；trash 后残留扫描
- [ ] 启动期 NFC 自愈扫描挂入现有 init hook
- [ ] handler 入口统一 `kind === 'root'` 判定（不依赖 UI disabled）
- [ ] F1 幽灵行失败保留编辑态 + 切走二次确认 modal
- [ ] F2 同步乐观 + `pendingRenameIds` selection 冻结
- [ ] F4 drop 高亮 `var(--accent-emphasis)` 2px + ai_organized toast + 编辑行禁止 + `__ROOT__` 双向
- [ ] 工具栏 3 按钮齐全 + 灰显联动正确
- [ ] 右键菜单 3 kind 形态正确（`__ROOT__` 隐藏 rename/delete 不灰显）
- [ ] Rust 单测：越界 / 保留字 / ai_organized 写 / 前缀边界 / NFC
- [ ] Rust 集成测试：rename DB 同步 / `__ROOT__` round-trip / EXDEV 两阶段 / dirty recount
- [ ] 前端单测：inline 三态 / 幽灵行 / drop 拦截 / 双击进入筛选
- [ ] `pnpm test` + `cargo test` 全绿
- [ ] PR 描述附新列表截图 + 10s GIF（新建→重命名→拖入→删除四连）
- [ ] Commit 用中文 Conventional Commits，无无关改动

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|---|---|---|---|
| F1 新建文件夹 | P0 | 用户在悬浮窗导入页根级新建 root 文件夹 | L2：失败保留编辑态 + 切走二次确认；幽灵行不入 DB 直到 IPC 成功 |
| F2 重命名 | P0 | 选中 root 行改名 | L1：rename SQL 前缀必须带尾 `/` ESCAPE；L2：同步乐观 + selection 冻结；仅 root 可改 |
| F3 删除（移到回收站） | P0 | 选中 root 行 `⌘⌫` 触发二次确认 | L3 R4：事务内 recount，不一致返 `E_FOLDER_DIRTY`；写通道锁；trash 后残留扫描 |
| F4 拖拽移动素材 | P0 | 单素材从右栏拖入文件夹/拖回 `__ROOT__` | L1：`__ROOT__` 双向合法；L3 R1：EXDEV copy-first 两阶段；drop 编辑行禁止 |
| 5 新 IPC + IpcError | P0 | 前后端契约基线 | L2：结构化 JSON `{code,message,details}`；前端文案表唯一来源 |
| 工具栏 3 按钮 + ⌘⌫ | P0 | spec 入口完整性 | L4 Host 裁决：曾被 Proposer 错降 P1，恢复 P0 |
| `⌘⇧N` 快捷键 | P1 | F1 备用入口 | spec 标"可选" |
| 嵌套 / 多选拖 / Win-Linux 删除 / 列宽拖 | P2 | — | spec 明示推后 |

### 不可妥协的技术底线

1. `ai_organized` 前后端各拦一次（前端 disable + 后端 `E_PROTECTED_KIND`），handler 入口判定不依赖 UI disabled。
2. 所有写命令 `validate_and_canonicalize()` 后必须仍在 `project_workspace_dir(project_id)` 下；拒绝 `..` / 绝对路径 / symlink 越界。
3. 删除走 `trash` crate（macOS），删除后 `path.exists()` 复检；**严禁 `fs::remove_dir_all` 硬删**。
4. rename / move 在同一 SQL 事务中完成「物理 rename + DB 前缀替换」；SQL 前缀用 `LIKE :p || '/%' ESCAPE '\\'`，`:p` 强制带尾 `/`，预转义 `\ % _`。
5. EXDEV 走 **copy-first 两阶段**（copy→fsync→rename→COMMIT→remove src），失败不丢数据。
6. `__ROOT__` 仅 UI/IPC sentinel，**永不入 DB**；`assets.file_path` 根文件存裸文件名；`assets` 写路径加 `debug_assert!(!path.contains("__ROOT__"))`。
7. 写通道 `Mutex<ProjectId>` 串行 5 命令 + import；read & 缩略图除外。
8. 启动期 NFC 自愈扫描归一 NFD 字节文件。
9. 命名校验后端权威（禁 `/ \ :`、禁 `.` 开头、禁同级同名、禁保留字 `organized`）。
10. 错误统一 `IpcError JSON` 序列化；前端只按 code 出文案，后端 message 仅日志。

### 已识别的高风险项

| 风险 | 来源（Debate 哪一轮） | 当前状态 | 缓解策略 |
|---|---|---|---|
| EXDEV 跨卷 rename 失败 / 部分状态残留 | L3 R1 | 已解决 | copy-first 两阶段，COMMIT 后再 remove 源 |
| 路径越界（`..` / symlink） | L3 R2 | 已解决 | `validate_and_canonicalize` + 单测 |
| SQL `LIKE` 元字符 `%_\` 误伤 | L3 R3 | 已解决 | `ESCAPE '\\'` + 转义 + 强制尾 `/` |
| 删除非空 TOCTOU | L3 R4 | 已解决 | 事务内 recount + `E_FOLDER_DIRTY` + 写通道锁 + 残留扫描 |
| 写并发（move 与 rename 同时） | L3 R5 | 已解决 | 写通道锁覆盖 5 命令 |
| trash 沙盒静默失败 | L3 R6 | 已解决 | `path.exists()` 复检 → `E_TRASH_FAILED` toast |
| Tauri v2 DnD 不稳 | L3 R7 | 已解决 | 仅用 HTML5 DnD + dragenter 计数器 |
| NFC/NFD 不对称 | L3 R8 | 已解决 | 启动期自愈扫描 + 读时 nfc 比较 |
| 深色模式 drop 不可见 | L3 R9 | 已解决 | drop 高亮用 `var(--accent-emphasis)` 进 MVP |
| ⌘⌫ 绕过 disabled | L3 R10 | 已解决 | handler 入口统一 `kind === 'root'` 判定 |
| 跨进程并发锁 | L4 | 已搁置 | 单进程多窗口足够，跨进程 P2 评估 |

### MVP 边界声明

**做什么**：
- F1-F4 四件核心动作齐全
- 工具栏 3 按钮 + 右键菜单 3 kind 形态 + ⌘⌫ + Enter（⌘⇧N 为 P1）
- 5 个新 IPC（4 写 + 1 read count）+ 结构化 IpcError JSON
- inline 编辑同步乐观 + selection 冻结 + 幽灵行失败保留 + 切走二次确认
- F4 双向 drop（含 `__ROOT__`）+ drop 高亮深色 token 可见
- R1-R10 全部缓解到位
- 写通道 `Mutex` 串行 + handler 入口权限判定
- canonicalize + ai_organized 双层拦 + 命名后端权威 + NFC 自愈
- Rust 单测 + 4 个集成测试 + 前端组件单测 + 手动 PR GIF

**不做什么**（附原因）：
- 嵌套子文件夹 — spec 明示根级
- 多选 / 多素材拖拽 — spec 明示二期
- Windows/Linux 删除实现 — spec 对齐 reveal 命令
- 列宽拖 / 列排序 / 列显隐 / 视图切换 / 面包屑 — spec 明示 P2
- 像素级 Finder 复刻 / 动效精修 — spec 明示
- 多语言文案 — 项目无 i18n 框架
- 撤销栈 — 范围外
- 跨进程并发锁 — 单进程多窗口足够
- 递归 count、`E_DEPTH_LIMIT`、`E_CYCLE` — Host 裁回（与 MVP 不嵌套冲突）

### Debate 中未达成共识的争议

**无未决争议**。L1-L4 全部收敛，所有 ❓ 状态在过渡前已被解决或明确搁置为 P2。

Architect 在拆 task 时无需新做选择；可直接以本 PRD 与 `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` 为依据。

---

> **PRD v1 完。** 下一步：Conductor 进入 `ARCHITECTURE` 状态，由 Architect 读取本 PRD + `debate_conclusions.md`，按 §7 task 拆分把 T0-T6 落到 `sessions/conductor/tasks/task_00N_*/input.md` 中，每条 input.md 必须满足 `handoff_contracts.md §2` 必填项。
