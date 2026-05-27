# Task 交付 — task_013_frontmatter_writer

## 实现摘要

新建 `src-tauri/src/kc/frontmatter.rs`，实装 KC frontmatter writer：把 `&Asset` + `&ExtractionResult` + `&KcMeta` 序列化为衍生件 `.md` 头部的 YAML frontmatter 块（5 个 NC schema 主键 + 10 个 KC 扩展字段）。

**核心 API**：
- `pub fn build_kc_frontmatter(asset: &Asset, raw: &ExtractionResult, meta: &KcMeta) -> String`
  - 返回 `---\n<YAML>\n---`（**不含**尾部 `\n\n`，由调用方拼接 body；与 `enrichment::join_frontmatter_body` 配合）。
  - 字段顺序与 Architect output.md §"Frontmatter Schema（衍生件 .md 头部）" 严格一致：5 NC + 10 KC。

**核心设计决策**：

1. **手动 YAML 序列化，不引入 `serde_yaml`**：
   - 字段集合极简（仅 string / number / string[]）+ 字段顺序固定 + 字面值需逐位与 task_017 前端 snake_case 严格对齐——手写比 serde-derive 更可控。
   - 避免新增依赖触发 Cargo.lock 全量解析（兼顾"cache 已预热，不要 cargo full build"约束）。
   - 转义 / block scalar 等 corner 由 11 个单测显式覆盖。

2. **字段映射**（与 task_017 前端 `parseFrontmatter.ts::mapToCamelCase` 一一对齐）：

   | YAML key | 来源 | 类型 |
   |--|--|--|
   | `source_asset_id`    | `asset.id`                            | string |
   | `derivative_version` | `asset.derivative_version + 1`        | number |
   | `extracted_at`       | `chrono::Utc::now().to_rfc3339()`     | string |
   | `extractor_type`     | 推断（tags_source）                    | string |
   | `quality_level`      | `raw.quality_level`                   | number |
   | `kc_doc_id`          | `meta.doc_id`                         | string |
   | `kc_generated_at`    | `meta.generated_at`                   | string |
   | `kc_version`         | `meta.kc_version`                     | string |
   | `kc_tags_source`     | `meta.tags_source.as_str()`           | string |
   | `kc_enriched`        | 推断（tags_source）                    | string |
   | `ai_tags`            | `meta.ai_tags`                        | string[] |
   | `rule_tags`          | `meta.rule_tags`                      | string[] |
   | `ai_summary`         | `meta.ai_summary`（None 时跳过）        | string（block scalar 多行 / quoted 单行） |
   | `ai_qa_pairs_count`  | `meta.ai_qa_pairs.len()`              | number |
   | `paragraph_count`    | `meta.paragraph_count`                | number |

3. **`extractor_type` / `kc_enriched` 由 `tags_source` 隐式推断**：
   - `AiAndRule` → `extractor_type = "markitdown+kc"` / `kc_enriched = "true"`（Success 路径）
   - `RuleOnly`  → `extractor_type = "markitdown+kc:partial"` / `kc_enriched = "partial"`（PartialLlmUnavailable 路径）
   - **不变量**：本 builder 不被 Fallback 路径调用（`enrichment::resolve_outcome` Fallback 时直接用 `raw.structured_md` 不拼 frontmatter），故没有 `"false"` 情况进入。这与 `enrichment.rs::resolve_outcome` 中 `ResolvedEnrichment.extractor_type` 字面严格 round-trip。

4. **多行字符串用 YAML block scalar `|` 风格**：
   - `ai_summary` 含 `\n` 时走 `|` block scalar（每行 2 空格缩进），保留真实换行；
   - 单行走双引号 escape 风格（避免 plain style 与 YAML 保留字 `yes` / `null` / `true` 等冲突）；
   - CRLF（`\r\n`）也会触发 block scalar（基于 `\n` 检测）。

5. **空数组用 flow style `[]`**：保证 YAML 字面紧凑 + 与前端 `Array.isArray` 判定兼容。

6. **YAML 双引号 escape 规则**（参考 YAML 1.2 spec §5.7）：
   - `\` → `\\`，`"` → `\"`，`\n` → `\n`，`\r` → `\r`，`\t` → `\t`
   - 其他 C0 控制字符（U+0000-U+001F）→ `\xNN`
   - 中文 / emoji / `:` / `#` / `'` / 括号等 → 双引号包裹下原样保留

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src-tauri/src/kc/frontmatter.rs` | 新建 | KC frontmatter writer（443 行含 11 个单测） |
| `src-tauri/src/kc/mod.rs` | 修改（+1 行） | 注册 `pub mod frontmatter;` |

**Cargo.toml 改动**：无（手动序列化，未引入 `serde_yaml`）。

**未触及**：
- `src-tauri/src/kc/enrichment.rs`（task_011 范围）
- `src-tauri/src/kc/errors.rs`（task_005 范围）
- `src-tauri/src/extraction/scheduler.rs`（task_012 后续接入）

## 对 Architect 方案的遵守声明

- [x] 文件归属与 Architect 方案一致（沿用 `src-tauri/src/kc/` 模块根，与 `enrichment.rs` / `errors.rs` 同级单文件入口）。
- [x] API 命名与 input.md 一致（`build_kc_frontmatter` 与 input.md AC-2 命名一致；签名 `(asset, raw, meta)` 与 user instructions 一致）。
- [x] 数据模型与 Architect 方案一致（消费 `KcMeta` 11 字段；不新增类型）。
- [x] **字段映射与 Architect output.md §"Frontmatter Schema（衍生件 .md 头部）" 字面对齐**（5 NC 主键 + 10 KC 扩展）。
- [x] **字段名 / 大小写与 task_017 前端 `parseFrontmatter.ts::mapToCamelCase` whitelist 严格 round-trip**（snake_case YAML key，前端转 camelCase 在白名单内）。
- [x] 未引入计划外的新依赖（**不引入 `serde_yaml`**，手动序列化由单测守护）。

**偏离说明**：
- **签名取舍**：input.md "目标"段提到"扩展 `scheduler.rs::build_frontmatter`"，但 user instructions 与"约束"明确"新建 `src-tauri/src/kc/frontmatter.rs`、不动 `scheduler.rs`（task_012 范畴）"。本 task 按 user instructions 优先：新建独立模块，签名 `(asset, raw, meta)`，让 task_012 在 scheduler 改造时用闭包 `|meta| build_kc_frontmatter(asset, raw, meta)` 适配 `enrichment::resolve_outcome` 的单参数 `frontmatter_writer` 接口。
- **未引入 `serde_yaml`**：input.md "技术约束"提示"推荐 serde_yaml 或手动 escape（手动则覆盖 5+ 字符 case 测试）"。本 task 选**手动序列化**（覆盖 11 单测含 5+ 字符 case），换取零新增依赖 + 字面值完全可控。
- **frontmatter 字段集合**：Architect schema 不含 `response_size_bytes` / `duration_ms` / `ai_paragraph_links_count`（这些是 `conversion_meta` DB 列内容，task_015 范畴），故本 task 严格遵守 Architect schema 不写这些字段（user prompt 中曾提示可写 `ai_paragraph_links` 计数，但前端 task_017 `parseFrontmatter.ts` 不解析此字段——为避免前端静默丢字段 / schema 漂移，本 task 不写）。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test --lib kc::frontmatter   # 11 新测试
cargo test --lib                   # 整体回归
```

## 测试结果

**`cargo test --lib kc::frontmatter`**：

```
running 11 tests
test kc::frontmatter::tests::build_kc_frontmatter_escapes_tab_and_control_chars_in_single_line_summary ... ok
test kc::frontmatter::tests::build_kc_frontmatter_handles_emoji_and_high_unicode ... ok
test kc::frontmatter::tests::build_kc_frontmatter_partial_no_ai_summary ... ok
test kc::frontmatter::tests::build_kc_frontmatter_derivative_version_incremented ... ok
test kc::frontmatter::tests::build_kc_frontmatter_escapes_asset_id_special_chars ... ok
test kc::frontmatter::tests::build_kc_frontmatter_empty_arrays_serialize_as_empty_list ... ok
test kc::frontmatter::tests::build_kc_frontmatter_crlf_summary_uses_block_scalar ... ok
test kc::frontmatter::tests::build_kc_frontmatter_multiline_summary_uses_block_scalar ... ok
test kc::frontmatter::tests::build_kc_frontmatter_escapes_special_chars ... ok
test kc::frontmatter::tests::build_kc_frontmatter_field_order_is_stable ... ok
test kc::frontmatter::tests::build_kc_frontmatter_success_full_meta ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 496 filtered out; finished in 0.00s
```

**`cargo test --lib`（整体回归）**：

```
test result: ok. 507 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.14s
```

总数：**507 = baseline 496 + 本 task 新增 11**，0 失败，0 跳过，**0 退化**。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---------|----------|------|-----------|
| 正常路径 | AC-1：完整 meta（全部 11 KcMeta 字段有值）→ 15 个 frontmatter 字段全输出 | 已测 | `build_kc_frontmatter_success_full_meta` PASS |
| 正常路径 | AC-2 / AC-3：RuleOnly 模式 → 跳过 ai_summary + 空 ai_tags + `kc_enriched: "partial"` | 已测 | `build_kc_frontmatter_partial_no_ai_summary` PASS |
| 正常路径 | AC-3：title / summary 含 `:` `#` `"` `'` `\` 等 YAML 特殊字符 | 已测 | `build_kc_frontmatter_escapes_special_chars` PASS |
| 正常路径 | AC-4：多行 ai_summary 走 block scalar `|`，非 `"`-quoted `\n` escape | 已测 | `build_kc_frontmatter_multiline_summary_uses_block_scalar` PASS |
| 边界条件 | 空 ai_tags / rule_tags → 序列化为 `[]`（flow array） | 已测 | `build_kc_frontmatter_empty_arrays_serialize_as_empty_list` PASS |
| 边界条件 | emoji + 高 Unicode 码点（U+1F389 等）→ 原样保留 | 已测 | `build_kc_frontmatter_handles_emoji_and_high_unicode` PASS |
| 边界条件 | CRLF（`\r\n`）summary 触发 block scalar（基于 `\n` 检测） | 已测 | `build_kc_frontmatter_crlf_summary_uses_block_scalar` PASS |
| 边界条件 | asset.id 含 `:`（如 `"asset:with:colon"`）→ 双引号包裹安全 | 已测 | `build_kc_frontmatter_escapes_asset_id_special_chars` PASS |
| 边界条件 | derivative_version 进位（99 → 100） | 已测 | `build_kc_frontmatter_derivative_version_incremented` PASS |
| 边界条件 | 单行 summary 含 tab + control char（U+0007） → escape 为 `\t` + `\x07` | 已测 | `build_kc_frontmatter_escapes_tab_and_control_chars_in_single_line_summary` PASS |
| 不变量守护 | 15 个字段顺序严格固定（防 IDE 自动排序 / 维护回归） | 已测 | `build_kc_frontmatter_field_order_is_stable` PASS |
| 集成 | task_012 scheduler 接入后端到端拼接 frontmatter + body | 未测 | 由 task_012 / task_023 e2e 覆盖；本 task 仅守 build 出口 |
| 集成 | 前端 task_017 `parseFrontmatter.ts` 真实解析本函数输出 | 未测 | 前端单测在 NCdesktop/src/utils/__tests__/parseFrontmatter.test.ts；本 task 通过字段名 / 类型守护间接保证 round-trip |

## 已知局限

1. **未实测前端 round-trip**：本 task 仅通过 task_017 `parseFrontmatter.ts::mapToCamelCase` 函数白名单字段一一对齐保证 schema 兼容性，没有真正跑前端单测去 parse 本函数输出。如 Reviewer 担心字段漂移，可在 task_018 / task_019（Inspector / DocumentViewer 渲染）的集成测试里加一个 fixture：用 build_kc_frontmatter 输出真实字符串，写入临时 .md，前端解析后断言 15 个字段都 round-trip。

2. **block scalar `|` 风格的 leading whitespace 边界未覆盖**：当 `ai_summary` 多行内容某行**以空格开头**时，YAML block scalar 的"自动 indent indicator"可能引发歧义（YAML 1.2 spec §8.1.1.1）。本实装强制 2 空格缩进每行，对于"行首额外有空格"的内容，前端解析时缩进会被保留（不会丢字符）。这是有意行为：保留原文本忠实度。如有特定场景失败，可改为 `|2` 显式 indent indicator。

3. **不写 `response_size_bytes` / `duration_ms` 到 frontmatter**：这两个字段是 `conversion_meta` DB 列内容（task_015 范畴），不进衍生件 frontmatter（Architect schema 不含）。如未来产品决策"前端也要显示耗时"，需同时改 build_kc_frontmatter + task_017 前端 schema。

4. **`extractor_type` / `kc_enriched` 字面隐式映射到 `tags_source`**：本 builder 不接受显式的 `extractor_type` / `kc_enriched` 参数，全靠 `meta.tags_source` 推断。如未来 enrichment 引入第 4 种 outcome（如 KcMeta 带 AI 标签但无规则标签），需同步调整本函数的映射 + enrichment::resolve_outcome 的 ResolvedEnrichment 字面。当前 2 种映射由 `extractor_type_for` / `kc_enriched_for` 两个 helper 集中维护，未来扩展点单一。

## 需要 Reviewer 特别关注的地方

1. **YAML 转义健壮性 — 与 task_017 前端 schema 对接的字段名 / 类型 / 大小写一致性**：
   - 本函数输出的 YAML key 全部为 **snake_case**（`source_asset_id` / `kc_doc_id` / `ai_qa_pairs_count` 等），与 task_017 `parseFrontmatter.ts::mapToCamelCase` 的字段白名单**严格 round-trip**。任何 key 拼写漂移会导致前端**静默丢字段**（前端 `if (typeof raw.X === "string")` 判定不通过即跳过）。
   - 类型也严格匹配：number 字段走 `push_number_field`（无引号），string 字段走 `push_string_field`（双引号包裹），数组走 `push_string_array_field`（flow style `[...]`）—— 前端用 `typeof raw.X === "number"` / `Array.isArray(raw.X)` 判定，类型漂移同样静默丢字段。
   - 建议 Reviewer 对照 `src/utils/parseFrontmatter.ts` 第 65-90 行字段白名单与本文件模块 doc 字段对照表逐行核对。

2. **`kc_enriched` 字面值与 task_011 `ResolvedEnrichment.kc_enriched` round-trip**：
   - 本函数：`AiAndRule` → `"true"`，`RuleOnly` → `"partial"`（Fallback 路径不调本函数）。
   - task_011 `enrichment::resolve_outcome`：Success → `"true"`，PartialLlmUnavailable → `"partial"`，Fallback → `"false"`。
   - 两处字面一致——本函数覆盖前两种，第三种由 resolve_outcome 直出不走本函数。Reviewer 可挑战："是否应让 build_kc_frontmatter 也支持 Fallback 写一行 `kc_enriched: "false"` 让消费方区分集成前 vs 集成后失败？"。我的判断：Fallback 路径就是 markitdown 原版 MD（无 frontmatter），本 task 不破坏这一不变量；如需该信号，应由 scheduler 在写 markitdown 原版 MD 时拼一个**简化版** frontmatter（仅 NC 5 字段 + `kc_enriched: "false"`），那是 task_012 范畴。

3. **block scalar `|` vs `"`-quoted 风格的选择**：
   - 本函数：多行（含 `\n`）→ `|`，单行 → `"`。
   - 选 `|`（literal block scalar）而非 `"`（含 `\n` escape）的理由：前端 js-yaml `JSON_SCHEMA` 模式下，**两种风格都能解析为带真实换行的字符串**，但 `|` 更接近原文本（人类阅读 frontmatter 时更友好，editor diff 也更清晰）。
   - 如 Reviewer 偏好"统一双引号 escape"，可改为 `push_string_field` 单一路径——会让多行 summary 在编辑器内显示为 `ai_summary: "line1\nline2\n..."` 单行长串。当前选 `|` 是产品级取舍，非技术约束。

4. **手动 YAML 序列化 vs `serde_yaml`**：
   - 本 task 选手动序列化，**不**引入 `serde_yaml = "0.9"` 新依赖。
   - 优点：零新增依赖（避免触发 Cargo.lock 全量解析）、字段顺序完全可控、字面值逐位与前端 schema 对齐。
   - 缺点：转义 / block scalar 等 corner 需要测试守护——已用 11 个单测覆盖（5 个 AC 核心 + 6 个边界 + 1 个字段顺序不变量）。
   - 若 Reviewer 强烈倾向 `serde_yaml`（更标准 / 未来扩展更容易），可一句 `cargo add serde_yaml@0.9` 切回，框架已围绕"输出字符串"组织，迁移成本低（核心是把 `push_*` helpers 替换为 `serde_yaml::Value::Mapping` 构建 + `serde_yaml::to_string`）。

5. **字段顺序不变量测试 `build_kc_frontmatter_field_order_is_stable`**：
   - 显式枚举 15 个字段的期望出现顺序，任何 IDE 自动排序 / 维护回归会立刻 fail。
   - 顺序与 Architect output.md §"Frontmatter Schema（衍生件 .md 头部）"的代码块逐行一致——便于跨工具 diff（同一 doc_id 不同 version 的 `.md`）与 grep。
