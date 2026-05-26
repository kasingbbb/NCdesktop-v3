# Review Scorecard — task_006_build_macos_dmg_integration (T-F)

## 审查思考过程

1. **Task 意图**：把 T-A..T-E 的子脚本编排为一键 DMG 构建管线，叠加 ≤300MB 体积门禁 / DMG 内 symlink 完整性自检 / SHA256 落盘三道质量门。

2. **AC 检查结果**：
   - AC-1 ✅ 10 步顺序严格匹配 input.md 字面顺序，`grep -nE '\[step [0-9]+/10'` 验证 10 行步骤注释 + 实际调用点全部到位
   - AC-2 ✅ KB 整数比较（`du -sk`）+ 阈值 307200KB（= 300 MiB，1024-base，与 `du -sh "300M"` 字面一致）+ `dmg_size_report.txt` 含人类可读总量 / KB / python 子项 / venv 子项 / timestamp / git_rev / profile，report 先写后判，便于诊断
   - AC-3 ✅ `hdiutil attach -readonly -nobrowse -noverify -noautoopen`，断言 `[[ -L ${SHIM_PATH} ]]`，`readlink` → `is_relative_symlink_target` 拒绝 `/*`；`hdiutil create -srcfolder` 保留 symlink；trap 兜底 detach（`MOUNT_POINT` 变量在正常路径 detach 后置空避免二次 detach）
   - AC-4 ✅ `set -euo pipefail` + trap EXIT + `CURRENT_STEP` 变量打印失败步骤名 + `--release` / `--debug` 标志切换 target/{release,debug}/bundle 路径，help 文本完整
   - AC-5 ⚠ PENDING-CI（已在 output.md "已知局限"明确推迟到 task_013_clean_vm_smoke_test，input.md AC-5 本身就给出了"如无 macOS runner，本 AC 由本地脚本输出 + manifest checksum 替代"豁免条款，dev 选择合理）
   - AC-6 ✅ `read_tauri_version` 用 `python3 -c json`（macOS 自带）安全读取，缺字段返回空串且脚本显式校验空串→fail；`shasum -a 256` 输出经 awk 重写为 basename，shasum -c 兼容；落盘 `dist/<version>.sha256`

3. **关键发现**：
   - 双轨设计（`du -sk` 数值比较 + `du -sh` 仅作人类报告）是规避"1.2G < 300M"字符串陷阱的正确处理，mock 矩阵 5 个 case 全部判定正确
   - 步骤 9 重复 staple 的设计选择合理：notarize.sh 内部已 staple，step 9 仅在 notarize 跳过分支打印提示。stapler 对已 staple DMG 二次调用幂等，但保持日志清洁是合理工程选择
   - 红线扫描全 PASS：脚本内 0 处 `cp -L` / `cp -RL` / `cp -LR` 变体；step 4b inject 与 step 6 staging 均用 `cp -R`（保留 symlink）
   - 范围控制良好：本 task 只动 build-macos-dmg.sh 一文件；prepare-embedded-* 的 git diff 来自 task_001/002（mtime 18:07/18:39，远早于 task_006 开工 22:28）

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 6/6 AC 满足（AC-5 按 input.md 豁免条款合规推迟）；mock 矩阵覆盖边界 + 单位陷阱；trap 失败诊断验证可用 |
| 安全性 | 25% | 5 | `mktemp -d -t` 安全；`hdiutil attach -readonly` 只读挂载防污染；trap 兜底防挂载残留；版本读取无 shell 注入（python3 -c 用单引号包路径） |
| 代码质量 | 15% | 5 | 注释充足（每个 ADR / red line 在注释中复述）；步骤标题统一格式；helper 函数（check_size_gate / is_relative_symlink_target / read_tauri_version）单一职责且可独立测试 |
| 测试覆盖 | 15% | 4 | T1-T7 完整：步骤顺序 / cp -L 红线 / 体积矩阵 / 相对 symlink 矩阵 / trap 诊断 / SHA256 格式 / bash -n。tauri build 真实端到端按 input.md AC-5 豁免推迟到 task_013，合理 |
| 架构一致性 | 10% | 5 | ADR-001/003/004/005/010 全部落地：python-build-standalone 注入、`../../python/bin/python3` 相对 symlink、逆序签名（task_004 sign-bundle.sh）、notarytool + stapler（task_005 notarize.sh）、`dist/<version>.sha256` |
| 可维护性 | 10% | 5 | 配置外部化（`APP_NAME` / `SIZE_LIMIT_KB` / `SELFTEST` / `CODESIGN_IDENTITY` / `NOTARY_*` 全可注入）；--help 完整；错误日志统一前缀 `[build-macos-dmg]` |

**综合分：4.9/5**（加权：0.25×5 + 0.25×5 + 0.15×5 + 0.15×4 + 0.10×5 + 0.10×5 = 4.85）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER

无。

### MAJOR

无。

### MINOR（可选修复，不影响 PASS）

1. **STAGING_DIR 泄漏窗口**
   - **位置**：`build-macos-dmg.sh:224`，`STAGING_DIR=$(mktemp -d -t notecapt-staging.XXXXXX)`
   - **现象**：cleanup trap 只清理 `MOUNT_POINT`，未涵盖 `STAGING_DIR`；如果在 line 224 之后、line 254（`rm -rf "${STAGING_DIR}"`）之前失败，临时目录会在 `$TMPDIR` 下残留
   - **影响**：极小 — macOS 周期性清理 `$TMPDIR`，且文件只是 `.app` 的副本（无机密）
   - **建议**：在 cleanup() 内增加 `[[ -n "${STAGING_DIR:-}" && -d "${STAGING_DIR}" ]] && rm -rf "${STAGING_DIR}"`；并在 line 73 后声明 `STAGING_DIR=""`

2. **SIZE_REPORT 缺 KB 数值便于 CI 解析**
   - **位置**：`build-macos-dmg.sh:315-327`
   - **现象**：report 含 `(${DMG_KB} KB)` 在 dmg_total 行的圆括号内，下游 CI 若想精确解析需要写 awk/regex 抽 `\d+ KB`。可以独立加一行 `dmg_kb: ${DMG_KB}` 字段，让 CI 一行 `grep ^dmg_kb` 拿到
   - **影响**：极小 — 当前格式已可解析

3. **AC-2 阈值常量"300 MB"语义注释可更显式**
   - **位置**：`build-macos-dmg.sh:84-85`
   - **现象**：注释写 `300 MB == 300 * 1024 KB == 307_200 KB`，技术上是 300 MiB（1024-base），与下方 `du -sh` 输出的 "300M" 一致。input.md AC-2 写"≤ 300 MB"未明确二进制 / 十进制；建议在注释里写明"binary 300 MiB to match du -sh's output unit"以杜绝后续读者疑惑
   - **影响**：极小 — 行为正确，仅文档

## 给 Dev 的修复指引

无（PASS，无需修复）。MINOR 项 dev 可在后续 task 顺手优化或忽略。

## 红线扫描结果

| 红线 | 结果 |
|------|------|
| `cp -RL` / `cp -L` / `cp -LR` 复制目录 | PASS（grep 0 命中） |
| 触及非授权区（sign-bundle.sh / entitlements.plist / notarize.sh / prepare-*.sh / Rust） | PASS（git diff 范围限于 build-macos-dmg.sh；prepare-* 的 M 状态 mtime 18:07/18:39 << 22:28 开工时间，属 task_001/002 预存量） |
| 跳过 T-A..T-E 任一步骤 | PASS（10 步全在，调用点全在） |
| 体积门禁缺失或误判 | PASS（mock 矩阵 5 case 全对，含 1.2G 单位陷阱 + 300M+1KB 严格边界） |

## 关注点

| 关注点 | 结论 |
|--------|------|
| KB 阈值字面 307200 vs 300000 | 307200（300 MiB，binary）— 与 `du -sh "300M"` 字面一致，符合 macOS 工具链惯例；input.md "≤300MB" 在该上下文里二进制语义合理 |
| staple 重复（notarize.sh 内 + step 9 外）调用幂等性 | 已规避 — step 9 仅在 notarize 跳过分支打印提示，notarize 正常路径下不二次调用 stapler；dev 注释解释了 stapler 二次调用的幂等性作为兜底论证，合理 |
| 版本号读 tauri.conf.json 用 python3 健壮性 | 健壮 — `json.load(f).get('version', '')` 缺字段返回空串，外层 `if [[ -z "${VERSION}" ]]; then ... exit 1` 显式拦截；不依赖 jq |
| mount tmp 路径生成（mktemp -d）路径注入安全 | 安全 — `mktemp -d -t notecapt-mount.XXXXXX` 在 `$TMPDIR` 下生成不可预测的名字；hdiutil attach 用 `-mountpoint "${MOUNT_POINT}"` 双引号包裹防词法切分 |

## AC 总览

| AC | 结果 |
|----|------|
| AC-1 | PASS |
| AC-2 | PASS |
| AC-3 | PASS |
| AC-4 | PASS |
| AC-5 | PENDING-CI（input.md 豁免条款 + 委托 task_013，合规） |
| AC-6 | PASS |
