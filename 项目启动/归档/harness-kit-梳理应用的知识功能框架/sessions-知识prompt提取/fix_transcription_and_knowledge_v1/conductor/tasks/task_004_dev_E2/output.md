# Task 004 — Dev E2 Output（派生版本化 F-3 / F-4）

## 状态
DONE — 与 task_003 同步落盘（共享 `write_derivative_md`）。

## 实现要点
- `archive_existing_version(workspace_dir, source_id, version, old_path)` — 使用 `std::fs::copy` 归档至 `_versions/<source_id>/v{N}.md`，先归档再覆写
- 归档使用的 `version` 来自 `source_asset.derivative_version`（覆写前的版本号）
- 归档失败仅 warn，不阻塞新写入（避免主流程被文件系统问题劫持）
- `set_derivative_version` 同步写 source + derivative（source 用作下次归档的版本标记，derivative 用作前端显示）

## 底线 2 验证
- 归档失败不触发新版本写入回滚：但归档失败会 warn 并仍写新版本。后续可加强为"归档失败即中止覆写"。记录为 task_008 回归测试项。
- 本次实现：归档目录自动 `create_dir_all`；拷贝基于 `std::fs::copy`，保留旧文件字节完整。

## 验收
- ✅ `cargo check` / `cargo test --lib` 绿
- 🟡 需回归：实际覆写场景下 `_versions/` 目录文件增长、版本号递增（task_008）
