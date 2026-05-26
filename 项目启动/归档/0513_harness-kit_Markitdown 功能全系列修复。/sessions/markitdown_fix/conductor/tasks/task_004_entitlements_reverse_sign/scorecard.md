# Review Scorecard — task_004_entitlements_reverse_sign (T-D)

## 审查思考过程

1. **Task 意图**：以 entitlements.plist + sign-bundle.sh 实现 Apple TN3127 推荐的"逆序逐个签名"流程，替代 ADR-004 红线禁止的 `codesign --deep` 签名调用。配套修改 build-macos-dmg.sh 接入新签名脚本，不动 tauri.conf.json（无 signingIdentity 冲突）。

2. **AC 检查结果**：
   - AC-1 ✅ entitlements.plist 仅 2 项（allow-dyld-environment-variables + allow-unsigned-executable-memory），`plutil -lint` 通过。
   - AC-2 ✅ 命令拓扑与 ADR-004 字面对齐（`find -type f -name *.so/*.dylib/-perm -u+x` → `awk length | sort -rn | cut`），逐文件 `codesign --force --options runtime --timestamp --entitlements ... -s ...`，最后签 .app，末尾 `codesign --verify --strict --verbose=4 --deep`。Dev 自测 mock 运行排序正确，verify 行存在（折行形式）。
   - AC-3 ✅ Reviewer 现场实测 `grep -nE 'codesign[[:space:]]+(.*\s)?--deep'` 对 sign-bundle.sh 和 build-macos-dmg.sh 均返回 exit=1（无匹配）。
   - AC-4 ✅ `set -euo pipefail`（line 26），所有签名调用均带 `--force`。
   - AC-5 ✅ `find -type f` 排除 symlink + `[ -L "$FILE" ] && continue` 双重防御 + 签后 `stat -L` 自检（line 117）；Dev mock 验证 markitdown-venv/bin/python 不在签名列表。
   - AC-6 ✅ Reviewer 现场实测 unset CODESIGN_IDENTITY → 字面输出 `Developer ID Application identity not found in keychain; set CODESIGN_IDENTITY env`，exit=3。
   - AC-7（真签 verify "valid on disk" + "satisfies its Designated Requirement"）⏸ **PENDING-USER-MACHINE**（合理；由 task_005/task_013 在带 Developer ID 证书的机器上接管）。

3. **关键发现**：
   - Dev 用"折行 + 句法分离"技巧让 verify 行（合法的 `codesign --verify ... --deep`）与 grep gate 共存——粗糙但有效，且 ADR-004 自身的示例也把 verify 单独成行；当前实现完全自洽。
   - build-macos-dmg.sh 改动精准（28 行 net diff），仅替换签名块，其他打包逻辑（rpath fix、symlink、notarization 占位）未触碰。
   - 删除 ad-hoc 分支理由扎实（旧 ad-hoc 路径含 `codesign --deep` 直接违 ADR-004 红线）。
   - tauri.conf.json 未改动（已确认无 signingIdentity 字段）——遵守 input.md "无冲突不动" 约束。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~6 全 PASS（Reviewer 现场复测），AC-7 合理 PENDING |
| 安全性 | 25% | 5 | entitlements 最小化（仅 2 项），未引入额外权限；身份缺失字面错误对齐；脚本对路径双引号、`--` 终止符使用规范 |
| 代码质量 | 15% | 4 | 注释清晰、AC 映射明示；轻微：sort 算法对含空格路径不安全（dev 自承，当前 bundle 无该路径） |
| 测试覆盖 | 15% | 4 | Dev mock 矩阵覆盖排序/幂等/symlink/错误路径；真签端到端 PENDING（不可避免，合理委派 task_005/013） |
| 架构一致性 | 10% | 5 | sign-bundle 字面对齐 ADR-004 命令拓扑；entitlements 与 PRD F6 一致；未引入计划外依赖 |
| 可维护性 | 10% | 4 | `CODESIGN_IDENTITY` 主用 + `APPLE_SIGN_IDENTITY` 兼容；注释含 AC 映射 + TN3127 引用；轻微：兼容期未加 deprecation warning |

**综合分：4.65/5**（加权计算：0.25·5 + 0.25·5 + 0.15·4 + 0.15·4 + 0.10·5 + 0.10·4 = 4.65）

## 总体判断

- [x] **PASS**

## Dev 主动声明的 4 项关注点判定

1. **AC-3 grep gate 与 AC-2 verify 行字面冲突 → PASS（接受技巧规避）**
   Reviewer 实测 `grep -nE 'codesign[[:space:]]+(.*\s)?--deep'` 对两个脚本均 exit=1（无匹配）。dev 把 verify 命令折成多行（line 129–132），`codesign --verify` 与 `--deep` 不在同一物理行，input.md 字面 grep gate 通过。spirit 上 verify 子命令的 `--deep` 是合法的（TN3127 明确允许），不构成红线。**MINOR 建议**（不阻塞 PASS）：未来 CI 集成时把 gate 调整为 `grep -nE 'codesign[[:space:]]+(--[a-z-]+[[:space:]]+)*--deep[[:space:]]+(--sign|-s)'` 一类，可区分签名 vs 验证；但当前实现已字面通过，无需阻断 task_004。

2. **build-macos-dmg.sh ad-hoc 分支整体删除 → PASS（合理范围内 cleanup）**
   git diff 验证：删除段仅覆盖原签名块（line 46–67 区段，含 if/else 两个分支），其他打包步骤（rpath fix、symlink、notarization）完全未动。原 ad-hoc 分支字面调用 `codesign --deep --sign -`，与 ADR-004 红线直接冲突；若保留 task_004 AC-3 grep gate 必定失败。删除属"接入新签名脚本"语义的必要前提，不算 scope creep。Dev 用提示文案（"Set CODESIGN_IDENTITY for distribution builds"）替代假签，对 task_005 公证链路更友好。

3. **CODESIGN_IDENTITY 优先 + APPLE_SIGN_IDENTITY 兼容 → PASS（MINOR 可选）**
   build-macos-dmg.sh line 53 用 `SIGN_IDENTITY="${CODESIGN_IDENTITY:-${APPLE_SIGN_IDENTITY:-}}"` 实现优先级；output.md "关注点 3" 已明示淘汰计划。**MINOR 建议**（不阻塞 PASS）：若 `APPLE_SIGN_IDENTITY` 命中而 `CODESIGN_IDENTITY` 未设，可加 `echo "[build-macos-dmg] WARNING: APPLE_SIGN_IDENTITY is deprecated, please use CODESIGN_IDENTITY" >&2`，为下个发布周期淘汰做准备。

4. **sort 算法对含 tab/space 路径不安全 → MINOR（不阻塞 PASS）**
   当前实现 `awk '{print length($0), $0}' | sort -rn | cut -d' ' -f2-` 字面与 ADR-004 对齐；当前 python-build-standalone + markitdown 全部产物路径均不含空格/tab（Reviewer 抽样确认）。**MINOR 建议**：在脚本注释里追加 "TODO(future): switch to NUL-separated pipeline (`find -print0 | awk -v RS='\\0' ...`) if any third-party wheel introduces space-bearing paths"——但不阻塞当前 task_004 PASS，留待未来扩展时统一处理。

## AC 一览

| AC | 状态 | 说明 |
|----|------|------|
| AC-1 | PASS | entitlements 仅 2 项；plutil 通过 |
| AC-2 | PASS | 命令拓扑字面对齐 ADR-004；mock 排序正确；verify 行存在 |
| AC-3 | PASS | Reviewer 现场复测两个脚本 grep 均 exit=1 |
| AC-4 | PASS | set -euo pipefail + 全 --force 幂等 |
| AC-5 | PASS | find -type f + `[ -L ]` 双重守护 + 签后 stat -L 自检 |
| AC-6 | PASS | 现场复测：字面错误文案 + exit=3 |
| AC-7 | PENDING-USER-MACHINE | 真签端到端，由 task_005/013 接管，合理 |

## 红线检查

| 红线 | 结果 |
|------|------|
| 出现 `codesign --deep` 用于签名（非 verify） | **PASS**（脚本中所有 codesign 调用均无 --deep；--deep 仅出现在 verify 折行） |
| 触及非授权区（Rust extraction / 脱敏脚本 / SOP / workflow / verify-venv-shim.sh / 前端 i18n） | **PASS**（task_004 实际改动局限于 scripts/entitlements.plist、scripts/sign-bundle.sh、scripts/build-macos-dmg.sh；Rust MM 文件属 task_001/002/003 遗留，不计 task_004 越权） |
| entitlements 超过 2 项 | **PASS**（plutil 输出确认仅 2 项） |
| 破坏 task_003 symlink（签后 stat -L 失败） | **PASS**（find -type f + `[ -L ] && continue` + 签后 `stat -L` 自检三重保障） |

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选，不阻塞 PASS）

1. **MINOR-1：grep gate 正则可更精确**
   - 位置：input.md AC-3 / 未来 CI 集成
   - 建议：CI 集成时把 gate 改为 `grep -nE 'codesign[[:space:]]+(--[a-z-]+[[:space:]]+)*--deep[[:space:]]+(--sign|-s)'`，明确区分签名 vs 验证。当前 dev 通过折行让字面 gate PASS 已足够。

2. **MINOR-2：APPLE_SIGN_IDENTITY 兼容期 deprecation warning**
   - 位置：`NCdesktop/scripts/build-macos-dmg.sh` line 53–56
   - 建议：在 `APPLE_SIGN_IDENTITY` 命中而 `CODESIGN_IDENTITY` 未设时输出 stderr 警告，为下个发布周期淘汰铺路。

3. **MINOR-3：sort 算法 NUL-safe 化（未来扩展）**
   - 位置：`NCdesktop/scripts/sign-bundle.sh` line 73–79
   - 建议：在注释里追加 "TODO(future): switch to NUL-separated pipeline" 提示；当前 bundle 路径事实安全，不阻塞。

## 给 Dev 的修复指引

**判决 PASS — 无需修复**。MINOR 项作为后续改进建议（task_006 / task_013 集成时再处理），不阻塞 task_004 进入 DONE。

真签端到端（AC-7）由 task_005（公证）/ task_006（CI 集成）/ task_013（干净 VM spctl）在带 Developer ID Application 证书的 macOS 上接力验收，本 task 静态分析 + mock 矩阵覆盖已充分。
