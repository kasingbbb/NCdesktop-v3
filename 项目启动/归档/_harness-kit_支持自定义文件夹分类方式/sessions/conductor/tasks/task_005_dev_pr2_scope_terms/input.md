# Task 输入 — task_005_dev_pr2_scope_terms

## 目标
在 Rust 后端引入 `ProjectFolderRoot`（磁盘）/ `ProjectFolderScope`（导入上下文谓词）类型；`workspace.rs::assert_scope` 在 dropzone 入口断言；不改动外部命令签名（前端零感知）。

## 前置条件
- 依赖 task：task_004（PR-1 完成）
- 必须先存在的文件/接口：`workspace.rs`、`commands/dropzone.rs::resolve_import_project_id`

## 验收标准（AC）
1. 新增类型 `ProjectFolderRoot(PathBuf)`、`ProjectFolderScope { project_id: String, relative_path: Option<String> }`，`workspace.rs` 内
2. `assert_scope(scope: &ProjectFolderScope) -> Result<ProjectFolderRoot, String>`：校验 project 存在、relative_path 不含 `..`、不跨项目
3. `resolve_import_project_id` 重构内部使用上述类型；外部命令签名不变
4. 写盘前在 `workspace.rs::write_under_root` 再次断言路径在 `ProjectFolderRoot` 下（双重防御）
5. 单测：(a) 跨项目 path 拒绝 (b) `..` 拒绝 (c) 正常 path 通过 (d) 软链接陷阱

## 技术约束
- 类型仅在 Rust 内部使用，前端无感
- `assert_scope` 错误返回中文友好

## 参考文件
- `项目启动/NCdesktop/src-tauri/src/workspace.rs`
- `项目启动/NCdesktop/src-tauri/src/commands/dropzone.rs::resolve_import_project_id`
- task_001 output.md §安全考量

## 预估影响范围
- 修改：`workspace.rs`（+150）、`commands/dropzone.rs`（+50）
- 测试：`src-tauri/tests/scope.rs`（新 ~80）

## Reviewer 重点关注
- 软链接 / mount point 越界
- 路径规范化是否一致（`canonicalize` vs `components`）
