# 技术方案 — NCdesktop · 悬浮窗导入页面工作区文件夹管理

> Architect 产出物（2026-05-12 重置后版本）。基于 `product/prd/workspace_folder_mgmt_prd_v1.md` v1 + `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` + `session_context.md`。
> 真实仓库根：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop`（下称「NCdesktop/」）。

---

## 思考协议（Architect 内部推理）

### 1. 需求理解复述

把"项目 → 悬浮窗导入"页面右栏顶部的只读 chip 条，升级为可写的 Finder 列表风组件 `WorkspaceFolderListView`，闭合「F1 新建/F2 重命名/F3 删除→回收站/F4 拖拽移动单素材」四件人工整理动作。后端新增 4 写 + 1 read 共 5 个 Tauri 命令，统一走结构化 `IpcError` JSON 协议；前后端双层拦 `ai_organized`，rename/move 在同事务内完成「物理 rename + DB `file_path` 前缀替换」，删除走 `trash` crate（macOS）禁硬删，跨卷走 copy-first 两阶段；写操作以项目级 `Mutex<ProjectId>` 串行；启动期 NFC 自愈扫描归一 NFD 字节文件。MVP 不嵌套、不多选、Win/Linux 删除返未支持。

### 2. 硬约束清单（来自 session_context §3 + PRD 桥接摘要 10 条底线）

1. `ai_organized` 前后端各拦一次；handler 入口判定不依赖 UI disabled
2. 所有写命令 `validate_and_canonicalize()` 后 `starts_with(workspace_root_canonical)`
3. 删除走 `trash` crate + `path.exists()` 复检；禁 `fs::remove_dir_all`
4. rename/move 同 SQL 事务；`LIKE :p || '/%' ESCAPE '\'`、`:p` 强制带尾 `/`、预转义 `\ % _`
5. EXDEV copy-first 两阶段（copy→fsync→rename→COMMIT→remove src）
6. `__ROOT__` 仅 UI/IPC sentinel，永不入 DB；assets 写路径 `debug_assert!(!contains("__ROOT__"))`
7. 写通道 `Mutex<ProjectId>` 串行 5 命令 + import；read & 缩略图除外
8. 启动期 NFC 自愈扫描
9. 命名校验后端权威（禁 `/ \ :`、禁 `.` 开头、禁同级同名、禁 `organized`）
10. 错误统一 `IpcError JSON` 序列化；前端只按 code 出文案

### 3. 风险扫描（PRD 桥接摘要 R1-R10 + 新发现）

R1-R10 见风险登记表。新发现：
- **NEW-R12 cleanup_pending 时间窗**：EXDEV 失败时源副本短期残留 → 启动 hook 扫描 `*.cleanup_pending` / `*.cross_device.tmp` 兜底
- **NEW-R13 旧多素材 `move_asset_to_workspace_folder` 调用方**：既有 `commands/asset.rs` 中可能存在多素材签名 → T3 收敛为单素材并迁移到 `commands/workspace_folders.rs`，旧入口注销 invoke_handler
- **NEW-R14 NFC 自愈目标已存在**：同名 NFC 已存在时不可覆写 → 仅 log 跳过

### 4. 技术决策点清单 → ADR 序列

- IpcError JSON 序列化协议 → ADR-001
- EXDEV copy-first 两阶段 → ADR-002
- 写通道 `Mutex<ProjectId>` 边界 → ADR-003
- `__ROOT__` sentinel 编解码 → ADR-004
- NFC 自愈 hook 挂点 → ADR-005
- SQL 前缀 ESCAPE 策略 → ADR-006
- handler 入口统一权限判定 → ADR-007
- 命名校验后端权威 → ADR-008
- 列表数据流单向（不新增 store） → ADR-009
- count 数据走前端聚合 + 后端 invoke 双源 → ADR-010
- 拖拽栈：HTML5 DnD + 编辑互斥 → ADR-011
- `trash` crate（macOS）+ Win/Linux `E_PLATFORM_UNSUPPORTED` → ADR-012

---

## 项目概述

把「项目 → 悬浮窗导入」只读 chip `WorkspaceFolderStrip` 升级为可写 Finder 列表 `WorkspaceFolderListView`，落地 F1-F4 四件 P0；新增 4 写 + 1 read Tauri 命令，全部走结构化 `IpcError`；写操作覆盖路径越界拒绝、ai_organized 双层拦、写通道串行、同事务前缀替换、EXDEV copy-first、删除事务内 recount + trash + 残留扫描、NFC 启动自愈。

## 技术选型

| 维度 | 选择 | 理由 |
|---|---|---|
| 错误模型 | 结构化 `IpcError` JSON over Tauri error string | PRD §4.3；前端文案唯一来源 = `errorMessages[code]` |
| 跨卷迁移 | copy-first 两阶段 | R1 缓解；不丢数据 |
| 删除实现 | `trash` crate（macOS）+ `path.exists()` 复检 | 底线 3 |
| 并发控制 | 进程内 `Mutex<ProjectId>` | R5 缓解 |
| Unicode 归一 | `unicode-normalization` NFC + 启动自愈 | R8 缓解 |
| 拖拽栈 | HTML5 DnD + dragenter 计数器 | R7 缓解 |
| 命名校验 | 后端权威 + 前端即时反馈 | 底线 9 |
| `__ROOT__` | 仅 UI/IPC sentinel；DB 内空相对路径 | Debate §2 |

---

## Architecture Decision Records (ADR)

### ADR-001: IpcError JSON 序列化协议
- **状态**：已接受
- **上下文**：Tauri v2 invoke 边界仅支持 `Err(String)`；PRD §4.3 要求结构化 `{code, message, details}` 以驱动前端文案表。
- **决策**：后端 `Result<T, IpcError>`；`impl From<IpcError> for String` 通过 `serde_json::to_string` 转单行 JSON；前端 `parseIpcError` 在 catch 处 `JSON.parse` 还原；parse 失败降级 `E_INTERNAL`。`code` 是 11 项闭集（PRD §4.3）；`message` 仅日志；`details` 为各 code 专属 schema（contracts.md §A.4）。错误码 ↔ 中文文案映射集中在 `src/lib/ipc-errors.ts` 的 `errorMessages[code](details)`。
- **被排除项**：`Result<T, String>` 前缀字符串、Tauri Event 推错。
- **后果**：所有新命令必须实现 `From<E> → IpcError`；既有 `Result<T, String>` 命令本期不动；新增 `src-tauri/src/utils/ipc_error.rs` + `src/lib/ipc-errors.ts`。
- **落点**：`src-tauri/src/utils/ipc_error.rs`、`src/lib/ipc-errors.ts`、`src/types/workspace.ts`。

### ADR-002: EXDEV copy-first 两阶段
- **状态**：已接受
- **上下文**：跨卷 `fs::rename` 抛 EXDEV(errno 18)；直接 copy+delete 顺序错误会丢数据/孤儿。
- **决策**：`utils/safe_rename.rs::safe_rename(src,dst) -> RenameOutcome`：先试 `fs::rename`，捕 `raw_os_error() == Some(18)` 走两阶段：(1) `copy_dir_all(src→dst.cross_device.tmp)` (2) 递归 `fsync` (3) `rename(tmp→dst)` (4) caller 在 DB COMMIT 后调 `remove_src_after_commit(src)`。失败仅 `.cleanup_pending` 标记 + log，启动期 `cleanup_pending_scan` 兜底。
- **被排除项**：`copy → remove_src → DB update`（crash 在 remove 后/DB 前丢一致性）；直接抛 `E_CROSS_DEVICE`（F4 跨卷场景不可用）。
- **后果**：跨卷场景最坏=源盘短暂占用源副本到下次启动清理；绝不丢数据。
- **落点**：`src-tauri/src/utils/safe_rename.rs`、`src-tauri/src/startup.rs`（挂 `cleanup_pending_scan`）。

### ADR-003: 写通道 `Mutex<ProjectId>` 边界
- **状态**：已接受
- **上下文**：5 写命令 + 既有 `import_drop_paths` 并发改同一项目工作区会让 fs/DB race；同窗口 rename + move 会让前缀替换误伤。
- **决策**：新增 `Database` state 之外的 `WorkspaceWriteGuard`（双层 `Mutex<HashMap<ProjectId, Arc<Mutex<()>>>>`）；通过 `app.manage(WorkspaceWriteGuard::new())` 注入。覆盖命令集：`{create, rename, delete, move, import}`。每个写命令首行 `let lock = guard.lock_for(&project_id); let _g = lock.lock()`。**read 命令（`list_*`、`count_folder_assets`、`get_*`）与缩略图不取锁**。
- **被排除项**：全局 `Mutex<()>` 跨项目无必要串行；SQLite `BEGIN EXCLUSIVE` 只锁 DB 不锁 fs。
- **后果**：实测 latency 上限叠加锁等待；`import_drop_paths` 入口需追加 guard（最小侵入）。
- **落点**：`src-tauri/src/utils/write_guard.rs`、`src-tauri/src/lib.rs`、4 写命令首行。

### ADR-004: `__ROOT__` sentinel 编解码
- **状态**：已接受
- **上下文**：UI 把"项目根目录"作为可选行参与 selection/筛选/drop；但 DB 不能污染 sentinel（既有数据按"裸文件名 / 子目录/文件名"约定）。
- **决策**：
  - **入站**：前端 → 后端 invoke 入参可为 `"__ROOT__"`；后端单点 `workspace::resolve_relative_path(rel)` / `validate_and_canonicalize` 归一为空相对路径再拼 workspace root。
  - **DB**：`assets.file_path` 存 workspace 相对正斜杠路径；任何 INSERT/UPDATE 前 `debug_assert!(!path.contains("__ROOT__"))`。
  - **出站**：`list_project_workspace_folders` 首行仍返 `__ROOT__`；其余命令返 `WorkspaceFolderEntry` 时根目录场景仍用 `__ROOT__`。
- **被排除项**：`file_path` 加 `__ROOT__/` 前缀（破坏既有数据）；前端层翻译（违反单点原则）。
- **后果**：所有 fs 写路径必经 `resolve_relative_path` 后拼接；T4 集成测试 `test_round_trip_root_to_folder_to_root` 验证。
- **落点**：`src-tauri/src/workspace.rs::resolve_relative_path`、`validate_and_canonicalize`。

### ADR-005: NFC 自愈 hook 挂点
- **状态**：已接受
- **上下文**：macOS HFS+/APFS readdir 倾向 NFD；DB 既有数据按 NFC 存；不一致出现"列表行存在但 select 不到 asset"鬼影。
- **决策**：`src-tauri/src/startup.rs` 末尾、`PipelineScheduler::recover` 之前调 `nfc::nfc_heal_workspace()`：枚举 `~/Downloads/NoteCaptWorkPlace/*/` 下每个项目，递归 readdir；若 `nfc(B) != B` 且 NFC 目标不存在 → `fs::rename(B → nfc(B))` + `log::info`；若 NFC 目标已存在 → `log::warn` 跳过。同 hook 同时调 `safe_rename::cleanup_pending_scan` 清理 `*.cleanup_pending` / `*.cross_device.tmp` 残留。
- **被排除项**：实时 readdir 归一（IO 重）、INSERT 时归一（太晚）。
- **后果**：首次启动慢几百 ms；后续无感。NEW-R14 已记。
- **落点**：`src-tauri/src/utils/nfc.rs`、`src-tauri/src/startup.rs`。

### ADR-006: SQL 前缀 ESCAPE 策略
- **状态**：已接受
- **上下文**：rename 时 `参考` vs `参考资料` 前缀冲突；`%` `_` `\` 是 LIKE 元字符。
- **决策**：仅一处封装 `db::asset::rename_path_prefix(tx, old_prefix, new_prefix)`：
  ```sql
  UPDATE assets
  SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1)
  WHERE file_path = :old_prefix_no_slash
     OR file_path LIKE :old_prefix || '/%' ESCAPE '\';
  ```
  Rust 侧 `escape_like_prefix()`：对 `\ % _` 按顺序 backslash 转义；`:old_prefix` 强制带尾 `/`；同事务 `safe_rename(src,dst) → rename_path_prefix(tx,old,new) → tx.commit()`。
- **被排除项**：无 ESCAPE 的 `LIKE :p || '%'`（前缀冲突 + 元字符误伤）；后端正则替换（破坏原子事务）。
- **后果**：T3 单测「100% off vs 100」边界必须覆盖。
- **落点**：`src-tauri/src/db/asset.rs::rename_path_prefix`、`commands/workspace_folders.rs::rename_workspace_folder_impl`、`count_assets_under_prefix`。

### ADR-007: handler 入口统一权限判定
- **状态**：已接受
- **上下文**：UI disabled 仅视觉；`⌘⌫`、右键菜单、a11y 都可能绕；后端若仅信赖 UI 会被绕。
- **决策**：
  - **前端**：每个写动作 handler 首行 `if (selection.kind !== 'root') return;`，不依赖 disable 属性。`⌘⌫`、右键、工具栏共享同一 handler。
  - **后端**：每个写命令首行从 `relative_path` 推 `kind`（`organized/` 前缀 → ai_organized；`__ROOT__` → root_import），命中受保护类返 `E_PROTECTED_KIND`。即使直 invoke 也拒。
- **被排除项**：仅前端拦、仅后端拦。
- **后果**：双层在 4 写命令入口测试中各加一例直 invoke 验证。
- **落点**：4 写命令首行 + `WorkspaceFolderListView` 内 handler 入口。

### ADR-008: 命名校验后端权威
- **状态**：已接受
- **上下文**：JS 字符串校验可绕；保留字 `organized` 与 AI 归类目录冲突；macOS 对 `/ \ :` 有限制。
- **决策**：后端 `workspace::validate_folder_name(name)` 唯一权威：禁 `/ \ :`、禁 `.` 开头、禁空白、禁 > 255 字节、禁 `organized`、禁同级 NFC 同名（同级查重 `assert_no_sibling_nfc_dup` 由 T3 fs read 后执行）。命中各返 `E_NAME_INVALID` / `E_NAME_RESERVED` / `E_NAME_DUP`。前端 `src/lib/folder-name-validate.ts` 同步函数仅作即时红框反馈，后端仍最终权威。
- **被排除项**：仅前端 / 仅后端校验。
- **后果**：T3 Rust 单测覆盖 5 类非法；前端单测仅校验 UI 反馈。
- **落点**：`src-tauri/src/workspace.rs::validate_folder_name`、`src/lib/folder-name-validate.ts`。

### ADR-009: 列表数据流单向（不新增 store）
- **状态**：已接受
- **上下文**：PRD §5 明示不新增 store；既有 `uiStore` 已有 `workspaceFolderRelativePath`，本期再加 4 字段。
- **决策**：5 字段（`workspaceFolderRelativePath`、`editingFolderPath`、`pendingNewFolder`、`pendingRenameIds: Set<string>`、`dragOverPath`）全部并入 `uiStore`，**禁止**进入 `partialize` 持久化白名单（瞬态）。组件内部 `useState` 仅承载非全局 controlled value。folders 数据通过既有 `listProjectWorkspaceFolders` 在 `AssetListView` 顶部加载后下传 props。
- **被排除项**：新增 `useWorkspaceFolderStore`（违反 PRD §5）。
- **后果**：`pendingRenameIds` setter 必须返新 Set 实例触发 zustand 浅比较。
- **落点**：`src/stores/uiStore.ts`。

### ADR-010: count 数据走前端聚合 + 后端 invoke 双源
- **状态**：已接受
- **上下文**：列表「项目数」列每行实时数；本期非递归（根级无嵌套），可在前端从 `assetStore.assets` 聚合（O(N) 一次）；删除前 `expected_count` 必须取后端避免 UI 缓存陈旧导致 dirty 误报。
- **决策**：
  - 列表展示：前端 `firstSegment(a.file_path) === folder.relativePath` 聚合；`__ROOT__` = 不含 `/` 的文件计数。
  - 删除 confirm 文案 N 取前端聚合；invoke `delete_workspace_folder` 入参 `expected_count` 由前端聚合提供；后端事务内 recount 不一致返 `E_FOLDER_DIRTY{old, now}`，前端用 `now` 重弹 modal。
- **被排除项**：仅后端每行 N 次 invoke（太重）。
- **后果**：前端 `firstSegment` 与后端 `LIKE` 计数算法必须等价（`__ROOT__` = 不含 `/` direct 文件；其余 = `LIKE folder/% + 等值`）。
- **落点**：`WorkspaceFolderListView`、`count_folder_assets_impl`。

### ADR-011: 拖拽栈 HTML5 DnD + 编辑互斥
- **状态**：已接受
- **上下文**：Tauri v2 原生拖拽行级 hit-testing 不稳；HTML5 子元素冒泡抖动是常见坑。
- **决策**：
  - 仅用 HTML5 DnD（`onDragEnter / onDragOver / onDragLeave / onDrop`）；**不接** `tauri://drag-drop`。
  - `useRef<number>(0)` dragenter 计数器：enter ++、leave --、归零才清 `dragOverPath`。
  - drop 到 `kind === 'ai_organized'` → 前端 `preventDefault` 不 dispatch + toast；不发 IPC。
  - drop 到 `editingFolderPath === row.relativePath` → 禁止图标 + toast「目标正在编辑中」。
  - drop 高亮 = `boxShadow: inset 0 0 0 2px var(--accent-emphasis)`，深浅色均可见（R9）。
  - `__ROOT__` 双向合法（源/目标）。
- **被排除项**：Tauri 原生 DnD（R7）；整行反色。
- **后果**：F4 实测在两种主题下手动验证（PR 截图）；T6 集成测试覆盖编辑行禁止 + ai_organized 拦截。
- **落点**：`WorkspaceFolderListView` 行级 onDragEnter/Over/Leave/Drop。

### ADR-012: `trash` crate + 平台保护
- **状态**：已接受
- **上下文**：底线 3 严禁 `fs::remove_dir_all` 硬删；Win/Linux 不在 MVP 范围。
- **决策**：macOS `trash::delete(&abs_path)` + `abs_path.exists()` 复检（残留 → `E_TRASH_FAILED`）；Win/Linux 编译期 `cfg!(not(target_os = "macos"))` 返 `E_PLATFORM_UNSUPPORTED { feature: "trash", platform }`，与现有 `reveal_project_workspace_folder` 行为对齐。测试用 `TrashAdapter` trait 注入 stub 验证复检逻辑。
- **被排除项**：`osascript`（外部进程开销、签名差异）。
- **后果**：依赖 `trash = "5"`；T3 单测覆盖 stub 撒谎复检失败 → `E_TRASH_FAILED`。
- **落点**：`Cargo.toml`、`commands/workspace_folders.rs::delete_workspace_folder_impl`。

---

## 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│ Frontend (src/)                                                 │
│  AssetListView                                                  │
│    └─ WorkspaceFolderListView (新)                              │
│         ├─ FolderListRow（含 inline 输入、drop 高亮）            │
│         ├─ FolderListToolbar（+ 新建 / 重命名 / 移到废纸篓）      │
│         └─ FolderContextMenu（3 kind 形态）                      │
│  uiStore (+5 瞬态字段、不进 partialize)                          │
│  tauri-commands.ts (+5 camelCase wrapper)                       │
│  ipc-errors.ts (errorMessages 文案表 + invokeWithIpcError)       │
│  folder-name-validate.ts (前端即时校验)                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Tauri invoke (Result<T, IpcError-JSON>)
┌──────────────────────────┴──────────────────────────────────────┐
│ Backend (src-tauri/src/)                                        │
│  commands/workspace_folders.rs                                  │
│    ├─ create_workspace_folder       ┐                           │
│    ├─ rename_workspace_folder       │ guard + validate +        │
│    ├─ delete_workspace_folder       │ kind 拦 + IpcError         │
│    ├─ move_asset_to_workspace_folder│ + same-tx prefix          │
│    └─ count_folder_assets (read)    ┘                           │
│  workspace.rs (resolve_relative_path / validate_and_canon /     │
│                validate_folder_name)                            │
│  utils/ipc_error.rs (IpcError enum + Serialize + From<>)        │
│  utils/nfc.rs (nfc_normalize / nfc_eq / nfc_heal_workspace)      │
│  utils/safe_rename.rs (copy-first + cleanup_pending_scan)        │
│  utils/write_guard.rs (WorkspaceWriteGuard)                     │
│  db/asset.rs::rename_path_prefix                                │
│  startup.rs (挂 nfc_heal_workspace + cleanup_pending_scan)       │
│  lib.rs (invoke_handler! 注册 5 命令 + manage(WriteGuard))       │
└─────────────────────────────────────────────────────────────────┘
```

模块间依赖：
- 前端 ListView 仅依赖 tauri-commands + uiStore + ipc-errors；不直接接 invoke。
- 后端写命令依赖 `workspace::validate_and_canonicalize` + `write_guard` + `db::asset::rename_path_prefix`；缺一不可。

## 数据模型

不新增表。沿用 `assets.file_path`（workspace 相对正斜杠路径）。

新增内存结构：
```rust
pub enum IpcErrorCode { ENameInvalid, ENameDup, ENameReserved, EPathEscape,
    EProtectedKind, ENotFound, ECrossDevice, EPlatformUnsupported,
    ETrashFailed, EFolderDirty, EInternal } // 11 项闭集

pub struct IpcError { code, message, details: Option<serde_json::Value> }
pub struct DeleteReport { trashed: u32 }
pub enum RenameOutcome { SameVolume, CrossDevice { pending_remove_src: PathBuf } }
pub struct WorkspaceWriteGuard { locks: Mutex<HashMap<String, Arc<Mutex<()>>>> }
```

## API 设计

完全遵守 PRD §5.1（不改签名）：

```rust
#[tauri::command]
pub fn create_workspace_folder(project_id: String, name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;

#[tauri::command]
pub fn rename_workspace_folder(project_id: String, relative_path: String, new_name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;

#[tauri::command]
pub fn delete_workspace_folder(
    project_id: String, relative_path: String,
    confirm_non_empty: bool, expected_count: u32,
) -> Result<DeleteReport, IpcError>;

#[tauri::command]
pub fn move_asset_to_workspace_folder(asset_id: String, target_relative_path: String)
    -> Result<Asset, IpcError>;

#[tauri::command]
pub fn count_folder_assets(project_id: String, relative_path: String)
    -> Result<u32, IpcError>;
```

注：旧 `commands::asset::move_asset_to_workspace_folder`（多素材 `Vec<String>`）退役；本期收敛为单素材且改归属到 `commands/workspace_folders.rs`，从 `invoke_handler!` 注销旧入口（已在 `lib.rs` L94-95 注释中体现）。

前端封装：5 个 camelCase wrapper 集中在 `src/lib/tauri-commands.ts`；invoke 解包统一走 `src/lib/ipc-errors.ts::invokeWithIpcError<T>`。

## 目录结构

**新建**：
- `NCdesktop/src-tauri/src/utils/ipc_error.rs`
- `NCdesktop/src-tauri/src/utils/nfc.rs`
- `NCdesktop/src-tauri/src/utils/safe_rename.rs`
- `NCdesktop/src-tauri/src/utils/write_guard.rs`
- `NCdesktop/src-tauri/tests/workspace_folders_integration.rs`
- `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx`
- `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`
- `NCdesktop/src/lib/ipc-errors.ts`
- `NCdesktop/src/lib/folder-name-validate.ts`

**修改**：
- `NCdesktop/src-tauri/src/lib.rs`（`invoke_handler!` 注册 5 新命令 + `app.manage(WorkspaceWriteGuard::new())`；注销旧 `commands::asset::move_asset_to_workspace_folder`）
- `NCdesktop/src-tauri/src/workspace.rs`（追加 `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name`）
- `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（追加 4 写命令 + count）
- `NCdesktop/src-tauri/src/commands/asset.rs`（旧 move 退役）
- `NCdesktop/src-tauri/src/db/asset.rs`（追加 `rename_path_prefix` helper）
- `NCdesktop/src-tauri/src/startup.rs`（挂 `nfc_heal_workspace()` + `cleanup_pending_scan`）
- `NCdesktop/src-tauri/src/utils/mod.rs`（新增 4 mod 声明）
- `NCdesktop/src-tauri/Cargo.toml`（+ `trash = "5"`、`unicode-normalization = "0.1"`）
- `NCdesktop/src/lib/tauri-commands.ts`（5 新 wrapper）
- `NCdesktop/src/types/workspace.ts`（追加 `IpcError`、`IpcErrorCode`、`DeleteReport`）
- `NCdesktop/src/stores/uiStore.ts`（追加 5 字段 + setter；不进 partialize）
- `NCdesktop/src/components/features/AssetListView.tsx`（用 `WorkspaceFolderListView` 替换 `WorkspaceFolderStrip`）
- `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（本期删除文件）

## 安全考量

对应 PRD §4.1 / session_context §3 / 桥接摘要 10 条底线 的代码落点：

1. **路径越界**（底线 2 / §4.1.1）→ `workspace::validate_and_canonicalize`：拼接后 `canonicalize()` + `starts_with(workspace_root_canonical)`；symlink/`..`/绝对路径全拒。Rust 单测覆盖 3 例（`../../etc` / `/etc/passwd` / symlink → `E_PATH_ESCAPE`）。
2. **ai_organized 双层拦**（底线 1 / §4.1.2-3）→ 前端 handler 首行 `if (selection.kind !== 'root') return;` + 后端首行 `kind_from_relative_path()` 返 `E_PROTECTED_KIND`。direct invoke 也拒（T3 测试覆盖）。
3. **回收站非硬删**（底线 3）→ `trash::delete(&abs)` + `abs.exists()` 复检；存在 → `E_TRASH_FAILED`；Win/Linux `cfg!` 返 `E_PLATFORM_UNSUPPORTED`。
4. **rename/move 同事务**（底线 4 / §4.2.1）→ `safe_rename(src,dst)` → `tx.execute(rename_path_prefix sql)` → `tx.commit()`；ESCAPE 与转义见 ADR-006。
5. **EXDEV copy-first**（底线 5 / §4.2.2）→ `safe_rename` 内 EXDEV 分支两阶段；COMMIT 后 `remove_src_after_commit`，失败留 `.cleanup_pending` 标记 + 启动期清理。
6. **`__ROOT__` 永不入 DB**（底线 6）→ `resolve_relative_path` 在 fs 拼接处单点消费；`assets` 写路径加 `debug_assert!(!path.contains("__ROOT__"))`。
7. **写通道串行**（底线 7）→ `WorkspaceWriteGuard::lock_for(&project_id)` 覆盖 5 命令 + import；read 与缩略图不取锁。
8. **NFC 自愈**（底线 8）→ `startup.rs` 末尾调 `nfc::nfc_heal_workspace()`；首次启动一次性扫描归一。
9. **命名校验后端权威**（底线 9）→ `validate_folder_name`：禁 `/ \ :`、`.` 开头、空白、>255 字节、`organized`；同级 NFC 同名查重在 T3 fs read 后执行。
10. **错误 IpcError JSON**（底线 10）→ ADR-001。

## 风险登记表

| # | 风险 | 概率 | 影响 | 缓解 | 对应 |
|---|---|---|---|---|---|
| R1 | EXDEV 跨卷失败/状态残留 | 中 | 高 | copy-first 两阶段 + cleanup_pending 启动扫描 | ADR-002 |
| R2 | 路径越界（`..` / symlink） | 中 | 高 | `validate_and_canonicalize` + 3 例单测 | ADR-004/§4.1 |
| R3 | SQL `LIKE` 元字符误伤 | 中 | 中 | `ESCAPE '\'` + 预转义 + 强制尾 `/` | ADR-006 |
| R4 | 删除非空 TOCTOU | 中 | 高 | 事务内 recount + `E_FOLDER_DIRTY` + 写锁 + 残留扫描 | §4.2 |
| R5 | 写并发（move/rename 同时） | 中 | 高 | `Mutex<ProjectId>` 覆盖 5 命令 | ADR-003 |
| R6 | trash 沙盒静默失败 | 低 | 高 | `path.exists()` 复检 → `E_TRASH_FAILED` | ADR-012 |
| R7 | Tauri v2 DnD 不稳 | 高 | 中 | 仅 HTML5 DnD + dragenter 计数器 | ADR-011 |
| R8 | NFC/NFD 不对称 | 中 | 中 | 启动期自愈扫描 + 读时 NFC 比较 | ADR-005 |
| R9 | 深色模式 drop 不可见 | 中 | 低 | `var(--accent-emphasis)` 2px inset | ADR-011 |
| R10 | ⌘⌫ 绕过 disabled | 中 | 中 | handler 入口统一 `kind === 'root'` | ADR-007 |
| 跨进程并发锁 | 低 | 中 | MVP 单进程多窗口足够；P2 评估 | — |
| **NEW-R12** | cleanup_pending 残留时间窗 | 中 | 低 | 启动 hook 扫描 + log；非阻塞 | ADR-002 |
| **NEW-R13** | 旧多素材 move caller | 中 | 中 | 旧入口注销 invoke；调用方循环单素材 | T3 改造 |
| **NEW-R14** | NFC 自愈目标已存在 | 低 | 低 | 跳过 + log；不覆写 | ADR-005 |

---

## Task 清单（与 progress.md 一一对应，8 个）

- [ ] task_002_T0_contracts — 契约冻结：IpcError shape、5 命令签名、`__ROOT__` 编解码、错误码 ↔ 文案表
- [ ] task_003_T1_backend_utils — 后端工具层（依赖 T0）
- [ ] task_004_T2_frontend_ipc — 前端 IPC 封装（依赖 T0；与 T1 并行）
- [ ] task_005_T3_write_commands — 4 写命令 + Rust 单测（依赖 T1）
- [ ] task_006_T4_count_state_integration — count + uiStore 5 字段 + 2 集成测试（依赖 T1；与 T3 并行）
- [ ] task_007_T5a_list_skeleton — 列表骨架 + 工具栏 + 右键菜单 + 三 kind 灰显（依赖 T2）
- [ ] task_008_T5b_inline_edit — inline 编辑状态机 + 幽灵行 + selection 冻结 + 切走二次确认（依赖 T5a）
- [ ] task_009_T6_drag_drop — F4 拖拽 + 2 集成测试 + GIF（依赖 T3/T4/T5b）

## Task 依赖拓扑

```
T0 (contracts)
 ├─→ T1 (backend utils) ─┬─→ T3 (write commands) ────────────────┐
 │                       └─→ T4 (count + state + IT part 1) ────┤
 └─→ T2 (frontend ipc) ─→ T5a (list skeleton) ─→ T5b (inline) ──┤
                                                                  ↓
                                                T6 (drag drop + IT part 2 + GIF)
```

**可并行**：
- T1 ↔ T2（共依 T0，前后端独立）
- T3 ↔ T4（共依 T1，T3 是写命令，T4 是 read + 前端状态 + 集成测试）

**集成测试分配**（避免重复/遗漏）：
- **T4** 拥有 `test_rename_db_path_sync`、`test_round_trip_root_to_folder_to_root`（与 read + count 紧耦合）
- **T6** 拥有 `test_exdev_two_phase`、`test_delete_dirty_recount`（与 EXDEV + 删除完整链路紧耦合）

PRD §6.1 全部 Rust **单测** 归 T3。

---

## Task 粒度自检

| Task | 单一目标 | 可独立测试 | 规模适中 (<2000 LOC) | 依赖清晰 | AC 可验证 |
|------|---------|-----------|---------------------|---------|----------|
| T0 contracts | ✅ contracts.md + TS/Rust shape stub | ✅ 字段对照 PRD §4.3/§5.1 | ✅ <300 LOC | ✅ 无 | ✅ shape diff |
| T1 backend utils | ✅ 5 utils + 启动 hook + 依赖 | ✅ `cargo test utils::*` | ✅ ~600 LOC | ✅ 仅 T0 | ✅ cargo test |
| T2 frontend ipc | ✅ 5 wrapper + errorMessages + types | ✅ `pnpm test ipc-errors` | ✅ <400 LOC | ✅ 仅 T0 | ✅ tsc + vitest |
| T3 write commands | ✅ 4 命令 + 单测全套 | ✅ `cargo test workspace_folders::*` | ✅ ~1200 LOC | ✅ T1 | ✅ PRD §6.1 |
| T4 count+state+IT | ✅ count + uiStore + 2 IT | ✅ `cargo test --test workspace_folders_integration test_rename_db_path_sync` | ✅ ~600 LOC | ✅ T1 | ✅ 两 IT 名 |
| T5a list skeleton | ✅ 骨架 + 工具栏 + 右键 + 键盘 handler | ✅ `pnpm test WorkspaceFolderListView` | ✅ ~800 LOC | ✅ T2 | ✅ 渲染 + 选中 + 工具栏激活 |
| T5b inline edit | ✅ 三态 + 幽灵 + 冻结 + 切走 modal | ✅ `pnpm test` inline 用例 | ✅ ~600 LOC | ✅ T5a | ✅ Enter/Esc/blur 4 用例 |
| T6 drag drop + IT | ✅ HTML5 DnD + 2 IT + GIF | ✅ `cargo test` + `pnpm test` drop + 手动 | ✅ ~900 LOC | ✅ T3/T4/T5b | ✅ `test_exdev_two_phase` + `test_delete_dirty_recount` + drop 单测 |

**全部 ✅**，无需进一步拆分。规模上 T3 接近上限；若 Dev 实现触及 1500+ LOC 可考虑把 move 命令 + 单测独立成 T3b，Conductor 阶段保留此选项。

---

## Architect 识别的新增风险（PRD 未覆盖）

1. **NEW-R12 cleanup_pending 时间窗** — 已落 `startup::cleanup_pending_scan`。
2. **NEW-R13 旧多素材 move caller 迁移** — 既有 `commands/asset.rs` 中多素材签名需在 T3 收敛；调用方（AssetListView 拖拽、AssetContextMenu）改为循环调用单素材新命令。
3. **NEW-R14 NFC 自愈目标已存在** — 跳过 + log 警告；不主动告警，复用 `E_NOT_FOUND` 在 reveal/delete/count 时反映。

> 本方案与 PRD §7 task 拆分 1:1 对齐；与 progress.md 当前 task 队列完全一一对应；未对 task 顺序/依赖做任何调整。
