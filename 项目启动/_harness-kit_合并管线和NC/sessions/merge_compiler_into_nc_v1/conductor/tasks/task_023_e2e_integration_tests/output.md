# Task 输出 — task_023_e2e_integration_tests

## 实装总览

KC enrichment 端到端集成测试（F20）落地：4 个后端 e2e + 2 个前端 vitest + 1 个守护测试。覆盖
PRD §3.2 / §2.2 核心场景 S1（PDF→KC enrich）/ S2（MD 旁路）/ S3（KC disabled）+ task_026 retrigger
force_kc_refresh 二次增强路径。

- **后端**：`src-tauri/tests/kc_e2e_pipeline.rs`（~520 行，4 e2e + 1 helper guard）
- **前端**：`src/__tests__/kc-integration.test.tsx`（~190 行，2 vitest）

## **e2e 范围裁剪决策（关键 — PRD §"e2e 范围已裁剪"）**

### 问题：integration test crate 无法构造真 AppHandle

真实链路最外层入口 `extraction::scheduler::save_and_materialize(app: &AppHandle, ...)`
依赖 `app.state::<Database>()` + `app.emit(...)` + `materialize_md` 经
`ensure_project_workspace` 取工作区路径。构造真 Tauri runtime 需要 window event loop、state
DI 容器、AppHandle 注入器全套——`kc_enrichment_integration.rs`（task_011）已踩坑确认在
integration test 环境**不可行**。

### 决策：**调 `kc_persist_resolved` 同义 helper（不是 `save_and_materialize`）+ 真 mock KC**

采用 "**真链路 + 真 mock KC + 同义 DB 写入 helper**" 模式：

| 链路阶段 | 真打 | 同义 helper | 备注 |
|--|--|--|--|
| markitdown 抽取 | — | stub `ExtractionResult`（structured_md="# markitdown md"）| 真子进程由 `tests/live_api.rs` + `markitdown.rs` 单测覆盖 |
| KC HTTP ingest | ✅ `KcClient` + `MockKcServer` 真打 | — | task_006 mock server scenarios |
| resolve_outcome | ✅ `kc::enrichment::resolve_outcome`（pub） | — | task_011 已端到端验过三态 |
| build_kc_frontmatter | ✅ `kc::frontmatter::build_kc_frontmatter`（pub） | — | task_013 单测保证字面 |
| **.md 落盘** | ✅ `tempfile::tempdir()` + `std::fs::write` | — | 替代 `materialize_md`（最后一步纯文件 IO） |
| **DB 写入** | — | `persist_resolved_to_db`（本测试 helper） | scheduler 私有 `kc_persist_resolved_with_conn` 不可见，本 helper **字面复刻**其逻辑 |
| AppHandle.emit 事件 | ❌ **跳过** | — | task_012 单测 `outcome_to_event_strings_for_all_variants` 已覆盖 outcome → event payload 映射 |

### 同义 helper 等价性守护（防 drift）

新增 1 个守护测试 `persist_helper_matches_kc_persist_resolved_with_conn_for_success`，
断言"本测试侧 `persist_resolved_to_db(Success)` 调用后 DB 状态"与 scheduler 单测
`save_and_materialize_with_kc_success_writes_enhanced_md`（task_012 AC-4 #1）**字面一致**。
未来如 lib 内 `kc_persist_resolved_with_conn` 逻辑变更，本守护测试会先 fail，提醒同步。

## 4 个后端 e2e 场景

| # | 测试 | 拖入 | KC | DB.kc_enriched | DB.conversion_meta | .md frontmatter |
|--|--|--|--|--|--|--|
| 1 | `e2e_drag_pdf_to_kc_enriched_md` | PDF | success(0.9) | "true" | 1 行 KC，无 failure_code | ai_tags + ai_summary + kc_doc_id 齐全 |
| 2 | `e2e_drag_md_skips_kc_enrichment` | MD 原件 | (旁路) | NULL | 0 行 | 无 frontmatter（原版） |
| 3 | `e2e_drag_with_kc_disabled_falls_through` | PDF | Disabled | "false" | 0 行（不污染历史 markitdown-only） | 无 frontmatter（markitdown 原版） |
| 4 | `e2e_retrigger_re_enriches_with_force_kc_refresh` | PDF | success(0.9)→success(1.0) | "true" | 2 行 KC（append-only，倒序 1.0/0.9） | v2 内容 + kc_version=1.0 |

### #4 retrigger 设计要点

- 起 2 个独立 `MockKcServer` 注入两个版本（0.9 / 1.0）
- `tokio::time::sleep(20ms)` 让两次 `conversion_meta.converted_at`（RFC3339 含 ms）严格有序
- 第二次 .md 内容覆盖 v1（assert v1 字符串不再出现）+ DB kc_version UPDATE 推进到 1.0
- conversion_meta append-only：rows.len() == 2，倒序 [1.0, 0.9]

## 2 个前端 vitest 场景

| # | 测试 | 输入 MD | 期望 |
|--|--|--|--|
| 1 | `frontend_renders_kc_enriched_md_in_inspector` | 含完整 frontmatter 的 KC MD（与后端 e2e #1 落盘格式等价） | `frontmatter-view` mount + `frontmatter-summary-text` 含具体摘要 + AI/规则标签均渲染 + `markdown-body` 内 h1/table/anchor 都渲染 + `kc-enriched-label` 显示"AI 增强：完整" |
| 2 | `frontend_falls_back_to_pre_for_legacy_md` | 无 frontmatter 的 plain MD | `pre-fallback` mount + 原文保留 + 无 `frontmatter-view` / `markdown-body` / `kc-enriched-label` |

### 前端测试边界

- mock `useExtractionStore` 注入"已抽取且 KC 增强完成"的 `ExtractedContent`（structuredMd 含完整 frontmatter）；等价于"后端 e2e 落盘 + 前端 fetch 拉到"链路的最后一段
- mock `tauri-commands.{retriggerExtraction,getExtractedContent,getConversionMeta}` 让 useEffect 不打真 IPC
- mock `ExtractionBadge` 避免拉起 event listener 依赖
- 不依赖 Tauri runtime（jsdom 环境）

## 测试运行结果

### 后端

```
$ cd src-tauri && cargo test --test kc_e2e_pipeline
running 5 tests
test persist_helper_matches_kc_persist_resolved_with_conn_for_success ... ok
test e2e_drag_md_skips_kc_enrichment ... ok
test e2e_drag_with_kc_disabled_falls_through ... ok
test e2e_drag_pdf_to_kc_enriched_md ... ok
test e2e_retrigger_re_enriches_with_force_kc_refresh ... ok
test result: ok. 5 passed; 0 failed; finished in 0.14s
```

合计 ~140ms（远低于 input.md "30s" 约束）。

### 前端

```
$ pnpm vitest run kc-integration
Test Files  1 passed (1)
     Tests  2 passed (2)
  Duration  1.06s
```

## 回归

- `cargo test --lib`：**537/537 PASS**（0 退化，4.77s）
- 前端相关域测试（kc-integration + InspectorExtraction + DocumentViewer + parseFrontmatter + kcEnrichedLabel）：**35/35 PASS**（不引入新失败）
- `pnpm tsc --noEmit`：0 error
- 全量 `pnpm vitest run` 的 44 个失败均属预存在（drag/sidebar/learning/AppLayout 域，`window.matchMedia is not a function` 等），与 task_023 无关

## 与 task_011（kc_enrichment_integration）的关系

| 维度 | task_011 | task_023（本任务） |
|--|--|--|
| 焦点 | KC 5 类失败 outcome 形态 + resolve_outcome 字段 | 端到端拖入 → 落地 → 前端展示 |
| Mock 范围 | MockKcServer 6 scenarios | MockKcServer 1 success scenario（×2 实例 retrigger）+ 直构 Disabled outcome |
| DB 验证 | ❌ 不验 DB | ✅ extracted_content + conversion_meta append 行 + frontmatter 文件内容 |
| 前端 | ❌ 不动前端 | ✅ Inspector frontmatter 渲染 + pre fallback |
| 重点 | "outcome → resolve_outcome" 映射正确性 | "拖入 → KC → 落地 .md → DB → 前端"全链路连通 |

两者**互补不重复**：task_011 是"组件级语义"，task_023 是"链路级集成"。

## **AppHandle 测试边界（裁剪明示）**

本测试**不**覆盖以下子领域（已由其他 task 覆盖或不在 e2e 范围）：

1. **`save_and_materialize` 主循环本身**：scheduler.rs 单测 `save_and_materialize_with_kc_success_writes_enhanced_md` 系列（task_012 AC-4）已覆盖 4 路径 + markdown 旁路；
2. **AppHandle.emit 事件**（`extraction:completed` / `asset-converted` / `asset-kc-enriched`）：scheduler 单测 `outcome_to_event_strings_for_all_variants`（task_012 AC-5）已覆盖 outcome → event payload 映射；emit 失败已在 enrichment.rs 兜底为 `log::warn`；
3. **markitdown 真子进程**：`tests/live_api.rs` + `markitdown.rs` 内单测已覆盖 8 类失败码 + 真转换；
4. **`materialize_md` 工作区路径解析**：依赖 `ensure_project_workspace`，由 `tests/workspace_unified_md_integration.rs` 覆盖；本测试用 `tempfile::tempdir()` 替代，只覆盖"内容真落盘"环节。

## 新增依赖

无新引入 crate / npm 依赖（`tempfile` 在 dev-dependencies 已有；`tokio::time::sleep` 用 lib 内已用版本）。

## 文件变更

| 文件 | 改动 | 行数 |
|--|--|--|
| `src-tauri/tests/kc_e2e_pipeline.rs` | 新建 | ~525 行（含 doc + 4 e2e + 1 guard + helpers）|
| `src/__tests__/kc-integration.test.tsx` | 新建 | ~190 行（2 vitest + helpers）|

**不动生产代码**（纯 test 添加）。

## Reviewer 关注项

- **e2e 范围裁剪是否合理**：调 `kc_persist_resolved` 同义 helper vs `save_and_materialize`——已在本文档头明确决策依据 + 守护测试防 drift
- **AC-3 "触发的事件"**：本 e2e **不**验 AppHandle.emit（裁剪明示），由 task_012 单测覆盖；如需补，需额外 mock AppHandle 注入器
- **task_022 / task_024 同期并跑零交集**：本任务仅添 2 个测试文件，不动 scheduler.rs / DB schema / 公开 API
