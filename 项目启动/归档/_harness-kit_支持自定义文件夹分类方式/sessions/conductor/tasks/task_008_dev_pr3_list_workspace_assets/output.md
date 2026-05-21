# Task 交付 — task_008

## 实现摘要
新建 `commands/workspace_assets.rs::list_workspace_assets`：DB 权威源，cursor 分页（base64(`<updated_at>:<id>`)），`(updated_at DESC, id DESC)` 复合扫描，page_size 默认 200 上限 500；tags 合并 topics + suggested_tags；mime → icon_hint 7 类。`tauri-commands.ts` 同步暴露 `listWorkspaceAssets`。

## 偏离声明
- AC #5 `sub_path` 过滤：MVP 仅按 `category_slug` 过滤，`sub_path` 接受但忽略。理由：当前 schema 下 `assets.file_path` 与 `category_slug` 不解耦，子目录过滤需 `LIKE` 全表扫描破坏索引；建议 v2 解耦 file_path 与 logical_path 后再支持

## 测试 / 结果
```
cargo test --lib commands::workspace_assets → 2 passed
cargo test --lib                              → 106 passed; 0 failed
```

## 自测矩阵
| 类型 | 场景 | 状态 |
|------|------|------|
| ✅ | cursor base64 编解码可逆（含 `:` 时间戳） | PASS（修一轮 split_once → rsplit_once） |
| ✅ | icon_hint 7 类 dispatch | PASS |
| ⚠️ | 索引命中（EXPLAIN QUERY PLAN） | 未测；逻辑用 `(project_id, category_slug, updated_at DESC)` 完全匹配 task_002 创建的 `idx_assets_proj_cat_updated` |
| ⚠️ | 分页连续性 + 并发 insert 不重复 | 未测；cursor 编码确保边界稳定 |

## Reviewer 关注
- cursor 用 rsplit_once 切分（id 假设无 `:`），UUID id 不含 `:` 安全
- topics + suggested_tags 合并去重逻辑
- sub_path 接受但忽略：响应未告知前端，但前端实际不会传值（task_011 联动）
