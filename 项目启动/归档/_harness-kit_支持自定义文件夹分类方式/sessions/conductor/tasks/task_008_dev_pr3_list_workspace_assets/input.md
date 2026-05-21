# Task 输入 — task_008_dev_pr3_list_workspace_assets

## 目标
新建 Tauri command `list_workspace_assets(project_id, category_slug?, sub_path?, cursor?, page_size?) -> { items: AssetView[], next_cursor: Option<String> }`；DB 权威源 + cursor 分页（ADR-003）。

## 前置条件
- 依赖 task：task_004（PR-1 完成）
- 必须先存在的文件/接口：`assets.category_slug` 列、`idx_assets_proj_cat_updated`

## 验收标准（AC）
1. 新建 `src-tauri/src/commands/workspace_assets.rs`，实现命令
2. `AssetView` 结构含：id、name、category_slug、tags、size_bytes、mime、updated_at、relative_path、icon_hint
3. cursor 编码 `(updated_at_unix:u64, id:i64)` → base64
4. 默认 `page_size=200`，最大 500；越界报错
5. `category_slug` 为空时返回项目全资产；`sub_path` 为相对路径过滤
6. 走 `idx_assets_proj_cat_updated` 索引；`EXPLAIN QUERY PLAN` 必须含该索引
7. 命令注册于 `lib.rs`；`tauri-commands.ts` 封装
8. 单测：(a) 分页连续性 (b) 并发 insert 不重复 (c) 索引命中

## 技术约束
- `Result<T, String>` 中文错误
- 无 N+1：tags 通过 `ai_analyses` JOIN 一次取出

## 参考文件
- task_001 output.md §API 设计 + ADR-003
- `src-tauri/src/commands/workspace_folders.rs`（参考既有命令风格）

## 预估影响范围
- 新建：`commands/workspace_assets.rs`（~250）
- 修改：`lib.rs`、`src/lib/tauri-commands.ts`
- 测试：`src-tauri/tests/list_workspace_assets.rs`（~80）

## Reviewer 重点关注
- cursor base64 编码可逆性
- 跨页 boundary 的稳定排序
- 索引未命中场景
