# Task 输入 — task_003_venv_shim_symlink_cold_boot (T-C)

## 目标
建立 `Resources/markitdown-venv/bin/python` symlink venv-shim（指向嵌入的 standalone Python），并提供"干净 shell 冷启动 import 全部 extras"的验证脚本。

## 前置条件
- 依赖 task：task_001（python 已就位）、task_002（extras 已安装到 site-packages）
- 必须先存在的文件/接口：`src-tauri/resources/python/bin/python3.12`

## 验收标准（Acceptance Criteria）
1. AC-1：脚本 `scripts/prepare-venv-shim.sh` 创建：
   - `Resources/markitdown-venv/bin/python` → `../../python/bin/python3.12`（相对 symlink）
   - `Resources/markitdown-venv/bin/python3` → `python`
2. AC-2：禁止使用 `python -m venv` 或 `--copies`；脚本里显式注释引用 ADR-003。
3. AC-3：验证脚本 `scripts/verify-venv-shim.sh` 在 `env -i PATH=/usr/bin:/bin` 下运行：
   ```
   Resources/markitdown-venv/bin/python -c "import ebooklib, bs4, pdfminer, pptx, mammoth, openpyxl, PIL; print('ok')"
   # 注：E-2 (2026-05-13) 后 imports 探针字面对齐 ADR-010 / runtime-manifest，docx → mammoth
   ```
   必须输出 `ok` 且 exit code 0。
4. AC-4：symlink 是相对路径（可通过 `readlink` 验证不以 `/` 开头），保证移动 `.app` 后仍有效。
5. AC-5：在 macOS 12 / macOS 14 两版 arm64 干净 VM（无 brew）冷启验证均通过；若 CI 无 macOS runner，本 AC 由本地手测代办，并在 output.md 记录。

## 技术约束
- 严禁 `cp -R` 替代 symlink（绕过 H4 即视为不通过）。
- shim 目录只含 symlink，无 `pyvenv.cfg`（避免 venv 语义触发 rpath 修改）。

## 参考文件
- ADR-003 / H4
- `src-tauri/src/extraction/scheduler.rs:531-532`（运行时探测）
- PRD §5 技术约束

## 预估影响范围
- 新建文件：`scripts/prepare-venv-shim.sh`、`scripts/verify-venv-shim.sh`
- 产物目录：`src-tauri/resources/markitdown-venv/`（CI 时生成）
