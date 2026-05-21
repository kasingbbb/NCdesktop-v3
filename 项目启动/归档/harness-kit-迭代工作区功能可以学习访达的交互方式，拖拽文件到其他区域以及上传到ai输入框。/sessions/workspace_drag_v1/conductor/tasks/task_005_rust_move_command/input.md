# Task 输入 — task_005_rust_move_command

## 目标

在 `src-tauri/src/commands/asset.rs` 中新增 `move_asset_to_workspace_folder` 命令，实现将指定文件列表原子性地移动到当前项目 workspace 内的某个子目录，磁盘与 DB 同步更新，任一步骤失败时全量回滚。

## 前置条件

- 依赖 task：无（可与 task_002、task_003、task_004 并行开发）
- 必须先存在的文件/接口：
  - `src-tauri/src/commands/asset.rs`（追加函数）
  - `src-tauri/src/workspace.rs`：提供 `project_workspace_dir(&project_id) -> Result<PathBuf, String>`
  - `src-tauri/src/db/asset.rs`：提供 `get_by_id`、`update_name_and_path`
  - `src-tauri/src/lib.rs`：`invoke_handler![]` 注册入口
  - `src/lib/tauri-commands.ts`：追加 TS 包装

## 验收标准（Acceptance Criteria）

1. **AC-1**：调用命令移动单个文件后，物理文件存在于目标子目录，DB 中 `file_path` 字段已更新为新路径。
2. **AC-2**：调用命令移动 2 个文件，两个均成功移动，DB 均更新。
3. **AC-3**：模拟第 2 个文件移动失败（目标路径文件名冲突/只读等），第 1 个已移动的文件被回滚到原路径，DB 不更新，命令返回 `Err`。
4. **AC-4**：传入 `target_relative_path = "../../etc/passwd"`（越界路径），命令返回 `Err`，不产生任何文件操作。
5. **AC-5**：传入 `target_relative_path = "__ROOT__"`，文件被移动到 workspace 根目录，DB 更新为根目录下的新路径。
6. **AC-6**：Rust 单元测试覆盖：正常移动（使用 tempdir）、越界路径拒绝、回滚逻辑（使用 tempdir 模拟中途失败）。

## 技术约束

### Rust 函数签名
```rust
#[tauri::command]
pub fn move_asset_to_workspace_folder(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_relative_path: String,
    project_id: String,
) -> Result<(), String>
```

### 原子性实现规格

```
1. workspace_root = workspace::project_workspace_dir(&project_id)?
2. 构建 target_dir:
   if target_relative_path == "__ROOT__" → workspace_root.clone()
   else → workspace_root.join(&target_relative_path)
3. 若 target_dir 不存在，fs::create_dir_all(&target_dir)?
4. canonical_target = target_dir.canonicalize()?
5. canonical_root = workspace_root.canonicalize()?
6. 断言 canonical_target.starts_with(&canonical_root)
   否则 return Err("路径越界: ...")
7. 获取数据库连接
8. 收集 (asset_id, src_path, dest_path) 三元组列表:
   - dest_path = canonical_target / filename（若同名则 unique_path 避免冲突）
9. 执行 rename 循环，维护 moved: Vec<(PathBuf, PathBuf)>:
   - 每次 rename 失败时：reverse rename 所有 moved 中的记录，return Err
10. 所有 rename 成功后，统一更新 DB:
    for each (asset_id, _, dest) in triplets:
        db::asset::update_name_and_path(&conn, &asset_id, &filename, &dest.to_string_lossy())?
11. return Ok(())
```

**不查询、不操作任何关联文件**（不读 sourceAssetId，不追加关联文件到 asset_ids）。

### TS 包装（追加到 `src/lib/tauri-commands.ts`）
```typescript
export async function moveAssetToWorkspaceFolder(
  assetIds: string[],
  targetRelativePath: string,
  projectId: string
): Promise<void> {
  return invoke<void>("move_asset_to_workspace_folder", {
    assetIds,
    targetRelativePath,
    projectId,
  });
}
```

### lib.rs 注册
在 `invoke_handler![]` 宏内追加：`commands::asset::move_asset_to_workspace_folder`

## 参考文件

- `src-tauri/src/commands/asset.rs`（L139-182 `move_assets` 实现，参考 rename + DB 更新模式）
- `src-tauri/src/workspace.rs`（`project_workspace_dir` 接口）
- `src-tauri/src/db/asset.rs`（`update_name_and_path` L161-173）
- `src-tauri/src/lib.rs`（`invoke_handler![]` 注册位置）
- `src/lib/tauri-commands.ts`（L94 `revealProjectWorkspaceFolder`、L158 `moveAssets` 参考封装形式）
- Architect output.md §2.4

## 预估影响范围

- 新建文件：无
- 修改文件：
  - `src-tauri/src/commands/asset.rs`（追加约 60 行函数 + 单元测试约 40 行）
  - `src-tauri/src/lib.rs`（+1 行注册）
  - `src/lib/tauri-commands.ts`（+6 行 TS 包装）
