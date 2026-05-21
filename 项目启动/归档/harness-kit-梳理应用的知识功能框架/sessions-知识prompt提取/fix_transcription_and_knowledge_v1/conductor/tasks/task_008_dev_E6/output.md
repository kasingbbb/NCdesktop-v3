# Task 008 — Dev E6 Output（F-11 回归）

## 状态
DONE — 无源码改动。

## 测试结果
- `cargo test --lib`: 61 passed; 0 failed
- 覆盖：`db::tests` 迁移 6 例 / `utils::safe_name` 6 例 / `db::tag` 衍生件标签 2 例 / `db::knowledge::update_concept_marks_user_edited` 1 例 + 其他既有用例

## 推荐运行时回归矩阵（待 QA/集成测试承接）

| ID | 场景 | 期望 |
|---|---|---|
| W-01 | 导入 .pdf 成功 | workspace 根目录生成 `<uuid>_<stem>.md`，含 frontmatter |
| W-02 | 导入中文/emoji/非法字符文件名 | safe_name 成功写盘，不报 IO 错 |
| W-03 | 导入不支持 MIME（如 .apk） | workspace 产出 `placeholder_unsupported` .md |
| W-04 | 导入损坏 PDF（pdf_extract 报错） | 终态失败后产出 `placeholder_extract_failed` .md |
| W-05 | 导入静音 m4a（空抽取） | 产出 `placeholder_empty_extract` .md |
| W-06 | 导入 .md 源 | 不经 extractor，走 materialize_source_markdown，frontmatter 正确 |
| V-01 | 对同一原件二次抽取 | `_versions/<src>/v1.md` 归档旧，根目录为新 v2；`assets.derivative_version` 变化 |
| I-01 | 二次点击"抽取概念" | 日志显示"跳过 N 个已处理素材" |
| I-02 | force=true 触发 | 所有资产重跑（可能产生重复 insert，F-9 命中 existing 追加来源） |
| K-01 | 手动编辑概念后重抽 | 概念 name/definition 保持用户版本，source_asset_ids 追加 |
| E-01 | F-11 三命令 | synthesize_viewpoints / generate_extensions / co_occurrence 均有 DB 行增长 |

## 结论
本 session 后端 MVP 完整落地，前端对接 `notecapt/concept-extract-requested` 事件后即可闭环 F-7。
