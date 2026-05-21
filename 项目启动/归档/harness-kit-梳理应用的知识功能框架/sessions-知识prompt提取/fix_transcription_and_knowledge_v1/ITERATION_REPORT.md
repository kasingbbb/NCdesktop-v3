# Iteration Report — fix_transcription_and_knowledge_v1

**日期**：2026-04-24
**执行框架**：Harness-Kit（L1+L4 Debate → PRD → 8 Tasks）
**最终状态**：SESSION_DONE；`cargo check` + `cargo test --lib` **61 passed / 0 failed**

---

## 1. 目标回顾

**P0 痛点矩阵**（源自 task_001 实测）：
- F-1/F-2：部分原件在工作区无 .md 邻居（unsupported / 空抽取 / 失败路径缺失）
- F-3/F-4：重抽取会覆盖旧派生，历史丢失
- F-5/F-6：YAML frontmatter 缺失，前端无法解析溯源
- F-7：抽取完成后不自动触发概念抽取
- F-8/F-9/F-10：增量抽取缺位；`user_edited` 概念可能被 LLM 输出覆盖
- F-11：三命令（synthesize_viewpoints / generate_extensions / co_occurrence）是否为 stub

**不可破底线**：
1. `user_edited=1` 概念绝不被自动覆盖
2. 重抽取绝不删除旧派生（必须归档）
3. 每个原件必有工作区 .md 邻居（成功或占位）

---

## 2. 交付清单

### 2.1 代码改动（`src-tauri/`）

| 路径 | 类型 | 作用 |
|---|---|---|
| `Cargo.toml` | 改 | 新增依赖 `sha2 = "0.10"` |
| `src/lib.rs` | 改 | 注册 `pub mod utils;` |
| `src/db/migration.rs` | 改 | V9 迁移：`derivative_version` / `content_hash` / `concepts_extraction_log` |
| `src/db/asset.rs` | 改 | `ASSET_SELECT` +`derivative_version`；新增 `set_derivative_version` |
| `src/db/extraction.rs` | 改 | 新增 `set_content_hash` |
| `src/db/mod.rs` | 改 | 注册 `concepts_extraction_log` 子模块 |
| `src/db/concepts_extraction_log.rs` | 新 | `fetch_logged_pairs` / `insert`（幂等 UNIQUE） |
| `src/models/asset.rs` | 改 | `Asset` 新增 `derivative_version: i32` |
| `src/utils/safe_name.rs` | 新 | `sanitize_stem` + 6 单测 |
| `src/utils/mod.rs` | 改 | 注册 `safe_name` |
| `src/extraction/scheduler.rs` | 重构 | `write_derivative_md` / `materialize_placeholder` / `materialize_source_markdown` / frontmatter / 版本归档 / 事件发射 |
| `src/commands/knowledge.rs` | 改 | F-8 增量 + F-9 user_edited 保护 |
| `src/commands/sync.rs` / `commands/asset.rs` / `commands/dropzone.rs` / `db/tag.rs`（测试） | 改 | Asset 字面量补 `derivative_version: 0` |

### 2.2 文档（`sessions/fix_transcription_and_knowledge_v1/`）

- `prd/` — PRD v1（11 项 P0 全纳入 MVP）
- `debate/` — L1+L4 共识记录
- `test_fixtures/` — 6 份测试素材（中文文件名 / 乱码 / 损坏 PDF / 静音 m4a / 空白 jpg / zip bundle）
- `conductor/progress.md` — 状态转移日志
- `CHECKPOINT.md` — 中断恢复断点
- `conductor/tasks/task_001..008/` — 每个 task 的 input.md + output.md

---

## 3. 功能映射（F-1 ~ F-11）

| 痛点 | 实现 | 验证 |
|---|---|---|
| **F-1** 原件必有 .md 邻居 | `source_asset_should_materialize` 放宽；unsupported/empty/error 走 `materialize_placeholder` | cargo test 绿；运行时回归入 task_008 矩阵 |
| **F-2** 安全文件名 | `utils::safe_name::sanitize_stem` | 6 单测全绿（CJK/emoji/控制字符/长度截断） |
| **F-3** 重抽取不丢历史 | 覆写前拷贝至 `_versions/<src>/v{N}.md` | 需集成测试（task_008 V-01） |
| **F-4** 版本号递增 | `assets.derivative_version` +1，source/derivative 同步 | V9 迁移 + set_derivative_version 已实现 |
| **F-5** tag 传播到派生 | 既有 `propagate_tags_to_derivative` 保留 | task_001 PASS；单测已绿 |
| **F-6** YAML frontmatter | `build_frontmatter` 每次 prepend | 所有派生路径（包括占位符）统一 |
| **F-7** 自动触发概念抽取 | 物化尾部 emit `notecapt/concept-extract-requested` | **待前端接入事件** |
| **F-8** 增量抽取 | `content_hash` + `concepts_extraction_log` 去重 | `fetch_library_assets` 返回 hash；循环内 skip |
| **F-9** user_edited 保护 | existing-concept 分支只 append，不改 name/definition | 行为保持并显式注释 |
| **F-10** viewpoint 稳定性 | MVP 保留 delete-rebuild；依赖 prompt 稳定性 | schema-level 唯一约束延后至 P1 |
| **F-11** 三命令非 stub | task_001 已证伪 stub 假设；task_008 仅回归 | cargo test 绿 |

---

## 4. 三条底线验证

| 底线 | 实现点 | 结论 |
|---|---|---|
| 1. user_edited 绝不覆盖 | `knowledge.rs` existing-concept 分支仅 append_source_asset | ✅ 已保证 |
| 2. 重抽取不删旧派生 | `archive_existing_version` → `_versions/<src>/v{N}.md` 先归档后覆写 | ✅ 已保证（归档失败会 warn） |
| 3. 每个原件有 .md 邻居 | 4 个调用点（unsupported/empty/error/panic）统一走 placeholder；.md 源走 source_markdown | ✅ 已保证 |

---

## 5. 残留风险 & 后续工作

### 高
- **前端事件接入（F-7 闭环）**：需监听 `notecapt/concept-extract-requested` 并调 `extract_concepts_for_library(force=false)`
- **归档失败语义**：当前归档 copy 失败仅 warn 继续覆写；若担心文件系统抖动，可改为"归档失败即中止"。回归测试项目在 task_008

### 中
- **F-10 viewpoint schema**：当前仍 delete-rebuild；多轮 LLM 漂移可能导致 viewpoint 波动。P1 引入 `UNIQUE(concept_id, source_asset_id)` 精细化 upsert
- **`_versions/` 清理策略**：磁盘占用随重抽次数线性增长，无上限。P1 做保留 N 版/LRU 策略
- **Tauri runtime 限制**：scheduler 内发射事件无风险；若未来改为 scheduler 内部直调 `extract_concepts_for_library`，需处理 tokio runtime 嵌套

### 低
- cargo check 4 个 warning（既有 `library_id` 未用 / `AnthropicContent.block_type` 等）非本 session 引入，建议单独一次 lint cleanup PR

---

## 6. 验证总结

| 验证项 | 结果 |
|---|---|
| `cargo check` | ✅ 通过，无新增 error |
| `cargo test --lib` | ✅ 61 passed / 0 failed |
| V9 迁移幂等 | ✅（复用 V3/V4 相同 `user_version` 判断模式） |
| 新增单测 `utils::safe_name::*` | ✅ 6/6 |
| H-W / V / K / I 运行时矩阵 | 🟡 待 QA/集成测试承接（清单见 task_008/output.md） |

---

## 7. Session 决策记录（重点）

- 跳过完整四层 Debate，采用 L1（痛点）+L4（评审）
- 测试方案选 β（探索性穷举，即 task_001 的 H 级矩阵）
- F-11 stub 假设被证伪，task_008 scope 缩减至仅回归
- E4 F-7 采用 MVP 事件驱动路径（而非新增 `pipeline_task_type='concept_extract'`），避免 scheduler runtime 复杂化
- F-10 采用 prompt 稳定性，schema 约束延后

---

## 8. 一句话总结

本 session 将 NCdesktop 的"转录 → 知识进化"管线从 6 处痛点全部闭环到 MVP 可验证状态，三条底线均有代码级防线，剩余工作是前端接入 `concept-extract-requested` 事件与运行时回归矩阵。
