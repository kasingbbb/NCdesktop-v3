# Task 输入 — task_006_build_macos_dmg_integration (T-F)

## 目标
整合 T-A..T-E 全部产出，让 `scripts/build-macos-dmg.sh` 一键产出可分发的已公证 DMG，并增加 ≤300MB 体积门禁 + DMG 内 symlink 完整性自检。

## 前置条件
- 依赖 task：task_001..005 全部 DONE
- 必须先存在的文件/接口：上述脚本与产物

## 验收标准（Acceptance Criteria）
1. AC-1：一键执行 `scripts/build-macos-dmg.sh` 顺序完成：
   `prepare-embedded-python.sh → prepare-embedded-markitdown-runtime.sh → prepare-venv-shim.sh → tauri build → sign-bundle.sh → hdiutil create dmg → sign dmg → notarize.sh → stapler staple → du -sh`
2. AC-2：DMG 大小 ≤ 300 MB（CI 失败门禁，超出即非零退出）；输出 `dmg_size_report.txt` 含 `du -sh` + `du -sh Resources/python`/`Resources/markitdown-venv` 子项。
3. AC-3：DMG 挂载后 `ls -l Resources/markitdown-venv/bin/python` 显示 symlink 且 `readlink` 输出相对路径；用 `hdiutil` 制作过程保留 symlink（不可用 `cp -L`）。
4. AC-4：脚本 `set -euo pipefail` + 幂等；`--release` / `--debug` 标志切换；任何步骤失败立即中止并打印失败步骤名。
5. AC-5：CI workflow（macOS runner 可用时）跑通端到端；如无 macOS runner，本 AC 由本地脚本输出 + manifest checksum 替代，在 output.md 记录。
6. AC-6：产物 SHA256 同步写入 `dist/<version>.sha256`，供官网公示。

## 技术约束
- 严禁在脚本里 `cp -RL`（会跟随 symlink 复制目录，破坏 ADR-003）。
- 严禁跳过任何 T-A..T-E 步骤（即使本地缓存）；幂等通过"产物 hash 比对"实现而非"跳过执行"。

## 参考文件
- `NCdesktop/scripts/build-macos-dmg.sh`（现有，重写）
- task_001..005 input/output
- PRD §6 MVP 验收门禁

## 预估影响范围
- 修改文件：`scripts/build-macos-dmg.sh`
- 新建文件：`dist/`（产物目录，不入 git）
