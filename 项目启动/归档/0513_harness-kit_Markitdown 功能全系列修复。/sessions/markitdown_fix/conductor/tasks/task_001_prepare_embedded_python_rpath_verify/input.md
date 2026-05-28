# Task 输入 — task_001_prepare_embedded_python_rpath_verify (T-A)

## 目标
改造 `scripts/prepare-embedded-python.sh`，下载并就位 `python-build-standalone` cpython 3.12.7 aarch64-apple-darwin 到 `src-tauri/resources/python/`，并产出可独立运行的 `scripts/verify-rpath.sh` 验证 `@executable_path` rpath 完整可解析。

## 前置条件
- 依赖 task：无（打包链起点）
- 必须先存在的文件/接口：`src-tauri/resources/`（已存在）

## 验收标准（Acceptance Criteria）
1. AC-1：脚本下载固定 release（`cpython-3.12.7+20241016-aarch64-apple-darwin-install_only.tar.gz`），校验 SHA256 与 release notes 一致；下载 URL + SHA256 写常量，禁止动态拼接。
2. AC-2：解压目录结构在 `src-tauri/resources/python/` 下满足 `bin/python3.12`、`lib/python3.12/`、`include/python3.12/` 三者齐全。
3. AC-3：`verify-rpath.sh` 对 `python3.12` 与 `lib/**/*.so/*.dylib` 运行 `otool -L`，断言所有路径以 `@executable_path/` 或 `/usr/lib/` 或 `/System/Library/` 开头；任何 `/usr/local/`、`/opt/homebrew/`、绝对开发机路径均报错退出。
4. AC-4：脚本幂等：第二次运行不重新下载，输出 `python already present, skipping`。
5. AC-5：`set -euo pipefail` + 所有路径变量双引号；clean 模式 `--force` 显式重下。
6. AC-6：在干净 shell（`env -i PATH=/usr/bin:/bin`）中运行 `Resources/python/bin/python3.12 -c "import sys; print(sys.version)"` 成功输出 `3.12.7`。

## 技术约束
- 禁止使用系统 brew/python（H1 / ADR-001）。
- 禁止修改 standalone Python 的内部目录结构（破坏 rpath）。
- 写入 `runtime-manifest.json` 的 `python.source/version/build` 字段由 task_002 完成；本 task 仅准备文件。

## 参考文件
- `NCdesktop/scripts/prepare-embedded-python.sh`（现有，需改造）
- `tasks/task_001_architect/output.md` ADR-001 / ADR-003
- session_context.md §5 代码规范（Bash 与 Python 嵌入段）

## 预估影响范围
- 新建文件：`scripts/verify-rpath.sh`
- 修改文件：`scripts/prepare-embedded-python.sh`
- 产物目录：`src-tauri/resources/python/`（不入 git，CI 时生成）
