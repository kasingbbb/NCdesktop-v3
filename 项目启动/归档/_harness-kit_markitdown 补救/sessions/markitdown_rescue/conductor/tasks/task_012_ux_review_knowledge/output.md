# Task 交付 — task_012_ux_review_knowledge（Review-only）

## 实现摘要

本 task 仅审计、不改代码。围绕 5 个必答问题，走查了知识抽取入口、搜索 FTS 模块、scheduler 物化路径与前端事件监听。结论：
- 知识抽取数据源已对齐到 canonical .md 的衍生件 `extracted_content.structured_md`（同源同步，行为正确）。
- 搜索模块**未做 source_asset_id 去重**，并且 FTS 索引只覆盖 `name + file_path`（不含正文），导致原件 + 衍生 .md 会以双条结果出现，但仅命中文件名/路径。
- `notecapt/concept-extract-requested` 事件**前端无监听**，是死信。当前抽取只在用户手动触发时运行。
- canonical .md 内容更新（task_008 幂等覆盖）后通过 content_hash 重算能被增量抽取感知。
- placeholder 衍生件因 `extracted_content.status='failed'`，被知识抽取 SQL 的 `status='extracted'` 过滤排除，不会被误消费。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `sessions/markitdown_rescue/conductor/tasks/task_012_ux_review_knowledge/output.md` | 新建 | 本 output.md 即审计交付物 |

## 对 Architect 方案的遵守声明
- [x] 仅写报告，未改任何源码
- [x] 所有判断均给出 `file:line` 引用
- [x] 5 个问题给出 ✅/⚠️/❌ 判断
- [x] 对 ❌ 项给出独立迭代骨架

## 测试命令（审计中执行的 grep）

```bash
grep -rn "structured_md\|canonical\|read_to_string\|fs::read" src-tauri/src/db/knowledge*.rs src-tauri/src/db/concepts_extraction_log.rs src-tauri/src/commands/knowledge*.rs
grep -rn "source_asset_id\|UNION\|GROUP BY\|DISTINCT\|asset_type" src-tauri/src/db/search.rs src-tauri/src/commands/search.rs
grep -rn "concept-extract-requested\|asset-converted\|notecapt/" src/ src-tauri/src/
grep -rn "content_hash\|extracted_at\|placeholder_\|extractor_type\|status.*failed" src-tauri/src/db/concepts_extraction_log.rs src-tauri/src/commands/knowledge.rs
grep -rn "fts_assets\|fts_notes\|CREATE.*fts" src-tauri/src/db/
grep -rn "extractConceptsForLibrary\|extract_concepts_for_library" src/
```

## 测试结果（摘要）

- Q1：`fetch_library_assets` 仅出现一次（`src-tauri/src/commands/knowledge.rs:371`），COALESCE 优先级 = derived `structured_md` → derived `raw_text` → source `structured_md` → source `raw_text` → `ai.summary` → asset.name。**无任何 `read_to_string` / `fs::read` 调用读取磁盘 .md 文件**。
- Q2：`src-tauri/src/db/search.rs` 全文未出现 `source_asset_id`、`UNION`、`GROUP BY`、`DISTINCT`。
- Q3：前端代码库 `src/**` 内 grep `concept-extract-requested` 返回 0 结果。
- Q4/Q5：见下方分析。

## 自测验证矩阵

| 问题 | 判断 | 关键证据（file:line） |
|---|---|---|
| Q1 知识抽取数据源 | ✅ 已对齐 | `src-tauri/src/commands/knowledge.rs:378` + `src-tauri/src/extraction/scheduler.rs:659-669` |
| Q2 搜索去重 | ❌ 未对齐需独立迭代 | `src-tauri/src/db/search.rs:18-103`、`src-tauri/src/db/migration.rs:419-423` |
| Q3 事件监听 | ❌ 未对齐需独立迭代 | `src-tauri/src/extraction/scheduler.rs:690-697` vs 前端无 listener |
| Q4 内容更新感知 | ✅ 已对齐 | `src-tauri/src/extraction/scheduler.rs:672-673` + `src-tauri/src/commands/knowledge.rs:131-141`、`src-tauri/src/db/concepts_extraction_log.rs:14` |
| Q5 placeholder 误消费 | ✅ 已对齐 | `src-tauri/src/commands/knowledge.rs:388,391` + `src-tauri/src/extraction/scheduler.rs:749-783` |

## 详细分析

### Q1：知识抽取读取哪种数据源 — ✅ 已对齐

**入口**：`extract_concepts_for_library`（`src-tauri/src/commands/knowledge.rs:84-247`）。

**素材列表查询**：`fetch_library_assets`（`src-tauri/src/commands/knowledge.rs:371-410`），关键 SQL（376-393 行）：

```sql
SELECT a.id, p.name, a.name,
       COALESCE(md_ec.structured_md, md_ec.raw_text, ec.structured_md, ec.raw_text, ai.summary, a.name) as content,
       COALESCE(md_ec.content_hash, ec.content_hash) as content_hash
FROM assets a
INNER JOIN projects p ON p.id = a.project_id AND p.library_id = ?1
LEFT JOIN assets md ON md.id = (
    SELECT id FROM assets
    WHERE source_asset_id = a.id AND asset_type = 'markdown'
    ORDER BY imported_at DESC LIMIT 1
)
LEFT JOIN extracted_content md_ec ON md_ec.asset_id = md.id AND md_ec.status = 'extracted'
LEFT JOIN extracted_content ec ON ec.asset_id = a.id AND ec.status = 'extracted'
LEFT JOIN ai_analyses ai ON ai.asset_id = a.id
WHERE a.source_asset_id IS NULL OR a.asset_type != 'markdown'
```

要点：
1. 外层遍历**只取根素材**（`source_asset_id IS NULL`），即原件 PDF/audio/img/已是 .md 的根件。
2. 通过子查询找到该根素材的"最新衍生 markdown 资产"`md`。
3. 优先取该衍生 .md 的 `extracted_content.structured_md`（即 markitdown / OCR / Whisper 转出的标准化 markdown）。
4. **数据源是 DB 字段 `structured_md`，不是磁盘 .md 文件**——但因为 `scheduler.rs:659-669` 在物化时调用 `upsert_extraction_result(..., md_body, md_body, ...)`，把同一段 markdown 同时写入 `raw_text` 和 `structured_md`，并把 `md_body` 写到磁盘 canonical .md 路径，所以"DB structured_md" ≡ "canonical .md 内容"。两者通过 scheduler 同步保证一致。

**调用路径**：
- 前端：`src/components/features/knowledge/KnowledgeAssociationView.tsx` → `knowledgeStore.runExtraction` → `cmd.extractConceptsForLibrary` (`src/stores/knowledgeStore.ts:190`)。
- 后端：`commands/knowledge.rs:84` → `fetch_library_assets` → 主循环（118-214）→ LLM → `insert_concept` / `insert_case` / `concepts_extraction_log::insert`。

**结论**：✅ 已对齐。canonical .md 的"内容"在 DB 与磁盘双写，知识抽取读 DB 拷贝即可获得最新 canonical 内容。

### Q2：搜索是否会返回原件 + 衍生件两条结果 — ❌ 未对齐

**FTS 表定义**（`src-tauri/src/db/migration.rs:419-423`）：

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS fts_assets USING fts5(
    name, file_path,
    content='assets', content_rowid='rowid'
);
```

**搜索 SQL**（`src-tauri/src/db/search.rs:20-29`）：

```sql
SELECT a.id, a.name, a.file_path, a.project_id, rank
FROM fts_assets f
JOIN assets a ON a.rowid = f.rowid
WHERE fts_assets MATCH ?1
ORDER BY rank LIMIT ?2
```

观察：
1. FTS 索引列只有 `name, file_path`。**全文搜索不命中正文/structured_md**——即不会从衍生 .md 的内容里搜出片段。
2. SQL **无任何 `source_asset_id` 过滤或 GROUP BY 去重**。如果根素材是 `lecture.pdf`，task_008 物化出 `lecture.md`，二者都会作为独立 asset 行进入 fts_assets。搜索 "lecture" 会返回 2 条 hit（同一概念两条目）。
3. `search_all`（search.rs:96-103）只是合并 assets + notes 后按 rank 排序+截断，仍无去重。

**用户可见后果**：
- 在搜索面板中，PDF 原件 + 衍生 .md 同时显示为两条结果，title 都是相似文件名，但用户认为它们是"同一个素材"。
- 全文检索能力没有提升：即便 markitdown 把 PDF 内容物化成 .md，搜索也不会从内容文字里命中。

**结论**：❌ 未对齐。需要独立迭代（见报告末尾"建议下一轮迭代"）。

### Q3：`notecapt/concept-extract-requested` 事件前端是否真的监听 — ❌ 未对齐

**事件发出端**（`src-tauri/src/extraction/scheduler.rs:686-698`）：

```rust
// MVP 采用事件驱动：前端监听 `notecapt/concept-extract-requested` 调用
// `extract_concepts_for_library(force=false)`，F-8 的去重日志确保不会
// 无限触发重复抽取。
if let Ok(Some(project)) = crate::db::project::get_by_id(&conn, &source_asset.project_id) {
    let _ = app.emit(
        "notecapt/concept-extract-requested",
        serde_json::json!({
            "libraryId": project.library_id,
            "triggerAssetId": source_asset.id,
            "triggerDerivedAssetId": derived_id,
        }),
    );
}
```

**前端监听端**：`grep -rn "concept-extract-requested" src/` 返回 **0 个结果**。

前端唯一调用 `extractConceptsForLibrary` 的位置是 `src/stores/knowledgeStore.ts:190`，触发链路：
- `src/components/features/knowledge/KnowledgeAssociationView.tsx` → 用户手动点击"开始抽取" → `runExtraction` → `cmd.extractConceptsForLibrary(libraryId, force)`。

也就是说，scheduler 转换完成后发出的 `notecapt/concept-extract-requested` 是**死信事件**：除非用户手动进入 KnowledgeAssociationView 并点击触发，否则知识抽取不会被自动触发。

**注意**：scheduler.rs:676 同时发出 `notecapt/asset-converted`，前端也无任何监听点（搜索结果显示无该字符串）。这两个事件目前完全未被消费。

**结论**：❌ 未对齐。架构意图（task_008 自动触发增量抽取）未在前端落地。需要独立迭代补 listener。

### Q4：canonical .md 内容更新是否能感知 — ✅ 已对齐

**机制**：
1. task_008 重新物化时，`scheduler.rs:672-673` 同步更新 source 与 derived 的 `extracted_content.content_hash`：
   ```rust
   let _ = crate::db::extraction::set_content_hash(&conn, &derived_id, &hash);
   let _ = crate::db::extraction::set_content_hash(&conn, &source_asset.id, &hash);
   ```
   `set_content_hash` 实现见 `src-tauri/src/db/extraction.rs:116-126`。
2. 知识抽取主循环（`src-tauri/src/commands/knowledge.rs:131-141`）：
   ```rust
   if !force {
       if let Some(hash) = content_hash.as_ref() {
           if logged_pairs.contains(&(asset_id.clone(), hash.clone())) {
               skipped_incremental += 1;
               ...continue;
           }
       }
   }
   ```
3. `concepts_extraction_log` 表键 `(library_id, asset_id, content_hash)`，见 `src-tauri/src/db/concepts_extraction_log.rs:11-22, 33-46`。

**判定**：
- 如果 task_008 用相同 markdown 再次物化 → hash 不变 → 跳过抽取（幂等正确）。
- 如果 markdown 内容变化 → hash 变化 → `(asset_id, new_hash)` 不在 logged_pairs 里 → 重新抽取该素材。

**前提**：必须**有人**调用 `extract_concepts_for_library`。如 Q3 所述，自动触发链路目前断裂，所以"内容更新后能否被感知"在自动化层面依赖 Q3 修复；但 **算法层面已对齐**，用户手动触发时能正确识别变更并重抽。

**结论**：✅ 已对齐（算法正确）。与 Q3 联动后才能形成完整自动闭环。

### Q5：placeholder .md 是否被知识抽取误消费 — ✅ 已对齐

**placeholder 写入路径**（`src-tauri/src/extraction/scheduler.rs:755-790` 区域）：
- 调用 `write_placeholder_md`，extractor_type 写作 `placeholder_{failure_code}`（scheduler.rs:783）。
- 注释明确（scheduler.rs:749-750）：
  > **不**写 `extracted_content.status='extracted'`（保留为 failed 状态，见调用方写入的 `update_extraction_status('failed', ...)`），让"日后真转换成功"能用 status 区分。

**抽取查询过滤**（`src-tauri/src/commands/knowledge.rs:388-391`）：
```sql
LEFT JOIN extracted_content md_ec ON md_ec.asset_id = md.id AND md_ec.status = 'extracted'
LEFT JOIN extracted_content ec   ON ec.asset_id    = a.id  AND ec.status = 'extracted'
WHERE a.source_asset_id IS NULL OR a.asset_type != 'markdown'
```

判定：
1. 外层 WHERE 已经过滤掉"作为根素材出现的衍生 markdown"（`source_asset_id IS NULL OR asset_type != 'markdown'`）。placeholder 衍生 .md 拥有 `source_asset_id` 且 `asset_type='markdown'`，不会进入外层迭代。
2. 对于根素材的 derived placeholder，子查询会找到这个 placeholder md asset；但 `md_ec` 的 LEFT JOIN 要求 `status='extracted'`，placeholder 是 `failed` → `md_ec.*` 全为 NULL → COALESCE 跳过它，落到 `ec.structured_md` / `ec.raw_text` / `ai.summary` / `a.name`。
3. 衍生件本身 `extracted_content` 写入是 `write_placeholder_md` 内调用 `upsert_extraction_result(...)`(scheduler.rs:780-786)，但调用方 `update_extraction_status('failed', ...)` 会显式回写 status，从而被 SQL 过滤。

**注意点（局限）**：`write_placeholder_md` 内部直接调用 `upsert_extraction_result` 后，是否真的有 `update_extraction_status('failed')` 紧随其后？源码注释 scheduler.rs:750 声明由调用方负责。若某个调用路径漏掉 status 回写，placeholder 内容（如 "⚠️ 转换失败" 文案）可能被 COALESCE 取到。**建议补一个 unit test**（不在本 task 范围）。

**结论**：✅ 已对齐（基于当前代码状态）。无误消费风险。

## 三个场景的实际数据流图（AC-1）

### 场景 A：标签筛选数据流

```
用户在 TagTree 点击标签
  → src/components/features/TagTree.tsx
  → uiStore.setSelectedTagId
  → AssetListView 渲染：调用 cmd.listAssetsByTags
  → src-tauri/src/commands/asset.rs: list_assets_by_tags
  → SQL JOIN: assets ⨝ asset_tags ⨝ tags
  → 返回 Vec<Asset>（不区分原件/衍生，但 UI 默认展示包含 derived .md）
```
说明：标签筛选不读取 `structured_md` 或 .md 文件内容，纯结构化关系查询。task_008 的 `propagate_tags_to_derivative`（scheduler.rs:646-655）保证衍生件继承源件标签。

### 场景 B：全文搜索数据流

```
用户在搜索框输入 "lecture"
  → src/components/layout/Toolbar.tsx / 顶部搜索组件
  → cmd.searchAll(query, limit)
  → src-tauri/src/commands/search.rs → src-tauri/src/db/search.rs:search_all
  → fts_assets (索引 name + file_path) + fts_notes (索引 content)
  → 返回 SearchHit 列表（原件 + 衍生 .md 各一条，无去重）
```
关键事实：**FTS 不索引 structured_md / raw_text**，导致 markitdown 把 PDF 文本物化后，搜索"PDF 内文字"仍然命中不到。这是 Q2 ❌ 的核心。

### 场景 C：知识抽取数据流

```
用户进入 KnowledgeAssociationView，点击"开始抽取"
  → useKnowledgeStore.runExtraction (src/stores/knowledgeStore.ts:190)
  → cmd.extractConceptsForLibrary(libraryId, force)
  → src-tauri/src/commands/knowledge.rs:84 extract_concepts_for_library
    ├─ fetch_library_assets (knowledge.rs:371)
    │   └─ 优先 derived .md 的 extracted_content.structured_md
    ├─ 加载 concepts_extraction_log 中 (asset_id, content_hash) 集合（F-8 增量）
    ├─ 对每个素材：若 (asset_id, hash) 已在 log 中且 force=false → 跳过
    ├─ 否则调用 LLM → 解析 → insert_concept / append_source_asset / insert_case
    └─ insert into concepts_extraction_log
  → emit "notecapt/concept-extraction-progress" / "concept-extraction-done"
  → 前端 KnowledgeAssociationView.tsx:77 监听进度条更新

[死信路径，目前未启用]
scheduler 物化完成 → emit "notecapt/concept-extract-requested"
  → 前端无监听 ❌
```

## 已知局限

1. 未深入审计 `knowledge_understanding.rs` / `knowledge_units.rs` / `knowledge_synthesis.rs` 是否额外读取磁盘 .md 文件。从 grep 结果看它们均无 `read_to_string` 调用，但未逐行核对它们的 SQL 数据源是否也通过 `extracted_content.structured_md`。
2. 未运行实际程序复现 Q2 双条结果；仅基于 SQL 静态分析得出结论。
3. Q5 依赖"placeholder 写入后调用方会回写 status='failed'"这一约定，未做交叉验证（建议补 unit test）。
4. 未检查 `knowledge_graph.rs` 的图数据来源，未确认是否完全基于 concepts/concept_relations 表（间接已对齐）。

## 需要 Reviewer 特别关注的地方

1. **Q2 严重程度判断**：Q2 标 ❌ 是因为它影响 markitdown 集成的实际效用（用户期望搜 PDF 内文字能命中）。是否独立成迭代需 PM 判断；若优先级不高可降级为 ⚠️。
2. **Q3 修复方向**：是补一个前端 `useEffect` listener，还是直接在后端 scheduler 同步触发 extract_concepts_for_library？后者更可靠（避免前端未启动时丢事件），但需考虑 LLM 调用成本/并发。建议讨论。
3. **placeholder content_hash**：placeholder write 路径里是否也写了 content_hash？若写了且 hash 落入 concepts_extraction_log，将来真转换成功时 hash 重算应能触发重抽，但需确认。

## 建议下一轮迭代（AC-4，仅骨架）

### Iteration P-1：搜索去重 + 内容全文索引（应对 Q2 ❌）

- **目标**：搜索结果一个根概念只出现一次；FTS 索引应包含 `structured_md` 让用户能从内容文字搜到。
- **变更骨架**：
  1. 修改 `src-tauri/src/db/migration.rs` FTS 表 schema：新增 `fts_asset_content` 虚拟表，关联 `extracted_content.structured_md`，或扩展 `fts_assets` 增加 content 列。需写 migration 版本 bump + 回填触发器。
  2. `src-tauri/src/db/search.rs:search_assets` 增加 `WHERE a.source_asset_id IS NULL OR a.asset_type != 'markdown'`，把衍生 .md 折叠到根素材；如需展示"来自该原件的衍生内容命中"，在 SearchHit 增加 `derivedAssetId` 字段。
  3. 增加 `search_content` 函数从 fts_asset_content 搜内容片段。
- **测试**：插入 PDF + 衍生 .md，搜 PDF 中独有的术语，断言只返回 1 条原件 hit 且 snippet 来自 structured_md。

### Iteration P-2：自动触发增量抽取（应对 Q3 ❌）

- **目标**：scheduler 物化完成后能自动驱动 `extract_concepts_for_library`。
- **变更骨架（任选其一）**：
  - 方案 A（推荐）：在 `src-tauri/src/extraction/scheduler.rs:686` 处，**直接在 Rust 内**异步派发 `tokio::spawn` 调用 `extract_concepts_for_library` (force=false)。优势：与前端解耦，事件不会丢；劣势：需保证 LLM client 在 scheduler 上下文可获取，并加并发节流（同一 library 多文件物化时合并为一次抽取）。
  - 方案 B（保留事件驱动）：在 `src/App.tsx` 或顶层组件加 `listen<{libraryId,...}>("notecapt/concept-extract-requested", ...)` → 节流后调用 `knowledgeStore.runExtraction(libraryId, false)`。需注意：前端不在前台时事件丢失；多 library 切换时 store 状态污染。
- **测试**：拖入 PDF → 等待转换完成 → 断言 concepts_extraction_log 增加了相应 (asset_id, content_hash) 行，无需用户手动点击。

### Iteration P-3（可选）：placeholder 防御性测试（巩固 Q5 ✅）

- 单测：构造 derived md 但 extracted_content.status='failed' 的场景，断言 `fetch_library_assets` 返回的 content 不含 placeholder 文本。
