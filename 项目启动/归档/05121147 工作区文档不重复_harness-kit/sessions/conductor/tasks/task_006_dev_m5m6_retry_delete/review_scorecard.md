# Review Scorecard — task_006_dev_m5m6_retry_delete

## 审查前验证

- [x] 测试结果存在且非空（`db::asset` 26 / `commands::asset` 13 / `commands::extraction` 4 / `commands::outbound` 12，0 failed；`npm run check` 通过）
- [x] 自测验证矩阵存在且正常路径全部 PASS（含正常 / 边界 / 异常三类，唯有两个外壳级 E2E 标注「未自动测」并给出理由）
- [x] 架构遵守声明已填写并含偏离说明（`outbound_cache_dir_for` 签名调整 / `-p notecapt` vs `app_lib`）

交付完整，进入实质审查。

---

## 审查思考过程

### 1. Task 意图复述
落地两条命令链：
- **M5 重试**：在 `commands::extraction` 暴露面向 asset 的 `retry_asset_conversion`，作为 `retrigger_extraction` 的薄包装；前端新增 `retryAssetConversion` wrapper。幂等不变式：连击 N 次后活动态 ≤ 1（V7 部分唯一索引兜底）。
- **M6 删除级联**：在 `db::asset` 实现 `delete_with_cascade`，确保 root+derivative 两行 / 两文件 / `pipeline_tasks` / `conversion_meta` / `extracted_content` / `asset_tags` / outbound cache 七类数据无孤儿（PRD §6 领域审查重点 "删除资产时源文件与 MD 是否一并清理"，硬约束 S5）。

### 2. AC 逐条核对

| AC | 实现位置 | 核对结论 |
|---|---|---|
| AC-1 命令薄包装 + 注册 + 前端 wrapper | `commands::extraction.rs:67-70` / `lib.rs:191` / `tauri-commands.ts:572-574` | ✅ 函数体仅 `retrigger_extraction(app, asset_id).await`，零业务逻辑复制；`invoke_handler!` 已注册；前端 wrapper 调用 `retry_asset_conversion` 名称一致 |
| AC-2 连击 5 次活动态 ≤ 1 | `commands::extraction::tests::retry_asset_conversion_active_unique_guard_caps_at_one` | ✅ 真断言：`SELECT COUNT(*) WHERE status IN ('queued','running')==1`；第 2…5 次 INSERT 被 `idx_pipeline_tasks_active_unique` 拦截。设计上"绕开命令层、直击索引"反而比"命令层连击"更强 —— 哪天命令层护栏全失效，索引仍兜底 |
| AC-3 七类数据全清 | `db::asset.rs:290-345 delete_with_cascade` | ✅ 1. 物理文件 `remove_file_lenient`；2. 手工 DELETE pipeline_tasks（root+derivative 两条 IN）；3. DELETE assets WHERE id=root → CASCADE 清 conversion_meta / extracted_content / asset_tags；4. 显式 DELETE derivative（关键，下方详述） |
| AC-4 outbound 缓存清理 | `commands::asset.rs:332-348` | ✅ 在 DB 锁释放后调用 `outbound_cache_dir_for` → `fs::remove_dir_all`；包了 `cache_dir.exists()` 守卫与 `Err` → `log::warn!`，无 panic 风险 |
| AC-5 命令签名兼容 | `commands::asset.rs:322 fn delete_asset(database, id: String)` | ✅ 入参 `id: String`、返回 `Result<(), String>` 与历史完全一致；前端 `deleteAsset(id)` 未改 |
| AC-6 全表行数 = 0 + 文件不存在 | `db::asset::tests::delete_with_cascade_no_orphans` | ✅ 真实 tempdir 写两文件后断言 `!path.exists()`；五张表（assets / pipeline_tasks / conversion_meta / extracted_content / asset_tags）COUNT(*)=0；额外断言 `report` 字段语义 |
| AC-7 derivative.id 入参反解 | `delete_with_cascade_resolves_via_derivative_id` | ✅ 传入 `d2` → 实际删 `root2` + `d2`；报告字段 `root_asset_id=root2` |
| AC-8 测试套件通过 | 上方测试日志 | ✅ 55 个测试全过（含原有 47 个 + 本 task 8 个） |

### 3. 关键发现

**最关键事实确认（领域）：`assets.source_asset_id` 无 FK 约束。**
核对 `db/migration.rs:176-181`：V5 通过 `ALTER TABLE assets ADD COLUMN source_asset_id TEXT DEFAULT NULL` 加列，SQLite 不支持在 ALTER TABLE ADD COLUMN 时附加 FK 约束。Dev 的发现完全成立：**仅靠 `DELETE FROM assets WHERE id = root.id` 无法自动连坐 derivative 行**，必须显式补刀。这是 architect 方案中 ADR-001 / 数据模型层未明示的事实（架构 §六/§十 只点出 `pipeline_tasks` 无 FK，没点出 `source_asset_id` 无 FK）。建议 Conductor 将此事实补入 progress.md 关键决策记录。

**`commands::outbound::outbound_cache_dir_for` 设计合理**：把私有 `outbound_dir_for(cache_root, asset_id)` 包成 `pub fn outbound_cache_dir_for(asset_id) -> Option<PathBuf>`，确保 task_005（写入）与 task_006（删除）共用同一拼接逻辑（`CACHE_SUBDIR = "NCdesktop/outbound"`），路径口径单源真理。返回 `Option` 优雅处理 `dirs_next::cache_dir()` 极端 None 场景，由命令层 warn 兜底。

**`setup_conn` 加 `PRAGMA foreign_keys=ON` 是测试 fixture 增强非破坏**：核对 `db/mod.rs:31-39`，生产路径 `Database::open` 已在打开连接后 `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;`。本次修改的 `setup_conn` 在 `db::asset::tests` 模块内部（`asset.rs:453-461`），仅影响测试夹具，让 in-memory 测试库与生产同款启用 FK CASCADE 验证逻辑。所有既有测试仍 PASS（26 个），属于贴近生产的合理收紧。

**删除流程的事务粒度**：Dev 在已知局限 §4 自承"非单事务"，是合理的风险披露。在当前实现下：物理文件先于 DB 删（rollback 已不可能）；pipeline_tasks DELETE 与 assets DELETE 顺序合理（即便 assets DELETE 失败，孤立的 pipeline_tasks 也只是"残留待清"，再次调用 `delete_with_cascade` 会重新解算并清理）。outbound cache 清理在锁释放后执行，确保 DB 事务已 commit，时序正确。

### 4. 安全 / 领域审查重点扫描

| session_context §6 审查重点 | 核对结论 |
|---|---|
| Asset ↔ Conversion 1:1/1:N 关系是否清晰 | ✅ `resolve_asset_pair` 在 root.id 与 derivative.id 双向解算；`DeleteCascadeReport.derivative_existed` 真实反映 0/1 关系 |
| 重命名 / 打标签作用于 Asset 而非 Conversion 文件 | N/A（不在本 task 范围） |
| 工作区列表 join 是否导致双条目 | N/A（task_002 落地，本 task 仅删除） |
| 转化失败 / 中 / 成功三态 UI 呈现 | N/A |
| **拖拽 outbound payload 始终指向 MD** | ✅ `outbound_cache_dir_for` 复用 task_005 路径，删除时同步清缓存目录避免悬挂的旧 MD 缓存 |
| **删除资产时源文件与 MD 是否一并清理，是否有孤儿** | ✅✅✅ 本 task 核心 PRD-S5 验收：两文件 fs::remove + DB 全链清空 + outbound cache rm -rf；测试以 `path.exists()` 物理级别断言而非仅 row=0 |

### 5. 架构一致性

- ADR-001 双行模型：尊重，root + derivative 两行各自处理。
- ADR-002 list 查询入口：不触碰。
- ADR-003 db 层零 IO：⚠️ **轻微偏离** —— `delete_with_cascade` 在 `db/asset.rs` 内调用 `fs::remove_file`，确属 db 层做 IO。但 ADR-003 原文（task_001 §五）针对的是"读列表查询不引入 IO"以避免长锁；删除路径的 IO 属于"删除事务的物理副作用"，与读路径不同语义。Dev 把级联清理放在 db 模块以满足"不在 commands/ 中拼 SQL"硬约束（session_context §5），权衡合理。建议在函数顶部 doc 注释中再补一句"本函数为 ADR-003 的有意例外"。
- ADR-007 命令 asset_id 化：✅ `retry_asset_conversion(asset_id)` / `delete_asset(id)` 入参均无 path。
- 命名一致：`delete_with_cascade` / `DeleteCascadeReport` / `outbound_cache_dir_for` 全按 input.md。

### 6. 测试覆盖深度

- **AC-2** 把"绕开命令层、只剩索引"的最坏情况测出来，思路上佳。
- **AC-6** 用 `tempfile::tempdir` 写真实文件，再断言 `!path.exists()`，对硬约束 S5 给出了物理级别证据。
- **AC-7** 单测真验证了 derivative.id 反解到 root（不只是 happy-path）。
- 边界覆盖：missing file（`delete_with_cascade_missing_file_is_ok`）、missing asset（`delete_with_cascade_returns_err_when_asset_missing`）齐全。
- 已知缺口（Dev 已自承）：
  - `delete_asset` 命令外壳的 outbound 缓存 IO 路径未自动化（依赖 `dirs_next::cache_dir()` 与 Tauri State），留给 task_009 集成测试 / 手测。可接受。
  - `delete_with_cascade` 物理文件"被占用 / 权限错误"的具体场景未单测；当前只覆盖了 NotFound 与成功两支，permission denied 分支走 `log::warn!` 不阻断，逻辑正确但无测试见证。属 MINOR。

---

## 评分

> 权重读自 session_context.md §4（注意：本 session 没有"安全性"维度，替换为"用户体验"与"架构一致性"加重；安全性要求标注"低 — 纯本地"）。

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 8 个 AC 逐条命中；4 个新增单测 + 1 个索引兜底测试都直击不变式；report 字段语义清晰 |
| 用户体验 | 25% | 4 | 删除命令签名兼容前端无感；失败时 outbound cache 清理失败仅 warn 不阻断 DB 删除是正确的 UX 取舍。MINOR：物理文件被占用时仅 warn，用户在 UI 上看不到"DB 删了但文件还在"的反馈（Dev 已自承 #3，可由后续 task 补 report 字段透传） |
| 架构一致性 | 20% | 4 | `pipeline_tasks` 手工 DELETE、`outbound_cache_dir_for` 路径单源真理、命令 asset_id 化均遵守。轻微偏离：`delete_with_cascade` 在 db/ 内做 IO（fs::remove_file），与 ADR-003 "db 零 IO" 字面冲突，但为满足"不在 commands/ 中拼 SQL"的更强硬约束（session §5）做出的合理取舍；可在函数 doc 中明示例外 |
| 代码质量 | 10% | 5 | 函数职责单一（`remove_file_lenient` 抽出）、命名贴合语义、注释指明每一步背后的 schema 事实（V1/V5/V6/V7/V8 标号清楚）；偏离说明诚实详细 |
| 测试覆盖 | 10% | 5 | AC-6 用 `path.exists()` 物理级断言而非仅 row=0；AC-2 用"绕过命令层只剩索引"测试不变式；覆盖 happy-path + missing file + missing asset + via-derivative-id 四象限 |
| 可维护性 | 10% | 5 | `DeleteCascadeReport` 让命令层与单测都拿到结构化结果；doc 注释指出未来若给 source_asset_id 加 FK 可去掉补刀步骤；偏离说明含"为什么"而非"是什么" |

**综合分：4.6/5**（加权：0.25×5 + 0.25×4 + 0.20×4 + 0.10×5 + 0.10×5 + 0.10×5 = 1.25 + 1.0 + 0.8 + 0.5 + 0.5 + 0.5 = **4.55**）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

无 BLOCKER；无 MAJOR；两条 MINOR 不影响合入。

---

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **`delete_with_cascade` 在 db 层做 IO 的 ADR-003 例外未在 doc 中明示**
   - 代码位置：`src-tauri/src/db/asset.rs:290-345`
   - 建议：在函数顶 doc 注释最后追加一句"⚠️ ADR-003 'db 层零 IO' 的有意例外：删除路径的物理文件清理放在 db 层是为满足 session_context §5 '禁止在 commands/ 拼 SQL' 的更强约束。如未来重构出 `services/` 层，可上提此函数。"
   - 验证：reviewer 第二遍读时能直接从 doc 看到取舍依据。

2. **物理文件 permission-denied 分支无测试**
   - 代码位置：`src-tauri/src/db/asset.rs:349-360 remove_file_lenient`
   - 建议：在 `delete_with_cascade_missing_file_is_ok` 之后再加一个 `delete_with_cascade_locked_file_only_warns`（macOS 上可用 `chmod 0000` 父目录或 `O_EXLOCK` 占用文件）—— 当前实现行为已正确（warn + 继续），只是无回归保护。可推迟到 task_009 集成测试覆盖。

3. **`delete_with_cascade` 非单事务，物理文件先删、DB 后删，rollback 不可能**
   - Dev 已在已知局限 §4 自承。当前实现下"残留 pipeline_tasks 行再次调用 delete_with_cascade 会重新清理"的容错路径成立，但若 root DELETE 失败，物理文件已先删 → 出现"DB 行还在但文件不在"的悬挂态。建议未来 task 用 `conn.transaction()` 包住步骤 2–4，把物理文件清理挪到事务 commit 后执行。本 task 范围可接受。

---

## 关键发现给 Conductor

1. **`assets.source_asset_id` 无 FK 约束** 是 V5 ALTER TABLE 的物理后果（SQLite 不支持 ALTER ADD COLUMN 带 FK），Dev 通过显式补刀 DELETE derivative 正确处理。此事实在 architect §六/§十 未明示（只点出 pipeline_tasks 无 FK），建议补入 progress.md 关键决策。未来若想用 FK 自动联动 derivative，需单独 V9 migration 重建表并复制数据 —— 不在本 session 范围。

2. **核心 PRD-S5 "删除资产时源文件与 MD 是否一并清理，是否有孤儿"在本 task 完整落地**：物理文件 + 五张表（assets / pipeline_tasks / conversion_meta / extracted_content / asset_tags） + outbound cache 目录共七类数据，单测以 `path.exists()` 物理级别 + `COUNT(*)=0` 双重断言。session_context §6 该条审查重点视作通过。

3. 老命令 `retrigger_extraction` 保留兼容 task_011 既有调用方；本 task 起新 caller 一律用 `retryAssetConversion`。Conductor 可在 task_010 ux_review 决定是否在后续 PR 中删除旧 wrapper。
