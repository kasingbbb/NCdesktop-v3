# Review Scorecard — task_014_legacy_unverified_migration

## 审查思考过程

1. **Task 意图**：对存量 `conversion_meta` 中"成功但抽取内容空"的旧记录回填 `failure_code='legacy_unverified'`，避免老用户升级后感知退步；同时提供 ConversionState 三态查询接口与前端三态 badge，避免下游知识进化系统把"未验证"误当"成功"。
2. **AC 检查结果**：
   - AC-1 V14 migration：✅ SQL 与 input.md 末尾 Conductor 裁决段（2026-05-13）字面一致（JOIN extracted_content + cm 最新一行约束 + failure_code IS NULL 守卫）；dispatcher `if current_version < 14` 已加；末尾 `PRAGMA user_version = 14`。
   - AC-2 幂等：✅ `v14_is_idempotent` 单测断言二次 `conn.changes() == 0`；额外 `CREATE INDEX IF NOT EXISTS idx_conversion_meta_failure_code_legacy ON conversion_meta(failure_code) WHERE failure_code = 'legacy_unverified'` 部分索引。
   - AC-3 三态查询：✅ `ConversionState { Success(String), LegacyUnverified, Failed(FailureCode) }` 枚举 + `get_conversion_state(conn, asset_id) -> Result<Option<ConversionState>, String>`；本地 `parse_failure_code` helper 在 conversion_meta.rs 自建；最新一行 ORDER BY converted_at DESC LIMIT 1。
   - AC-4 前端三态 badge：✅ `AssetStateBadge` 加 `failureCode?: string | null` 入参；`legacy_unverified` → ⚠️ AlertTriangle + "旧记录未校验" + "重新转录"按钮；8 错误码 + state=failed → 中文文案；`WorkspaceAssetView.extractionFailureCode` IPC 字段同步落地。
   - AC-5 单测：✅ migration 新增 5 条（v14_backfills_extracted_with_empty_content / v14_keeps_null_when_content_present / v14_does_not_overwrite_existing_failure_code / v14_is_idempotent / v14_only_touches_latest_row_per_asset）；conversion_meta 新增 5 条（三态各场景 + None + null-fc-empty-content 保守判）。
   - AC-6 消费侧已知点：✅ R1 output 已列 3 处（`commands/knowledge.rs:378-389`、`commands/knowledge_unit_learning.rs:322`、`db/asset.rs:1118`）；按 Conductor 裁决"仅标注"，filter 改造转 follow-up。
3. **关键发现**：
   - V14 函数包含 `tables_ready` 双表存在守卫（`sqlite_master` 检查 conversion_meta + extracted_content），属裁决段字面外的"残缺 schema 防御加固"。dev 在 output.md §Reviewer 关注点 #2 已显式说明其触发场景与设计动机（与 V11 处理 V9/V10 残留同源思路）。生产路径必然 V8 已建表，不引入语义差异，建议保留。
   - `parse_failure_code` 在 `conversion_meta.rs` 内本地自建（**未**改 `failure_code.rs`）。`legacy_unverified` **不**作为 FailureCode 变体新增，纯字符串字面，与 PRD R-④ 一致。
   - `cm.failure_code` 通过 `list_root_assets` SQL 在 row.get(24) 注入 `AssetListJoinRow.latest_failure_code`；列数核对：ASSET_SELECT 15 列 + 10 JOIN 列（rendition_id..ec_extractor_type..cm.failure_code）→ index 24 正确。
   - 前端 `AssetStateBadge` 中 `isLegacyUnverified` 优先级高于 `isPlaceholder` 与四态 state，确保即便 state=done，只要 failureCode='legacy_unverified' 也强制显示"旧记录未校验"+"重新转录"按钮（PRD R-④"老用户感知不退步"兜底）。
   - "重新转录"按钮已实际 wire-up：与既有 "重试" 按钮共用 `handleRetry` → `retryAssetConversion(assetId)`（task_006 唯一入口）+ 1s 防抖（lastClickRef）+ `isRetrying` 状态。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~6 全满足；V14 SQL 字面与裁决段一致；三态判定优先级正确（'legacy_unverified' → LegacyUnverified；8 码 → Failed；NULL → 看 ec 内容；ec 全空 → 保守 LegacyUnverified） |
| 安全性 | 25% | 5 | 全 SQL 参数化或常量；未触红线（V1~V13 / failure_code.rs / audio_asr_iflytek.rs / runtime_check.rs / scheduler.rs / markitdown.rs / extractors/* / knowledge*.rs / scripts/）；`tables_ready` 守卫避免残缺 schema 阻塞启动 |
| 代码质量 | 15% | 4.5 | 命名规范（v14_legacy_unverified_backfill / parse_failure_code / ConversionState）；注释充足，关键设计依据（最新一行约束、保守边界判定）有 doc comment 说明；唯一可优化点：`parse_failure_code` 与 `FailureCode::as_str()` 是反向手抄表，若 failure_code.rs 后续新增变体需手工同步——已在 doc 中说明，可接受 |
| 测试覆盖 | 15% | 5 | 10 条新增单测全过；覆盖 AC-1 正路径、真成功不动、已有码不覆盖、幂等、仅最新一行；三态枚举 + None + 保守边界全覆盖；`cargo test --lib` 195 passed 0 failed，无退步（baseline 185 → 195） |
| 架构一致性 | 10% | 5 | 未引入新依赖；V14 在末尾追加，未改 V1~V13；ConversionState 枚举与 ADR-007 / Debate Layer 3 R-④ 对齐；AC-6 filter 改造按裁决段转 follow-up；遵守 ADR-004 append-only（仅回填 failure_code 列，不复制内容到日志表） |
| 可维护性 | 10% | 4.5 | `parse_failure_code` 维护点单一（本地，不污染 failure_code.rs）；`ConversionState` 三态语义清晰；`extraction_failure_code` IPC 字段双端注释充分；唯一遗留是 follow-up 的消费侧 filter 改造（已显式列 3 处）|

**综合分：4.85/5**（加权计算：0.25×5 + 0.25×5 + 0.15×4.5 + 0.15×5 + 0.10×5 + 0.10×4.5 = 1.25 + 1.25 + 0.675 + 0.75 + 0.5 + 0.45 = 4.875）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **`parse_failure_code` 与 `FailureCode::as_str()` 反向同步**
   - **代码位置**：`src-tauri/src/db/conversion_meta.rs:186-198`
   - **修复方向**：当前是合理的（task_014 红线明确不动 failure_code.rs）。后续若 FailureCode 新增变体，需手工同步 parse_failure_code；建议在 `failure_code.rs` 顶部加注释指向本 helper，或在 follow-up 中把 reverse-map 移到 failure_code.rs 内（封装在同一文件，避免漂移）。
   - **验证标准**：非阻塞项；标注即可。

2. **前端 vitest 未跑**
   - **代码位置**：`src/lib/asset-state.tsx` 新增 legacy_unverified 分支
   - **修复方向**：dev 已在 R2 §Reviewer 关注点 #5 说明（props 新增可选字段向后兼容；本轮 baseline 仅要求 cargo test）。可在后续轮次补一条 vitest（`failureCode="legacy_unverified"`，断言文案 + 按钮 label="重新转录"）。
   - **验证标准**：非阻塞项；后续 task 补即可。

## 红线核查（8 项）

| # | 红线项 | 状态 |
|---|-------|------|
| 1 | 修改 V1~V13 函数 | ✅ PASS（V14 在末尾追加） |
| 2 | 修改 `failure_code.rs` 加 FailureCode 变体 | ✅ PASS（legacy_unverified 是字符串字面） |
| 3 | 修改 `audio_asr_iflytek.rs` | ✅ PASS（本 task 未触及） |
| 4 | 修改 `runtime_check.rs` / `scheduler.rs` / `markitdown.rs` / `extractors/*` | ✅ PASS（本 task 未触及） |
| 5 | 修改 `commands/knowledge.rs` / `commands/knowledge_unit_learning.rs` | ✅ PASS（AC-6 filter 改造转 follow-up） |
| 6 | 修改 task_004~006 scripts/ | ✅ PASS（本 task 未触及） |
| 7 | migration 非幂等 | ✅ PASS（v14_is_idempotent 测试 second_changes == 0） |
| 8 | 把 legacy_unverified 标为 failed | ✅ PASS（三态独立枚举，前端 isLegacyUnverified 独立分支） |
| 附加 | cargo test --lib 退步（< 195 baseline） | ✅ PASS（195 passed / 0 failed） |

## 4 关注点结论

1. **`parse_failure_code` helper 位置**：合理。本地 helper 不污染 failure_code.rs，符合"红线不动 failure_code.rs"；维护点单一（reverse-map 在同一文件内），doc 已说明语义。MINOR 建议在 follow-up 中考虑反向迁回 failure_code.rs（非必须）。
2. **三态判定优先级与并发竞态**：正确。failure_code='legacy_unverified' 优先返回；NULL 时 fallback ec 内容判定，ec 全空 → 保守 LegacyUnverified（与 V14 backfill 语义对齐）；NULL + 无 ec → LegacyUnverified（scheduler 刚写完 cm、ec 尚未落库的中间态兜底）。无 race，judgment 收敛。
3. **AssetListJoinRow 字段顺序**：未破坏。rusqlite 用 `row.get(<index>)` 按列序读取（不是 name），新加的 `cm.failure_code` 在 SELECT 末尾追加（index 24），匹配 ASSET_SELECT 15 列 + 10 JOIN 列 = 25 总列；既有 row.get(15)..row.get(23) 全部维持原 index。
4. **"重新转录"按钮 onClick wire-up**：实际接入。与 "重试" 按钮共用 `handleRetry` → `retryAssetConversion(assetId)`（task_006 唯一入口）；带 1s 防抖（lastClickRef）+ isRetrying 状态 + 错误回调 onError。aria-label 区分 "重新转录" vs "重试"；data-failure-code 注入 DOM 便于 UI 测试断言。

## V14 SQL 字面对照（与 input.md 末尾裁决段）

**结论：YES 一致**。`db/migration.rs:90-110` 的 UPDATE 语句与 input.md AC-1 字面修订段（2026-05-13 Conductor 裁决）字符级一致：
- 表名：`conversion_meta` + JOIN `extracted_content`
- WHERE：`failure_code IS NULL` + `ec.status = 'extracted'` + raw_text/structured_md 双空判定
- 最新一行约束：`cm.id = (SELECT id FROM conversion_meta WHERE source_asset_id = cm.source_asset_id ORDER BY converted_at DESC LIMIT 1)`
- 末尾 `PRAGMA user_version = 14;`（与 `CREATE INDEX IF NOT EXISTS` 同 execute_batch）

唯一裁决段字面外的增量：函数体开头 `tables_ready` 双表存在守卫 —— dev 在 output.md §Reviewer 关注点 #2 已显式说明（残缺 schema 防御加固，与 V11 同源思路；生产路径必然 V8 已建表，不引入语义差异）。建议保留。

## cargo test --lib 实测

```
cargo test --lib
test result: ok. 195 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
```

- baseline = 185（本轮起始）→ 195（+10：V14 5 + ConversionState 5）
- 无任何 failed / ignored
- 无 baseline 退步

## 给 Dev 的修复指引

**无需修复**。判决 PASS。若希望优化 MINOR 项（reverse-map 迁回 failure_code.rs / 前端 vitest 补 legacy_unverified 单测），建议作为独立 follow-up，不阻塞本 task 流转。

## Reviewer 自检

- [x] 已逐条检查 AC（按 input.md 末尾裁决段为真相源）
- [x] 已检查领域审查重点（migration 幂等性、red-line 文件未触、ADR-004 append-only 一致性、PRD R-④ 老用户感知）
- [x] 实测 cargo test --lib 通过（195/0）
- [x] V14 SQL 字面与 input.md 末尾裁决段 verified
- [x] 评分诚实（综合 4.85/5，未给满分留出 MINOR 维护点空间）
