# Task 输入 — task_012_real_sample_matrix_ci_secret

## 目标
搭建 7 格式 × ≥5 真实样本端到端验收矩阵，使用 task_000 的加密样本仓 + CI secret 解密；目标真实样本通过率 ≥95%。

## 前置条件
- 依赖 task：task_000（样本仓 + 解密链）、task_002（manifest）、task_006（DMG）、task_007/008/009/010（运行时 + 错误码 + 路由）
- 必须先存在的文件/接口：已签名 DMG、`samples-private/` 加密样本

## 验收标准（Acceptance Criteria）
1. AC-1：新增 `tests/real_samples/` 集成测试 + `scripts/run-real-sample-matrix.sh`：
   - decrypt samples 到临时目录；
   - 对每个样本：通过 Tauri command 触发 markitdown 转录路径，断言：① 返回结构化 markdown；② `failure_code IS NULL`（除已知 Out 类，如扫描 pdf）；③ markdown 至少含 1 个标题或段落。
2. AC-2：样本覆盖矩阵（每格式 ≥5，共 ≥35）：
   - `pdf-text`：5 个文本型（含含图文本型 ≥2）；
   - `pdf-scan`：3 个扫描型（用于 `is_scan_pdf` true 路径）；
   - `docx`、`pptx`、`xlsx`：每类 5 个（含含表格 / 含中文 / 含 emoji 边界）；
   - `html`：5 个（含含 `<script>` 与 `<iframe>` 的输入，预期被 markitdown 安全剥离）；
   - `epub`：5 个（必含 1 个生产已知失效样本，本 AC 须验证现在通过）；
   - `image`：5 个（含 EXIF + alt-text，未配 LLM 走 markitdown_image_fallback）。
3. AC-3：通过率 ≥ 95% 才视为 PASS；失败样本必须落入"已知失败清单"并触发 RCA（不可静默忽略）。
4. AC-4：CI workflow `real-samples-matrix.yml`（macOS runner 可用时）：
   - 注入 `MARKITDOWN_SAMPLES_KEY`；
   - 调用解密脚本；
   - 执行 `scripts/run-real-sample-matrix.sh`；
   - 上传 `report.json`（per-sample 状态）作为 artifact。
5. AC-5：本地脚本与 CI 共用同一断言库；通过率报告含 per-format 细分。
6. AC-6：epub 已知生产失效样本：本次必须 PASS；若仍失败 → ESCALATE 而非 mark known-fail（验证 markitdown 0.1.5 本身 bug）。

## 技术约束
- 严禁把样本明文写入构建产物或日志。
- 严禁跳过 epub 与扫描 pdf 类（最易回归）。
- 单样本 wall-clock 上限 = MARKITDOWN_TIMEOUT + 10s，超出标 `ETimeout90s`。

## 参考文件
- task_000 input.md
- PRD §3.1 F8 / §6 MVP 验收门禁
- ADR-009

## 预估影响范围
- 新建：`NCdesktop/scripts/run-real-sample-matrix.sh`、`NCdesktop/tests/real_samples/`（集成测试入口）、CI workflow
- 不修改：业务代码（本 task 是验收脚本，非业务实现）
