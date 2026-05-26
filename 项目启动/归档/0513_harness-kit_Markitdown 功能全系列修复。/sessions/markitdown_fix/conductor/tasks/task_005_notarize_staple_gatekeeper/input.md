# Task 输入 — task_005_notarize_staple_gatekeeper (T-E)

## 目标
对已签名的 `.app` / `.dmg` 调用 `xcrun notarytool submit --wait` + `xcrun stapler staple`，并在两台干净 macOS arm64 VM（12 / 14）首启验证 Gatekeeper 通过。

## 前置条件
- 依赖 task：task_004（签名完成）
- 必须先存在的文件/接口：已签名的 `NoteCapt.app`
- 凭证：App Store Connect API key（`key_id` / `issuer_id` / `.p8`）通过环境变量或 CI secret 提供

## 验收标准（Acceptance Criteria）
1. AC-1：`scripts/notarize.sh` 执行：
   - `xcrun notarytool submit "$DMG" --key-id $KEY_ID --key "$P8_PATH" --issuer $ISSUER --wait` 退出码 0；
   - 输出 JSON 中 `status == "Accepted"`；任何 `Invalid`/`Rejected` 抓取 log 并退出非零。
2. AC-2：成功后 `xcrun stapler staple "$DMG"` 必须输出 `The staple and validate action worked!`。
3. AC-3：在干净 macOS 12 arm64 VM 上：
   - 拷贝 DMG → 双击挂载 → 拖入 Applications → 双击启动；
   - `spctl -a -vv -t open --context context:primary-signature "$DMG"` 输出 `accepted: Notarized Developer ID`；
   - 首次启动**无任何"未识别开发者"对话框**。
4. AC-4：同样流程在 macOS 14 arm64 VM 通过。
5. AC-5：脚本支持本地与 CI 两种凭证来源；secret 不得写入日志（`set +x` 包围敏感行）。
6. AC-6：失败重试策略：notarytool 504/网络错最多重试 3 次，间隔指数退避。

## 技术约束
- 严禁使用已退役的 `altool`。
- API key `.p8` 文件权限必须 600；CI 用完即销毁。
- 不得绕过公证（即使本地测试也走完整流程，避免"本地 OK 上线挂"）。

## 参考文件
- ADR-005
- PRD §3.1 F9 / §4.2

## 预估影响范围
- 新建文件：`scripts/notarize.sh`
- 修改文件：`scripts/build-macos-dmg.sh`（调用本脚本）、CI workflow（注入 secret）
