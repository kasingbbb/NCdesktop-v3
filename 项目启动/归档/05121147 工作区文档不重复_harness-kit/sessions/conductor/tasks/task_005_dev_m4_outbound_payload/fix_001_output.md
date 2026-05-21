# task_005 Fix 001 — outbound filename "新名.md.md" 缺陷修复

**模式**：FIX（Dev Worker）
**作用域**：`commands/outbound.rs` + `tests/workspace_unified_md_integration.rs::s2`
**日期**：2026-05-13
**关联**：task_005（M4 outbound payload）/ task_009 dev output.md 第 154-158 行

---

## 修复说明（Fix 模式头部）

### Step 1：根因分析

**问题原因分类**：业务逻辑缺陷（命名拼接顺序错误） + 测试 escape hatch 掩盖问题。

**根本原因**：
- `prepare_outbound_payload` 在解算 `AssetStateInput.display_name` 时取 `asset.name`，
  而 `asset` 来自 `list_root_assets` 的 root 行 —— 即 **root.name**（rename 后形如
  "新名.pdf"）。
- 调用路径：`sanitize_outbound_filename("新名.pdf", id)` → 输出 "新名.pdf"
  （sanitize 规则不剥扩展名），再 `format!("{sanitized}.md")` → **"新名.pdf.md"**。
- 在 task_009 集成测试 s2 中复现：rename root.name="新名.pdf" 后，
  outbound 等价层产出 "新名.md.md"（derivative.name="新名.md" 又被 sanitize + 拼 .md）。
- task_005 的 sanitize 单元测试只验证 stem 层规则，未覆盖"root.name 含扩展名"路径；
  task_009 s2 用注释"除非 sanitize 改写"escape hatch 跳过断言。

**违反约束**：PRD §S2 元数据一致硬约束 —— rename 后 (a) 列表显示 (b) outbound 落盘
文件名 (c) DB display_name 三处必须一致。修复前 (b) ≠ (a)=(c)。

**影响范围**：
- 任何走 outbound 多选拖出的资产文件名均出现重复扩展名（"X.pdf.md" / "X.md.md"）；
- 接收方（ChatGPT / Claude 桌面端）虽仍能识别 `.md`，但显示名错误，降低用户信任；
- 不影响 DB 状态、不影响 derivative 落盘磁盘文件名（`{asset_id}.md`）。

### Step 2：系统性修复

**修复策略**：把"剥离原扩展名 + sanitize stem + 拼 .md"封装为新 helper
`outbound_filename_from_root(root_name, asset_id)`，与 task_004
`commands::asset::derivative_name_from_root` 同源逻辑（但调用 outbound 自有的
PRD §4.4 sanitize 规则，与 `utils::safe_name::sanitize_stem` 的规则集不同，
不强行复用）。

**修改点**：

1. `src/commands/outbound.rs`：
   - 新增长度常量 `MAX_FILENAME_BYTES=200` / `MD_EXT_LEN=3` / `TRUNC_SUFFIX_LEN=9`；
     `MAX_STEM_BYTES = 200 - 3 - 9 = 188`（为 `.md` + 可能的 `_<id8>` 截断后缀留位）。
   - 新增 `pub fn outbound_filename_from_root(root_name, asset_id) -> String`：
     `rfind('.')` 切 stem（首位 `.` 不算分隔） → `sanitize_outbound_filename(stem)` →
     `format!("{stem}.md")`。
   - `prepare_outbound_payload` 的拼接点改为调 `outbound_filename_from_root`，
     不再外部拼 `.md`。
   - 新增 4 个单元测试：strip-ext / no-ext / truncate / slash-in-stem。

2. `tests/workspace_unified_md_integration.rs::s2_rename_writes_root_and_derivative_consistently`：
   - 删除 escape hatch 注释；
   - 强化为正向断言：`outbound_filename_from_root(root_after.name, root_id) == "新名.md"`
     且 `== derivative_after.name`。

**未变更**（按 FIX 范围限定）：
- `sanitize_outbound_filename` 的 6 条 PRD §4.4 规则保持不变（仅截断上限由 200 → 188，
  既有 truncate 测试 `got.len() <= MAX_STEM_BYTES + 1 + 8` 仍通过）；
- `commands/asset.rs::derivative_name_from_root`、`db/asset.rs`、rename 命令均未触碰；
- 不连带重构 `utils::safe_name`（PRD §4.4 与 `sanitize_stem` 规则集差异需独立讨论）。

### Step 3：回归验证

#### 单元测试 `commands::outbound`

```
$ cargo test -p notecapt --lib commands::outbound

running 16 tests
test commands::outbound::tests::outbound_error_serializes_to_camel_case_json ... ok
test commands::outbound::tests::outbound_filename_handles_no_ext ... ok
test commands::outbound::tests::outbound_filename_strips_original_ext_and_appends_md ... ok
test commands::outbound::tests::outbound_filename_sanitizes_slash_in_stem ... ok
test commands::outbound::tests::outbound_filename_truncates_long_stem_with_asset_id_suffix ... ok
test commands::outbound::tests::classify_state_single_non_done_returns_state_not_done ... ok
test commands::outbound::tests::classify_state_mixed_returns_mixed_states_with_offending ... ok
test commands::outbound::tests::sanitize_preserves_cjk_and_emoji ... ok
test commands::outbound::tests::classify_state_all_done_passes ... ok
test commands::outbound::tests::sanitize_replaces_slash_and_backslash ... ok
test commands::outbound::tests::sanitize_trailing_dot_or_space_appends_underscore ... ok
test commands::outbound::tests::sanitize_strips_control_chars_and_del ... ok
test commands::outbound::tests::sanitize_truncates_long_utf8_and_appends_asset_id_suffix ... ok
test commands::outbound::tests::sanitize_windows_reserved_appends_underscore ... ok
test commands::outbound::tests::reset_outbound_dir_is_idempotent_and_empties_existing_files ... ok
test commands::outbound::tests::link_or_copy_rendition_happy_path_creates_file_with_same_content ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 121 filtered out
```

新增 4 个测试 + 既有 12 个测试全 PASS。其中：
- `outbound_filename_strips_original_ext_and_appends_md`：覆盖 "新名.pdf"→"新名.md"、
  "音频笔记.m4a"→"音频笔记.md"、"archive.tar.gz"→"archive.tar.md"（仅剥最后一个 ext）；
- `outbound_filename_handles_no_ext`：覆盖 "笔记"→"笔记.md" 与 ".env"→".env.md"
  （首位 `.` 不退化为空 stem）；
- `outbound_filename_truncates_long_stem_with_asset_id_suffix`：覆盖 100×"好"+.pdf
  → 截断 + `_<id8>` + `.md` + 总长 ≤ 200 字节；
- `outbound_filename_sanitizes_slash_in_stem`：覆盖 "a/b.pdf"→"a_b.md"
  （PRD §4.4 规则仍生效）。

#### 集成测试 `workspace_unified_md_integration`

```
$ cargo test -p notecapt --test workspace_unified_md_integration

running 7 tests
test s2_rename_writes_root_and_derivative_consistently ... ok
test s1_uniqueness_import_files_core_yields_n_root_rows ... ok
test s3_three_states_visible_in_list ... ok
test s4_failed_asset_still_supports_rename_and_tag ... ok
test s5_delete_with_cascade_no_orphans ... ok
test s7_retry_unique_index_caps_active_at_one ... ok
test s8_source_missing_marks_flag_but_state_done_and_outbound_ok ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

s2 escape hatch 已替换为正向断言，PRD §S2 三处一致硬约束在测试层锁定。

---

## handoff_contracts §3 / 交付清单

### 变更文件

| 文件 | 类型 | 摘要 |
|---|---|---|
| `src-tauri/src/commands/outbound.rs` | 修改 | 新增 `outbound_filename_from_root` helper + 4 个测试；改写 `prepare_outbound_payload` 调用点；常量重排 |
| `src-tauri/tests/workspace_unified_md_integration.rs` | 修改 | s2 用 `outbound_filename_from_root` 正向断言 "新名.md"，删除 escape hatch 注释 |

### 未触碰文件（FIX 范围限定）

- `src-tauri/src/commands/asset.rs` —— `derivative_name_from_root` 已正确（task_004）
- `src-tauri/src/db/asset.rs` —— rename 时 derivative.name="新名.md" 本身不算 bug
- `src-tauri/src/utils/safe_name.rs` —— 与 outbound 自有 sanitize 规则集不同，保持独立

### 行为对照

| root.name | 修复前 outbound filename | 修复后 outbound filename |
|---|---|---|
| `新名.pdf` | `新名.pdf.md`（错） | `新名.md` ✓ |
| `音频笔记.m4a` | `音频笔记.m4a.md`（错） | `音频笔记.md` ✓ |
| `笔记`（无 ext） | `笔记.md` ✓（已对） | `笔记.md` ✓ |
| `a/b.pdf` | `a_b.pdf.md`（错） | `a_b.md` ✓ |
| 200+ 字节 stem | 失败 / 超长 | 截断 + `_<id8>.md`，总长 ≤ 200 ✓ |

### 遗留 / 后续建议

- **无功能遗留**。`outbound_filename_from_root` 是 `pub` 且签名清晰，前端 / dropzone
  调用方无需改动（命令仍返回相同 `OutboundEntry` 结构）。
- **建议（非本 Fix 范围）**：未来若把 `utils::safe_name::sanitize_stem` 升级为完整
  PRD §4.4 实现，可让 outbound 与 asset rename 共享同一 helper，消除两套 sanitize
  规则集（当前 outbound 用更严格的 PRD §4.4，asset rename 用 `sanitize_stem`，
  互不冲突但有重复维护成本）。
