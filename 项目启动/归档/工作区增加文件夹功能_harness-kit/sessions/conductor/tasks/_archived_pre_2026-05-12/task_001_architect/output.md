# 技术方案 — NCdesktop · 悬浮窗导入页面工作区文件夹管理

> Architect 产出物。基于 `product/prd/workspace_folder_mgmt_prd_v1.md` v1 + `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` + `session_context.md`。
> 真实仓库根：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop`（下称「NCdesktop/」）。

---

## 项目概述

将「项目 → 悬浮窗导入」页面的只读 chip 条 `WorkspaceFolderStrip` 升级为可写的 Finder 列表风组件 `WorkspaceFolderListView`，闭合 F1 新建 / F2 重命名 / F3 删除（移到回收站）/ F4 拖拽移动素材四件 P0。后端新增 4 写 + 1 read 共 5 个 Tauri 命令，全部走结构化 `IpcError` JSON 协议；写操作覆盖路径越界拒绝、`ai_organized` 双层拦截、写通道 `Mutex<ProjectId>` 串行、同事务前缀替换、EXDEV copy-first 两阶段、删除事务内 recount + trash crate + 残留扫描、NFC 启动自愈。

## 技术选型

| 维度 | 选择 | 理由 |
|---|---|---|
| 错误模型 | 结构化 `IpcError` JSON over Tauri error string | PRD §4.3 + Debate §3；前端文案唯一来源 = `errorMessages[code]` |
| 跨卷迁移 | copy-first 两阶段（copy→fsync→rename→COMMIT→remove src） | R1 缓解；不丢数据，最坏多占用 |
| 删除实现 | `trash` crate（macOS）+ `path.exists()` 复检 | session_context 底线 3；禁 `fs::remove_dir_all` |
| 并发控制 | 进程内 `Mutex<ProjectId>` 写通道 | R5 缓解；read & 缩略图除外 |
| Unicode 归一 | `unicode-normalization` NFC + 启动自愈扫描 | R8 缓解；DB 永远存 NFC |
| 拖拽栈 | HTML5 DnD + dragenter 计数器 | R7 缓解；不接 `tauri://drag-drop` |
| 命名校验 | 后端权威 + 前端即时反馈 | session_context 底线 5 |
| `__ROOT__` | 仅 UI/IPC sentinel；DB 内空相对路径 | Debate §2 |

---

## Architecture Decision Records (ADR)

### ADR-001：IpcError JSON 序列化协议
- **状态**：已接受
- **上下文**：Tauri v2 `invoke` 边界仅支持 `Err(String)`；但 PRD 要求结构化 `{code, message, details}` 以驱动前端文案表与受控 UI 行为。
- **决策**：后端命令统一签名 `Result<T, IpcError>`；通过自定义 `Into<String>` / `serde::Serialize` 将 `IpcError` 实例 `serde_json::to_string(&err)` 转为单行 JSON string 抛出；前端 invoke wrapper 在 catch 处 `JSON.parse(str)` 还原。`code` 是闭集枚举（11 项，PRD §4.3）；`message` 仅日志/上报；`details` 为 `Record<string, unknown>`（如 `{old, now}`、`{name}`）。所有错误码 ↔ 文案映射集中在 `src/lib/ipc-errors.ts` 的 `errorMessages[code](details)`。
- **被排除项**：
  - 复用既有 `Result<T, String>` 自定义前缀（如 `"E_PROTECTED_KIND: 受保护"`）：脆弱、前端易写成正则匹配，违反 Debate §3「结构化」共识。
  - 直接走 Tauri Event 推错：破坏 invoke 同步语义，无法返回成功值。
- **后果**：所有新写命令必须实现 `From<E>` → `IpcError` 的统一映射；既有 `Result<T, String>` 命令本期不动；前端 `invoke` wrapper 需加 `try/catch + JSON.parse + fallback E_INTERNAL`（JSON.parse 失败时降级）。

### ADR-002：EXDEV copy-first 两阶段
- **状态**：已接受
- **上下文**：跨卷（如外接盘工作区）`fs::rename` 抛 EXDEV；直接报错会让 F2 rename、F4 move 在跨卷场景下完全不可用；若简单 fallback 为 copy+delete 顺序错误会产生孤儿/数据丢失。
- **决策**：写命令实现 `safe_rename(src, dst)` helper：先尝 `fs::rename`，捕 `ErrorKind::CrossesDevices`（或 raw OS errno EXDEV / EXDEV 在不同平台数值）后走两阶段：
  1. `copy_dir(src → dst.tmp)` 递归复制（保留 mtime）
  2. 对每个新文件 `File::open` + `sync_all()` (fsync)
  3. `fs::rename(dst.tmp → dst)`（同卷必成）
  4. `BEGIN; UPDATE assets SET file_path = …; COMMIT;`（同事务前缀替换）
  5. COMMIT 成功后才 `fs::remove_dir_all(src)`；若 remove 失败仅 `log::warn!("cleanup_pending src={...}")`，不回滚事务。
- **被排除项**：
  - `copy → remove_src → DB update`（即旧顺序）：crash 在 remove 之后/DB 之前会丢失数据库一致性。
  - 直接对外暴露 `E_CROSS_DEVICE`：F4 用户体感坏；保留 `E_CROSS_DEVICE` 仅给"copy 阶段失败"用。
- **后果**：rename / move 在跨卷场景下最坏=源盘短暂占用源副本（直到下一次启动期清理）；绝不丢数据/产孤儿。`cleanup_pending` 需要在启动 hook 中扫描并清理（与 NFC 自愈共用一次启动扫描）。

### ADR-003：写通道 `Mutex<ProjectId>` 边界
- **状态**：已接受
- **上下文**：5 个写命令 + 既有 `import_drop_paths` 可能并发改动同一项目工作区目录；同时段 rename + move 会让 DB 前缀替换 race；跨进程虽不在 MVP 范围（P2），但单进程多窗口必须串行。
- **决策**：在 `Database` state 之外新增 `WorkspaceWriteGuard`：`Arc<Mutex<HashMap<ProjectId, Arc<Mutex<()>>>>>` 双层 map，每个项目独立锁。覆盖命令集：`{create_workspace_folder, rename_workspace_folder, delete_workspace_folder, move_asset_to_workspace_folder, import_drop_paths}`。每个写命令首行 `let _guard = workspace_write_guard.lock_for(&project_id)?;`，函数返回时自动释放。**read 命令（`list_project_workspace_folders` / `count_folder_assets` / `get_project_workspace_root`）与缩略图生成不取锁**。
- **被排除项**：
  - 全局 `Mutex<()>`：跨项目串行无必要，体验差。
  - SQLite `BEGIN EXCLUSIVE`：只能锁 DB，不能锁 fs 操作；fs/DB 之间仍有 TOCTOU。
- **后果**：所有受控写命令的实测 latency 上限叠加锁等待；写通道串行保证 R4 删除 recount + R5 写并发安全。`import_drop_paths` 入口需要追加 guard（最小侵入：在既有命令首部加一行）。

### ADR-004：`__ROOT__` sentinel 编解码
- **状态**：已接受
- **上下文**：UI 需要把"项目根目录"作为可选行参与选中/筛选/drop；但 DB `assets.file_path` 不能污染 sentinel（已有大量 prod 数据按"裸文件名 / 子目录/文件名"约定）。
- **决策**：
  - **入站**（前端 → 后端 invoke 入参）：相对路径字符串可以是 `"__ROOT__"` 或子目录相对路径；后端单点 `fn resolve_relative_path(rel: &str) -> &str { if rel == "__ROOT__" { "" } else { rel } }` 归一为内部空字符串。
  - **DB**：`assets.file_path` 存 workspace 根的正斜杠相对路径（根文件 = 裸文件名；子目录文件 = `folder/file.png`）；任何 INSERT/UPDATE 前 `debug_assert!(!path.contains("__ROOT__"))`。
  - **出站**（后端 → 前端列表）：`list_project_workspace_folders` 仍返回 `__ROOT__` 作为第一行 `relative_path`（保留既有契约）；其余命令返回 `WorkspaceFolderEntry` 时根目录场景仍用 `__ROOT__`。
- **被排除项**：
  - 让 `file_path` 加 `__ROOT__/` 前缀：破坏既有数据；prune/migration 成本高。
  - 在前端层做 sentinel 翻译：每个 invoke 点都要写，违反单点原则。
- **后果**：所有 fs 写路径必须经 `resolve_relative_path` 后再拼接；`debug_assert!` 在 debug 构建命中即为契约违反必须修；测试用 `test_round_trip_root_to_folder_to_root` 验证。

### ADR-005：NFC 自愈 hook 挂点
- **状态**：已接受
- **上下文**：macOS HFS+/APFS 倾向 NFD（分解形式），DB 既有数据按 NFC 存；若 readdir 返回 NFD 字节与 DB NFC 字节不等，会出现"列表行存在但 select 不到 asset"鬼影。
- **决策**：在 `startup::bootstrap` 末尾、`PipelineScheduler::recover` 之前新增一次 `nfc_heal_workspace()` 串行扫描：
  1. 枚举 `~/Downloads/NoteCaptWorkPlace/*/` 下所有项目工作区
  2. 对每个目录 readdir 拿到字节 B
  3. 若 `nfc(B) != B` 且 `nfc(B)` 目标路径不存在，`fs::rename(B → nfc(B))` + `log::info!("nfc_healed: B={} N={}")`
  4. 若 `nfc(B)` 目标已存在（极少），仅 `log::warn!`，不覆写
  - 启动期同一 hook 同时扫描 `*.tmp`（来自 EXDEV 中断的残留）和孤立的源目录（cleanup_pending），按窗口期 > 24h 静默清理。
- **被排除项**：
  - 实时在 readdir 处归一：每次列表都做归一 + rename，IO 成本高，且并发时危险。
  - 在 INSERT 时归一：太晚，rename/delete/move 已经看到错位。
- **后果**：首次启动可能慢几百 ms；后续无感。本期不做异步增量自愈（P2）。

### ADR-006：SQL 前缀 ESCAPE 策略
- **状态**：已接受
- **上下文**：rename 时需要把 `参考/a.png` 改成 `参考资料/a.png`；如果用 `LIKE :p || '%'`，名为 `参考` 的目录会误伤同级 `参考资料/` 下所有 asset（前缀冲突），且 `%` `_` `\` 字符会被 SQL 当通配元字符。
- **决策**：
  - SQL 模板（仅此一处，封装在 `db::asset::rename_path_prefix` helper）：
    ```sql
    UPDATE assets
    SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1)
    WHERE file_path = :old_prefix_no_slash
       OR file_path LIKE :old_prefix || '/%' ESCAPE '\';
    ```
  - 注：`:old_prefix_no_slash` = 不带尾 `/` 的旧路径（用于命中"被改名的目录自身在 DB 中无文件但有 asset 直接命名为它"的边界，**实际 MVP 下不可能命中**因为根级 user folder 内才有 asset；这一支留作防御）；通常 `file_path` 是 `folder/file.png` 格式，由 LIKE 子树支命中。
  - 入参前置处理（Rust 侧 `escape_like_prefix()`）：对 `\ % _` 按顺序 backslash 转义，再 `format!("{}/", escaped)` 强制尾 `/`，再注入 `:old_prefix`；`:new_prefix` 同样强制带尾 `/` 并转义同样字符（用于 `||` 拼接的字符串字面值无需转义，但保持习惯一致）。
  - 同事务：物理 `safe_rename(src, dst)` → `rename_path_prefix(tx, old, new)` → tx.commit()。
- **被排除项**：
  - `WHERE file_path LIKE :p || '%'`（无 ESCAPE、无 `/`）：前缀冲突 + 元字符误伤。
  - 后端正则替换 = 不能用 SQL 原子事务。
- **后果**：rename 单 SQL 完成所有 asset 路径前缀替换；Rust 单测 §6.1 "100% off vs 100" 边界覆盖。

### ADR-007：handler 入口统一权限判定
- **状态**：已接受
- **上下文**：UI disabled 仅是视觉；`⌘⌫` 键盘事件、右键菜单点击、未来的 a11y 入口都可能绕过；后端若仅信赖 UI 会被绕。
- **决策**：
  - **前端**：每个写动作 handler 首行
    ```ts
    if (selection.kind !== 'root') return;
    if (mode === 'creating' || mode === 'renaming') {
        // 编辑态下另行处理（如 drop 禁止 + toast）
    }
    ```
    不依赖 disable 属性。
  - **后端**：每个写命令首行从 DB / 路径反推 `kind`，命中 `ai_organized` 或 sentinel 路径返 `Err(E_PROTECTED_KIND { kind })`。即使前端绕过、direct invoke 也拒。
- **被排除项**：
  - 仅前端拦：被 a11y / 自动化测试绕过即可写穿。
  - 仅后端拦：UI 给出晚反馈、错误码弹 toast 体验差。
- **后果**：双层拦截需在 4 个写命令的入口测试中各加一例直 invoke 测试。

### ADR-008：命名校验后端权威
- **状态**：已接受
- **上下文**：前端 JS 字符串校验可被绕过；保留字 `organized` 与 ai 归类目录冲突；macOS 文件系统对 `/ \ :` 字符有限制。
- **决策**：
  - 后端 `validate_folder_name(name: &str)` 唯一权威：拒 `/ \ :`、拒 `.` 开头、拒空白、拒长度 > 255 字节（UTF-8）、拒保留字 `organized`、拒同级同名（NFC 归一后比较）。命中各自返 `E_NAME_INVALID` / `E_NAME_RESERVED` / `E_NAME_DUP`。
  - 前端 `src/lib/folder-name-validate.ts` 提供同步函数，仅用于即时红框 + Enter 失败时本地禁止提交；后端仍是最终权威。
- **被排除项**：仅前端校验 / 仅后端校验。
- **后果**：前端单测仅校验 UI 反馈；Rust 单测覆盖 5 个非法分类的 unit test。

### ADR-009：列表数据流单向（avoid store 膨胀）
- **状态**：已接受
- **上下文**：PRD §4 不允许新增 store；既有 `uiStore` 已有 `workspaceFolderRelativePath`，本期再加 4 字段（编辑态、幽灵行标志、selection 冻结、drop 候选）。
- **决策**：5 字段全部并入 `uiStore`（PRD §5.2）。组件 `WorkspaceFolderListView` 内部 `useState` 仅承载非全局的"输入框 controlled value"。folders 数据通过既有 `listProjectWorkspaceFolders` 在 `AssetListView` 顶部加载后下传 props。
- **被排除项**：新增 `useWorkspaceFolderStore`（违反 PRD §5）。
- **后果**：`uiStore.ts` 的接口 surface 增长可控；`pendingRenameIds` 使用 `Set<string>` 时 zustand setter 需返回新 Set 实例。

### ADR-010：count 数据走前端聚合 + 后端 invoke 双源
- **状态**：已接受
- **上下文**：列表第二列「项目数」需要每行实时数。本期非递归（根级文件夹无嵌套），可在前端从 `assetStore.assets` 按 `file_path` 起始段聚合（O(N) 一次）；后端 `count_folder_assets` 用于删除前的 `expected_count` 取数（避免 UI 缓存陈旧导致 dirty 误报）。
- **决策**：
  - 列表展示用前端聚合：`assets.filter(a => firstSegment(a.file_path) === folder.relativePath).length`；`__ROOT__` 行 = 无 `/` 文件计数。
  - 删除 confirm 文案 N 取前端聚合即可；invoke `delete_workspace_folder` 入参 `expected_count` 由前端聚合得到；后端事务内 recount 不一致返 `E_FOLDER_DIRTY{old, now}`，前端用 `now` 重弹 modal。
- **被排除项**：仅后端 `count_folder_assets`（每行 N 次 invoke 太重）。
- **后果**：前端 `firstSegment()` 与后端 `LIKE` 计数必须算法一致（`__ROOT__` = 不含 `/`；其余 = `LIKE 'folder/%'` + 等值匹配）。

### ADR-011：拖拽事件栈与编辑互斥
- **状态**：已接受
- **上下文**：Tauri v2 原生拖拽（`tauri://drag-drop`）当前在工作区行级 hit-testing 不稳；HTML5 DnD + dragenter 子元素抖动是常见坑。
- **决策**：
  - drop target 用 HTML5 DnD（`onDragEnter / onDragOver / onDragLeave / onDrop`）。
  - 用 `useRef<number>(0)` dragenter 计数器避免子元素冒泡抖动：enter ++、leave --，归零才清除 `dragOverPath`。
  - drop 到 `kind === 'ai_organized'`：前端 `preventDefault` 不 dispatch + toast「AI 归类目录受保护」；不发 IPC。
  - drop 到 `editingFolderPath === row.relativePath`：禁止图标 + toast「目标正在编辑中」。
  - drop 高亮 = `boxShadow: inset 0 0 0 2px var(--accent-emphasis)`，深浅色均可见（R9）。
- **被排除项**：Tauri 原生 DnD（R7）；整行反色（破坏列表节奏）。
- **后果**：F4 实测需在两种主题下手动验证（PR 截图）。

---

## 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│ Frontend (src/)                                                 │
│  AssetListView                                                  │
│    └─ WorkspaceFolderListView (新)                              │
│         ├─ FolderListRow (内部，含 inline 编辑 input)            │
│         ├─ FolderListToolbar (3 按钮)                            │
│         └─ FolderContextMenu (3 kind 形态)                       │
│  uiStore (+5 字段)                                               │
│  tauri-commands.ts (+5 camelCase wrapper)                       │
│  ipc-errors.ts (新) — errorMessages 文案表 + invoke 解包         │
│  folder-name-validate.ts (新) — 前端即时校验                     │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Tauri invoke (Result<T, IpcError-JSON>)
┌──────────────────────────┴──────────────────────────────────────┐
│ Backend (src-tauri/src/)                                        │
│  commands/workspace_folders.rs                                  │
│    ├─ create_workspace_folder      ┐                            │
│    ├─ rename_workspace_folder      │ 加 write_guard +           │
│    ├─ delete_workspace_folder      │ validate +                 │
│    ├─ count_folder_assets          ┘ ipc_error                  │
│  commands/asset.rs                                              │
│    └─ move_asset_to_workspace_folder (重构 → IpcError + 单素材) │
│  workspace.rs (+resolve_relative_path / validate_and_canon)     │
│  utils/ipc_error.rs (新) — IpcError enum + Serialize            │
│  utils/nfc.rs (新) — nfc_normalize / nfc_heal_workspace         │
│  utils/safe_rename.rs (新) — copy-first 两阶段 + EXDEV 探测      │
│  utils/write_guard.rs (新) — WorkspaceWriteGuard                │
│  startup.rs (挂 nfc_heal_workspace + cleanup_pending 扫描)      │
│  lib.rs (注册 5 新命令 + manage(WorkspaceWriteGuard))            │
└─────────────────────────────────────────────────────────────────┘
```

模块间依赖：
- 前端 listview 仅依赖 tauri-commands + uiStore + ipc-errors；不直接接 invoke。
- 后端写命令依赖 `workspace::assert_scope` + `validate_and_canonicalize` + `write_guard` + `db::asset::rename_path_prefix`；任何一处缺失均不可写入。

## 数据模型

**不新增表**。沿用现有 `assets.file_path`（workspace 相对路径，正斜杠）。

新增内存结构：
```rust
pub struct IpcError {
    pub code: IpcErrorCode,  // 11 项闭集枚举
    pub message: String,     // 仅日志
    pub details: Option<serde_json::Value>,
}

pub struct DeleteReport { pub trashed: u32 }

// move 单素材，返回 Asset（与 PRD §5.1 对齐，对既有签名做收敛）
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

注：现有 `commands::asset::move_asset_to_workspace_folder` 签名为 `(asset_ids: Vec<String>, target_relative_path, project_id) -> Result<(), String>`，**本期需收敛为单素材 + `IpcError`**，并改归属到 `commands/workspace_folders.rs`（与 PRD §5.1 一致），旧入口标 `#[deprecated]` 或直接替换。Conductor 已在 progress.md 标 task 串行执行，可安全替换。

前端封装：5 个 camelCase wrapper 集中在 `src/lib/tauri-commands.ts`；invoke 解包统一走 `src/lib/ipc-errors.ts` 的 `invokeWithIpcError<T>(cmd, args)`。

## 目录结构

新建：
- `NCdesktop/src-tauri/src/utils/ipc_error.rs`
- `NCdesktop/src-tauri/src/utils/nfc.rs`
- `NCdesktop/src-tauri/src/utils/safe_rename.rs`
- `NCdesktop/src-tauri/src/utils/write_guard.rs`
- `NCdesktop/src-tauri/tests/workspace_folder_integration.rs`（集成测试）
- `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`
- `NCdesktop/src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx`
- `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`
- `NCdesktop/src/lib/ipc-errors.ts`
- `NCdesktop/src/lib/folder-name-validate.ts`

修改：
- `NCdesktop/src-tauri/src/lib.rs`（invoke_handler! 注册 5 新命令；`app.manage(WorkspaceWriteGuard::new())`）
- `NCdesktop/src-tauri/src/workspace.rs`（追加 `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name`）
- `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（追加 4 写命令 + count）
- `NCdesktop/src-tauri/src/commands/asset.rs`（旧 `move_asset_to_workspace_folder` 退役 / 重构）
- `NCdesktop/src-tauri/src/startup.rs`（挂 `nfc_heal_workspace()` 与 `cleanup_pending_scan()`）
- `NCdesktop/src-tauri/src/utils/mod.rs`（新增 4 mod 声明）
- `NCdesktop/src-tauri/Cargo.toml`（+ `trash = "5"`、`unicode-normalization = "0.1"`）
- `NCdesktop/src/lib/tauri-commands.ts`（5 新 wrapper）
- `NCdesktop/src/types/workspace.ts`（追加 `DeleteReport`、`IpcError`、`IpcErrorCode`）
- `NCdesktop/src/stores/uiStore.ts`（追加 5 字段 + setter；不进 partialize 白名单）
- `NCdesktop/src/components/features/AssetListView.tsx`（用 `WorkspaceFolderListView` 替换 `WorkspaceFolderStrip`，删 import）
- `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（本期删除文件）

## 安全考量

1. **路径越界**：所有写命令 first call `workspace::validate_and_canonicalize(project_id, rel)` —— 拼接后 `canonicalize()` + `starts_with(workspace_root_canonical)`；symlink 越界、`..`、绝对路径全拒。Rust 单测覆盖 3 例。
2. **`ai_organized` 双层拦截**：handler 入口判断 kind（前端）+ 后端首行从 `relative_path` 推 `kind`（前缀 `organized/` 视为 ai_organized）返 `E_PROTECTED_KIND`。
3. **回收站非硬删**：`trash::delete(&abs_path)` 后 `abs_path.exists()` 复检；存在 → `E_TRASH_FAILED`。Win/Linux 编译期 `cfg!` 返 `E_PLATFORM_UNSUPPORTED`。
4. **命名校验后端权威**：`validate_folder_name` 见 ADR-008。
5. **写通道串行**：`Mutex<ProjectId>` 防 R5。
6. **debug_assert! 防 `__ROOT__` 写穿**：`db::asset` 所有 INSERT/UPDATE 入口加 `debug_assert!(!path.contains("__ROOT__"))`。

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 | 对应 PRD |
|---|---|---|---|---|
| R1 EXDEV 跨卷 rename 失败 / 部分状态残留 | 中 | 高 | copy-first 两阶段 + cleanup_pending 启动扫描（ADR-002） | 桥接摘要 |
| R2 路径越界（`..` / symlink） | 中 | 高 | `validate_and_canonicalize` + 3 例单测 | §4.1 |
| R3 SQL `LIKE` 元字符 `%_\` 误伤 | 中 | 中 | `ESCAPE '\'` + 预转义 + 强制尾 `/`（ADR-006） | §4.2 |
| R4 删除非空 TOCTOU | 中 | 高 | 事务内 recount + `E_FOLDER_DIRTY` + 写锁 + 残留扫描 | §4.2 |
| R5 写并发（move/rename 同时） | 中 | 高 | `Mutex<ProjectId>` 写通道（ADR-003） | 底线 7 |
| R6 trash crate 沙盒静默失败 | 低 | 高 | `path.exists()` 复检 → `E_TRASH_FAILED` | 底线 3 |
| R7 Tauri v2 DnD 不稳 | 高 | 中 | 仅用 HTML5 DnD + dragenter 计数器（ADR-011） | 桥接摘要 |
| R8 NFC/NFD 不对称 | 中 | 中 | 启动期自愈扫描（ADR-005）；读时 NFC 比较 | 底线 8 |
| R9 深色模式 drop 不可见 | 中 | 低 | `var(--accent-emphasis)` 2px inset shadow | §4.5 |
| R10 ⌘⌫ 绕过 disabled | 中 | 中 | handler 入口统一 `kind === 'root'`（ADR-007） | 底线 1/3 |
| 跨进程并发锁 | 低 | 中 | MVP 单进程多窗口足够；P2 评估 | Debate §7 |
| **NEW-R12 cleanup_pending 残留时间窗** | 中 | 低 | 启动 hook 扫描 + 24h 静默清理；非阻塞 | ADR-002 |
| **NEW-R13 旧 `move_asset_to_workspace_folder` 多素材 caller** | 中 | 中 | 灰度搜索调用方（仅 AssetListView 拖拽与 AssetContextMenu）；改造为循环调用单素材新命令 | ADR-005/API |
| **NEW-R14 NFC 自愈 rename 失败** | 低 | 低 | 失败仅 log + 跳过；不阻塞启动 | ADR-005 |

## Task 清单

T0-T6 共 8 task（T5 拆 a/b）。

- [ ] task_002_T0_contracts — 契约冻结：IpcError shape、5 命令签名、`__ROOT__` 编解码、错误码 ↔ 文案表
- [ ] task_003_T1_backend_utils — 后端工具层（依赖 T0）
- [ ] task_004_T2_frontend_ipc — 前端 IPC 封装（依赖 T0）
- [ ] task_005_T3_write_commands — 4 写命令 + Rust 单测（依赖 T1）
- [ ] task_006_T4_count_state_integration — count + uiStore 字段 + 集成测试 2 个（依赖 T1）
- [ ] task_007_T5a_list_skeleton — 列表骨架 + 工具栏 + 右键菜单（依赖 T2）
- [ ] task_008_T5b_inline_edit — inline 编辑状态机 + 幽灵行 + selection 冻结（依赖 T5a）
- [ ] task_009_T6_drag_drop — F4 拖拽 + 集成测试 2 个 + GIF（依赖 T3/T4/T5b）

## Task 依赖拓扑

```
T0 (contracts)
 ├─→ T1 (backend utils) ─→ T3 (write commands) ─┐
 │                          └─→ T4 (count + state + 集成测试 part 1) ─┐
 └─→ T2 (frontend ipc) ─→ T5a (list skeleton) ─→ T5b (inline edit) ──┤
                                                                      ↓
                                                          T6 (drag drop + 集成测试 part 2 + GIF)
```

集成测试分配（避免重复/遗漏）：
- **T4** 拥有 `test_rename_db_path_sync`、`test_round_trip_root_to_folder_to_root`（与 read + count 紧耦合）
- **T6** 拥有 `test_exdev_two_phase`、`test_delete_dirty_recount`（与 EXDEV/删除 + 集成完整链路紧耦合）

PRD §6.1 全部 Rust **单测** 归 T3。

---

## Task 粒度自检

| Task | 单一目标 | 可独立测试 | 规模适中 (<2000 LOC) | 依赖清晰 | AC 可验证 |
|------|---------|-----------|---------------------|---------|----------|
| T0 contracts | ✅ 仅产 contracts.md + types stub | ✅ 文档对照 PRD §4.3/§5.1 | ✅ <300 LOC | ✅ 无依赖 | ✅ 字段对齐检查表 |
| T1 backend utils | ✅ 5 个 utils + 启动 hook + 依赖 | ✅ `cargo test utils::*` | ✅ ~600 LOC | ✅ 仅依 T0 | ✅ cargo test 命令 |
| T2 frontend ipc | ✅ 5 wrapper + errorMessages + types | ✅ `pnpm test ipc-errors` 单测 | ✅ <400 LOC | ✅ 仅依 T0 | ✅ 类型编译 + 单测 |
| T3 write commands | ✅ 4 写命令实现 + 全套单测 | ✅ `cargo test workspace_folders::*` | ✅ ~1200 LOC | ✅ 依 T1 | ✅ PRD §6.1 全套 |
| T4 count+state+IT | ✅ count + uiStore 5 字段 + 2 个 IT | ✅ `cargo test --test workspace_folder_integration test_rename_db_path_sync` | ✅ ~600 LOC | ✅ 依 T1 | ✅ 两个 IT 名 + uiStore 单测 |
| T5a list skeleton | ✅ 列表骨架 + 工具栏 + 右键菜单 + 键盘 handler | ✅ `pnpm test WorkspaceFolderListView` | ✅ ~800 LOC | ✅ 依 T2 | ✅ 渲染 + 选中 + 工具栏激活规则 |
| T5b inline edit | ✅ 三态状态机 + 幽灵行 + 冻结 + 二次确认 modal | ✅ `pnpm test WorkspaceFolderListView` inline 用例 | ✅ ~600 LOC | ✅ 依 T5a | ✅ Enter/Esc/blur/失败保留 4 用例 |
| T6 drag drop + IT | ✅ HTML5 DnD + 2 集成测试 + GIF | ✅ `cargo test` IT + `pnpm test` drop 用例 + 手动 GIF | ✅ ~900 LOC | ✅ 依 T3/T4/T5b | ✅ `test_exdev_two_phase` + `test_delete_dirty_recount` + drop 拦截单测 |

**全部 ✅**，无需拆分。规模上 T3 接近上限（4 命令 × 含单测），如 Dev 实现中触及 1500 LOC 可考虑把"move 命令 + 单测"独立成 T3b，Conductor 阶段保留此选项。

---

## 我（Architect）识别但 PRD 未覆盖的新增风险

1. **NEW-R13 旧 `move_asset_to_workspace_folder` 多素材调用方**：现有 NCdesktop 中 `AssetListView`、`AssetContextMenu` 可能通过既有命令做多素材移动（签名 `Vec<String>`）。新命令收敛为单素材，需 Dev 在 T3 改造调用方为循环调用（或在 wrapper 层批处理）。已在 ADR/影响范围内提醒。
2. **NEW-R12 cleanup_pending 残留时间窗**：EXDEV 失败时源副本可能停留 1+ 启动周期；用户磁盘容量紧张时体感差。需要启动 hook + 24h 阈值清理（不阻塞启动）。
3. **NEW-R14 NFC 自愈 rename 失败**：若同名 NFC 目标已存在（极少，但可能存在用户手动两次创建），不可覆写；策略为跳过 + 日志，可能让某些 asset 持续找不到。如发生则后端 reveal/delete/count 都会一致返 `E_NOT_FOUND`，前端可以提示"路径异常，请在 Finder 中检查"（本期不做主动告警）。

