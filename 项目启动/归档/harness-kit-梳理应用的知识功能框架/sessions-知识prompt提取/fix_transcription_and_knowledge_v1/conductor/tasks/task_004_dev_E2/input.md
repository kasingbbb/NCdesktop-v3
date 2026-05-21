# Task 004 — Dev E2（派生版本化 F-3 / F-4）

与 task_003 同一代码路径（`scheduler.rs::write_derivative_md`），随 E1 一并实现。

## 验收
- 覆写派生 .md 前归档旧版本到 `_versions/<source_asset_id>/v{N}.md`
- `assets.derivative_version` 每次覆写 +1（source 与 derivative 同步）
- 重抽取不删除任何历史版本（底线 2）
