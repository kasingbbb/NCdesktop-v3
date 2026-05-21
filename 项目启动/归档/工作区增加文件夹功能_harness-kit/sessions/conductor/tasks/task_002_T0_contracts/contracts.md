# Contracts — 工作区文件夹管理（T0 契约冻结）

> **红线声明（必读）**
>
> 本文件是 task_002_T0_contracts 的最终产出，为 **T1-T6 唯一权威契约**：
> 1. **IpcError JSON shape**、**11 项错误码闭集**、**5 个新 Tauri 命令签名**、**`__ROOT__` 编解码三处单点**、**错误码 ↔ 中文文案表** 全部以本文件为准；
> 2. **下游 Dev / Reviewer 不得擅自改动**任何字段名 / 字面量 / 类型 / 渲染规则。如需变更，必须**回到 T0 task 修订本文档**，再由 Conductor 重新传导给所有下游 task；
> 3. PRD §4.3 / §5.1 是上游来源，本文件与 PRD **逐字一致**；若发现不一致，以本文件 + PRD 双向核对后选 PRD，并立即提 issue 修订；
> 4. **MVP 错误码闭集 = 11 项**，**禁止**新增 `E_DEPTH_LIMIT` / `E_CYCLE` / `E_PERMISSION` 等任何码。
>
> 上游：`product/prd/workspace_folder_mgmt_prd_v1.md` §4.3 / §5.1 / §5.2、`sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §2/§3/§5、`sessions/conductor/tasks/task_001_architect/output.md` ADR-001/ADR-004/ADR-008。
> 下游：T1 (backend utils) / T2 (frontend ipc) / T3 (write commands) / T4 (count+state+IT) / T5a / T5b / T6。

---

## §A IpcError JSON Shape（双向规范）

### A.1 序列化协议（来自 ADR-001）

- 后端命令统一签名 `Result<T, IpcError>`；通过 `impl From<IpcError> for String { serde_json::to_string(&err).unwrap_or_else(|_| INTERNAL_FALLBACK_JSON) }` 转成单行 JSON 抛过 Tauri invoke 边界。
- 前端 catch 处 `JSON.parse` 还原 → `IpcError` 对象；parse 失败 → 降级为 `{ code: "E_INTERNAL", message: <raw>, details: undefined }`。
- **Rust enum `IpcErrorCode` 的 `#[serde(rename = "...")]` 字面量 必须与 TS `IpcErrorCode` 联合类型字面量字符级一致**（含大小写、下划线）。任何一侧漂移都视为契约破坏。

### A.2 TypeScript shape（前端权威拷贝；T2 落地为 `src/types/workspace.ts` + `src/lib/ipc-errors.ts`）

```ts
export type IpcErrorCode =
  | "E_NAME_INVALID"
  | "E_NAME_DUP"
  | "E_NAME_RESERVED"
  | "E_PATH_ESCAPE"
  | "E_PROTECTED_KIND"
  | "E_NOT_FOUND"
  | "E_CROSS_DEVICE"
  | "E_PLATFORM_UNSUPPORTED"
  | "E_TRASH_FAILED"
  | "E_FOLDER_DIRTY"
  | "E_INTERNAL";

export interface IpcError {
  code: IpcErrorCode;
  message: string;                         // 仅日志/上报，不展示给用户
  details?: Record<string, unknown>;       // 各 code 专属，见 §A.4
}
```

### A.3 Rust shape（后端权威拷贝；T1 落地为 `src-tauri/src/utils/ipc_error.rs`）

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")] // 形式说明；为避免 Rust 标识符限制，逐项 rename
pub enum IpcErrorCode {
    #[serde(rename = "E_NAME_INVALID")]        ENameInvalid,
    #[serde(rename = "E_NAME_DUP")]            ENameDup,
    #[serde(rename = "E_NAME_RESERVED")]       ENameReserved,
    #[serde(rename = "E_PATH_ESCAPE")]         EPathEscape,
    #[serde(rename = "E_PROTECTED_KIND")]      EProtectedKind,
    #[serde(rename = "E_NOT_FOUND")]           ENotFound,
    #[serde(rename = "E_CROSS_DEVICE")]        ECrossDevice,
    #[serde(rename = "E_PLATFORM_UNSUPPORTED")] EPlatformUnsupported,
    #[serde(rename = "E_TRASH_FAILED")]        ETrashFailed,
    #[serde(rename = "E_FOLDER_DIRTY")]        EFolderDirty,
    #[serde(rename = "E_INTERNAL")]            EInternal,
}

#[derive(Debug, Clone, Serialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,                       // 仅日志/上报
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,    // 各 code 专属，见 §A.4
}

// Tauri 边界：Result<T, IpcError> 通过 From<IpcError> for String 序列化
impl From<IpcError> for String {
    fn from(err: IpcError) -> String {
        serde_json::to_string(&err).unwrap_or_else(|_| {
            // 序列化兜底；理论不可达，保留单行有效 JSON 防前端 parse 死
            r#"{"code":"E_INTERNAL","message":"ipc_error_serialize_failed"}"#.to_string()
        })
    }
}
```

**Rust ↔ TS 字面量对照表（字符级一致；任何修改必须双向同步）**

| Rust variant | `#[serde(rename)]` 字面量 | TS 字面量 |
|---|---|---|
| `ENameInvalid`        | `"E_NAME_INVALID"`        | `"E_NAME_INVALID"`        |
| `ENameDup`            | `"E_NAME_DUP"`            | `"E_NAME_DUP"`            |
| `ENameReserved`       | `"E_NAME_RESERVED"`       | `"E_NAME_RESERVED"`       |
| `EPathEscape`         | `"E_PATH_ESCAPE"`         | `"E_PATH_ESCAPE"`         |
| `EProtectedKind`      | `"E_PROTECTED_KIND"`      | `"E_PROTECTED_KIND"`      |
| `ENotFound`           | `"E_NOT_FOUND"`           | `"E_NOT_FOUND"`           |
| `ECrossDevice`        | `"E_CROSS_DEVICE"`        | `"E_CROSS_DEVICE"`        |
| `EPlatformUnsupported`| `"E_PLATFORM_UNSUPPORTED"`| `"E_PLATFORM_UNSUPPORTED"`|
| `ETrashFailed`        | `"E_TRASH_FAILED"`        | `"E_TRASH_FAILED"`        |
| `EFolderDirty`        | `"E_FOLDER_DIRTY"`        | `"E_FOLDER_DIRTY"`        |
| `EInternal`           | `"E_INTERNAL"`            | `"E_INTERNAL"`            |

### A.4 11 项 code 闭集 — `details` schema 逐项定义

> 约定：`details` 字段一律走 camelCase（前端直接消费）；Rust 侧通过 `serde_json::json!({...})` 构造，键名以本表为准。**前端文案渲染依赖的字段标 `必填`**，缺失视为契约破坏。

| # | code                     | 触发来源                                                                 | `details` schema（`必填` / 可选）                                                                                                          | 备注 |
|---|--------------------------|--------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|------|
| 1 | `E_NAME_INVALID`         | `workspace::validate_folder_name`：含 `/ \ :`、`.` 开头、空白、>255 字节 | `{ name: string 必填, reason: "slash"\|"dot_prefix"\|"whitespace"\|"too_long"\|"empty" 必填 }`                                              | reason 用于文案精细化；T1 实现时枚举值必须取本闭集 |
| 2 | `E_NAME_DUP`             | 同级 NFC 同名（含 root 文件夹与根直接文件同名）                          | `{ name: string 必填, parentRelativePath: string 必填 }` (`parentRelativePath = ""` 表示根级；**不得**用 `"__ROOT__"`)                       | NFC 比对后认为重名 |
| 3 | `E_NAME_RESERVED`        | 命名命中保留字（当前仅 `organized`）                                     | `{ name: string 必填, reserved: "organized" 必填 }`                                                                                          | 保留字闭集仅 `organized` |
| 4 | `E_PATH_ESCAPE`          | `validate_and_canonicalize` 拼接后未 `starts_with(workspace_root)`       | `{ requestedPath: string 必填 }`                                                                                                              | 不暴露 canonical 后绝对路径，避免敏感泄漏 |
| 5 | `E_PROTECTED_KIND`       | handler 入口推 kind 命中 `ai_organized` / `root_import` 写动作            | `{ kind: "ai_organized"\|"root_import" 必填, action: "create"\|"rename"\|"delete"\|"move_in"\|"move_out" 必填 }`                            | direct invoke 也走此分支 |
| 6 | `E_NOT_FOUND`            | 目标 folder / asset 物理或 DB 不存在                                     | `{ target: "folder"\|"asset" 必填, identifier: string 必填 }` (folder = relativePath；asset = assetId)                                       | 文案合并展示 |
| 7 | `E_CROSS_DEVICE`         | EXDEV 且 copy-first 两阶段最终失败（非 happy path）                       | `{ src: string 可选, dst: string 可选, stage: "copy"\|"fsync"\|"rename_tmp"\|"remove_src" 可选 }`                                            | 正常 EXDEV 走 happy path 不抛此码；仅失败时抛 |
| 8 | `E_PLATFORM_UNSUPPORTED` | Win/Linux 删除走 trash 分支                                              | `{ feature: "trash" 必填, platform: "windows"\|"linux"\|"unknown" 必填 }`                                                                  | 与既有 `reveal_project_workspace_folder` 行为对齐 |
| 9 | `E_TRASH_FAILED`         | `trash::delete` 成功返回但 `path.exists()` 复检仍存在；或 trash crate Err | `{ path: string 必填, reason: "still_exists"\|"crate_error" 必填 }`                                                                          | 兜底 |
| 10| `E_FOLDER_DIRTY`         | `delete_workspace_folder` 事务内 recount ≠ `expected_count`              | `{ old: number 必填, now: number 必填 }` (`old = expected_count` 入参；`now = recount 实际值`)                                              | **前端文案模板必须用 `now` 渲染**；用户重弹 modal 时也用 `now` 作为新 expected_count |
| 11| `E_INTERNAL`             | 其他未归类异常（DB / IO / panic-catch / 序列化失败兜底）                  | `{ where?: string }`（可选，仅用于上报；前端文案统一）                                                                                       | 前端兜底分支 |

**不变量**：
- 11 项闭集封闭；**任何新增/合并/删除必须先回 T0 修订本表**。
- `message` 字段允许后端写入任意人类可读字符串，**仅用于日志/上报**；前端**绝不**渲染 `message`。
- `details` 字段可选但**一旦存在**必须满足上表 schema；前端渲染时缺 `必填` 字段 → 降级为通用文案 + 上报 telemetry。

---

## §B 5 命令签名（最终）

### B.1 Rust 后端签名（逐字搬运 PRD §5.1）

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
) -> Result<DeleteReport, IpcError>;

#[tauri::command]
pub fn move_asset_to_workspace_folder(asset_id: String, target_relative_path: String)
    -> Result<Asset, IpcError>;

#[tauri::command]
pub fn count_folder_assets(project_id: String, relative_path: String)
    -> Result<u32, IpcError>;
```

**返回值数据结构（权威）**

```rust
// 来自既有 commands/workspace_folders.rs（T0 不动代码，仅锁字段）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolderEntry {
    pub relative_path: String,    // 根目录直接文件场景 = "__ROOT__"
    pub display_label: String,
    pub kind: String,             // "root" | "root_import" | "ai_organized"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub trashed: u32,
}

// Asset 沿用既有 src-tauri/src/models.rs::Asset，T0 不改其字段
```

### B.2 前端 camelCase Wrapper 签名（T2 落地为 `src/lib/tauri-commands.ts`）

> 异常契约：**所有 wrapper 内部走 `invokeWithIpcError<T>()`**；catch 处 `JSON.parse` 还原 `IpcError`，parse 失败降级 `E_INTERNAL`。wrapper 对外**只抛 `IpcError` 类型异常**（throw 实例满足 `IpcError` shape）。

```ts
import type { Asset, IpcError, WorkspaceFolderEntry, DeleteReport } from "@/types/workspace";

/** F1 新建 root 文件夹 */
export function createWorkspaceFolder(
  projectId: string,
  name: string,
): Promise<WorkspaceFolderEntry>;

/** F2 重命名 root 文件夹（同事务前缀替换） */
export function renameWorkspaceFolder(
  projectId: string,
  relativePath: string,            // 入参可为 "__ROOT__"，后端 resolve_relative_path 归一
  newName: string,
): Promise<WorkspaceFolderEntry>;

/** F3 删除 root 文件夹到回收站（含事务内 recount） */
export function deleteWorkspaceFolder(
  projectId: string,
  relativePath: string,            // 入参可为 "__ROOT__"
  confirmNonEmpty: boolean,
  expectedCount: number,           // 来自前端聚合（见 ADR-010）
): Promise<DeleteReport>;

/** F4 单素材移动到目标文件夹 / 回根目录 */
export function moveAssetToWorkspaceFolder(
  assetId: string,
  targetRelativePath: string,      // 入参可为 "__ROOT__"
): Promise<Asset>;

/** 文件夹 asset 计数（删除前 recount 备份；列表 UI 不用，列表走前端聚合） */
export function countFolderAssets(
  projectId: string,
  relativePath: string,            // 入参可为 "__ROOT__"
): Promise<number>;
```

**Tauri invoke 参数命名（关键）**：Tauri v2 默认会把 Rust snake_case 参数名映射为 invoke payload 的 camelCase；wrapper 内部以 camelCase key 传：
- `create_workspace_folder` → `{ projectId, name }`
- `rename_workspace_folder` → `{ projectId, relativePath, newName }`
- `delete_workspace_folder` → `{ projectId, relativePath, confirmNonEmpty, expectedCount }`
- `move_asset_to_workspace_folder` → `{ assetId, targetRelativePath }`
- `count_folder_assets` → `{ projectId, relativePath }`

### B.3 注册与退役

- T3 完成时，5 命令统一注册在 `src-tauri/src/lib.rs::invoke_handler!`。
- **退役**：旧 `commands::asset::move_asset_to_workspace_folder`（多素材 `Vec<String>` 签名）从 `invoke_handler!` 注销；调用方（`AssetListView` 拖拽、`AssetContextMenu`）改为循环调用单素材新命令（详见 NEW-R13 / Architect output.md）。

---

## §C `__ROOT__` Sentinel 编解码三处单点

> ADR-004 落地规则。`__ROOT__` **仅作 UI/IPC sentinel**，**永不入 DB**；任何 fs 写路径在拼接前必经 `resolve_relative_path` 归一。

### C.1 入站（前端 invoke 入参 → 后端 fs 拼接前）

**单点位置**：`src-tauri/src/workspace.rs::resolve_relative_path(rel: &str) -> &str`（T1 落地）。

```rust
/// 归一前端传入的 relative_path / target_relative_path：
///   - "__ROOT__"  → ""        （空相对路径 = 项目工作区根）
///   - 其余        → 原样返回（仍需后续 validate_and_canonicalize 做越界拒）
pub fn resolve_relative_path(rel: &str) -> &str {
    if rel == "__ROOT__" { "" } else { rel }
}
```

适用命令：`rename_workspace_folder`、`delete_workspace_folder`、`move_asset_to_workspace_folder`、`count_folder_assets`（凡入参含 `relative_path` / `target_relative_path`）。`create_workspace_folder` 入参是 `name`（不带路径），其 parent 隐含 = 根，不走此函数。

### C.2 DB 写入（`assets.file_path` 永不含 `__ROOT__`）

**单点位置**：所有 `INSERT INTO assets` 或 `UPDATE assets SET file_path = ...` 入口（T1/T3 检视；T1 在 `db/asset.rs::rename_path_prefix` 内加断言）。

```rust
// 任何 assets 写路径前置防御
debug_assert!(!path.contains("__ROOT__"),
    "assets.file_path 严禁包含 __ROOT__ sentinel；path={}", path);
```

- `assets.file_path` 语义：项目工作区根的相对正斜杠路径。
- 根目录直接文件 → `file_path = "a.png"`（裸文件名，不含 `/`）。
- 子目录文件 → `file_path = "参考资料/a.png"`。
- **绝不允许** `__ROOT__/a.png` / `__ROOT__` / 含 `__ROOT__` 子串的任何值入库。

### C.3 出站（后端返回前端）

**单点位置**：`commands/workspace_folders.rs::list_project_workspace_folders` 首行返回（已既有逻辑，T0 不改代码）。

- `list_project_workspace_folders` 返回数组**首行**仍为根行 `WorkspaceFolderEntry { relative_path: "__ROOT__", display_label: "（根目录）" 或既有 label, kind: "root_import" }`。
- 其余 4 命令（`create/rename/delete/move/count`）返回的 `WorkspaceFolderEntry` 在根目录场景**也用 `relative_path = "__ROOT__"`**（与 list 出口对齐，前端不需要分支）。
- `move_asset_to_workspace_folder` 返回 `Asset`，其 `file_path` 字段**不带** `__ROOT__`（与 DB 一致）。

### C.4 红线检查清单（Reviewer 校验点）

- [ ] `resolve_relative_path` 是入站归一唯一入口；4 个含 `relative_path` 入参的命令第一步必经
- [ ] `assets` 写路径 4 处（rename 前缀替换、move、delete 内 `delete_asset`、import）全部有 `debug_assert!` 防御
- [ ] `list_project_workspace_folders` 首行仍是 `__ROOT__`
- [ ] 单元/集成测试 `test_round_trip_root_to_folder_to_root` 验证三处单点闭环（T4 拥有）

---

## §D 错误码 → 中文文案模板（前端唯一来源）

**单点位置**：`src/lib/ipc-errors.ts::errorMessages: Record<IpcErrorCode, (details?: Record<string, unknown>) => string>`（T2 落地）。

> 渲染规则：
> 1. 前端 catch `IpcError` → `errorMessages[err.code](err.details)` 得到用户可见文案；
> 2. **`message` 字段绝不展示**（仅 `console.warn` / 上报）；
> 3. **缺失 `details` 必填字段** → 降级返回该 code 的通用文案（不抛二次错），并 `console.warn` 一条「ipc_error_details_missing」上报；
> 4. 文案口吻：中文、动名词短句、不加感叹号、不加 emoji；建议长度 ≤ 32 字。

| # | code                     | 文案模板（建议；T2 实现时可微调，但**字段名 / 渲染语义不变**）                                       | 依赖 `details` 字段              |
|---|--------------------------|------------------------------------------------------------------------------------------------------|----------------------------------|
| 1 | `E_NAME_INVALID`         | 「名称不合法（{reasonText}）」 — `reasonText` 由 `reason` 映射：`slash`→`不能包含 / \ :`、`dot_prefix`→`不能以 . 开头`、`whitespace`→`不能含空白`、`too_long`→`超过 255 字节`、`empty`→`不能为空` | `name`, `reason` 必填             |
| 2 | `E_NAME_DUP`             | 「同级已存在同名文件夹『{name}』」                                                                  | `name` 必填；`parentRelativePath` 可选用于上报 |
| 3 | `E_NAME_RESERVED`        | 「『{name}』是保留名称，请换一个」                                                                  | `name` 必填                       |
| 4 | `E_PATH_ESCAPE`          | 「路径越界，已拒绝」（不展示原始 path 防泄漏）                                                       | `requestedPath` 仅上报，不展示    |
| 5 | `E_PROTECTED_KIND`       | 「AI 归类目录 / 导入副本不支持{actionText}」 — `actionText` 由 `action` 映射：`create`→`新建子文件夹`、`rename`→`重命名`、`delete`→`删除`、`move_in`→`移入`、`move_out`→`移出` | `kind`, `action` 必填             |
| 6 | `E_NOT_FOUND`            | `target=folder` → 「文件夹不存在或已被删除」；`target=asset` → 「素材不存在或已被删除」                | `target` 必填；`identifier` 仅上报 |
| 7 | `E_CROSS_DEVICE`         | 「跨卷迁移失败，请稍后重试」                                                                         | 全部可选；仅上报                  |
| 8 | `E_PLATFORM_UNSUPPORTED` | 「当前系统暂不支持{featureText}」 — `featureText` 由 `feature` 映射：`trash`→`移到回收站`              | `feature`, `platform` 必填        |
| 9 | `E_TRASH_FAILED`         | 「移到回收站失败，请稍后重试」                                                                       | `path` 仅上报；`reason` 可选      |
| 10| `E_FOLDER_DIRTY`         | 「内容已变化：当前包含 {now} 个素材，请重新确认」                                                    | **`now` 必填**（渲染依赖）；`old` 必填（仅日志） |
| 11| `E_INTERNAL`             | 「操作失败，请稍后重试或重启应用」                                                                   | 全部可选；仅上报                  |

**测试约束（T2 单测）**：
- 11 项 code 全部覆盖 `errorMessages[code]` 调用，验证返回字符串非空且不含 `undefined` / `[object Object]`；
- `E_FOLDER_DIRTY` 测例必须断言渲染结果包含 `details.now` 的字符串形式（如 `"3"`）。

---

## §E 变更管控

任何下游 task 若发现：
1. PRD / Debate / Architect 之间存在与本文件矛盾的字段；
2. 实现中发现 `details` schema 需要新增字段（如 `E_TRASH_FAILED` 需要返 `errnoCode`）；
3. 错误码不足以表达某个失败语义（如 trash crate 区分用户取消 / 配额不足）；

→ **不得**直接在 T1-T6 修改实现；正确做法：
1. 向 Conductor 提 issue「T0 contracts 修订请求」并附**具体字段 diff**；
2. Conductor 评估后回到 task_002_T0_contracts 修订本文件；
3. 修订完成后由 Conductor 重新通知所有下游 task；
4. 不允许「先实现再补契约」。

---

## §F 与既有代码的对齐情况（仅信息，T0 不动代码）

| 已存在文件                                              | 与本契约的关系                                                                 |
|---------------------------------------------------------|--------------------------------------------------------------------------------|
| `NCdesktop/src/types/workspace.ts`                      | 已包含 `IpcErrorCode` / `IpcError` / `DeleteReport`，与 §A.2 / §B.1 字段一致 |
| `NCdesktop/src-tauri/src/commands/workspace_folders.rs` | 已包含 `WorkspaceFolderEntry` / `DeleteReport` 结构，与 §B.1 字段一致；4 写命令 stub 已存在但实现归 T3 |
| `NCdesktop/src-tauri/src/utils/ipc_error.rs`            | 由 T1 落地；其 `IpcErrorCode` enum 必须严格遵循 §A.3 rename 表                |
| `NCdesktop/src/lib/ipc-errors.ts`                       | 由 T2 落地；其 `errorMessages` 必须严格遵循 §D 渲染规则                       |

> 既有代码字段若与本契约不一致，以**本契约**为准；T1/T2/T3 实现时直接覆写既有 stub。
