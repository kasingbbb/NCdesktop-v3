# Task 输入 — task_009_fts_content_search

## 目标
将 `extracted_content.raw_text` 接入 FTS5 全文搜索，扩展现有 `search_all` 函数以包含提取内容的搜索结果。

## 前置条件
- 依赖 task：task_002（fts_content 表和触发器已创建）
- 必须先存在的文件/接口：`db/extraction.rs`、`db/search.rs`

## 验收标准（Acceptance Criteria）
1. AC-1：`db/search.rs` 新增 `search_content` 函数，搜索 `fts_content` 并返回 `SearchHit`
2. AC-2：`search_content` 结果的 `hit_type` 为 `"content"`，`snippet` 截取匹配上下文（前后各 60 字符）
3. AC-3：`search_all` 函数扩展为同时搜索 assets + notes + content，合并排序
4. AC-4：搜索结果可溯源到原始素材（`asset_id` 字段不为空）
5. AC-5：前端 `SearchHit` TypeScript 类型更新，支持 `content` hit_type
6. AC-6：搜索面板 UI 正确渲染内容类搜索结果（显示素材名 + 匹配片段）

## 技术约束
- FTS5 MATCH 查询使用参数化，防注入
- snippet 使用 FTS5 内置 `snippet()` 函数或手动截取
- 搜索性能目标：< 200ms（10,000 条记录）

## 参考文件
- `src-tauri/src/db/search.rs` — 现有搜索实现
- `src-tauri/src/commands/search.rs` — 搜索 IPC 命令
- PRD §3.2 F05 — FTS5 全文索引

## 预估影响范围
- 修改文件：`src-tauri/src/db/search.rs`（新增 search_content + 扩展 search_all）
- 修改文件：前端搜索相关类型和组件（如有 SearchHit 类型定义）
