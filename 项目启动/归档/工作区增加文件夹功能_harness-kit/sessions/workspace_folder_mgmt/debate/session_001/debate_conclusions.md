# Debate Conclusions — workspace_folder_mgmt / session_001

> Host 综合 4 层 Debate 后定稿。每条结论可直接引用至 PRD。
> Host 裁决 2 处：① Proposer 越界引入的"递归 count / E_DEPTH_LIMIT / i18n" 被裁回；② Proposer 在 R4 主张"仅删空文件夹"被否决，恢复用户 spec 的"非空可删+事务内 recount+写锁+残留扫描"。

---

## 1. 问题定义（来自 L1 共识）

- **核心问题**：把 NCdesktop「悬浮窗导入」页从只读 chip 升级为可写 Finder 列表，闭合"新建/重命名/删除/拖拽移动"四件人工整理回路；与 AI 归类正交、互不污染。
- **系统边界**：
  - 范围内：F1-F4 + 项目根级 root + 替换 chip 为 `WorkspaceFolderListView`。
  - 范围外：嵌套子文件夹、多选拖拽、ai_organized 任何写、Win/Linux 删除、跨项目移动、撤销栈、像素级 Finder 复刻。
- **可验收断言（替换 SLO）**：
  - M1：P0 验收清单 1-6 全过（用户原 spec §验收）。
  - M2：集成测试 `test_round_trip_root_to_folder_to_root` 通过。
  - M3：`ai_organized` 四类写均返回 `Err` + 前端 drop 拦截单测。
  - M4：`cargo test` + `pnpm test` 全绿；rename 集成测试断言「DB 受影响行数 = 物理子树文件数」。

## 2. `__ROOT__` Canonical 契约（L1 关键产出）

- `__ROOT__` **仅前端 UI + IPC 入参 sentinel，严禁出现在 DB**。
- `assets.file_path` = 项目工作区根的相对正斜杠路径；根目录散文件存裸文件名，子目录文件存 `参考资料/a.png`。
- 后端单点 `resolve_relative_path()`：`"__ROOT__" → ""`；其余原样。
- F2 rename 前缀替换：`UPDATE assets SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1) WHERE file_path = :old_prefix OR file_path LIKE :old_prefix || '/%' ESCAPE '\'`，`:old_prefix` 必须带尾 `/`。
- F4 `__ROOT__` 双向 drop 合法；根目录同名冲突返回 `IpcError{code:"E_NAME_DUP"}`。
- 防御：`assets` 写路径处加 `debug_assert!(!path.contains("__ROOT__"))`。

## 3. 错误模型（L2 产出）

```ts
type IpcError = {
  code: 'E_NAME_INVALID' | 'E_NAME_DUP' | 'E_NAME_RESERVED'
      | 'E_PATH_ESCAPE' | 'E_PROTECTED_KIND' | 'E_NOT_FOUND'
      | 'E_CROSS_DEVICE' | 'E_PLATFORM_UNSUPPORTED'
      | 'E_TRASH_FAILED' | 'E_FOLDER_DIRTY' | 'E_INTERNAL';
  message: string;     // 仅日志/上报，前端不展示
  details?: Record<string, unknown>;
}
```
- 前端文案表 `errorMessages[code](details)` 唯一来源。
- 序列化：后端 invoke 返回 `Result<T, IpcError>`；Tauri 边界以 `serde_json::to_string(&IpcError)` 转 string 抛错；前端 `JSON.parse` 还原。
- **无 i18n（中文常量即可）**、**无 E_DEPTH_LIMIT / E_CYCLE**（MVP 不嵌套）。

## 4. 交互状态机（L2 产出）

- **选中态** = `uiStore.workspaceFolderRelativePath === row.relativePath`
- **编辑态** = `uiStore.editingFolderPath === row.relativePath`，互斥于拖拽
- **drop 候选态** = `uiStore.dragOverPath === row.relativePath` 且 kind ∈ {root, __ROOT__} 且非编辑态
- **pending 态** = 节点 id ∈ `uiStore.pendingRenameIds`，selection 冻结
- 失焦同步乐观提交：本地立即替换名称、`pendingRenameIds` 加入；IPC resolve 前禁止 selection 离开节点；失败 → 行内 inline error + selection 自动回到该节点。
- F1 幽灵行失败：保留编辑态+红框；Esc 直接丢弃；切走（点其他行/拖拽）触发二次确认 modal「放弃新建『xxx』？」。
- Drop 到编辑行：禁止图标 + toast「目标正在编辑中」，不打断编辑。

## 5. 5 个新 IPC（L2-L3 定稿）

```rust
pub fn create_workspace_folder(project_id: String, name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;
pub fn rename_workspace_folder(project_id: String, relative_path: String, new_name: String)
    -> Result<WorkspaceFolderEntry, IpcError>;
pub fn delete_workspace_folder(project_id: String, relative_path: String, confirm_non_empty: bool, expected_count: u32)
    -> Result<DeleteReport /* { trashed: u32 } */, IpcError>;
pub fn move_asset_to_workspace_folder(asset_id: String, target_relative_path: String)
    -> Result<Asset, IpcError>;
pub fn count_folder_assets(project_id: String, relative_path: String)
    -> Result<u32, IpcError>;
```
- **MVP 内非递归**：count 不下钻（项目无嵌套）。
- `delete_workspace_folder` 入参 `expected_count`：confirm 时取到的 N；后端事务内 recount，不一致返回 `E_FOLDER_DIRTY{old, now}`，前端弹"内容已变化，请重新确认"。

## 6. 风险登记表（L3 定稿）

| # | 风险 | 缓解 |
|---|---|---|
| R1 | EXDEV 跨卷 | copy-first 两阶段：`copy_dir→fsync→rename(tmp→final) → BEGIN/UPDATE/COMMIT → COMMIT 后 remove 源`（失败仅记 `cleanup_pending` 日志） |
| R2 | 路径越界 | `validate_and_canonicalize`：拼接后 `canonicalize()`，`starts_with(workspace_root_canonical)`；拒 `..`/绝对路径/symlink 越界 |
| R3 | SQL `LIKE` 元字符 | `ESCAPE '\\'`；`p` 预转义 `\ % _`；旧路径 `:p` 强制带尾 `/` |
| R4 | 删除 TOCTOU | **保留 spec 非空可删**；事务内 recount 比对 `expected_count`，不一致 → `E_FOLDER_DIRTY`；写通道锁阻塞并发；trash 后扫描残留物理文件，DB 内每个 trashed asset 调用 `delete_asset` |
| R5 | 写并发 | 写通道锁覆盖 5 命令 `{create, rename, delete, move, import}`；read & 缩略图除外 |
| R6 | trash 静默失败 | `trash::delete` 后 `path.exists()` 复检；仍存在 → `E_TRASH_FAILED` |
| R7 | Tauri v2 DnD | 仅用 HTML5 DnD，不接 `tauri://drag-drop`；`dragenter` 计数器避免子元素抖动 |
| R8 | NFC/NFD | readdir 真实字节 B → `nfc(B)=N` 查 DB；miss → 入库存 N；hit B≠N → 一次性 FS `rename(B→N)` + `nfc_healed` 日志（启动期串行扫描自愈） |
| R9 | 深色 drop 不可见 | drop 高亮用 `var(--accent-emphasis)` 2px 边框 + 8% alpha；其余深色细节 POST-MVP |
| R10 | ⌘⌫ 绕过 disabled | handler 入口统一 `if (selection.kind !== 'root') return;`，不依赖 UI disabled |

## 7. MVP 边界声明

**做什么**：F1-F4 + 工具栏三按钮 + 三入口齐全（含 ⌘⌫）+ ⌘⇧N（P1）+ 右键菜单 3 形态 + inline 三态状态机 + 幽灵行 + selection 冻结 + 拖拽双向 + R1-R10 缓解 + 单测+1 集成测试。

**不做什么（附原因）**：
- 嵌套子文件夹（spec 明示）；
- 多选拖拽（spec 明示，二期）；
- Windows/Linux 删除（spec 对齐现有 reveal，返回未支持）；
- 像素级 Finder 复刻（spec 明示）；
- 列宽拖/列排序/列显隐（spec 明示 P2）；
- 视图切换（spec 明示 P2）；
- 多语言文案（项目无 i18n 框架，写中文常量）；
- 跨进程并发锁（单进程多窗口足够，跨进程留 P2）。

## 8. Debate 中未达成共识的争议（Architect 须明确选择）

无未决争议。所有 ❓ 已在 L3/L4 收敛。
- 跨进程并发锁：已共识接受"MVP 仅单进程多窗口"，跨进程 P2 评估。

---

## Task 拆分（L4 定稿，8 task）

| # | Task | 依赖 | 备注 |
|---|---|---|---|
| T0 | 契约冻结：`contracts.md`（IpcError shape + 5 命令签名 + `__ROOT__` 编解码 + 错误码枚举表） | — | 前置所有 |
| T1 | 后端工具层：`trash`+`unicode-normalization` 依赖；`validate_and_canonicalize`、`nfc_normalize`、`__ROOT__→""`、写通道 `Mutex`、`IpcError` enum、启动期 NFC 自愈扫描 hook | T0 | 与 T2 并行 |
| T2 | 前端 IPC 封装：`tauri-commands.ts` 5 个 camelCase + `IpcError` TS 类型 + `errorMessages` 表 | T0 | 与 T1 并行 |
| T3 | 4 个写命令 + handler 入口判定 + Rust 单测（越界/保留字/ai_organized 写/前缀边界/NFC） | T1 | 与 T4 并行 |
| T4 | `count_folder_assets` + uiStore 字段 + 集成测试（rename DB 同步、`__ROOT__` round-trip） | T1 | 与 T3 并行 |
| T5a | `WorkspaceFolderListView` 骨架：列表渲染 + 选中态 + 工具栏三按钮 + 右键菜单 + 三 kind 灰显逻辑 + 键盘 `Enter`/`⌘⌫` handler 入口判定 | T2 | — |
| T5b | inline 编辑状态机：`mode: 'idle'\|'creating'\|'renaming'` + 三态 + 幽灵行 + selection 冻结 + 失败保留态 + 切走二次确认 modal；含组件单测 | T5a | — |
| T6 | F4 拖拽：drop 高亮 2px（深色 token）+ `__ROOT__` 双向 + ai_organized toast；rename/delete 集成测试；PR 截图+10sGIF | T3,T4,T5b | — |
