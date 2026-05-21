# task_005_rust_move_command — 输出报告

## 实现摘要

在 `src-tauri/src/commands/asset.rs` 追加了 `move_asset_to_workspace_folder` Rust 命令，在 `src-tauri/src/lib.rs` 的 `invoke_handler![]` 中注册，在 `src/lib/tauri-commands.ts` 追加了 TS 包装函数。该命令将多个素材移动到当前项目 workspace 内的指定子目录，支持路径安全校验、rename 回滚和 DB 原子更新。

## 修改的文件

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `src-tauri/src/commands/asset.rs` | 追加函数 + import 调整 | 追加 `move_asset_to_workspace_folder`；`use std::path::Path` 扩展为 `use std::path::{Path, PathBuf}` |
| `src-tauri/src/lib.rs` | 注册命令 | 在 `move_assets` / `copy_assets` 后追加 `commands::asset::move_asset_to_workspace_folder` |
| `src/lib/tauri-commands.ts` | 追加函数 | 在 `getFileContent` 后追加 `moveAssetToWorkspaceFolder` 包装函数 |

## 架构遵守声明

- 不查询、不操作任何 `source_asset_id` 关联文件
- DB 更新在所有 `rename` 成功后统一执行（第二个 for 循环）
- `__ROOT__` 映射到 `workspace_root.clone()`
- 错误以 `Result<(), String>` 返回，无 `panic!`
- 复用已有 `unique_path` 辅助函数

## cargo check 输出

```
warning: unused import: `Deserialize`
  --> src/commands/knowledge_unit_learning.rs:19:13
  （预存警告，与本次改动无关）

warning: unused variable: `library_id`
  （预存警告，calendar.rs，与本次改动无关）

warning: fields `block_type` and `thinking` are never read
  （预存警告，llm/chat.rs，与本次改动无关）

warning: `notecapt` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.85s
```

**结论：0 编译错误，4 条预存警告均与本次改动无关。**

## 自测验证矩阵

| # | 验收标准 | 满足？ | 说明 |
|---|----------|--------|------|
| 1 | 函数签名与规格完全一致 | ✓ | 参数名、类型、返回值 `Result<(), String>` 均一致 |
| 2 | 越界路径（如 `../../etc`）返回 Err，不产生文件操作 | ✓ | `canonicalize` + `starts_with` 校验在 rename 之前执行 |
| 3 | `__ROOT__` 映射到 workspace 根目录 | ✓ | `if target_relative_path == "__ROOT__" { workspace_root.clone() }` |
| 4 | rename 逐一回滚逻辑存在 | ✓ | `moved` Vec 维护已移动记录，失败时 `.iter().rev()` 逐一回滚 |
| 5 | DB 更新在所有 rename 成功后统一执行 | ✓ | 两阶段设计：第一 for 循环只做 rename，第二 for 循环统一调用 `update_name_and_path` |
| 6 | lib.rs 已注册新命令 | ✓ | `commands::asset::move_asset_to_workspace_folder` 已插入 invoke_handler |
| 7 | tauri-commands.ts 已追加包装函数 | ✓ | `moveAssetToWorkspaceFolder` 已追加，参数与 Rust 端对应 |
| 8 | `cargo check` 无编译错误 | ✓ | Finished dev profile，0 errors |
