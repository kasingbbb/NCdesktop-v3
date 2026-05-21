# Task 005 — Dev E3 Output（YAML frontmatter F-6）

## 状态
DONE — 与 task_003 同步落盘。

## 实现
`build_frontmatter(source_id, version, extractor_type, quality_level)` 返回：
```
---
source_asset_id: <uuid>
derivative_version: <N+1>
extracted_at: <RFC3339>
extractor_type: <name>
quality_level: <int>
---

<body>
```
每次 `write_derivative_md` 都注入；占位符路径使用 `extractor_type = "placeholder_<code>"` 与 `quality_level = 0`。

## F-5 回归
- `db::tag::propagate_tags_to_derivative` 在 `write_derivative_md` 首次创建与覆写路径都被调用（既有逻辑保留）
- `sync_tags_to_canonical_derivatives_updates_existing_markdown_assets` 单测已通过

## 验收
- ✅ 所有派生 .md 都带 frontmatter
- ✅ 占位 .md 与正常 .md frontmatter 格式一致（前端/编辑器可统一解析）
- 🟡 实际前端渲染回归（task_008）
