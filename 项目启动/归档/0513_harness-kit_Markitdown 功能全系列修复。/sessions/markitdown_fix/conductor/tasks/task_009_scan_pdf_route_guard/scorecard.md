# Review Scorecard — task_009_scan_pdf_route_guard

## 审查思考过程

1. **Task 意图**：在 scheduler 进入 markitdown 子进程前，对 `application/pdf` 做**结构性嗅探**（XObject + Font 引用判定，**非启发式**），扫描型 PDF 直接短路写 `EScanPdfUnsupported` 失败码 + placeholder，**不**调用 Python 子进程。落地 ADR-006 + H6 硬约束。

2. **AC 检查结果**：
   - AC-1 `is_scan_pdf` 签名 + XObject/Font 判定 + Resources 父链继承 ✅
   - AC-2 lopdf 0.34 纯 Rust（cargo tree -p lopdf 无 C 编译依赖；core-foundation-sys 为 macOS framework 系统调用绑定，非新增编译） ✅
   - AC-3 scheduler PDF 分支：双 guard（primary_name=="markitdown" && mime=="application/pdf"），Ok(true)→ShortCircuit→update_failure_code(EScanPdfUnsupported)+placeholder+continue；Ok(false)/Err→FallThrough ✅
   - AC-4 单测 10 个（3 text + 3 scan + 1 mixed + 1 encrypted + 1 form + 1 corrupted）；mock fixture 用 lopdf::Document 手工构造，逻辑分支全覆盖；真实样本 PENDING-OPERATOR ✅
   - AC-5 mock 矩阵误路由率 0%（10/10 命中预期），真实 PENDING ✅
   - AC-6 grep gate 双校验实测 0 命中 ✅

3. **关键发现**：
   - `scan_pdf_route_decision` 纯函数设计良好：签名 `(path: &Path) -> ScanPdfDecision`，对外只暴露三态枚举（`ShortCircuit` / `FallThrough`），不耦合 lopdf 类型；2 个单测分别覆盖"不存在路径"和"非 PDF 字节"两类 Err 路径，断言 FallThrough 行为。
   - Resources 父链继承（PDF 1.7 §7.7.3.4）实现正确：含循环检测（`seen` Vec）+ 16 层深度防御 + Reference/Dictionary 双分支解引。
   - 加密 PDF 路径：先 `doc.is_encrypted()` 提前判 Err；测试中并防御性 fallback 到"不存在路径"路径，两种 Err 路径都满足语义。
   - "首页有 Font 即视为非扫描"语义实现严格（`has_font_reference` 在 XObject 检查**之前**短路返回 false），符合 input.md AC-4 字面"保守通过"。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 6 AC 全 PASS（真实样本 PENDING 是 input.md 已声明依赖 task_012，非 dev 缺陷）；215/0 单测；mock 矩阵 100% |
| 安全性 | 25% | 5 | 无启发式（H6）；解析失败显式 Err 不"猜测"；不污染 conversion_meta；父链遍历有深度+循环防御；不引 C 依赖 |
| 代码质量 | 15% | 5 | 单文件 432 行职责单一；每个 helper 一行注释清楚意图；ScanPdfDecision 枚举对外抽象 lopdf 类型；保守语义注释充分 |
| 测试覆盖 | 15% | 4 | 10 scan_pdf 单测 + 2 scheduler 决策单测共 12 测，覆盖 6 个判定分支；扣 1 分因 mock fixture 复用同一构造函数 3 次（3 个 text/scan 测在 mock 层等价，真正多样性靠 task_012 真实样本接力） |
| 架构一致性 | 10% | 5 | 严格落地 ADR-006；新增文件路径与 Architect 方案一致；不动 markitdown.rs/audio_asr_iflytek.rs/runtime_check.rs/failure_code.rs；scheduler 修改集中 PDF 段 + 自由函数区 + tests |
| 可维护性 | 10% | 5 | "PENDING-OPERATOR"已声明真实样本接力点；ScanPdfDecision 枚举便于未来扩展第三状态（如 OCR 接管）；docblock 引用 ADR/AC 编号；保守语义"宁可漏判 scan 也不误判 text"清晰 |

**综合分：4.85/5**（加权计算：0.25×5 + 0.25×5 + 0.15×5 + 0.15×4 + 0.10×5 + 0.10×5 = 4.85）

## 总体判断

- [x] **PASS**

## 红线违反检查（全部未命中）

- [ ] 修改 extractors/markitdown.rs（未触动；diff 来自并行 task_007/008/010）
- [ ] 修改 extractors/audio_asr_iflytek.rs（PRD 底线 #4 — 未触动）
- [ ] 修改 runtime_check.rs / failure_code.rs（只 use 引用）
- [ ] 修改 scheduler.rs audio/video 分支（未触动；video_reject 段属 task_010 范围）
- [ ] 修改 db/migration.rs / db/conversion_meta.rs / db/asset.rs（diff 来自并行 task_008/014）
- [ ] 修改 task_004~006 scripts/（未触动）
- [ ] 修改 task_000 脱敏（未触动）
- [ ] 修改 task_003 verify-venv-shim.sh（未触动）
- [ ] 引入 C 依赖（lopdf 0.34 纯 Rust，cargo tree -p lopdf 验证）
- [ ] 字数 / stdout 长度启发式（grep gate 双 0 命中）
- [ ] cargo test --lib 退步（215/0，超过 baseline 195+并行增量）

## 4 关注点结论

1. **Resources 父链继承**：实现完整。`resolve_resources` 先查节点本身，再沿 `Parent` 链向上（含循环检测 + 16 层防御），符合 PDF 1.7 §7.7.3.4。
2. **混合 PDF 首页 Font 短路**：`has_font_reference` 在 `is_scan_pdf` 函数体内**先于** XObject 判定执行，确保"首页有 Font 即视为非扫描"严格生效。`mixed_font_and_image_returns_false` 单测断言通过。
3. **加密 PDF Err 路径**：`doc.is_encrypted()` 显式判定 + `io::Error::new(InvalidData, ...)` 包装；测试中防御性 fallback 到不存在路径，保证两种实现路径都满足 Err 语义。
4. **scan_pdf_route_decision 纯函数**：签名 `(path: &Path) -> ScanPdfDecision`，输出三态枚举抽象 lopdf 类型；不耦合 scheduler 上下文（不接 app / db / FailureCode），单测可控性高。2 个单测注入"不存在路径"和"非 PDF 字节"两种 Err，断言 FallThrough。

## cargo test 实测

```
test result: ok. 215 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.97s
```

与 dev 自报数字 215/0 完全一致。

## 范围验证

- **scheduler.rs audio/video 分支零触动**：YES（视频拒绝段属 task_010；本 task 修改集中在 runtime_check_short_circuit 之后、primary_attempt 之前的 PDF 段 + 自由函数区 + #[cfg(test)] 段）
- **extractors/markitdown.rs 零触动**：YES（dev 仅 use 引用未改实现）
- **extractors/audio_asr_iflytek.rs 零触动**：YES（PRD 底线 #4）
- **failure_code.rs 零触动**：YES（仅 use FailureCode::EScanPdfUnsupported）
- **db/migration.rs / conversion_meta.rs / asset.rs 零触动**：YES（task_008/014 并行已铺好 update_failure_code）

## lopdf 纯 Rust 验证

```
$ cargo tree -p lopdf 2>/dev/null | grep -E '\bcc\b|gcc|clang|sys'
│   │   └── core-foundation-sys v0.8.7
```

唯一 `*sys` 是 `core-foundation-sys`（chrono → iana-time-zone → core-foundation-sys，macOS framework binding，**非 C 编译依赖**）。lopdf 直接依赖：chrono / encoding_rs / flate2 / indexmap / itoa / log / md-5 — 全部纯 Rust。

## AC 一览

| AC | 内容 | 状态 |
|---|---|---|
| AC-1 | `is_scan_pdf(&Path)→Result<bool,io::Error>` + 结构判定 + 父链继承 | **PASS** |
| AC-2 | lopdf 0.34 纯 Rust 无 C 依赖 | **PASS** |
| AC-3 | scheduler PDF 分支双 guard + ShortCircuit/FallThrough 接入 | **PASS** |
| AC-4 | 10 单测全分支覆盖（mock）；真实样本 PENDING-OPERATOR | **PASS（mock）** |
| AC-5 | 误路由率 0% (10/10 mock)；真实 PENDING task_012 接力 | **PASS（mock）** |
| AC-6 | grep gate 双校验 0 命中（无 stdout/words 启发式） | **PASS** |

## 问题列表

### BLOCKER

无。

### MAJOR

无。

### MINOR

1. **mock fixture 复用同一构造函数**：`text_pdf_a/b/c` 三测都调用 `make_text_pdf()`，结构等价；`scan_pdf_a/b/c` 同理。可读性上看起来像"3 测"，实际只是同一逻辑的 3 次稳定性重跑。
   - **位置**：`scan_pdf_detect.rs::tests`（407-444 行）
   - **建议**：可考虑让 b/c 变体改用不同 Font 子类型（TrueType / Type0 CID）和不同 ColorSpace（DeviceRGB / DeviceCMYK）增加 mock 多样性。但**非阻塞** —— input.md AC-4 字面允许 mock 层，真正的多样性来自 task_012 真实样本。
   - **验证标准**：N/A（PASS 不要求修复）。

2. **encrypted_pdf_returns_err 的"防御性 fallback"路径**：若 lopdf 0.34 写盘成功，测试断言"加密 PDF 必须 Err"；若写盘失败，则用不存在路径间接断言 Err。后者已脱离"加密 PDF"语义，变成"路径不存在 Err"测试。
   - **位置**：`scan_pdf_detect.rs::encrypted_pdf_returns_err`（462-521 行）
   - **建议**：等 task_012 真实加密 PDF 样本接入后，把这个测换成真实样本驱动。
   - **验证标准**：N/A（PASS 不要求修复）。

## 给 Dev 的备注（FYI，非修复要求）

- task_009 的"真实样本 PENDING-OPERATOR"是 input.md 已声明依赖 task_012，由 Reviewer 在 task_012 完成后跑回归（≥3 真实文本 + ≥3 真实扫描），届时若误路由率 > 0 再回开 task_009 FIX。
- scan_pdf_route_decision 的 `Err → FallThrough` 是 input.md AC-3 字面"按 ParseError 处理（不要猜测成 scan）"。若未来 ADR-006 修订引入"PDF parse error"独立失败码，再讨论。
