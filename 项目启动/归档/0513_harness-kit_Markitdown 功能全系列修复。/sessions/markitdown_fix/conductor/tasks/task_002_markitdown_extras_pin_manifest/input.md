# Task 输入 — task_002_markitdown_extras_pin_manifest (T-B)

## 目标
改造 `scripts/prepare-embedded-markitdown-runtime.sh`，把 markitdown 0.1.5 + extras（含 `ebooklib`、`beautifulsoup4`）pin 安装到嵌入 Python 的 `site-packages`，并生成 `runtime-manifest.json` 写入 `src-tauri/resources/`。

## 前置条件
- 依赖 task：task_001（嵌入 Python 必须已就位）
- 必须先存在的文件/接口：`src-tauri/resources/python/bin/python3.12`

## 验收标准（Acceptance Criteria）
1. AC-1：用 `Resources/python/bin/python3.12 -m pip install --no-cache-dir --no-deps -r requirements.lock` 安装；`requirements.lock` 显式 pin：
   ```
   markitdown[pdf,docx,pptx,xlsx]==0.1.5
   beautifulsoup4==4.12.3
   ebooklib==0.18
   ```
   并通过 `pip-compile` 或手工补齐传递依赖到 lock 文件（含 `pdfminer.six`、`python-docx`、`python-pptx`、`openpyxl`、`mammoth`、`Pillow` 等）。
2. AC-2：安装完成后，`Resources/python/bin/python3.12 -c "import ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL"` 全部成功，0 报错（E-2 后字面对齐 mammoth）。
3. AC-3：生成 `src-tauri/resources/runtime-manifest.json`，含字段（按 ADR-010）：`schema_version=1`、`runtime_id`、`python.{source,version,build}`、`markitdown.{version,extras}`、`extras_extra`、`imports`（**7 个关键模块**：`ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL`）、`build_timestamp`、`arch`。
   - 修订记录（2026-05-13 E-2 裁决）：原字面 `docx` 改为 `mammoth`，详见 ADR-010 补强说明。
4. AC-4：脚本两次源：requirements.lock 与 runtime-manifest 的版本字段必须由同一变量生成，禁止两处独立维护；CI 增加 `verify-manifest-consistency.sh` 校验。
5. AC-5：脚本幂等 + `set -euo pipefail`；`--force` 清空 site-packages 重装。
6. AC-6：site-packages 体积 ≤ 300MB（du -sh 报告 < 300M）。
   - 修订记录（2026-05-13 E-1 裁决）：原 200MB 阈值调整为 300MB；根因是 markitdown 0.1.5 默认带 magika(→onnxruntime+pandas+numpy)，pin upstream as-is 原则下不在本 task 裁；magika 三巨头裁剪推迟到 task_006 (DMG 门禁) 与 task_011 (保留 vs 修改矩阵) 作为独立决策点。

## 技术约束
- 禁止 `pip install markitdown`（不带 extras 即生产事故根因）。
- 禁止使用 wheelhouse 之外的源（reproducibility）。
- 不得污染嵌入 Python 之外的 site-packages。

## 参考文件
- `NCdesktop/scripts/prepare-embedded-markitdown-runtime.sh`（现有，需改造）
- ADR-002 / ADR-010
- PRD §3.1-F1 / F2

## 预估影响范围
- 修改文件：`scripts/prepare-embedded-markitdown-runtime.sh`
- 新建文件：`scripts/requirements.lock`、`scripts/verify-manifest-consistency.sh`、`src-tauri/resources/runtime-manifest.json`（CI 时生成）
