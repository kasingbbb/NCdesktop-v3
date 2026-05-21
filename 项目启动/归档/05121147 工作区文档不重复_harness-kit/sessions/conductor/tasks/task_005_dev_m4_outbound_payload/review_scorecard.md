# Review Scorecard — task_005_dev_m4_outbound_payload

## 审查思考过程

### 1. Task 意图
为多选 done 态资产生成稳定文件名的 `.md` 投影到 `~/Library/Caches/NCdesktop/outbound/{asset_id}/`，
供 tauri-plugin-drag 启动 NSFilenamesPboardType 拖出；非 done / 混合 / rendition 缺失等异常以结构化 JSON 错误抛回前端，禁用 startDrag 并 toast。落盘走 hardlink → 跨卷 copy fallback。

### 2. AC 检查结果（逐条对照代码 + 测试）

| AC | 结论 | 证据 |
|----|------|------|
| AC-1 命令签名 + 结构化错误 JSON 字符串 | ✅ | `outbound.rs:250-254`、`OutboundError::to_json` |
| AC-2 sanitize 6 个规则分支 + 单测 | ✅ | `sanitize_outbound_filename` 顺序：替换→去控制→截断+id8后缀→尾随dot/space→Windows保留→空兜底；6 个分支单测全绿 |
| AC-3 缓存目录路径 + 幂等重建 | ✅ | `CACHE_SUBDIR = "NCdesktop/outbound"`；`reset_outbound_dir` 用 `remove_dir_all`→`create_dir_all`，有 `stale.md` 清空单测 |
| AC-4 hardlink → CrossesDevices fallback copy | ✅ | `link_or_copy_rendition`，与 `dropzone.rs::try_rename_or_copy_remove` 完全同源；Rust 1.94 stable 已稳定该 ErrorKind |
| AC-5 单选 / 多选 / rendition 缺失状态错误 | ✅ | `classify_state` + 第 270-281 行磁盘 stat 检查；3 个 classify 单测 |
| AC-6 lib.rs 注册 + 前端 wrapper | ✅ | `lib.rs:177` 已注册；`tauri-commands.ts:592` `prepareOutboundPayload` + `parseOutboundError` |
| AC-7 cargo test 通过 + ≥ 8 单测 | ✅ | 12 passed; 0 failed |
| AC-8 NSStringPboardType 留 spike | ✅ | 仅写 NSFilenamesPboardType，文件名带 `.md` |

### 3. 关键发现
1. **架构合规性高**：未在 commands 层拼任何 SQL，全部走 `resolve_asset_pair` / `list_root_assets` / `compute_asset_state`；ADR-005 / ADR-007 / ADR-008 严格落实。
2. **EXDEV 探测略缩水**：仅依赖 `ErrorKind::CrossesDevices`，未叠加 `safe_rename.rs::is_exdev` 中 `raw_os_error() == 18` 的兜底。与 `dropzone.rs::try_rename_or_copy_remove` 同源 —— Rust 1.94 已稳定此 ErrorKind，本期可接受；但与 `safe_rename.rs` 存在风格不一致。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 8 个 AC 全部满足；状态四态判定走 compute_asset_state；rendition 双向（DB + 磁盘）校验；sanitize 7 步规则与 PRD §4.4 一致 |
| 用户体验 | 25% | 5 | 错误结构化为联合类型，每个 variant 有中文 `message` 供前端直接 toast；前端 `parseOutboundError` 配套到位；camelCase 字段名前后端对齐 |
| 架构一致性 | 20% | 5 | 严守"不在 commands 拼 SQL"硬约束；ADR-005/007/008 完全落实；未引入新依赖；目录与命名与 Architect 方案 1:1 |
| 代码质量 | 10% | 4 | 函数拆分清晰（sanitize / classify / reset / link 四个纯函数）；文档注释充分；少量 `expect()` 与 hardcoded 重复（`_<id8>` 兜底两处），不影响正确性 |
| 测试覆盖 | 10% | 4 | 12 个单测覆盖 sanitize 6 分支 + 3 个 classify 路径 + cache 幂等 + hardlink happy；跨卷 copy fallback + 端到端 Database 集成留待 task_009（已在 output.md 标注） |
| 可维护性 | 10% | 4 | 模块文档头清晰；`CACHE_SUBDIR` / `MAX_STEM_BYTES` 已抽常量；缺一个对外暴露的 `outbound_cache_dir(asset_id)` 供 task_006 删除级联调用（task_006 需自行复用路径，目前需要硬复制常量字符串） |

**综合分：4.65 / 5**
（0.25·5 + 0.25·5 + 0.20·5 + 0.10·4 + 0.10·4 + 0.10·4 = 1.25 + 1.25 + 1.00 + 0.40 + 0.40 + 0.40 = 4.70）

## 总体判断

- [x] **PASS**

无 BLOCKER；无 MAJOR；2 个 MINOR 可在 task_006 / task_009 中顺手解决。

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **EXDEV 兜底不一致**：`link_or_copy_rendition` 仅匹配 `io::ErrorKind::CrossesDevices`，未叠加
   `raw_os_error() == 18` 兜底；`utils/safe_rename.rs::is_exdev` 已有现成实现。
   - 代码位置：`commands/outbound.rs:191-199`
   - 建议：未来如需降级 Rust 工具链或迁移 Linux，可抽 `utils::safe_rename::is_exdev` 共用。当前 Rust 1.94 stable + dropzone 已用同 pattern，可保留。
2. **缺 task_006 删除级联接入点**：模块未对外暴露 `pub fn outbound_cache_dir(asset_id: &str) -> PathBuf`；
   task_006 实现 `delete_with_cascade` 时需自行复用 `CACHE_SUBDIR` 字符串字面量或重复实现路径计算。
   - 代码位置：`commands/outbound.rs:94-95`（`CACHE_SUBDIR`）+ `:173-175`（`outbound_dir_for` 当前是 `fn`，非 `pub fn`）
   - 建议：把 `outbound_dir_for` 改为 `pub fn outbound_cache_dir(asset_id: &str) -> Option<PathBuf>`（内部调用 `dirs_next::cache_dir()`），供 task_006 调用以保持单一来源。**本任务可不修，task_006 review 时再追责该 task 自行接入。**
3. **`sanitize_truncates_long_utf8_and_appends_asset_id_suffix` 未断言后缀长度恰好 9（`_` + 8 char）**：
   测试中 id8 取的是 `asset_id.chars().take(8)` 而非字节，如果 asset_id 含多字节字符，长度上限断言可能溢出。当前生产 asset_id 都是 hex，不会触发；记录在案。
   - 代码位置：`commands/outbound.rs:133` `asset_id.chars().take(8)`

## 给 Dev 的修复指引

PASS — 不需要回炉。建议把 MINOR-2（暴露 `outbound_cache_dir`）作为 task_006 的输入条件之一传递给下一轮 Conductor。

## 审查前验证结论

- [x] 测试结果存在且非空（12 passed, 0 failed）
- [x] 自测验证矩阵完整（含 ✅ / ⚠️ / ❌ 三类）
- [x] 架构遵守声明已填写（ADR-005/007/008 三项均逐条对应）
- [x] 已 diff 核对实际代码（`outbound.rs` / `commands/mod.rs` / `lib.rs:177` / `tauri-commands.ts:568+`）

---

## FIX_001 复审（2026-05-13）

### 1. 修复审查思考

#### 1.1 根因修复正确性（`outbound_filename_from_root`）

| 项 | 实现 | 判定 |
|---|---|---|
| `rfind('.')` 切 stem，首位 `.` 不算分隔 | `Some(idx) if idx > 0 => &root_name[..idx]` | ✅ ".env" / "笔记" 保持原样 |
| stem 走 PRD §4.4 sanitize | `sanitize_outbound_filename(stem_raw, asset_id)` | ✅ 6 条规则不变 |
| 长度预算 188 stem 字节 | `MAX_STEM_BYTES = 200 - 3 - 9 = 188` | ✅ 注释与算式一致 |
| 截断对齐字符边界 | `while cut > 0 && !buf.is_char_boundary(cut) { cut -= 1; }` | ✅ 沿用既有逻辑 |
| 拼 `.md` | `format!("{stem}.md")` | ✅ |
| 多 `.` 仅剥最后一个 | `rfind` | ✅ "archive.tar.gz" → "archive.tar.md" 已测 |

#### 1.2 PRD §S2「三处一致」语义判定（关键）

PRD §S2 原文：
> rename 后 (a) 列表显示，(b) outbound 落盘文件名（经 sanitize），(c) DB display_name 三处一致。

**关键判定：「一致」= stem 一致（不是完整 filename 一致）。**

证据链：
1. (a) 列表显示来源 = `list_root_assets` 返回的 `asset.name`，rename 后为 `"新名.pdf"`（保留用户指定的扩展名口径，task_004）。
2. (c) DB `display_name` = root.name 列，rename 写入 `"新名.pdf"`。
3. (b) outbound 落盘文件名 = `outbound_filename_from_root(root.name, id)` = `"新名.md"`（fixed extension `.md`，PRD §4.4 + task_004 决议：内容是 markdown，扩展名必须 `.md` 才能被 ChatGPT/Claude 桌面端识别）。

如果按"完整 filename 一致"解读，(b) 永远 ≠ (a)=(c)，PRD 自相矛盾。因此 PRD §S2「一致」唯一自洽解读 = **stem 一致**：用户改名 `"新名"` 后，在三处都能看到 `"新名"` 作为可识别 stem，扩展名差异（pdf vs md）是技术细节，对用户心智无影响。

**FIX_001 当前断言 `outbound_filename == derivative.name == "新名.md"` 与 `root_after.name == "新名.pdf"` 并存——stem 都是 "新名"，三处一致（stem 语义）达成。**

附注：fix_001_output.md 第 27-28 行写"(b) ≠ (a)=(c)，修复前"——这一表述在 stem 语义下精确（修复前 (b)="新名.pdf.md" stem 是 "新名.pdf" ≠ "新名"）。语义判定与 dev 的实际修复方向一致，仅文字未点破。

#### 1.3 集成测试 s2 强化

| 检查项 | 状态 |
|---|---|
| escape hatch 注释已删除 | ✅（第 331-333 行改为正向描述 + PRD §S2 引用） |
| 正向断言 `outbound_filename == "新名.md"` | ✅ 第 338-341 行 |
| 交叉断言 `outbound_filename == derivative_after.name` | ✅ 第 342-345 行 |
| 三处一致语义注释清晰 | ✅ "list / outbound 文件名 / DB display_name" 在源码注释 |

s2 不再放过 "新名.md.md" 等错误产物。

#### 1.4 回归

- 原 12 个 outbound 单测 → 16 passed，0 failed（fix_001_output.md 第 91 行）。常量上限 200 → 188 后，既有 `sanitize_truncates_long_utf8_and_appends_asset_id_suffix` 仍 PASS（断言 `got.len() <= MAX_STEM_BYTES + 1 + 8`，是 var 不是字面量 200，自然适配）✅。
- workspace_unified_md_integration 7 测全绿 ✅。

#### 1.5 新增 4 测试质量

| 测试 | 覆盖核心场景 | 判定 |
|---|---|---|
| `outbound_filename_strips_original_ext_and_appends_md` | "新名.pdf"/"音频笔记.m4a"/"archive.tar.gz" 三例 | ✅ 直接打中根因 |
| `outbound_filename_handles_no_ext` | "笔记" + ".env"（首位 `.` 边界） | ✅ 边界覆盖 |
| `outbound_filename_truncates_long_stem_with_asset_id_suffix` | 100×"好"+.pdf → 总长 ≤ 200，body 仅含 "好" | ✅ 长度预算 + UTF-8 边界 |
| `outbound_filename_sanitizes_slash_in_stem` | "a/b.pdf" → "a_b.md" | ✅ 验证 stem-level sanitize 仍生效 |

未额外覆盖但可接受的次要边界：
- 仅扩展名 `"."` → `rfind('.')` 命中 idx=0，走 `_` 分支，stem 保留 `"."`，sanitize 后 trailing-dot 兜底成 `"._"`，最终 `"._.md"`。无单测但行为合理。
- 大小写扩展名如 `"X.PDF"` → 与小写同（剥后 stem="X"），逻辑对称。

#### 1.6 代码质量

- helper 命名 `outbound_filename_from_root` 清晰，与 `commands::asset::derivative_name_from_root` 同源命名风格 ✅。
- 文档注释含"不要把 derivative.name 传进来"warning，避免调用方误用 ✅。
- 常量分解 `MAX_FILENAME_BYTES / MD_EXT_LEN / TRUNC_SUFFIX_LEN / MAX_STEM_BYTES`，可读性高 ✅。
- 未连带重构 `utils::safe_name::sanitize_stem`（保持 FIX 范围限定）✅。

### 2. FIX 评分

| 维度 | 权重 | 分数 | 说明 |
|---|---|---|---|
| 根因定位 | 25% | 5 | 准确识别 sanitize 不剥扩展名 + 命令层外拼 .md 是错点；PRD §S2 违反归因清晰 |
| 修复完备性 | 25% | 5 | helper 封装、4 个边界单测、s2 escape hatch 转正向断言三管齐下 |
| 回归保护 | 20% | 5 | 16 单测 + 7 集成全绿；既有断言用变量自适配 188 |
| 范围克制 | 15% | 5 | 仅改 outbound.rs + s2 测试；未连带动 safe_name / asset.rs / db |
| 文档/注释 | 10% | 4 | 行为对照表 + 流程注释充分；唯一遗漏：未在代码或 output.md 明示「PRD §S2 三处一致 = stem 一致」的语义判定 |
| 后续治理 | 5% | 4 | 后续建议（合并两套 sanitize 规则集）标注在 "遗留" 段，方向正确 |

**FIX 综合分：4.85 / 5**

### 3. 判定

- [x] **PASS**

无 BLOCKER；无 MAJOR；1 个 NIT。

### 4. 问题列表

#### BLOCKER
（无）

#### MAJOR
（无）

#### MINOR
（无）

#### NIT
1. **PRD §S2 语义判定未文字化**：fix_001_output.md 与代码注释均默认"三处一致"按 stem 解读，但未点破 (a)/(c) 是 "新名.pdf"、(b) 是 "新名.md"——理论上 reader 可能困惑"既然 (b) ≠ (a)，怎么算一致"。建议在 `outbound_filename_from_root` 文档注释或 s2 集成测试注释中加一行：「PRD §S2『一致』指 stem 一致（用户改名后的可识别部分），扩展名由各层用途决定（root=用户原扩展名 / outbound=固定 `.md`）」。**非阻塞**。

### 5. 给 Conductor 的回报

修复方向、执行质量、测试强化均合格，可关闭 task_005 / task_009 之间的 "新名.md.md" 缺陷工单。建议在归档时把 PRD §S2 语义判定（stem 一致而非完整 filename 一致）写入 ADR 或 PRD 脚注，避免未来 reviewer 重复争议。

### 6. 审查前验证结论

- [x] FIX 测试结果存在且非空（16 + 7 passed, 0 failed）
- [x] 修改文件清单 vs 实际 diff 一致（outbound.rs + workspace_unified_md_integration.rs）
- [x] 新增测试已逐个核对覆盖目标
- [x] PRD §S2 原文已回查（prd v1 第 139 行）
