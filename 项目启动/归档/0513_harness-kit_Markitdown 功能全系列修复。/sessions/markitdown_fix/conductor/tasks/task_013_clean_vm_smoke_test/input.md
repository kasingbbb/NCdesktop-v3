# Task 输入 — task_013_clean_vm_smoke_test

## 目标
在 macOS 12 与 macOS 14 干净 arm64 VM（无 brew / 无开发者权限 / 无 Python）上对最终 DMG 做端到端冒烟：Gatekeeper 通过 + 7 格式各 1 真实样本转录成功。

## 前置条件
- 依赖 task：task_005、task_006（已签名 staple DMG）、task_012（真实样本可用）
- 必须先存在的文件/接口：DMG 产物、解密样本

## 验收标准（Acceptance Criteria）
1. AC-1：使用 `tart` 或 `UTM` 或 Apple Virtualization framework 准备两个 base image：
   - macOS 12.x arm64（最近补丁）
   - macOS 14.x arm64（最近补丁）
   每次冒烟从 base snapshot 复原（保证"干净"）。
2. AC-2：自动化脚本 `scripts/vm-smoke.sh`：
   - 复原 snapshot；
   - 拷贝 DMG；
   - AppleScript（或 osascript）模拟"双击挂载 → 拖入 Applications → 双击启动"；
   - 等待启动完成（前端日志或 IPC ready 信号）；
   - AppleScript 触发文件拖入 → 等转录完成 → 抓 `conversion_meta`；
   - 7 格式各 1 样本：pdf-text / docx / pptx / xlsx / html / epub / image；扫描 pdf 单独跑一次验证 `E_SCAN_PDF_UNSUPPORTED`；
   - 输出 `vm_smoke_report.json`。
3. AC-3：三次冷启动（每次完整 snapshot 复原）100% 成功率；中间任何一次失败 → 整体不通过。
4. AC-4：Gatekeeper：首启**无任何**"未识别开发者"对话；`spctl -a -vv "$DMG"` 输出 `accepted: Notarized Developer ID`。
5. AC-5：DMG 体积报告与 `dmg_size_report.txt` 一致；冷启动到主窗口可交互 P95 < 2s（PRD §4.1）。
6. AC-6：若 CI 无 macOS runner，本 task 由本地手动执行 + 截屏 + 报告归档；output.md 记录"已知 CI 限制"。

## 技术约束
- 严禁在 VM 内预装任何依赖（违反"干净"前提）。
- AppleScript 必须有超时；任一步骤超时即标失败。

## 参考文件
- PRD §6 MVP 验收门禁
- ADR-005

## 预估影响范围
- 新建：`scripts/vm-smoke.sh`、`scripts/vm-base-image.md`（VM 制作 SOP）
- 报告归档目录：`sessions/markitdown_fix/conductor/tasks/task_013_clean_vm_smoke_test/artifacts/`
