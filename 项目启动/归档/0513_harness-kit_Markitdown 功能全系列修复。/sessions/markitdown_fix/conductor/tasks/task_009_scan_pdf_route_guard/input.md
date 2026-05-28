# Task 输入 — task_009_scan_pdf_route_guard

## 目标
在 `scheduler` 进入 markitdown 调度前对 `application/pdf` 做 mime + 文件头嗅探判定，扫描型 pdf 直接返回 `E_SCAN_PDF_UNSUPPORTED`，**禁止引入文本字数启发式**（H6）。

## 前置条件
- 依赖 task：task_008（错误码已落地）
- 必须先存在的文件/接口：`FailureCode::EScanPdfUnsupported`、`scheduler.rs` 路由分支

## 验收标准（Acceptance Criteria）
1. AC-1：新增 `fn is_scan_pdf(path: &Path) -> Result<bool, io::Error>`，实现：
   - 读 first page 的 page tree；
   - 若 `Resources.XObject` 仅含 Image XObject **且** 整页无 Font 字典引用 → `true`；否则 `false`；
   - 单页失败/解析异常返回 `Err`（按 ParseError 处理）。
2. AC-2：选用 `lopdf` crate（或同等纯 Rust 库），不引入 C 依赖。
3. AC-3：`scheduler.rs` 在 `mime == application/pdf` 分支：先 `is_scan_pdf` → true 则写 `conversion_meta.failure_code = EScanPdfUnsupported` 并 short-circuit 返回；前端显示文案"扫描型 pdf 暂未支持，请等待 OCR 版本"。
4. AC-4：单测矩阵（用 task_000 样本仓内的 fixtures，至少）：
   - 3 个真实文本 pdf → 全部 `false`；
   - 3 个真实扫描件 pdf → 全部 `true`；
   - 混合（文字+扫描页）→ `false`（首页有 Font 即视为非扫描，保守通过）；
   - 加密 pdf → `Err`，scheduler 按 ParseError 处理。
5. AC-5：误路由率 = 0%（在样本矩阵上）；CI 用 task_012 的解密样本运行。
6. AC-6：严禁出现"文本字数 < N 即视为扫描"或"运行 markitdown 看 stdout 长度"的实现（grep 检查作为 CI gate）。

## 技术约束
- H6：任何启发式/分类器走 P1。本 task 只做"结构性嗅探"（XObject + Font 引用），不是启发式。
- `lopdf` 解析失败必须降级为 `Err`，不可"猜测"。

## 参考文件
- ADR-006
- `src-tauri/src/extraction/scheduler.rs` PDF 路由分支
- PRD §3.1 F4

## 预估影响范围
- 新建：`src-tauri/src/extraction/scan_pdf_detect.rs`
- 修改：`src-tauri/src/extraction/scheduler.rs`、`Cargo.toml`（添加 lopdf）
