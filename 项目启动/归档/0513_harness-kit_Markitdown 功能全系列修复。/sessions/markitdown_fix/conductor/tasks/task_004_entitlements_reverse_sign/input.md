# Task 输入 — task_004_entitlements_reverse_sign (T-D)

## 目标
编写 `entitlements.plist` + `scripts/sign-bundle.sh`，使用**逆序逐个签名**对 `.app` 内所有可执行/动态库及最外层 bundle 做 Developer ID 签名（替换 `codesign --deep`）。

## 前置条件
- 依赖 task：task_001 / task_002 / task_003（DMG 内 Python + extras + shim 全部就位）
- 必须先存在的文件/接口：构建后的未签名 `NoteCapt.app`
- 凭证：本地或 CI keychain 中已导入 Developer ID Application 证书（identity 名通过环境变量传入）

## 验收标准（Acceptance Criteria）
1. AC-1：`scripts/entitlements.plist` 显式包含且仅包含：
   - `com.apple.security.cs.allow-dyld-environment-variables` = true
   - `com.apple.security.cs.allow-unsigned-executable-memory` = true
2. AC-2：`scripts/sign-bundle.sh`：
   - 收集所有 `*.so` / `*.dylib` / 有 `u+x` 权限的二进制；
   - 按路径长度**倒序**排序（深层先签）；
   - 每个文件 `codesign --force --options runtime --timestamp --entitlements <plist> -s "$IDENTITY"`；
   - 最后 `codesign --force --options runtime --timestamp --entitlements <plist> -s "$IDENTITY" "$APP"`；
   - 末尾 `codesign --verify --deep --strict --verbose=4 "$APP"` 必须输出 `valid on disk` + `satisfies its Designated Requirement`。
3. AC-3：**严禁出现 `codesign --deep` 调用**（grep 检查作为 CI gate）。
4. AC-4：脚本 `set -euo pipefail` + 幂等；`--force` 标志清除旧签名再签。
5. AC-5：签后 `Resources/markitdown-venv/bin/python` symlink 仍可解析（`stat -L` 成功）。
6. AC-6：未导入证书时报错文案明确：`Developer ID Application identity not found in keychain; set CODESIGN_IDENTITY env`。

## 技术约束
- entitlements 不得开放任何额外权限（最小化原则）。
- 不得直接修改 `tauri.conf.json` 的内置 signing 配置（避免与外部脚本冲突）；如有冲突，禁用 tauri 内置签名，全部交给本 task 的脚本。

## 参考文件
- ADR-004 / Apple TN3127
- PRD §3.1 F6/F7
- 现有 `scripts/fix-note-capt-gatekeeper.sh`（仅参考，不复用）

## 预估影响范围
- 新建文件：`scripts/entitlements.plist`、`scripts/sign-bundle.sh`
- 修改文件：`scripts/build-macos-dmg.sh`（task_006 中调用本脚本）、可能 `src-tauri/tauri.conf.json`（关闭内置签名）
