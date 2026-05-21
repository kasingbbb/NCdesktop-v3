# Task 输入 — task_009_integration_tests

## 目标
端到端集成测试覆盖 PRD §8 全部 happy/sad-path 成功标准（S1–S5 + S7–S8），并加入 1 万规模 list 性能 bench（ADR-003 回退条件验证）。

## 前置条件
- 依赖 task：task_002 / task_003 / task_004 / task_005 / task_006 / task_007 / task_008 全部完成
- 必须先存在的文件/接口：所有上述 task 产出的 Rust commands / DB API。

## 验收标准（AC）
1. **AC-S1（唯一性）**：新建 `src-tauri/tests/workspace_unified_md_integration.rs`，测试 `import_files_core` → `list_root_assets` 返回行数 == 导入文件数，且每行 `source_asset_id IS NULL`。
2. **AC-S2（元数据一致）**：`rename_asset(root, "新名.pdf")` 后：
   - `db::asset::get_by_id(root)` 返回 name="新名.pdf"
   - `db::asset::find_markdown_derivative(root)` 返回 derivative.name="新名.md"
   - `prepare_outbound_payload([root])` 返回 path 文件名 sanitize 后包含"新名.md"（除非 sanitize 改写）
3. **AC-S3（三态可见）**：构造三个 asset 分别处于 done / converting / failed，断言 `list_root_assets` 返回三行的 `state` 字段。
4. **AC-S4（失败降级）**：mock 一个 markitdown 永远失败的 asset → scheduler 跑完后 state=failed，调 `rename_asset` / 标签添加 仍成功。
5. **AC-S5（删除无孤儿）**：导入 1 文件 → 等 scheduler 完成 → `delete_asset(root)` → 断言：
   - root 文件 / derivative 文件磁盘上均不存在
   - `assets` / `conversion_meta` / `extracted_content` / `pipeline_tasks` 中该 asset_id 行数为 0
   - outbound cache 目录 `{cache}/NCdesktop/outbound/{asset_id}/` 不存在
6. **AC-S7（重试无重复）**：连续 `retry_asset_conversion(asset)` 5 次 → `pipeline_tasks` 活动态（queued+running）行数 ≤ 1（V7 idx 保证）；`list_root_assets` 行数恒等于导入数。
7. **AC-S8（source 失联）**：导入 → 等 scheduler 完成 → 手工 `fs::remove_file(asset.file_path)`（source 文件，不删 rendition） → 重新调用 `source_scan::scan_all_projects` → `list_root_assets` 返回的该行 `source_missing == true`，state 仍为 done；`prepare_outbound_payload([asset])` 仍成功（rendition 还在）。
8. **AC-Bench**：`tests/bench_list_root_assets.rs`（或 `#[ignore]` 默认跳过的 test）：构造 10000 root + 10000 derivative + 各自一行 pipeline_tasks/extracted_content/conversion_meta；测量 `list_root_assets` 耗时，断言 < 500ms（200ms 是回退阈值，500ms 留余量；超过则在测试输出 warn 提示需升 V9）。
9. **AC-9**：`cargo test -p app_lib --test workspace_unified_md_integration` 通过；bench 默认 `#[ignore]`，手动 `cargo test -- --ignored bench_list_root_assets`。

## 技术约束
- 集成测试使用 `tempfile::tempdir` 隔离的 SQLite 文件 + 工作区目录。
- 不调用真实 markitdown / 讯飞 ASR：通过 `ExtractOptions.markitdown_enabled = false` 强制走 text/markdown extractor，或 mock。
- bench 测试用 `#[ignore]`，CI 默认不跑。
- 测试中文断言一律走 `assert_eq!`，避免依赖 toast / UI 文案。

## 参考文件
- `src-tauri/tests/`（既有结构）
- `task_001_architect/output.md` §八 验收标准对照（PRD §8）
- 所有上述 dev task 的 output.md

## 预估影响范围
- 新建文件：
  - `src-tauri/tests/workspace_unified_md_integration.rs`
  - `src-tauri/tests/bench_list_root_assets.rs`
- 估算变更：~500 行
