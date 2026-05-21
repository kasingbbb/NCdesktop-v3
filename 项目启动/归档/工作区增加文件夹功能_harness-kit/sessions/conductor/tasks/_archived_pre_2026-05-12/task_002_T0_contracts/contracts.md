# T0 契约冻结 — NCdesktop 工作区文件夹管理

> 本文档是 T1-T6 所有 Dev/Reviewer 的**唯一契约基线**。下游 task 不得擅自改动以下任何字段；如需变更必须先回到 T0 修订本文档再传导。
>
> 依据：
> - PRD `product/prd/workspace_folder_mgmt_prd_v1.md` §4.3 / §5.1 / §5.2
> - Architect ADR-001（IpcError 序列化）/ ADR-004（`__ROOT__` 编解码）
> - Debate §2（`__ROOT__` Canonical）/ §3（错误模型）
>
> 范围声明（MVP 红线）：仅根级文件夹，错误码闭集 **11 项**，不增不减；**禁止**新增 `E_DEPTH_LIMIT` / `E_CYCLE` / 递归 count 等非 MVP 契约。

---

## (a) IpcError JSON Shape + 11 项 code 闭集

### A.1 序列化协议（ADR-001）

后端命令统一签名 `Result<T, IpcError>`。Tauri v2 invoke 边界仅允许 `Err(String)`，故 `IpcError` 通过 `serde_json::to_string(&err)` 序列化为单行 JSON 字符串，作为 error string 抛出；前端 invoke wrapper 在 catch 处 `JSON.parse(str)` 还原，若 parse 失败则降级为 `{ code: 'E_INTERNAL', message: <原 string>, details: undefined }`。

### A.2 TypeScript Shape（前端契约，逐字遵循 PRD §4.3）

```ts
type IpcErrorCode =
  | 'E_NAME_INVALID'
  | 'E_NAME_DUP'
  | 'E_NAME_RESERVED'
  | 'E_PATH_ESCAPE'
  | 'E_PROTECTED_KIND'
  | 'E_NOT_FOUND'
  | 'E_CROSS_DEVICE'
  | 'E_PLATFORM_UNSUPPORTED'
  | 'E_TRASH_FAILED'
  | 'E_FOLDER_DIRTY'
  | 'E_INTERNAL';

type IpcError = {
  code: IpcErrorCode;
  message: string;                          // 仅日志/上报，不展示给用户
  details?: Record<string, unknown>;        // 各 code 的结构见 A.4
};
```

### A.3 Rust Shape（后端契约）

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpcErrorCode {
    ENameInvalid,
    ENameDup,
    ENameReserved,
    EPathEscape,
    EProtectedKind,
    ENotFound,
    ECrossDevice,
    EPlatformUnsupported,
    ETrashFailed,
    EFolderDirty,
    EInternal,
}
```

> 注：枚举判别字符串以 `serde` 重命名规则导出为字符串字面量（如 `"E_NAME_INVALID"`），与 TS `IpcErrorCode` 字面量集合**字符级一致**。Reviewer 必查此一致性。

### A.4 11 项 code 闭集 + details schema（AC-2）

| # | code | 触发场景（来自命令） | details schema | 必填字段 |
|---|------|---------------------|----------------|----------|
| 1 | `E_NAME_INVALID` | `create_workspace_folder` / `rename_workspace_folder`：名称含 `/ \ :`、以 `.` 开头、空白、长度 > 255 字节、或其他非法字符 | `{ name: string, reason: 'has_slash' \| 'leading_dot' \| 'blank' \| 'too_long' \| 'other' }` | `name` |
| 2 | `E_NAME_DUP` | `create_workspace_folder` / `rename_workspace_folder`：NFC 归一后同级已存在同名 | `{ name: string }` | `name` |
| 3 | `E_NAME_RESERVED` | `create_workspace_folder` / `rename_workspace_folder`：名称命中保留字 `organized` | `{ name: string }` | `name` |
| 4 | `E_PATH_ESCAPE` | 任意写命令：`relative_path` / `target_relative_path` `canonicalize()` 后越出 `project_workspace_dir` | `{ relative_path: string }` | `relative_path` |
| 5 | `E_PROTECTED_KIND` | 任意写命令：目标命中 `ai_organized`（前缀 `organized/`）或其他受保护 kind | `{ kind: 'ai_organized', relative_path: string }` | `kind`、`relative_path` |
| 6 | `E_NOT_FOUND` | 任意命令：项目 / 文件夹 / asset 不存在 | `{ entity: 'project' \| 'folder' \| 'asset', id: string }` | `entity`、`id` |
| 7 | `E_CROSS_DEVICE` | `rename_workspace_folder` / `move_asset_to_workspace_folder`：EXDEV 两阶段 copy 阶段失败（rename 自身 EXDEV 走 copy-first 不返此码） | `{ src: string, dst: string }` | `src`、`dst` |
| 8 | `E_PLATFORM_UNSUPPORTED` | `delete_workspace_folder` 在非 macOS 平台 | `{ feature: 'trash', platform: 'windows' \| 'linux' \| 'other' }` | `feature`、`platform` |
| 9 | `E_TRASH_FAILED` | `delete_workspace_folder`：`trash` crate 返回成功但 `path.exists()` 仍为真，或 crate 自身报错 | `{ relative_path: string }` | `relative_path` |
| 10 | `E_FOLDER_DIRTY` | `delete_workspace_folder`：事务内 recount 与 `expected_count` 不一致 | `{ old: u32, now: u32 }` | `old`、`now` |
| 11 | `E_INTERNAL` | 任意命令：DB / IO / serde 等非预期失败 | `{}`（可选 `{ hint?: string }`，仅日志） | — |

> **闭集断言**：上表 11 项即全部，新增任何 code 必须先回 T0 修订。`E_DEPTH_LIMIT`、`E_CYCLE` 等 MVP 范围外的 code **不存在**。

---

## (b) 5 命令 Rust 签名（逐字复制 PRD §5.1）

> **红线**：以下签名（命令名、参数名、参数顺序、参数类型、返回类型）必须**字符级一致**。下游 task 不得改 arity、不得把 `relative_path` 改成 `path`、不得改 `expected_count: u32` 的类型。

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

### B.1 关联类型

```rust
pub struct DeleteReport {
    pub trashed: u32,
}

// WorkspaceFolderEntry / Asset 沿用既有定义（src/types/workspace.ts、
// src-tauri/src/commands/workspace_folders.rs），本 task 不重新定义。
```

### B.2 前端 camelCase Wrapper（`src/lib/tauri-commands.ts`）

下游 T2 必须按以下命名包装（参数名转 camelCase，顺序对齐）：

| 后端 snake_case | 前端 camelCase wrapper | 参数（camelCase） |
|---|---|---|
| `create_workspace_folder` | `createWorkspaceFolder` | `projectId, name` |
| `rename_workspace_folder` | `renameWorkspaceFolder` | `projectId, relativePath, newName` |
| `delete_workspace_folder` | `deleteWorkspaceFolder` | `projectId, relativePath, confirmNonEmpty, expectedCount` |
| `move_asset_to_workspace_folder` | `moveAssetToWorkspaceFolder` | `assetId, targetRelativePath` |
| `count_folder_assets` | `countFolderAssets` | `projectId, relativePath` |

### B.3 错误码 ↔ 命令矩阵（哪个命令可能抛哪些 code）

| 命令 \ code | NAME_INVALID | NAME_DUP | NAME_RESERVED | PATH_ESCAPE | PROTECTED_KIND | NOT_FOUND | CROSS_DEVICE | PLATFORM_UNSUPPORTED | TRASH_FAILED | FOLDER_DIRTY | INTERNAL |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `create_workspace_folder` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓(project) | — | — | — | — | ✓ |
| `rename_workspace_folder` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | — | — | ✓ |
| `delete_workspace_folder` | — | — | — | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | ✓ |
| `move_asset_to_workspace_folder` | — | — | — | ✓ | ✓ | ✓ | ✓ | — | — | — | ✓ |
| `count_folder_assets` | — | — | — | ✓ | — | ✓ | — | — | — | — | ✓ |

---

## (c) `__ROOT__` 编解码契约（ADR-004）

`__ROOT__` 是**仅 UI / IPC 边界使用的 sentinel 字符串**，代表「项目工作区根目录」。**DB 永不存储** `__ROOT__`。

### C.1 入站（前端 → 后端 invoke 入参）

- 任意接收 `relative_path` / `target_relative_path` 的命令入参，**允许**传入字面量 `"__ROOT__"`。
- 后端在命令首行（写命令）或 helper 入口（read 命令）调用单点函数：

```rust
fn resolve_relative_path(rel: &str) -> &str {
    if rel == "__ROOT__" { "" } else { rel }
}
```

- 归一后内部以**空字符串 `""`** 表示根目录，再进入 `validate_and_canonicalize` / 路径拼接。

### C.2 DB 存储契约

- `assets.file_path` 存 workspace 根的**正斜杠相对路径**：
  - 根文件 → `"file.png"`（裸文件名，无前导 `/`、**不含 `__ROOT__`**）
  - 子目录文件 → `"folder/file.png"`
- **写路径硬约束**：所有 `INSERT` / `UPDATE` 到 `assets.file_path` 的入口必须加：

```rust
debug_assert!(!path.contains("__ROOT__"),
    "BUG: __ROOT__ sentinel leaked into DB write path: {}", path);
```

- 此 `debug_assert!` 在 debug 构建命中即为契约违反，必须修，**不得**改成 release 静默。

### C.3 出站（后端 → 前端返回）

- `list_project_workspace_folders` 返回的第一行 `relative_path` 字段恢复为字面量 `"__ROOT__"`（既有契约，不变）。
- 其他命令返回 `WorkspaceFolderEntry` 时，若该 entry 对应根目录，`relative_path` 亦填 `"__ROOT__"`。
- 中间路径段（如 `folder/subfile`）**永远不含** `__ROOT__`。

### C.4 Round-trip 验证（下游 T4 集成测试承载）

- 测试名：`test_round_trip_root_to_folder_to_root`（PRD §6.2）。
- 路径：素材从 `__ROOT__` → 某 folder → `__ROOT__`，每跳后断言：
  1. 物理文件位于预期目录
  2. `assets.file_path` 不含 `__ROOT__` 子串（用 `LIKE '%\_\_ROOT\_\_%' ESCAPE '\'` 反向检查）
  3. `list_project_workspace_folders` 第一行 `relative_path == "__ROOT__"`

---

## (d) errorMessages 中文文案表

> 全中文常量，**无 i18n**（Debate §3、PRD §5）。前端 `src/lib/ipc-errors.ts` 实现 `errorMessages: Record<IpcErrorCode, (details?: Record<string, unknown>) => string>`，是用户可见文案的**唯一来源**；后端 `IpcError.message` 字段**禁止**直接展示。

### D.1 文案表

| code | 函数签名 | 中文文案模板 | 渲染说明 |
|---|---|---|---|
| `E_NAME_INVALID` | `(d: { name: string, reason?: string }) => string` | `名称「{name}」不合法，不能包含 / \ :、不能以 . 开头、不能为空、长度需在 255 字节内。` | 直接拼 `name`；`reason` 仅日志用，不进文案 |
| `E_NAME_DUP` | `(d: { name: string }) => string` | `同级目录下已存在名为「{name}」的文件夹。` | 直接拼 `details.name` |
| `E_NAME_RESERVED` | `(d: { name: string }) => string` | `「{name}」是系统保留名称，请换一个。` | 直接拼 `name` |
| `E_PATH_ESCAPE` | `(d: { relative_path: string }) => string` | `路径越界：「{relative_path}」不在当前项目工作区内。` | 直接拼 `relative_path` |
| `E_PROTECTED_KIND` | `(d: { kind: string }) => string` | `AI 归类目录受保护，不可手动操作。` | 文案固定，`kind` 仅日志确认 |
| `E_NOT_FOUND` | `(d: { entity?: string }) => string` | `目标不存在或已被移动，请刷新后重试。` | 文案固定 |
| `E_CROSS_DEVICE` | `(d?: { src?: string, dst?: string }) => string` | `跨磁盘迁移失败，请确认目标磁盘有足够空间后重试。` | 文案固定 |
| `E_PLATFORM_UNSUPPORTED` | `(d?: { feature?: string }) => string` | `当前平台暂不支持该操作（仅 macOS 支持移到废纸篓）。` | 文案固定 |
| `E_TRASH_FAILED` | `(d?: { relative_path?: string }) => string` | `移到废纸篓失败，请检查系统回收站权限。` | 文案固定 |
| `E_FOLDER_DIRTY` | `(d: { old: number, now: number }) => string` | `该文件夹内容已变化（原 {old} 个，现 {now} 个），请重新确认。` | **必须用 `details.now` 渲染**（PRD §4.2、AC-1） |
| `E_INTERNAL` | `(d?: Record<string, unknown>) => string` | `内部错误，请稍后重试。` | 文案固定；`d.hint` 仅日志用 |

### D.2 渲染参考实现

```ts
export const errorMessages: Record<IpcErrorCode, (details?: Record<string, unknown>) => string> = {
  E_NAME_INVALID: (d) =>
    `名称「${(d?.name as string) ?? ''}」不合法，不能包含 / \\ :、不能以 . 开头、不能为空、长度需在 255 字节内。`,
  E_NAME_DUP: (d) =>
    `同级目录下已存在名为「${(d?.name as string) ?? ''}」的文件夹。`,
  E_NAME_RESERVED: (d) =>
    `「${(d?.name as string) ?? ''}」是系统保留名称，请换一个。`,
  E_PATH_ESCAPE: (d) =>
    `路径越界：「${(d?.relative_path as string) ?? ''}」不在当前项目工作区内。`,
  E_PROTECTED_KIND: () => `AI 归类目录受保护，不可手动操作。`,
  E_NOT_FOUND: () => `目标不存在或已被移动，请刷新后重试。`,
  E_CROSS_DEVICE: () => `跨磁盘迁移失败，请确认目标磁盘有足够空间后重试。`,
  E_PLATFORM_UNSUPPORTED: () => `当前平台暂不支持该操作（仅 macOS 支持移到废纸篓）。`,
  E_TRASH_FAILED: () => `移到废纸篓失败，请检查系统回收站权限。`,
  E_FOLDER_DIRTY: (d) =>
    `该文件夹内容已变化（原 ${(d?.old as number) ?? 0} 个，现 ${(d?.now as number) ?? 0} 个），请重新确认。`,
  E_INTERNAL: () => `内部错误，请稍后重试。`,
};
```

> 实现注意：`E_FOLDER_DIRTY` 在文案中**必须读取 `details.now`** 用于「现 N 个」的渲染；同时前端在 catch 处需用 `details.now` 重新发起 `delete_workspace_folder(expected_count = now)` 的二次确认（ADR-010、PRD §4.2）。

---

## 消费方核对清单（AC-3）

> 下游 task 启动前，先按此表回查 contracts.md 对应小节，避免对 PRD 直接重读时再次解释偏离。

### T1 — 后端工具层（`task_005`/`003` Backend Utils）

需从本文档抽取的字段：
- (a) A.3 `IpcError` / `IpcErrorCode` 的 Rust 定义 → 落到 `src-tauri/src/utils/ipc_error.rs`
- (a) A.4 11 项 code 闭集与 `details` schema → 在 `From<E> for IpcError` 映射时严格匹配字段
- (c) C.1 `resolve_relative_path` 函数签名 → 落到 `workspace.rs`
- (c) C.2 `debug_assert!(!path.contains("__ROOT__"))` → 落到 `db::asset` 所有写入入口
- (b) B.1 `DeleteReport` 定义

### T2 — 前端 IPC 封装（`task_004` Frontend IPC）

需从本文档抽取的字段：
- (a) A.2 `IpcErrorCode` / `IpcError` TS 类型 → 落到 `src/types/workspace.ts`
- (a) A.4 11 项 code 字面量 + `details` 字段名 → 用于 `errorMessages` 函数签名类型
- (b) B.2 5 个 camelCase wrapper 名称与参数顺序 → 落到 `src/lib/tauri-commands.ts`
- (b) B.3 错误码 ↔ 命令矩阵 → 用于 catch 分支单测
- (c) C.1 出站 `__ROOT__` 字面量 → 在 wrapper 解包时**不要**翻译，直接透传给 UI
- (d) D.1 / D.2 文案表 → 落到 `src/lib/ipc-errors.ts`

### T3 — 4 写命令（`task_005` Write Commands）

需从本文档抽取的字段：
- (b) B 节 4 写命令的**精确签名**（逐字复制） → `commands/workspace_folders.rs` & `commands/asset.rs`
- (a) A.4 每个命令在 B.3 矩阵中标记 ✓ 的 code → 对应错误分支必须存在 + 有单测
- (c) C.1 入站归一 + C.2 写路径 `debug_assert!` → 每个写命令首行必经
- (c) C.3 出站填 `"__ROOT__"` → `WorkspaceFolderEntry` 返回时根目录场景

### T4 — count + 状态 + 集成测试（`task_006`）

需从本文档抽取的字段：
- (b) B 节 `count_folder_assets` 签名 → `commands/workspace_folders.rs`
- (c) C.4 Round-trip 测试要求 → `test_round_trip_root_to_folder_to_root` 集成测试断言点
- (a) A.4 `E_FOLDER_DIRTY` 的 `{ old, now }` schema → 集成测试 `test_delete_dirty_recount` 断言
- (d) D.1 `E_FOLDER_DIRTY` 文案使用 `details.now` 的契约 → 集成测试可用于回归该契约
