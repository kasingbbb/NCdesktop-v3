# Review Scorecard — task_003_dev_m1m2_list_state

## 审查思考过程

1. **Task 意图**：落地"工作区列表唯一查询入口 `list_root_assets`"（ADR-002，杜绝 root + derivative 双条目）+ "四态实时派生 `compute_asset_state` 纯函数"（ADR-003），并把 `get_assets` 命令切流到新 DTO `WorkspaceAssetView`；保持 db/ 层零 IO、commands/ 层做 stat。

2. **审查前验证（交付完整性）**：
   - [x] 测试结果真实粘贴（cargo test 18 个 db::asset + 6 个 commands::asset 全 PASS；本地复跑确认）
   - [x] 自测验证矩阵存在且正常路径全部 PASS（14 行场景）
   - [x] 架构遵守声明已填写，含 2 处偏离说明
   - [x] cargo build --lib 通过（仅 5 个无关 warning，与本 task 无关）

3. **AC 检查结果**：
   - **AC-1**（list_root_assets 单查询 + 窗口函数取最近一条）：✅。SQL 用 `ROW_NUMBER() OVER (PARTITION BY ... ORDER BY created_at/converted_at DESC, rowid DESC)` 子查询拿最近一条 pipeline_tasks / conversion_meta；LEFT JOIN canonical markdown 衍生件（`asset_type='markdown'`）取 rendition_id/path/size；LEFT JOIN extracted_content；`WHERE source_asset_id IS NULL AND project_id=?1`；`ORDER BY imported_at DESC`。**确实是单查询**（无 N+1）。`list_root_assets_joins_latest_pipeline_and_conversion_meta` 测试用同 asset_id 两条记录交叉验证"取最新"正确。
   - **AC-2**（compute_asset_state 纯函数 + 优先级）：✅。规则 1→2→3→4 实现正确；source-missing 显式忽略（`_source_exists` / `_source_missing_known` 加下划线前缀）；`compute_state_completed_without_rendition_falls_through` 覆盖兜底；8 个组合（done / converting×2 / failed×2 / offline×2 / source-missing 不变 / completed-without-rendition）全过。
   - **AC-3**（DTO 与前端镜像）：✅。Rust `WorkspaceAssetView` 字段 ↔ TS `workspaceAsset.ts` 字段完全对齐（id/projectId/assetType/name/originalName/filePath/fileSize/mimeType/capturedAt/importedAt/sourceType/sourceData/isStarred/derivativeVersion/renditionId/renditionPath/renditionSize/state/stateReason/sourceMissing）；`AssetState` lowercase 序列化已由 `build_view_serializes_camel_case` 验证。
   - **AC-4**（命令层 stat + try_state）：✅。`Path::exists()` 在 commands/asset.rs，db/ 内零 IO（搜索过 db/asset.rs 无任何 `Path::` / `fs::`）；`app.try_state::<SourceMissingSet>()` 容忍未注册；锁在调 list_root_assets 后立即 drop，后续 stat 不持锁（已避免跨 await 持锁——本函数同步，无 await）。
   - **AC-5**（get_by_project deprecated）：✅。`#[deprecated(note = "工作区列表请使用 list_root_assets...")]` 加齐 doc；非工作区残留 caller 仅 2 处（`commands/export.rs`、`commands/extraction.rs::extract_project_assets`），就地 `#[allow(deprecated)]`。两处都不是工作区列表路径。
   - **AC-6**（≥ 6 单测 + 全过）：✅。db::asset 18 个测试 + commands::asset 6 个测试，list_root_assets 6 个场景（空 / 排序 / 排除 derivative / 项目隔离 / 多状态混合 / 最近一条 join），compute_asset_state 8 组合，建 view 6 组。

4. **领域审查重点逐条核对**（session_context §6）：
   - "工作区列表是否会因 join 不当导致同一资产出现两行" → **没有**。SQL 用 `root.source_asset_id IS NULL` 过滤、LEFT JOIN derivative 不会膨胀行（同 root 唯一 markdown 衍生件由 ADR-001 保证；pt/cm 子查询 `rn=1` 已保证每 asset_id 至多一行）。`list_root_assets_excludes_markdown_derivative` 单测直接验证。
   - "重命名/打标签作用于 Asset 而非 Conversion" → 不在本 task 范围（task_004）。
   - "三态 UI 是否有明确呈现" → DTO 字段（state + stateReason + sourceMissing）足够支撑 task_008 渲染四态。
   - "拖拽 outbound payload 始终指向 MD" → 不在本 task 范围（task_005）。
   - "删除资产源文件 + MD 一并清理" → 不在本 task 范围（task_006）。

5. **关键发现**：
   - **正面**：SQL 设计正确，窗口函数 + rowid DESC 二级排序确保同时刻插入仍可稳定取最新；`compute_asset_state` 纯函数化彻底，单测覆盖率高；db / commands 边界严格守住（db 层零 IO 可被 grep 验证）；DTO 双侧对齐零偏差。
   - **可改进**：`compute_asset_state` 签名保留 `source_exists` / `source_missing_known` 但完全忽略（下划线前缀），属于"为 AC 签名一致性付出的代价" —— 未来若改派生逻辑要利用 source 状态，签名已就位（可以视作正向决策，但建议在函数文档已显式说明，OK）。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~AC-6 全达成；单查询保证；窗口函数选最新策略正确；不会出现双条目；空项目/项目隔离/混合态全覆盖 |
| 用户体验 | 25% | 4 | DTO 字段集合（state + stateReason + sourceMissing + rendition*）足以支撑 task_008 四态徽章与"源缺失"提示；本 task 自身无 UI，扣 1 是因为 `state_reason` 仅在 Failed 时填充，offline/源缺失态依赖前端在 task_008 自行组合中文文案，跨 task 依赖未在交付物中固化文案契约 |
| 架构一致性 | 20% | 5 | ADR-002 / ADR-003 / §六 数据模型 100% 兑现；db/ vs commands/ 边界清晰（零 IO）；命名严格按方案；偏离 2 处均为前瞻性微调，不破坏架构 |
| 代码质量 | 10% | 5 | 命名清晰、注释翔实（含 ADR 引用与"为什么这样做"）；列序对齐 ROOT_ASSET_COLS 与 row_to_asset 双向可追；常量提取（ROOT_ASSET_COLS / ASSET_SELECT）；纯函数抽离 build_workspace_view 利于单测 |
| 测试覆盖 | 10% | 5 | 24 个新测试覆盖 SQL join / 排序 / 项目隔离 / 状态派生 8 组合 / view 拼接 6 组 / 序列化 camelCase；噪声数据（其他 root 的 derivative、非 markdown 的 derivative）也纳入用例 |
| 可维护性 | 10% | 4 | 文档化优秀；`SourceMissingSet` 放在 commands::asset 而非 source_scan.rs 是已声明的临时位置，task_007 迁移时需注意；`compute_asset_state` 保留 2 个未用参数对未来读者略费解（已用注释解释） |

**综合分：4.75/5**（加权计算：0.25×5 + 0.25×4 + 0.20×5 + 0.10×5 + 0.10×5 + 0.10×4 = 1.25 + 1.00 + 1.00 + 0.50 + 0.50 + 0.40 = 4.65）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **`SourceMissingSet` 暂放在 `commands::asset` 而非 `source_scan.rs`**：偏离说明合理，但建议在 task_007 落地时显式迁移并在迁移 PR 描述中标注"消除偏离"。当前实现零破坏，task_007 可直接迁移；可维护性扣 0.5 分体现于此。
2. **`compute_asset_state` 的 `_source_exists` / `_source_missing_known` 完全未使用**：保留是为了 AC-2 签名一致性，已通过文档说明"source-missing 不改变 state"。若 task_008 在 UI 上不会反映"源缺失但 rendition 仍在但 pipeline 跑过的灰态"，可以考虑未来 cleanup 删除这两个参数。本期保留是合理决策。
3. **`extract_project_assets` 仍走 `get_by_project` 会把已有 markdown derivative 再次入队 extraction**：不在 task_003 范围，但既然 `#[allow(deprecated)]` 注释写"包含 derivative 也无副作用"，建议在 task_006 / task_009 集成测试中显式验证 derivative 重入队幂等。已记入 review 通知，不影响本 task PASS。
4. **`get_assets` 的 `AppHandle` 注入虽是 Tauri 标准模式，但前端 invoke 时 Tauri 自动注入 AppHandle，不影响 JS 调用方**。Dev 偏离说明 (b) 正确。建议在 task_009 端到端测试中显式验证一次 `try_state::<SourceMissingSet>()` 返回 None 的运行时路径（已纳入"已知局限 1"）。

## 偏离判断

- **偏离 (a) — `SourceMissingSet` 类型放在 commands::asset**：判定**合理**。input.md 明示 task_007 才落地启动期扫描；本 task 仅需"接受 Option<State<SourceMissingSet>> 兼容尚未注册场景"，类型骨架先行不破坏架构。task_007 迁移到 `source_scan.rs` 时零接口变更（pub use 即可）。
- **偏离 (b) — `get_assets` 签名新增 `AppHandle`**：判定**合理且必要**。`try_state` 需要从 AppHandle 拿 State；Tauri 自动注入 AppHandle，前端 invoke 签名不变，无破坏性影响。

## 给 Dev 的修复指引

无修复要求（PASS）。task_007 在迁移 `SourceMissingSet` 类型到 `source_scan.rs` 时需要更新 `commands::asset::get_assets` 的 `use` 路径（保持 `pub use` 转出可向后兼容）。
