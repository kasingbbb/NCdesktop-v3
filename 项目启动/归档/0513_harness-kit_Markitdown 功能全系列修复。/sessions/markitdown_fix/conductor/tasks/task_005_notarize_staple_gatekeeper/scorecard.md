# Review Scorecard — task_005_notarize_staple_gatekeeper (T-E)

## 审查思考过程

1. **Task 意图**：为已签名（task_004 产出）的 `.dmg` 走 App Store Connect API-key 模式的公证流程（`xcrun notarytool submit --wait`），成功后 `xcrun stapler staple` 将 ticket 嵌入 DMG，并以 `spctl` 本地预检 + 干净 VM（接力 task_013）作为 Gatekeeper 通过判定。secret 全链路不入日志，CI workflow 完成即销毁 .p8。

2. **AC 检查结果**：
   - AC-1 ✅：`xcrun notarytool submit --wait --output-format json` + 双 `python3 -c` 解析 `status` / `id`；`Accepted` 继续，`Invalid|Rejected` 抓 `xcrun notarytool log <submission-id>` 后 `exit 1`，不重试。
   - AC-2 ✅：`xcrun stapler staple` 后 `grep -F -q "The staple and validate action worked!"` 字面校验，失败 exit≠0。
   - AC-3 ✅（本地部分）：脚本字面 `xcrun spctl -a -vv -t open --context context:primary-signature "$DMG"`，匹配 `accepted` + `Notarized Developer ID` 双关键词。干净 VM 真实判定显式延期到 task_013（output.md 明确 PENDING-CLEAN-VM）。
   - AC-4 ✅（同 AC-3）：脚本本身与 macOS 版本无关；VM 烟测同样接力 task_013。
   - AC-5 ✅：preflight + 每次 `notarytool` 调用前 `set +x` 包裹敏感行；`.p8` 启动 `chmod 600`；CI workflow 仅打印 secret 长度 `${#VAR}`；T8 trace 三个 sentinel grep 全 0 命中。
   - AC-6 ✅：`for attempt in 1 2 3` + 指数退避 5/15/45s（`SLEEP_BIN` mock 可注入）；触发条件用 `grep -E -i` 匹配 `504|timeout|timed out|network|connection (refused|reset|closed)|temporary failure|could not connect`；`Invalid|Rejected` 路径不进入重试分支。

3. **红线检查**：
   - 未使用 `altool` ✅（grep 仅在注释中说明"已退役"，实际命令路径全部 `xcrun notarytool` / `xcrun stapler` / `xcrun spctl`）。
   - secret 不入日志 ✅（无 `echo $NOTARY_KEY_ID` / `cat *.p8` 等命中；trace 路径双重 `set +x` 防护）。
   - 未触及非授权区 ✅（`git diff` 验证 `entitlements.plist` / `sign-bundle.sh` / `verify-venv-shim.sh` / Rust 文件 / 脱敏区脚本均 0 行变更）。
   - 未跳过公证 ✅（无"本地测试不公证"分支；本地无 secret 时是显式 WARN + 后续 ADR-005 警示文案，无 silent bypass）。

4. **关键发现**：
   - JSON 解析用 `python3 -c` 内联 + `re.search(r"\{.*\}", raw, re.DOTALL)` 抽首个 JSON 块，并把 `json.loads` 异常捕获为 `sys.exit(0)`（输出空串）。对 notarytool 实际输出（"Conducting pre-submission checks..." + JSON）健壮，对空/坏 JSON 也不会 crash。优于 `plutil`（plutil 对混合人工前缀报错）。
   - `build-macos-dmg.sh` 实际 diff 28+/22- 比 dev 自报的范围大——其中签名块替换归 task_004（已在 task_004 scorecard 范围内），本 task 真实变更仅"删除旧 `APPLE_NOTARY_PROFILE` keychain-profile 块 + 在 hdiutil 后插入 notarize.sh 调用"，未误删 hdiutil / 安装说明 / staging dir 等任一打包逻辑。
   - CI workflow `if: always()` shred 用 `rm -P -f` 先覆写再删除（macOS 专有覆写删除），fallback 普通 `rm -f`；GH-hosted `macos-14` runner 本就 ephemeral，显式删除满足审计。
   - submission-id 提取与 status 解析共用同一正则，独立两次 `python3` 调用——简洁；缺失 id 时不传空参给 `notarytool log`（`if [[ -n "$SUBMISSION_ID" ]]` 守卫）。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 35% | 5 | AC-1~6 全部满足；mock 测试矩阵覆盖 happy / Invalid / 504×N retry / 504 exhausted / 缺 env-arg-file / JSON 前缀混杂；脚本字面命令与 ADR-005 完全对齐 |
| 安全性 | 10% | 5 | secret 三层防护（`set +x` × 调用点、`chmod 600` 启动、CI workflow `if: always()` `rm -P`）；T8 trace sentinel 三 0 命中证据扎实；红线无命中 |
| 代码质量 | 10% | 4 | 注释信息密度高，AC 映射逐条标注；唯一 nit 是两次 `python3 -c` 几乎重复（status / id 抽取），可合并为单次返回两字段，但当前形式更易读 |
| 测试覆盖 | 25% | 4 | mock 矩阵 12 项实测 PASS（T1-T12）；真公证回路与干净 VM 受客观限制（无 API key / 无 VM）未跑，但显式声明并接力 task_013；CI workflow 已就绪等 secret 注入 |
| 架构一致性 | 10% | 5 | 与 ADR-005 字面对齐（`notarytool submit --wait` + `stapler staple` + spctl context:primary-signature）；API-key 模式正是 ADR-005 排除 altool 后的选型；目录路径 `scripts/notarize.sh` 与 ADR-005 line 164 一致 |
| 可维护性 | 10% | 5 | `XCRUN_BIN` / `SLEEP_BIN` mock 注入点设计良好；CI workflow 三个 secret 名称、长度打印、shred step 均自带说明；幂等可重入（`chmod 600` 重复执行无副作用） |

**综合分**：5×0.35 + 5×0.10 + 4×0.10 + 4×0.25 + 5×0.10 + 5×0.10 = 1.75 + 0.50 + 0.40 + 1.00 + 0.50 + 0.50 = **4.65 / 5**

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **`python3 -c` 双调用可合并为单次双字段返回**
   - **代码位置**：`scripts/notarize.sh:121-147`
   - **现状**：status 与 submission-id 各跑一次 `python3 -c`，正则与 `json.loads` 重复。
   - **建议**：合并为单次 `print(obj.get("status",""), obj.get("id",""))`，bash 端 `read STATUS SUBMISSION_ID <<<"$(...)"`。
   - **不阻断 PASS 的原因**：当前形态运行成本可忽略（毫秒级），代码可读性反而更高，纯属优化项。

2. **stapler 步骤无重试**
   - **代码位置**：`scripts/notarize.sh:213-223`
   - **现状**：stapler 失败直接 exit 1。dev 已识别（output.md "已知局限 3"）：Apple CloudKit 查询慢偶有瞬时 stapler 失败。
   - **建议**：未来 CI 反复碰到时可加 2 次小重试（10s/30s）。input.md AC-2 字面只要求"成功后字面命中"，无重试硬约束，故本次不强求。
   - **不阻断 PASS 的原因**：当前命中频率不足以构成 MAJOR；增量优化候选。

3. **`case "$-" in *x*) ;; *) : ;;` 实际无副作用**
   - **代码位置**：`scripts/notarize.sh:115`
   - **现状**：注释声称"re-enable trace only if caller had it on"，但 `case` 两个分支都是空（`;;` 和 `: ;;`），实际什么都不做。`set +x` 之后从未 `set -x`。
   - **建议**：要么删除这段死代码 + 调整注释，要么把 `case` 分支真的写成 `set -x`。当前形态是无害的"装饰"。
   - **不阻断 PASS 的原因**：不影响 secret 不入日志的目标——子 shell 退出后 trace 状态自然恢复；AC-5 已通过。

## 给 Dev 的修复指引

判定 **PASS**，无需 Dev 修复。MINOR 项可作为后续打磨候选，不在本任务回环范围内。

## 后置确认事项（Conductor 转交 task_013）

- AC-3 / AC-4 真公证 + 干净 macOS 12 / 14 arm64 VM 首启 Gatekeeper 通过——本 task 显式 PENDING-CLEAN-VM，需 task_013 接力闭环。
- CI workflow `Notarize DMG` 需 PM/Owner 在 GitHub Settings → Secrets 注入 `NOTARY_KEY_ID` / `NOTARY_ISSUER_ID` / `NOTARY_KEY_P8_BASE64` 后方可端到端运行；workflow 自身已就绪。

---

**审查人**：Reviewer Agent
**审查时间**：2026-05-13
**所用权重来源**：session_context.md §4（功能正确性 35% / 安全性 10% / 代码质量 10% / 测试覆盖 25% / 架构一致性 10% / 可维护性 10%）
