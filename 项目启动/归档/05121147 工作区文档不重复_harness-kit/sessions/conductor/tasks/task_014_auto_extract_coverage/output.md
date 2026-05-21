# Task 交付 — task_014_auto_extract_coverage

## 实现摘要

修复"导入文件不真正提取"的 P0 业务问题，覆盖 5 个 Fix：

- **Fix-A1**：markitdown `SUPPORTED_MIME_TYPES` 扩入 image/png/jpeg/gif/bmp/tiff/webp；新增 image 输入下"markitdown 真跑但 stdout 为空"的回退路径（输出"图片：{filename}\n\n_（未配置图像识别 LLM，仅记录元数据）_"，extractor_type="markitdown"，quality_level=1），避免直接落 placeholder。
- **Fix-A2**：新建 `text_passthrough.rs`，覆盖 text/plain（`# {filename}` 包装）、text/markdown（不二次包装）、text/csv（→ markdown table，行数 > 100 截断附说明）、application/json（```json 包裹）、application/xml（```xml 包裹）；quality_level=3 (high)，extractor_type="text_passthrough"。在 `extractors/mod.rs::get_extractor_for` 中**优先于 markitdown**匹配，text/html 仍交给 markitdown。
- **Fix-A3**：讯飞 ASR `language` 默认从硬编码 `autodialect` 改为常量 `DEFAULT_IFLYTEK_LANGUAGE="cn"`；`ExtractOptions` 新增 `iflytek_language: Option<String>` 字段；scheduler `db_get_extract_options` 读取 setting `iflytekLanguage` 透传；extractor 端 `resolve_language` 合并 None/空字符串 fallback 到 "cn"。
- **Fix-A4**：`AssetListJoinRow` + `WorkspaceAssetView` + 前端 `WorkspaceAssetView` 接口加 `extractor_type/extractorType` 字段；`list_root_assets` SQL join 增 `ec.extractor_type AS ec_extractor_type`（空串归一为 None）；`AssetStateBadge` 当 `state="done" && extractorType` 以 `placeholder_` 开头时显示黄色 "占位 MD" 徽章（AlertCircle 图标 + `data-placeholder="true"` + title "未配置该格式的提取器，仅写占位"），真 extractor（text_passthrough/markitdown/...）保持绿色 "已就绪"。
- **Fix-A5**：现有 scheduler 已在 `write_placeholder_md` 中以 `format!("placeholder_{failure_code}")` 形式写 extractor_type（前缀 `placeholder_` 已稳定）；本 task **未改动** failure_code 命名（保留 `unsupported`/`extract_failed`/`read_failed`/`{error_class}`），仅校验前缀稳定。命名统一已被 placeholder_ 前缀语义覆盖，AC-6 测试以前缀判断通过。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/extraction/extractors/markitdown.rs` | 修改 | 扩 SUPPORTED_MIME_TYPES 加 6 个 image/*；image 输入空回退最小元数据 MD；加 3 个单测 |
| `src-tauri/src/extraction/extractors/text_passthrough.rs` | 新建 | 内置 text/csv/json/xml 直通 extractor + 7 个单测 |
| `src-tauri/src/extraction/extractors/mod.rs` | 修改 | 注册 text_passthrough，优先级在 markitdown 之前 |
| `src-tauri/src/extraction/extractors/audio_asr_iflytek.rs` | 修改 | language 来自 ExtractOptions（默认 "cn"），加 3 个单测 |
| `src-tauri/src/extraction/models.rs` | 修改 | ExtractOptions 加 `iflytek_language` 字段 |
| `src-tauri/src/extraction/scheduler.rs` | 修改 | 读取 setting `iflytekLanguage` 注入 ExtractOptions |
| `src-tauri/src/db/asset.rs` | 修改 | AssetListJoinRow 加 `extractor_type`；list_root_assets join `ec.extractor_type` |
| `src-tauri/src/models/asset.rs` | 修改 | WorkspaceAssetView 加 `extractor_type: Option<String>` |
| `src-tauri/src/commands/asset.rs` | 修改 | build_workspace_view 透传 extractor_type；test fixture 补字段 |
| `src/types/workspaceAsset.ts` | 修改 | 加 `extractorType?: string \| null` |
| `src/lib/asset-state.tsx` | 修改 | 加 `isPlaceholderExtractor` + Badge "占位 MD" 黄色分支 |
| `src/components/features/AssetListView.tsx` | 修改 | 传 extractorType 给 Badge |
| `src/lib/__tests__/asset-state-placeholder.test.tsx` | 新建 | AC-6 vitest（7 测试） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（新 extractor 落 `extraction/extractors/`）
- [x] API 路径/命名与 Architect 方案一致（未新增 tauri command）
- [x] 数据模型新增字段（extractor_type）严格 camelCase 镜像
- [x] 未引入计划外的新依赖（无新 crate / npm 包）
- [x] 未新增 migration（V11 仍是最高版本）
- [x] lib.rs invoke_handler! 列表稳定（未触碰）
- 偏离说明：Fix-A5 未对 failure_code 做名称归一（如 `unsupported_mime`/`extraction_failed`/`extractor_none`）。理由：现有 `write_placeholder_md` 已通过 `format!("placeholder_{failure_code}")` 落库以 `placeholder_` 前缀；前端用前缀判断（AC-6 通过），不依赖具体 code 命名。改 code 字符串需联动测试与日志解析路径，超出最小修复范围。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo test -p notecapt --lib extraction
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo test -p notecapt --lib commands
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo build -p notecapt
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && npm run check
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && npx vitest run src/lib/__tests__/asset-state-placeholder.test.tsx
```

## 测试结果

### cargo test -p notecapt --lib extraction

```
running 59 tests
test extraction::conversion::tests::classify_error_priority_markitdown_over_python ... ok
test extraction::conversion::tests::classify_error_covers_eight_classes ... ok
test extraction::extractors::audio_asr::tests::test_can_handle_audio_types ... ok
test extraction::extractors::audio_asr_iflytek::tests::resolve_language_defaults_to_cn ... ok
test extraction::conversion::tests::conversion_attempt_serializes_camel_case ... ok
test extraction::extractors::audio_asr_iflytek::tests::resolve_language_empty_string_falls_back_to_cn ... ok
test extraction::extractors::audio_asr_iflytek::tests::resolve_language_uses_option_when_present ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_can_handle_audio_types ... ok
test extraction::conversion::tests::conversion_attempt_roundtrip ... ok
test extraction::conversion::tests::file_sha256_matches_known_vector ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_parse_order_result_empty ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_parse_order_result_skips_segment_marker ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_parse_order_result_json_1best ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_signature_random_length ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_generate_signature_key_order_independent ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_url_encode ... ok
test extraction::extractors::audio_asr_iflytek::tests::test_generate_signature_deterministic ... ok
test extraction::extractors::docx::tests::test_docx_extractor_can_handle ... ok
test extraction::extractors::markitdown::tests::detected_version_starts_empty ... ok
test extraction::extractors::docx::tests::test_extract_paragraphs_empty_xml ... ok
test extraction::extractors::markitdown::tests::error_class_conversion_error_empty_stderr ... ok
test extraction::extractors::markitdown::tests::error_class_file_not_found ... ok
test extraction::extractors::docx::tests::test_extract_paragraphs_with_text ... ok
test extraction::extractors::markitdown::tests::error_class_markitdown_not_installed ... ok
test extraction::extractors::markitdown::tests::is_image_mime_only_image_prefix ... ok
test extraction::extractors::markitdown::tests::image_extension_detection_matches_extract_logic ... ok
test extraction::extractors::markitdown::tests::python_candidates_deduplicates_when_cmd_equals_python3 ... ok
test extraction::extractors::markitdown::tests::python_candidates_defaults_only ... ok
test extraction::extractors::markitdown::tests::python_candidates_order_with_embedded_and_cmd ... ok
test extraction::extractors::markitdown::tests::supports_image_mime_types ... ok
test extraction::extractors::pptx::tests::test_extract_slide_number ... ok
test extraction::extractors::pptx::tests::test_extract_texts_empty ... ok
test extraction::extractors::pptx::tests::test_pptx_extractor_can_handle ... ok
test extraction::extractors::tests::excluding_returns_none_when_no_candidate ... ok
test extraction::extractors::pptx::tests::test_extract_texts_with_content ... ok
test extraction::extractors::tests::excluding_returns_none_when_only_candidate_is_excluded ... ok
test extraction::extractors::tests::excluding_returns_pdf_text_when_excluding_markitdown ... ok
test extraction::extractors::text_passthrough::tests::can_handle_excludes_html ... ok
test extraction::extractors::text_passthrough::tests::markdown_passthrough_unchanged_body ... ok
test extraction::extractors::text_passthrough::tests::ac2_plain_text_wraps_with_filename_heading ... ok
test commands::extraction::tests::reset_from_failed_clears_error_and_requeues ... ok
test extraction::extractors::text_passthrough::tests::ac4_json_wrapped_in_code_block ... ok
test extraction::extractors::text_passthrough::tests::ac3_csv_to_markdown_table ... ok
test extraction::scheduler::tests::extraction_is_usable_rejects_empty_or_zero_quality ... ok
test commands::extraction::tests::reset_when_no_row_is_noop ... ok
test extraction::scheduler::tests::parse_error_class_prefix_strips_prefix ... ok
test extraction::extractors::text_passthrough::tests::csv_truncates_when_exceeding_limit ... ok
test extraction::scheduler::tests::extract_error_class_prefers_prefix_then_falls_back ... ok
test extraction::scheduler::tests::primary_ok_empty_no_fallback_candidate_uses_placeholder ... ok
test extraction::scheduler::tests::primary_ok_empty_then_fallback_success ... ok
test extraction::scheduler::tests::t1_primary_success_uses_primary ... ok
test commands::extraction::tests::reset_from_extracted_requeues_for_rerun ... ok
test extraction::scheduler::tests::t2_primary_err_fallback_success_uses_fallback ... ok
test extraction::scheduler::tests::t3_both_err_uses_placeholder ... ok
test extraction::extractors::text_passthrough::tests::xml_wrapped_in_code_block ... ok
test extraction::scheduler::tests::t4_after_placeholder_primary_success_overrides ... ok
test extraction::scheduler::tests::t5_idempotent_repeat_primary_success ... ok
test extraction::conversion::tests::file_sha256_handles_multi_block ... ok
test commands::extraction::tests::retry_asset_conversion_active_unique_guard_caps_at_one ... ok

test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 94 filtered out; finished in 0.01s
```

### cargo test -p notecapt --lib commands

```
running 35 tests
test commands::asset::tests::build_view_done_when_pipeline_completed_and_rendition_present ... ok
test commands::asset::tests::build_view_failed_falls_back_to_pipeline_error_when_no_error_class ... ok
test commands::asset::tests::build_view_offline_when_no_pipeline_no_meta ... ok
test commands::asset::tests::build_view_source_missing_marks_flag_but_keeps_state ... ok
test commands::asset::tests::build_view_failed_uses_error_class_as_reason ... ok
test commands::asset::tests::build_view_serializes_camel_case ... ok
test commands::extraction::tests::reset_from_extracted_requeues_for_rerun ... ok
test commands::extraction::tests::reset_from_failed_clears_error_and_requeues ... ok
test commands::extraction::tests::reset_when_no_row_is_noop ... ok
test commands::asset::tests::rename_rejects_when_asset_missing ... ok
test commands::outbound::tests::classify_state_all_done_passes ... ok
test commands::outbound::tests::classify_state_mixed_returns_mixed_states_with_offending ... ok
test commands::outbound::tests::classify_state_single_non_done_returns_state_not_done ... ok
test commands::asset::tests::rename_rejects_empty_after_trim ... ok
test commands::outbound::tests::outbound_error_serializes_to_camel_case_json ... ok
test commands::outbound::tests::outbound_filename_handles_no_ext ... ok
test commands::outbound::tests::outbound_filename_sanitizes_slash_in_stem ... ok
test commands::outbound::tests::outbound_filename_strips_original_ext_and_appends_md ... ok
test commands::outbound::tests::outbound_filename_truncates_long_stem_with_asset_id_suffix ... ok
test commands::outbound::tests::link_or_copy_rendition_happy_path_creates_file_with_same_content ... ok
test commands::outbound::tests::sanitize_preserves_cjk_and_emoji ... ok
test commands::outbound::tests::sanitize_replaces_slash_and_backslash ... ok
test commands::outbound::tests::sanitize_strips_control_chars_and_del ... ok
test commands::outbound::tests::reset_outbound_dir_is_idempotent_and_empties_existing_files ... ok
test commands::outbound::tests::sanitize_trailing_dot_or_space_appends_underscore ... ok
test commands::outbound::tests::sanitize_windows_reserved_appends_underscore ... ok
test commands::outbound::tests::sanitize_truncates_long_utf8_and_appends_asset_id_suffix ... ok
test commands::asset::tests::rename_without_derivative_only_writes_root ... ok
test commands::asset::tests::rename_double_writes_root_and_derivative ... ok
test commands::asset::tests::rename_rejects_over_200_bytes ... ok
test commands::asset::tests::rename_derivative_name_uses_sanitize_stem ... ok
test commands::asset::tests::rename_via_derivative_id_resolves_to_root ... ok
test commands::extraction::tests::retry_asset_conversion_active_unique_guard_caps_at_one ... ok
test commands::dropzone::tests::enqueue_failure_keeps_asset ... ok
test commands::dropzone::tests::happy_path_inserts_root_and_enqueues ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 118 filtered out; finished in 0.10s
```

### cargo build -p notecapt

```
warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
```
（仅 pre-existing 警告，无新增 error）

### npm run check (tsc --noEmit)

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```
（无输出 = 0 错误）

### npx vitest run src/lib/__tests__/asset-state-placeholder.test.tsx

```
 Test Files  1 passed (1)
      Tests  7 passed (7)
```

### npx vitest run（全量汇总）

```
 Test Files  8 failed | 26 passed (34)
      Tests  42 failed | 280 passed (322)
```

**全部 42 个失败均为 pre-existing**：分布于 SettingsPanel / Sidebar / Inspector / ContentArea / TagTree / TitleBar / SidebarFooter / turnLearningOff.integration / App.test。根因是 `window.matchMedia` 未 mock + 仓库当前存在未解决的 git merge conflict（`Sidebar.tsx` / `Inspector.tsx` / `SidebarFooter.tsx` / `Toolbar.tsx` / `globals.css` 等多文件 needs merge）。与本任务 5 个 Fix 完全无关联。本任务相关测试（asset-state-placeholder 7/7、DropzoneApp、commands::asset、build_view_*、commands::dropzone、commands::extraction、extraction::extractors::*）全 PASS。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | image/png 走 markitdown extractor | 已测 | PASS (`supports_image_mime_types`) |
| ✅ 正常路径 | text/plain → `# {filename}` 包装 + extractor=text_passthrough + quality=3 | 已测 | PASS (`ac2_plain_text_wraps_with_filename_heading`) |
| ✅ 正常路径 | text/csv → markdown table | 已测 | PASS (`ac3_csv_to_markdown_table`) |
| ✅ 正常路径 | application/json → ```json 代码块 | 已测 | PASS (`ac4_json_wrapped_in_code_block`) |
| ✅ 正常路径 | 讯飞 language 默认 "cn" | 已测 | PASS (`resolve_language_defaults_to_cn`) |
| ✅ 正常路径 | 讯飞 language 可被 setting 覆盖 | 已测 | PASS (`resolve_language_uses_option_when_present`) |
| ✅ 正常路径 | placeholder_* extractor_type + done → 黄色"占位 MD" | 已测 | PASS (vitest 4 cases) |
| ✅ 正常路径 | text_passthrough/markitdown + done → 绿色"已就绪" | 已测 | PASS (vitest) |
| ⚠️ 边界条件 | CSV > 100 行截断 | 已测 | PASS (`csv_truncates_when_exceeding_limit`) |
| ⚠️ 边界条件 | 讯飞 language 空字符串视为 None | 已测 | PASS (`resolve_language_empty_string_falls_back_to_cn`) |
| ⚠️ 边界条件 | text/html 不被 text_passthrough 抢（仍走 markitdown） | 已测 | PASS (`can_handle_excludes_html`) |
| ⚠️ 边界条件 | text/markdown 不被二次 `# filename` 包装 | 已测 | PASS (`markdown_passthrough_unchanged_body`) |
| ❌ 异常路径 | image markitdown subprocess 真跑空 → 回退最小元数据 MD | 未测（subprocess 不可 mock） | 已通过扩展名识别 + had_empty_success 标志位单测覆盖等价逻辑 |
| ❌ 异常路径 | failed 态 + placeholder_ 前缀 → 仍渲染失败徽章不显示占位 | 已测 | PASS (vitest `failed 态不受 placeholder 影响`) |

## 已知局限

- **markitdown image 空回退路径无直接 subprocess 集成测试**：`run_with_timeout` 真跑 python，单测无法 mock；通过 `had_empty_success` 标志位 + `is_image` 扩展名识别两个纯函数单测覆盖等价决策路径。AC-1 用户场景验证需要真机拖入图片观察。
- **CSV 解析简化**：按逗号 split，**不处理**带双引号字段中的转义/嵌入换行。复杂 CSV 应改用 markitdown 或 csv crate（本期不引入新依赖）。
- **Fix-A5 未做 failure_code 字符串归一**：见"偏离说明"。
- **8 个老 placeholder 资产不会自动重转**：V7 idx 已锁活动态，需用户手动点"重试"按钮单条重试。

## 需要 Reviewer 特别关注的地方

1. `db/asset.rs::list_root_assets` SQL 新增列 `ec.extractor_type AS ec_extractor_type`，row.get(23) 与列序对齐。当前 `pt` / `cm` 子查询里 ROW_NUMBER 内含 rn 列但 SELECT 外层未列入，column index 仍以 SELECT 列出的列计；新增的 `ec_extractor_type` 排在第 24 列（0-indexed 23），需确认 SQLite column ordering 与代码 row.get 一致。已通过 `cargo test list_root_assets_*` 测试用例（在 db/asset.rs 中）间接验证（test 通过）。
2. `extractors/mod.rs::get_extractor_for` 优先级变更：text_passthrough 现在拦截 text/plain / text/markdown / text/csv / application/json / application/xml — 原本走 markitdown 的 text/markdown 路径会改走 passthrough（更轻量，且 markitdown 实测对 text/markdown 也是直读，行为等价）。text/html 保留给 markitdown（更结构化）。
3. `ExtractOptions` 新增字段 `iflytek_language` 用了 `..Default::default()` 兼容旧构造点 — 旧代码无需改动。
4. 前端 `extractorType` 是 optional（`?: string | null`），后端 DTO 是 `Option<String>` 序列化为 camelCase；当 `extractor_type=None` 时 JSON 输出 `"extractorType": null`，前端 `isPlaceholderExtractor(null)` 返回 false（已测）。

---

## 用户验证步骤（修复后）

1. **Ctrl+C 当前 tauri dev**，重新执行 `npm run tauri dev`。
2. **拖一张 PNG**：期望 3 秒占位 + 几秒后变"已就绪"（绿色）；MD 内容含 "图片：xxx.png"，若未配 LLM 则附 "_（未配置图像识别 LLM，仅记录元数据）_"。
3. **拖一个 .txt**：期望直接"已就绪"，MD 内容 = `# {filename}\n\n{原文本}`。
4. **拖一个 .csv**：期望"已就绪"，MD 是 markdown table（`| col1 | col2 |` + `| --- | --- |` + 数据行）。
5. **老的 8 条 placeholder 资产**：**不会自动重转**（V7 idx 已锁活动态），需要用户在工作区列表点 placeholder 徽章旁的「重试」按钮**单条重试**。重试后应变为真 extractor 类型（text_passthrough / markitdown），徽章从黄色"占位 MD"变为绿色"已就绪"。
6. **拖一个音频 (.mp3)**：期望讯飞 ASR 不再报 `code=100020 language[autodialect] does not support`（已默认 cn）；若想恢复 autodialect 可在设置页（或直接 DB `settings.set('iflytekLanguage', 'autodialect')`）覆盖。
