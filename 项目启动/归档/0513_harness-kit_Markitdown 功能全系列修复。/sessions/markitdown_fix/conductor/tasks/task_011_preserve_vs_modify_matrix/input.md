# Task 输入 — task_011_preserve_vs_modify_matrix

## 目标
对 `markitdown.rs` 与 `scheduler.rs` 当前已实现的"易被误重构"行为做"保留 vs 修改"二维矩阵显式声明，避免后续 task 误删；并对应补充注释 + 单测 + 文档。

## 前置条件
- 依赖 task：task_008、task_010（错误码与路由已稳定）
- 必须先存在的文件/接口：现有 `markitdown.rs`、`scheduler.rs`

## 验收标准（Acceptance Criteria）
1. AC-1：在 `tasks/task_011_preserve_vs_modify_matrix/preserve_matrix.md` 输出表格，对至少以下 6 项明确"保留 / 修改 / 删除"：
   | 行为 | 现状 | 决策 | 替代实现指向 |
   |---|---|---|---|
   | 90s 子进程超时（`MARKITDOWN_TIMEOUT`） | 已实现 | 保留 | — |
   | image 输出为空 → 最小元数据 MD 回退 | 已实现 | 保留（但走 `markitdown_image_fallback` 类型） | task_008 |
   | markitdown 版本探测缓存（`probe_markitdown_version`） | 已实现 | 保留 | — |
   | `exit==0 && stdout==''` 判 success | 已实现 | **修改** → `classify_output` | task_008 |
   | `python_candidates` 顺序探测 | 已实现 | 修改 → 严格三级 | task_007 |
   | `SUPPORTED_MIME_TYPES` 含 audio/video | 当前已**不含** | 保留（grep gate） | task_010 |
2. AC-2：对每项"保留"行为在源码处增加 `// task_011 preserve: ...` 注释引用本矩阵；对每项"修改"在 PR 描述显式列出。
3. AC-3：补充单测覆盖"保留"行为：
   - 超时：mock 一个 95s sleep 的 fake python → 期望 `ETimeout90s`；
   - image 空回退：image 输入 + 子进程退出 0 + stdout='' → 返回 `markitdown_image_fallback` 类型 + quality_level=1；
   - 版本缓存：连续两次 extract 仅一次 `--version` 调用（mock 计数）。
4. AC-4：与 task_008 的 `classify_output` 联调，确保"image 空回退"不被 `EOutputEmpty` 误判。
5. AC-5：保留矩阵文档作为后续 review checklist 引用。

## 技术约束
- 不得在本 task 引入新 extractor、新分类器（H6）。
- 注释行数 ≤ 单文件 ≤ 30 行，避免噪音。

## 参考文件
- `src-tauri/src/extraction/extractors/markitdown.rs`
- ADR-007 / Debate Layer 2 R-③

## 预估影响范围
- 新建：`sessions/markitdown_fix/conductor/tasks/task_011_preserve_vs_modify_matrix/preserve_matrix.md`
- 修改：`markitdown.rs` 注释 + 测试段
