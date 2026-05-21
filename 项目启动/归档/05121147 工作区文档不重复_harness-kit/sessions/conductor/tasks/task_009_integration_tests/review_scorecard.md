# Review Scorecard — task_009_integration_tests

## 审查思考过程

1. **Task 意图**：在 `src-tauri/tests/` 下落地 PRD §8 S1–S5 + S7–S8 端到端集成测试 + 10000 规模 list bench；额外补 task_008 MAJOR（3 个剩余 OutboundError 变体的 vitest 参数化）。
2. **AC 检查结果**：
   - AC-S1 唯一性 — 实测 `import_files_core` 真路径 + `OkScheduler` mock，断言 3 创建、failures 空、`source_asset_id IS NULL` ✅
   - AC-S2 元数据一致 — `get_by_id.name="新名.pdf"` + `find_markdown_derivative.name="新名.md"` + sanitize 保留 CJK ✅；**但 rename_asset 命令外壳本体未直调**（见 MAJOR-1）
   - AC-S3 三态可见 — done/converting/failed 三态通过 `compute_asset_state` 派生正确 ✅
   - AC-S4 失败降级 — failed 态下 rename + tag 均 OK、状态仍 Failed ✅
   - AC-S5 删除无孤儿 — 两文件 + outbound cache 目录 + 4 表行数全清；含 FK CASCADE 验证 ✅；命令外壳的 `remove_dir_all` 由测试手工执行（同源逻辑等价）
   - AC-S7 重试无重复 — 5 次直 INSERT，第 2-5 次都 `is_err()` 拦截，活动态恒 1 ✅（**真实击中 V7 部分唯一索引**，非绕过）
   - AC-S8 source 失联 — `scan_with_conn` 返回 missing=1；state 仍 Done；outbound hardlink+copy fallback 成功 ✅
   - AC-Bench — 10k root+derivative+pt+ec+cm 全量构造（确认 5 张表各 10k 行），3 采样取 best=106ms < 200ms warn < 500ms 硬上限 ✅
   - AC-9 — `cargo test ... --test workspace_unified_md_integration` 全过；bench `#[ignore]` ✅
   - **task_008 MAJOR 补全**：`it.each` 3 条变体（MixedStates / RenditionMissing / IoFailed）均断言 `startDrag` 未调用 + 中文 toast title/message ✅
3. **关键发现**：
   - 测试质量高，AC 覆盖完整，bench 真实可信（10k 5 张表 + 3 采样）；S7 真实击中 V7 部分唯一索引（手工 INSERT 不带 IGNORE，UNIQUE 冲突会真抛错），未绕过。
   - **rename_asset / prepare_outbound_payload 命令外壳未直调**：S2/S8 在 db+fs 层验证等价语义，命令外壳由 task_004/005 单测兜底。这削弱了"重命名后 outbound 落盘文件名与 derivative.name 一致"的端到端契约——但 dev 偏离说明充分，task_004 / 005 / 006 已有 30 单测覆盖外壳分支，且 dev 主动暴露了 "新名.md.md" 这个真实的 sanitize+拼接结果（说明已认真核对过外壳行为）。本期接受为已知局限。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 4 | 7 个集成测试 + bench 全部通过；S2/S5/S8 命令外壳本体未直调（等价语义经 db/fs 验证）。S7 真击中 V7 索引，不是手工模拟。 |
| 用户体验 | 25% | 5 | 前端 3 变体 toast 文案逐字断言（"无法拖出"/"多选包含非 done 态资产"/"未找到转化后的 MD 文件"/"拖拽准备失败"/"EXDEV"），完整闭合 task_008 的 MAJOR。 |
| 架构一致性 | 20% | 5 | 全部沿用既有 public API（`import_files_core` / `list_root_assets` / `compute_asset_state` / `delete_with_cascade` / `scan_with_conn` / `outbound_cache_dir_for` / `sanitize_outbound_filename`），零新增 migration、零新依赖。`with_sandboxed_home` 沿用 `workspace_folders_integration.rs` 串行模式。 |
| 代码质量 | 10% | 4 | 脚手架（`materialize_done` / `insert_pipeline_task` / `OkScheduler`）抽象合理；测试名清晰；少量重复的 Asset 构造可抽 helper（minor）。 |
| 测试覆盖 | 10% | 4 | S1–S5+S7–S8 + bench + 3 前端变体齐备；S6（离线批量）按 input.md / architect §八 显式排除，非遗漏。命令外壳端到端是已知缺口。 |
| 可维护性 | 10% | 4 | HOME+XDG_CACHE_HOME 双覆盖 + 串行 Mutex 模式清晰，但 `unsafe` env 块 + 进程级状态使测试天然不并行，未来增至 30+ 测试时需重构（dev 已在"已知局限 §3"标出）。 |

**综合分：4.45/5**（加权计算：4×0.25 + 5×0.25 + 5×0.20 + 4×0.10 + 4×0.10 + 4×0.10 = 1.00 + 1.25 + 1.00 + 0.40 + 0.40 + 0.40 = 4.45）

## 总体判断

- [x] **PASS**

无 BLOCKER，仅 1 个 MAJOR（命令外壳未端到端）已被 dev 主动暴露并由前置 task 单测兜底，符合"已知局限"标准；不阻挡发布。

## 问题列表

### BLOCKER

（无）

### MAJOR

1. **问题**：`rename_asset` / `prepare_outbound_payload` Tauri 命令外壳本身未做端到端集成调用，导致"rename 后 outbound 落盘文件名与 derivative 双写一致"这条端到端契约只在 db+fs 等价语义层被验证。dev 已暴露真实拼接结果（"新名.md.md"），但缺少一个跨命令的回归用例。
   - **代码位置**：`workspace_unified_md_integration.rs:286-348`（S2 等价语义）；`:795-810`（S8 outbound 等价）
   - **修复方向**：**本期接受，不要求立修**。后续 spike：评估 `tauri::test::mock_builder()` + `app.manage(Database)` 注入方案，目标至少新增 1 个跨 rename → prepare_outbound_payload 的端到端用例。建议归入下一迭代的 P1。
   - **验证标准**：（如本期决定修）`rename_asset(new_name)` → `prepare_outbound_payload([root])` 返回 entry，断言 entry.path 文件名包含 sanitize(new_stem) 且文件落盘存在。

### MINOR

1. `unsafe { std::env::set_var }` 块虽来自既有模式，但 Rust 2024 edition 起 `set_var` 标记 unsafe；建议在文件顶部加 `// SAFETY: HOME_LOCK 串行化所有 env 改动，单测序内独占` 注释，已隐含但未显式。
2. S5 测试中 `cache_dir` 由测试手工 `remove_dir_all`，与命令外壳 `commands::asset::delete_asset` 的 `if exists { remove_dir_all }` 是同源 2 行；可考虑把该 2 行抽到 `commands::outbound::cleanup_outbound_cache_dir(asset_id)` 公共函数后，让测试与命令外壳调同一份代码（避免双轨）。
3. Asset 构造体在 5 个测试中重复 ~10 个字段；可抽 `make_root_asset(id, project_id, name) -> Asset` helper（~30 行收益）。
4. S2 测试断言只验 sanitize 保留 "新名"，未验"新名.md.md"这一真实落盘名；dev 已在"已知局限 §4"标注，可在测试源码注释里也明示一行避免 3 个月后读者疑惑。

## 给 Dev 的修复指引

**判定 PASS，无需修复。** 上述 MAJOR-1 接受为已知局限（dev 主动暴露 + 前置 task 单测兜底）；MINOR 1-4 为可选打磨，不阻挡本 task 收尾。建议把 MAJOR-1 的端到端命令外壳测试登记为下一迭代 spike。
