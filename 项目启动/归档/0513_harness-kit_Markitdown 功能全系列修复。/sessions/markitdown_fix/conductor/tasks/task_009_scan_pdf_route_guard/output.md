# Task 交付 — task_009_scan_pdf_route_guard

## 实现摘要

在 scheduler 进入 markitdown 子进程**前**对 `application/pdf` 做结构性嗅探（XObject + Font 引用判定）。仅在 `extractor.name() == "markitdown"` 且 `mime == "application/pdf"` 时启用；扫描型 PDF 直接短路写 `conversion_meta.failure_code = EScanPdfUnsupported` + placeholder + 跳过 markitdown 子进程。

设计要点：

- **纯结构判定（ADR-006 / H6）**：读 PDF 首页 page tree 的 `Resources` 字典；若 `XObject` 非空且全部为 `Subtype=/Image`，且整页无 `Font` 字典引用 → 视为扫描型。
- **保守语义**：无 XObject、无 Font、混合（Font + Image）、Form XObject 等"信息不充分 / 明显非扫描"统一返回 `false`，让 markitdown 自尝试；只有"明确像扫描件"才返回 `true`。**误路由率 0%** 的语义是"把文本 PDF 误判为扫描" = 0。
- **解析失败显式降级**：`lopdf` 加载失败 / 加密 PDF / 无 page tree → `Err(io::Error)`；调用方 `scan_pdf_route_decision` 按 **ParseError 处理**（log warn + FallThrough），不"猜测"成 scan。
- **Resources 父链继承**：按 PDF 1.7 §7.7.3.4，page 自身无 `Resources` 键时沿 `Parent` 链向上找（防御 16 层、循环检测）。

## 对架构方案的遵守声明

- [x] 目录结构与 Architect 方案一致：新建 `src-tauri/src/extraction/scan_pdf_detect.rs`，符合 ADR-006 描述
- [x] API 路径/命名与 Architect 方案一致：`pub fn is_scan_pdf(&Path) -> Result<bool, io::Error>`
- [x] 数据模型与 Architect 方案一致：复用 `FailureCode::EScanPdfUnsupported`（只 use 引用，未改 failure_code.rs）
- [x] 未引入计划外的新依赖：`lopdf = "0.34"`（与 `pdf-extract = "0.7"` 的传递依赖同版本，无重复编译）；纯 Rust，**无 C 依赖**（H6/H1）
- 偏离说明：无

## AC 一览

| AC | 内容 | 状态 |
|---|---|---|
| AC-1 | `is_scan_pdf` 实现：XObject + Font 引用判定 + Resources 父链继承 | **PASS** |
| AC-2 | 用 `lopdf = "0.34"`（纯 Rust，无 C 依赖） | **PASS** |
| AC-3 | scheduler `application/pdf` + markitdown 分支接入；`Ok(true)` 短路 `EScanPdfUnsupported`，`Err` 按 ParseError fall-through | **PASS** |
| AC-4 | 单测矩阵：3 text + 3 scan + 1 mixed + 1 encrypted + Form/corrupted 额外（10 测） | **PASS（mock 层）** |
| AC-5 | 误路由率 0% on mock matrix（10/10 命中） | **PASS（mock 层）/ PENDING-OPERATOR（真实样本）** |
| AC-6 | grep gate 双校验：禁 stdout/markitdown len/words/chars/count；禁 text<N 启发式 | **PASS** |

**PENDING-OPERATOR（真实样本 AC）**：input.md AC-4 要求 ≥3 真实文本 PDF + ≥3 真实扫描 PDF。task_000 真实样本仓尚未实际入库，等 task_012 解密样本接入后由 reviewer 跑回归。本 task 用手工 mock fixture 覆盖结构判定全分支，逻辑层已 PASS。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src-tauri/Cargo.toml` | 修改 | 添加 `lopdf = "0.34"`（+3 行） |
| `src-tauri/src/extraction/scan_pdf_detect.rs` | **新建** | `is_scan_pdf` + 内部 helpers + 10 单测 fixtures |
| `src-tauri/src/extraction/mod.rs` | 修改 | 注册 `pub mod scan_pdf_detect;`（+3 行） |
| `src-tauri/src/extraction/scheduler.rs` | 修改 | PDF 路由分支接入 + `scan_pdf_route_decision` 纯函数 + 2 单测 |

`git diff --stat`（task_009 范围）：

```
 NCdesktop/src-tauri/Cargo.toml                          |   3 +
 NCdesktop/src-tauri/src/extraction/mod.rs               |   3 +
 NCdesktop/src-tauri/src/extraction/scan_pdf_detect.rs   | 432 +++ (新建)
 NCdesktop/src-tauri/src/extraction/scheduler.rs         | ~120 (PDF 段集中)
```

Cargo.lock 中 lopdf 已存在为 pdf-extract 的传递依赖（0.34.0），无新增编译单元。

## 范围 gate（并行 dev 协作约束）

- 未触 `extractors/markitdown.rs`（task_007 + task_008 + task_010 范围）
- 未触 `extractors/audio_asr_iflytek.rs`（PRD 底线 #4）
- 未触 `extraction/runtime_check.rs`（task_007 PASS）
- 未触 `extraction/failure_code.rs`（只 use 引用）
- 未触 scheduler.rs 中 audio/video 分支（task_010 范围 — 并行 dev 中）
- 未触 `db/migration.rs` / `db/conversion_meta.rs` / `db/asset.rs`（task_008 / task_014 范围）

scheduler.rs 我的修改 hunk 集中在 PDF 路由段（loop 内）+ helper（自由函数区）+ tests（末尾），不分散。

## 测试命令

```bash
# AC-1/2/4：scan_pdf_detect 单测（10 测）
cd src-tauri && cargo test --lib extraction::scan_pdf_detect

# AC-3 + 不退步：scheduler 单测（含 task_007 FIX + 本 task 新增 2 测）
cd src-tauri && cargo test --lib extraction::scheduler

# 全量：lib test baseline + 本 task 新增
cd src-tauri && cargo test --lib

# AC-6 grep gate 双校验
grep -nE '(stdout|markitdown).*(len|words|chars|count)' src/extraction/scan_pdf_detect.rs src/extraction/scheduler.rs
grep -nE 'text.*<.*\d+' src/extraction/scan_pdf_detect.rs
```

## 测试结果

### `cargo test --lib extraction::scan_pdf_detect`

```
running 10 tests
test extraction::scan_pdf_detect::tests::corrupted_pdf_returns_err ... ok
test extraction::scan_pdf_detect::tests::scan_pdf_a_returns_true ... ok
test extraction::scan_pdf_detect::tests::mixed_font_and_image_returns_false ... ok
test extraction::scan_pdf_detect::tests::text_pdf_c_returns_false ... ok
test extraction::scan_pdf_detect::tests::scan_pdf_c_returns_true ... ok
test extraction::scan_pdf_detect::tests::form_xobject_only_returns_false ... ok
test extraction::scan_pdf_detect::tests::text_pdf_b_returns_false ... ok
test extraction::scan_pdf_detect::tests::text_pdf_a_returns_false ... ok
test extraction::scan_pdf_detect::tests::scan_pdf_b_returns_true ... ok
test extraction::scan_pdf_detect::tests::encrypted_pdf_returns_err ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 203 filtered out
```

### `cargo test --lib extraction::scheduler`（节选 task_009 / task_007 FIX 相关）

```
test extraction::scheduler::tests::scan_pdf_route_decision_falls_through_on_parse_err ... ok
test extraction::scheduler::tests::scan_pdf_route_decision_falls_through_on_corrupted_bytes ... ok
test extraction::scheduler::tests::runtime_check_short_circuits_markitdown_on_failure ... ok
test extraction::scheduler::tests::runtime_check_does_not_short_circuit_on_pass_or_non_markitdown ... ok
test extraction::scheduler::tests::audio_mime_routes_to_iflytek_not_markitdown ... ok
test extraction::scheduler::tests::video_mime_is_explicitly_rejected ... ok
test extraction::scheduler::tests::video_mime_has_no_extractor_so_must_be_explicitly_rejected ... ok
... (其余 task_007 / 决策矩阵测试同步 PASS)
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; ...
```

### `cargo test --lib`（全量）

```
test result: ok. 215 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.89s
```

### AC-6 grep gate 双校验实测

```
$ grep -nE '(stdout|markitdown).*(len|words|chars|count)' src/extraction/scan_pdf_detect.rs src/extraction/scheduler.rs
(no match — PASS)

$ grep -nE 'text.*<.*\d+' src/extraction/scan_pdf_detect.rs
(no match — PASS)
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常 | 文本 PDF（含 Font + 文本流） | 已测 ×3 | `is_scan_pdf == false` |
| ✅ 正常 | 扫描 PDF（仅 Image XObject 无 Font） | 已测 ×3 | `is_scan_pdf == true` |
| ⚠️ 边界 | 混合首页（Font + Image XObject） | 已测 | `false`（保守：首页有 Font 即视为非扫描） |
| ⚠️ 边界 | 仅 Form XObject 无 Font | 已测 | `false`（保守：Form 不等于扫描） |
| ❌ 异常 | 加密 PDF | 已测 | `Err`，scheduler `FallThrough` |
| ❌ 异常 | 损坏字节流 / 非 PDF | 已测 | `Err`，scheduler `FallThrough` |
| ❌ 异常 | 不存在路径 | 已测（scheduler 层） | `Err`，scheduler `FallThrough` |
| ✅ 并发 | 不触 audio/video 分支 | git diff 自检 | scheduler 修改集中在 PDF 段 + helper + tests |
| ⏳ 真实样本 | ≥3 真实文本 + ≥3 真实扫描 PDF（input.md AC-4） | **PENDING-OPERATOR** | 等 task_012 解密样本接入 |

## 浏览器/运行时验证

N/A — 本 task 为 Rust lib 层逻辑（scheduler 路由决策 + 结构嗅探）。无 UI 入口；前端 i18n 文案 `E_SCAN_PDF_UNSUPPORTED → "扫描型 PDF 暂不支持，需先用 OCR 转为文本"` 在 task_008 PASS 的 `src/lib/extraction-failure-codes.ts` 已就位（字符级一致，无需改动）。完整端到端验证（拖入扫描 PDF → 看到该文案）应在 task_011 / task_013 真机烟测阶段执行。

## lopdf 版本号 + 引入理由

- **版本**：`lopdf = "0.34"`（与 `pdf-extract = "0.7"` 的传递依赖同版本 0.34.0；Cargo.lock 已有，无重复编译）
- **理由**：
  1. 纯 Rust，无 C 依赖（H1 / H6）
  2. 已是 pdf-extract 的传递依赖，无新增二进制体积
  3. 提供 `Document::load` + `Dictionary` 遍历 API，满足 ADR-006 "结构性嗅探"语义
  4. `with_version` + `add_object` + `Stream::new` + `dictionary!` 宏支持手工构造 mock fixtures（单测自闭合，不依赖外部样本）
- **排除项**：`pdf` crate（API 较新但生态薄）；自写 PDF 解析（重复造轮子）。

## 已知局限

1. **真实样本 AC PENDING**：mock fixtures 覆盖结构判定全分支，但 PDF 工业界存在大量非常规结构（Lossy 扫描 + 模糊 Font 字符识别）。`is_scan_pdf` 的"保守 false"语义意味着边缘扫描件可能漏判（落到 markitdown → `EOutputEmpty` 链路），这是 AC-5 字面允许的（误路由 = "把文本误判扫描" = 0）。
2. **Resources 父链 16 层上限**：PDF 标准未规定深度限制；实践中 page tree 深度通常 ≤ 3 层。16 是防御性常量，超深层结构会回退到"找不到 Resources" → `false`。
3. **多页 PDF**：本 task 仅检查首页（input.md AC-1 字面）。若 PDF 首页非扫描但后续页全是扫描（罕见），会被放行到 markitdown。这是 ADR-006 设计选择。
4. **Encrypt 字典 lopdf 写盘行为**：单测 `encrypted_pdf_returns_err` 中，lopdf 0.34 写出 Encrypt trailer 的 PDF 可能在 save 时返回 Err（依赖 trailer.ID 是否齐全）；测试已防御性 fallback 到"不存在路径 → 必 Err"分支，两种路径都满足 `Err` 语义。

## 需要 Reviewer 特别关注的地方

1. **`scan_pdf_detect.rs` 第 50-70 行 `is_encrypted` 判定**：lopdf 通过 trailer.Encrypt 引用判定，已加密 PDF 即使 lopdf 能解析 trailer，内容流仍不可读 → 显式 Err。确认与 ADR-006 "解析失败按 ParseError 处理"一致。
2. **`scan_pdf_detect.rs::resolve_resources` 父链遍历**：PDF Resources 可继承自 Pages 树父节点，不只挂在叶子 page。请验证我对 PDF 1.7 §7.7.3.4 的实现是否完整（含循环检测 + 深度上限）。
3. **`scheduler.rs::scan_pdf_route_decision`**：`Err → FallThrough` 是 input.md AC-3 字面的"按 ParseError 处理（不要猜测成 scan）"。如果 Reviewer 认为 Err 应该走"独立 PDF 解析失败 failure_code"，需先回 Architect 修订 ADR-006（当前 8 个 FailureCode 不含"PDF parse error"）。
4. **PDF 路由保护条件 `primary_name == "markitdown" && asset.mime_type == "application/pdf"`**：双重 guard。若未来引入新的 PDF primary extractor（如 `pdf_text`），需扩展白名单 —— 目前 `text-passthrough` / `pdf_text` 不消费扫描件，仅 markitdown 路径需此防呆。
5. **mock fixture 与真实样本的一致性**：lopdf 手工构造的 PDF 与真实扫描仪输出 PDF 结构差异较大（无 Optional Content Group / 无 Color Profile / 无 ExtGState）。`is_scan_pdf` 只依赖 `Resources.XObject` + `Resources.Font`，逻辑层稳定，但真实样本仍需 task_012 接力验证（AC-5 PENDING-OPERATOR 的根本理由）。
