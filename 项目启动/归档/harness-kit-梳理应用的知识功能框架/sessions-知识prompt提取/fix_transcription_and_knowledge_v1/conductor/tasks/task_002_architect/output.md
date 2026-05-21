# Task 交付 — task_002_architect

## 实现摘要

基于 task_001 实测矩阵，设计 6 个工程模块 E1~E6，给出确切的文件变更、DB migration、数据模型决策。Q1~Q10 逐一作答。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `sessions/.../task_002_architect/output.md` | 新建 | 设计文档 |
| `sessions/.../task_003..008/input.md` | 新建 | 6 个 dev task 的输入 |

---

## Q1~Q10 答复

| # | 问题 | 决策 |
|---|---|---|
| Q1 | H 级用例代码映射确认 | 采纳 task_001 重构定义不变；作为后续回归测试 ID |
| Q2 | 占位 .md 字段 | YAML front-matter + `## 转录失败` 段；字段：`kind: placeholder` / `source_asset_id` / `mime` / `failure_code` (`unsupported`\|`empty`\|`error`\|`md_source`) / `failure_reason` / `extracted_at`。UI badge 判据：front-matter `kind == placeholder` 优先，fallback 到 DB `extraction_status in ('failed','unsupported')` |
| Q3 | .md 源入工作区 | **方案 A 变体**：解除 markdown 过滤；但走一条 `materialize_source_markdown` 专用路径（不经 extractor pipeline）：直接 `fs::copy` 到工作区 + 注入 front-matter。复用 `materialize_md` 的 DB 写入逻辑 |
| Q4 | safe-rename 规则 | stem = `sanitize(original_stem)`；替换字符：`/\\:*?"<>|` + 控制字符（< 0x20）→ `_`；连续空格 → 单空格；保留 CJK/emoji；超过 120 chars 截断；文件名最终 `<assetId>_<safeStem>.md` |
| Q5 | 版本化物理布局 | `_versions/<asset_id>/v{N}.md` 子目录（project workspace 下）；latest = 根目录硬拷贝（非软链，Finder 友好）；`derivative_version` INTEGER 列放 `assets` 表，默认 0，物化时 bump |
| Q6 | F-7 触发点 | **Rust scheduler 末尾直接 enqueue 概念抽取任务**（进程重启可恢复）；新增 `pipeline_tasks.task_type = 'concept_extract'`；前端不做触发，只监听 `notecapt/concept-extraction-done` 刷新 UI |
| Q7 | F-8 指纹位置 | `extracted_content` 表加 `content_hash TEXT`（SHA-256 of structured_md）；`concepts_extraction_log(library_id, asset_id, content_hash, extracted_at)` 新表记录"已对此指纹抽过概念" |
| Q8 | user_edited 保护粒度 | 仅保护 name/definition 不被自动覆盖；允许 append source_asset_ids + 新 cases。未来 UI 可加 diff 面板 |
| Q9 | 跨 project 合并 | 保持 library 范围合并（当前 PASS）；UI 显示 "出现于 N 门课"（本 session P1，仅留 DB 支持：`source_project_ids` 字段已存在） |
| Q10 | `_versions/` 清理 | 本 session 不清理；上限由文件系统托底。task_004 实现时要保证 `v{N}` 递增不会回退；P1 再做保留策略 |

---

## E1~E6 模块设计

### E1 — 工作区完整性（task_003；F-1, F-2）

**目标**：每个原文件有 `.md` 邻居（即使失败也要占位）。

**关键改动**：
1. `scheduler.rs:455` — `source_asset_should_materialize` 返回 `true` 对 `.md` 源（去掉 markdown 过滤；保留 `source_asset_id.is_none()` 过滤衍生）
2. `scheduler.rs:479` — 拆分 `materialize_md` 内容生成为独立函数 `build_materialized_content(asset, md_or_placeholder)`；新增 `materialize_placeholder(app, asset, failure_code, reason)`
3. `scheduler.rs` 主循环：
   - `unsupported` 分支（line 149）：调用 `materialize_placeholder("unsupported", mime)`
   - extraction 成功但 `structured_md.is_empty() || quality_level < 1`：`materialize_placeholder("empty", "空抽取")`
   - `db_handle_task_error` 达到 max_retries：`materialize_placeholder("error", err)`
   - `.md` 源：读原文件 → 注入 YAML header → 写 `<assetId>_<safeStem>.md`（不经 extractor）
4. safe-rename：新增 `src-tauri/src/utils/safe_name.rs` 模块，pub fn `sanitize_stem`

**影响文件**：
- 新建 `src-tauri/src/utils/safe_name.rs`
- 修改 `src-tauri/src/utils/mod.rs` 暴露 safe_name
- 修改 `src-tauri/src/extraction/scheduler.rs`

**占位 .md 模板**：
```md
---
kind: placeholder
source_asset_id: "{id}"
mime: "{mime}"
failure_code: "{code}"
failure_reason: "{reason}"
extracted_at: "{rfc3339}"
---

# ⚠️ 转录失败

**文件**：{original_name}
**原因**：{reason_human_readable}
**时间**：{extracted_at}
**原件位置**：{file_path}
```

---

### E2 — 派生版本化（task_004；F-3, F-4）

**目标**：重抽取保留历史。

**DB migration V9**:
```sql
ALTER TABLE assets ADD COLUMN derivative_version INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_assets_source_version ON assets(source_asset_id, derivative_version);
PRAGMA user_version = 9;
```

**`materialize_md` 改造**：
- 检测已有 derivative，读其 `derivative_version = N`
- 新版本 N+1：
  1. 旧内容备份到 `_versions/<source_asset_id>/v{N}.md`（物理拷贝）
  2. 写新 md 到 workspace 根 `<asset_id>_<stem>.md`（覆盖）
  3. UPDATE derivative row: `derivative_version = N+1`

**影响文件**：
- 修改 `src-tauri/src/db/migration.rs`（新增 `v9_derivative_version`）
- 修改 `src-tauri/src/db/mod.rs`（如有版本表列出）
- 修改 `src-tauri/src/db/asset.rs`（SELECT/INSERT 追加 derivative_version；model 追加字段）
- 修改 `src-tauri/src/models/asset.rs`（Asset 结构体加 `derivative_version: i32`）
- 修改 `src-tauri/src/extraction/scheduler.rs`（`materialize_md` 版本化）

---

### E3 — 内嵌 YAML front-matter（task_005；F-6）

**目标**：派生 .md 顶部可读 metadata。

**关键改动**：
- 新增 `build_frontmatter_prefix(asset, source_asset, version, tags) -> String`
- `materialize_md` 写入前在内容前 prepend front-matter
- `materialize_source_markdown`（来自 E1）也 prepend

**字段**：
```yaml
---
tags: [tag_name_1, tag_name_2]
source_asset_id: "{source_id}"
source_asset_name: "{original_name}"
derivative_version: {N}
extracted_at: "{rfc3339}"
extractor_type: "{type}"
---
```

**影响文件**：
- 修改 `src-tauri/src/extraction/scheduler.rs`
- 新增 helper 在 `scheduler.rs` 或独立 `extraction/frontmatter.rs`

---

### E4 — 自动抽取链路（task_006；F-7）

**目标**：extraction 完成 → 概念抽取自动触发。

**关键改动**：
- `scheduler.rs` 的 `db_save_extraction_result` 成功后 → 新增 `enqueue_concept_extract_for_asset(app, asset_id)`
- 新增 pipeline_task_type `'concept_extract'`（V9 migration 放宽 CHECK 约束）
- scheduler 主循环按 task_type 分派：`'extract'` 走现有路径；`'concept_extract'` 调用 `run_concept_extract_for_asset`（抽取单个 asset 的概念，复用 `extract_concepts_for_library` 的 LLM + insert 逻辑但 scope 缩小到单 asset）

**MVP 简化路径**（本次采用）：
- 不新增 task_type；改为在 scheduler extraction 成功 emit `extraction:completed` 之后，**同步调用** `try_enqueue_library_concept_extract(app, asset.project_id)`，再复用现有 `extract_concepts_for_library` 命令（force=false 走增量，由 E5 实现）
- 需避免无限触发：借用 `concepts_extraction_log` 表（E5 引入）

**影响文件**：
- 修改 `src-tauri/src/extraction/scheduler.rs`
- 新增辅助函数 `enqueue_concept_extract_if_needed` in `src-tauri/src/commands/knowledge.rs` 或独立模块

---

### E5 — 增量抽取 + 用户编辑保护（task_007；F-8, F-9, F-10）

**目标**：二次导入只抽新素材；user_edited 概念不被改。

**DB migration V9 追加**:
```sql
ALTER TABLE extracted_content ADD COLUMN content_hash TEXT;
CREATE TABLE IF NOT EXISTS concepts_extraction_log (
  id TEXT PRIMARY KEY,
  library_id TEXT NOT NULL,
  asset_id TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  extracted_at TEXT NOT NULL,
  UNIQUE(library_id, asset_id, content_hash)
);
```

**改动**:
1. `db_save_extraction_result`：保存时计算 `sha256(structured_md)` 写入 `extracted_content.content_hash`
2. `extract_concepts_for_library`：
   - 读取 `concepts_extraction_log` 的 (asset_id, content_hash) 集合
   - 遍历 asset 时：`if !force && already_logged(asset_id, hash) { skip }`
   - LLM 抽完后 `INSERT OR IGNORE concepts_extraction_log`
3. **F-9**：在 `existing_concepts` 命中分支，若 `concept.user_edited == true`，**跳过 name/definition 更新**，仅 `append_source_asset` + 插入 cases
4. **F-10**：`synthesize_viewpoints` 改为"增量合并"：
   - 收集 cases，对每个 source_asset_id 产出 N 个 viewpoint（已是单源）
   - 保存时按 `(concept_id, source_asset_id)` upsert（delete-by-source + insert），而非整体 delete-rebuild
   - 需要 schema：`concept_viewpoints` 加 UNIQUE(concept_id, source_asset_id, perspective)？本次不加；仅改写入顺序即可——对已存在 source_asset_id 的 viewpoint 保留，仅清理"源 asset 已被删除"的 viewpoint（P1）。MVP 简化：**保持 delete-rebuild**，但确保单源 cases 每次产出固定 perspective（prompt-level 稳定）

**影响文件**:
- 修改 `src-tauri/src/db/migration.rs`
- 修改 `src-tauri/src/db/extraction.rs`（save 时带 hash）
- 修改 `src-tauri/src/commands/knowledge.rs`（force 生效；user_edited 读取；viewpoint 合并策略）
- 新增 `src-tauri/src/db/concepts_extraction_log.rs` 或放在 `db/knowledge.rs`

---

### E6 — F-11 回归（task_008；仅 scope 观察）

**目标**：确认三个命令在本 session 前后行为无退化。

**关键改动**：
- **无源码改动**
- 新增集成测试用例 `src-tauri/src/commands/knowledge.rs` `#[cfg(test)]` 模块：
  - `test_synthesize_viewpoints_writes_db`：mock LLM 或者 skip（直接验证 insert 路径）
  - `test_generate_extensions_writes_db`
  - `test_co_occurrence_idempotent`
- 补充文档注释说明 Q1~Q10 决策

本 task 预计最轻量，主要是跑 `cargo test --package nc-desktop-app knowledge` 回归。

---

## DB Migration V9 完整 SQL

```sql
ALTER TABLE assets ADD COLUMN derivative_version INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_assets_source_version ON assets(source_asset_id, derivative_version);

ALTER TABLE extracted_content ADD COLUMN content_hash TEXT;
CREATE INDEX IF NOT EXISTS idx_extracted_content_hash ON extracted_content(content_hash);

CREATE TABLE IF NOT EXISTS concepts_extraction_log (
  id TEXT PRIMARY KEY,
  library_id TEXT NOT NULL,
  asset_id TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  extracted_at TEXT NOT NULL,
  UNIQUE(library_id, asset_id, content_hash)
);
CREATE INDEX IF NOT EXISTS idx_cel_library ON concepts_extraction_log(library_id);

PRAGMA user_version = 9;
```

**向前迁移检查**：
- `ALTER ADD COLUMN` 对存量表不破坏；默认值填入
- 新表 `CREATE IF NOT EXISTS` 幂等
- 存量 assets 的 `derivative_version = 0`（表示"pre-versioning 时代"），下次重抽直接 bump 到 1 即可

---

## 任务执行顺序 & 依赖

```
V9 migration 在 task_003 一并落地（作为 E1 的前置基建）
task_003 (E1) → task_004 (E2) → task_005 (E3) → task_006 (E4) → task_007 (E5) → task_008 (E6)
```

E1 需要先于 E2，因为版本化 diff 依赖 safe-name；E4 可以在 E1~E3 之后并行。按顺序执行最稳。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | Q1-Q10 全部作答 | 已做 | PASS |
| ✅ 正常路径 | E1~E6 设计覆盖所有 FAIL/PARTIAL 用例 | 已做 | PASS |
| ⚠️ 边界条件 | V9 migration 对存量数据的影响 | 已分析 | 无破坏，仅追加列 |
| ❌ 异常路径 | 未对比 CI 测试矩阵 | 未测 | 无 CI 可用；dev task 用 `cargo check` 兜底 |

## 已知局限

1. E4 采用 MVP 简化路径（library-level 增量触发），不做细粒度的单 asset concept extract；若未来需要更精细触发，再引入 pipeline_task_type
2. E5 的 F-10 viewpoint 合并仅保证 prompt-level 稳定而非 schema-level 唯一约束，跨轮次 LLM 输出漂移不在本 session 处理
3. 未验证 Tauri v2 的 `AppHandle` 能否被 scheduler 内部同步调用 `extract_concepts_for_library`（tokio runtime 限制），E4 实现时要注意

## 需要 Reviewer 特别关注的地方

- Q2 占位 .md 的 `failure_code` 取值是否足够覆盖 UI 后续分支（如 "需要密码的 PDF"、"加密 zip"）
- Q5 的 "latest 是硬拷贝" 决策意味着磁盘占用翻倍（latest + _versions/ 都存完整内容），是否能接受
- Q6 的 "Rust scheduler 末尾 enqueue" 简化路径，E4 实现时若发现 runtime 问题可降级到前端触发
