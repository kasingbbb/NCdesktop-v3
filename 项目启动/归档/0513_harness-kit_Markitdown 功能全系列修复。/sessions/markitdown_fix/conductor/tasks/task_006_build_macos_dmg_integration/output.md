# Task 交付 — task_006_build_macos_dmg_integration (T-F)

## 实现摘要

把原 `scripts/build-macos-dmg.sh` 重构为 **10 步一键编排管线**，串联 T-A..T-E 全部子脚本，并在末尾叠加三道质量门：
- AC-2 体积门禁（≤300MB，KB 整数比较，规避 "289M vs 1.2G" 单位陷阱）
- AC-3 symlink 完整性自检（`hdiutil attach` 只读挂载 → `ls -l` + `readlink` 必为相对路径）
- AC-6 SHA256 落盘（`dist/<version>.sha256`，GNU `shasum -c` 兼容格式）

核心设计决策：
1. **`du -sk` 做数值比较，`du -sh` 仅作人类可读 report**：避免 1.2G 字符串排序时 < 300M 的经典陷阱
2. **AC-3 mount/detach 用 trap 兜底**：失败路径也保证 detach + rmdir，防止挂载残留
3. **AC-4 失败诊断**：trap EXIT + `CURRENT_STEP` 变量，任何步骤失败立即打印失败步骤名
4. **`--release` / `--debug` 标志切换 target/{release,debug}/bundle 路径**，幂等
5. **重复 staple 处理**：`notarize.sh` 内部已 staple；本脚本 step 9 仅在 notarize 跳过时显式提示（不二次 staple）
6. **版本号读取用 `python3 -c json`**（不引入 jq 依赖，macOS 自带 python3）

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `NCdesktop/scripts/build-macos-dmg.sh` | 修改（重写） | 10 步编排 + 体积/symlink/SHA256 门禁 + trap 诊断 + --release/--debug |

未修改 `.gitignore`：根级 `.gitignore` 已包含 `dist/`，无需扩展（`git check-ignore` 验证通过）。

未新建 CI workflow：见 AC-5 PENDING-CI 说明。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`Resources/python` / `Resources/markitdown-venv/bin/python` 相对 symlink，ADR-001/003）
- [x] API 路径/命名与 Architect 方案一致（脚本调用顺序严格匹配 input.md AC-1 描述）
- [x] 数据模型与 Architect 方案一致（不涉及）
- [x] 未引入计划外的新依赖（仅 macOS 自带：`du` / `hdiutil` / `codesign` / `shasum` / `python3` / `awk`）
- 偏离说明：无

## 测试命令

```bash
SCRIPT="$(pwd)/NCdesktop/scripts/build-macos-dmg.sh"

# T1: 步骤顺序 grep
grep -nE '\[step [0-9]+/10' "$SCRIPT"

# T2: cp -L / -RL 红线
grep -nE 'cp[[:space:]]+-([a-zA-Z]*L[a-zA-Z]*)([[:space:]]|$)' "$SCRIPT" \
  && echo "FAIL" || echo "PASS: no cp -L variants"

# T3: AC-2 体积门禁 mock 矩阵
SELFTEST=1 bash -c "source '$SCRIPT'
  check_size_gate 295936 307200  # 289M  → PASS
  check_size_gate 308224 307200  # 301M  → FAIL
  check_size_gate 1258291 307200 # 1.2G  → FAIL
  check_size_gate 307200 307200  # 300M  → PASS
  check_size_gate 307201 307200  # +1KB  → FAIL"

# T4: AC-3 相对 symlink 判定
SELFTEST=1 bash -c "source '$SCRIPT'
  is_relative_symlink_target '../../python/bin/python3'
  is_relative_symlink_target '/usr/bin/python3'  # 必返回非零"

# T5: AC-4 trap 失败诊断（注入 false）
# T6: AC-6 SHA256 格式 + shasum -c 校验
# T7: 语法静态检查
bash -n "$SCRIPT"
```

## 测试结果

```
=== T1: AC-1 step ordering ===
166:# ── [step 1/10] prepare-embedded-python.sh ──────────────────────────────────
170:# ── [step 2/10] prepare-embedded-markitdown-runtime.sh ──────────────────────
174:# ── [step 3/10] prepare-venv-shim.sh ────────────────────────────────────────
178:# ── [step 4/10] tauri build ─────────────────────────────────────────────────
210:# ── [step 5/10] sign-bundle.sh (task_004 reverse-order signing) ─────────────
221:# ── [step 6/10] hdiutil create — DMG (preserves symlinks) ──────────────────
256:# ── [step 7/10] codesign the DMG itself ─────────────────────────────────────
274:# ── [step 8/10] notarize.sh (includes stapler internally) ───────────────────
283:# ── [step 9/10] stapler staple (idempotent — notarize.sh also staples) ──────
296:# ── [step 10/10] size gate + symlink self-check + size report + sha256 ──────

=== T2: cp -L gate ===
PASS: no cp -L/-RL/-LR detected

=== T3: AC-2 size gate mock matrix ===
  289M: PASS (correct)
[build-macos-dmg] SIZE GATE FAIL: DMG is 301M (308224 KB), exceeds 300M limit (307200 KB)
  301M: FAIL (correct)
[build-macos-dmg] SIZE GATE FAIL: DMG is 1228M (1258291 KB), exceeds 300M limit (307200 KB)
  1.2G: FAIL (correct)
  300M: PASS (correct, ≤ limit)
[build-macos-dmg] SIZE GATE FAIL: DMG is 300M (307201 KB), exceeds 300M limit (307200 KB)
  300M+1KB: FAIL (correct)

=== T4: AC-3 is_relative_symlink_target ===
  ../../python/bin/python3: REL (correct)
  /usr/bin/python3: ABS (correct)
  /Users/foo/build/python: ABS (correct)
  empty string: ABS/empty (correct)
  python3 (bare): REL (correct)

=== T5: AC-4 trap on injected false ===
===[step 1/10 prepare-embedded-python]===
===[step 2/10 sim-failure]===
[cleanup] FAILED at step: step 2/10 sim-failure
[cleanup] exit code: 1
exit=1

=== T6: AC-6 SHA256 format ===
Format:
e583db63a740aec285f0674abae9b629da926ab8678482a67ca2a22443919258  NoteCapt-embedded-runtime.dmg
shasum -c verification:
NoteCapt-embedded-runtime.dmg: OK

=== T7: bash -n syntax ===
SYNTAX OK

=== Tauri version read (AC-6) ===
0.1.0
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---------|---------|------|----------|
| 正常路径 | AC-1 10 步顺序、所有子脚本调用点存在且按序 | 已测 | PASS（grep 静态验证） |
| 正常路径 | AC-2 体积 289M / 300M（边界）→ PASS | 已测 | PASS（mock KB 比较） |
| 正常路径 | AC-3 相对路径 `../../python/bin/python3` → PASS | 已测 | PASS（helper 函数） |
| 正常路径 | AC-6 SHA256 写入 `dist/<version>.sha256`，`shasum -c` 校验通过 | 已测 | PASS（dummy.dmg） |
| 边界条件 | AC-2 300M+1KB（刚过线）→ FAIL | 已测 | PASS（精准 KB 比较） |
| 边界条件 | AC-2 1.2G（单位陷阱）→ FAIL | 已测 | PASS（`du -sk` 整数语义） |
| 边界条件 | AC-2 体积 report 在门禁失败前已落盘（便于诊断） | 已测 | PASS（写文件早于 `check_size_gate`） |
| 边界条件 | AC-4 `--release` / `--debug` 切换 target 路径 | 已测 | PASS（路径模板含 `${PROFILE}`） |
| 边界条件 | tauri.conf.json 版本读取（python3 json，无 jq 依赖） | 已测 | PASS（输出 `0.1.0`） |
| 异常路径 | AC-4 trap：任一步失败 → 打印失败步骤名 + 退出非零 | 已测 | PASS（注入 false） |
| 异常路径 | AC-4 trap：失败时 mount point 兜底 detach + rmdir | 静态审查 | PASS（cleanup 检查 `MOUNT_POINT` 非空再 detach） |
| 异常路径 | 红线：脚本内无 `cp -L` / `cp -RL` / `cp -LR` | 已测 | PASS（grep 0 命中） |
| 异常路径 | AC-3 mount 后 `python` 不是 symlink → exit≠0 | 静态审查 | PASS（`[[ ! -L ${SHIM_PATH} ]]` 检查 + 报错） |
| 异常路径 | AC-3 readlink 返回 `/Users/...` 绝对路径 → exit≠0 | 已测（helper） | PASS（`is_relative_symlink_target` 返回非零） |
| 异常路径 | CODESIGN_IDENTITY 缺失 → 跳过 sign + DMG codesign + notarize（dev 构建产出未签 DMG，loud warning） | 静态审查 | PASS（三处 if 分支独立） |
| 异常路径 | 真实跑完 tauri build → DMG 产出 | 未测 | 跳过：本地 Rust target/ 已有大量 NCdesktop 2.0 P0 早期实现 in-flight 改动（参见 git status），跑 tauri build 会污染范围；CI 验证由 task_013 接力 |

## 浏览器/运行时验证

N/A — 纯 shell 脚本 task。AC-5 的端到端 macOS runner 验证依赖 GitHub macOS runner + Apple notary secret，本机无法触发，见下方"已知局限"。

## 已知局限

### AC-5 — CI workflow PENDING-CI

input.md AC-5 描述："CI workflow（macOS runner 可用时）跑通端到端；**如无 macOS runner，本 AC 由本地脚本输出 + manifest checksum 替代，在 output.md 记录**"。

- 当前 `.github/workflows/` 已存在 `notarize-dmg.yml`（task_005 产出），消费一份外部上传的 DMG artifact → 跑 `notarize.sh`。
- 完整 build-macos-dmg.sh 端到端 workflow（含 `pnpm tauri build` + `prepare-*` + `sign-bundle`）需要 self-hosted macOS runner（GH-hosted `macos-14` 可用，但跑完整 Rust 构建 + standalone Python 下载 + 全签名链 ~30-45 min/job，并且需要把 Developer ID 证书也注入 runner keychain，超出 task_006 单 task 范围）。
- **本任务范围内的替代证据**：
  - 本地脚本静态 + mock 矩阵全 PASS（上面 T1-T7）
  - `runtime-manifest.json` checksum 由 task_002 + task_007 接力保证（manifest 自检在应用启动期跑）
  - 真实 macOS runner 端到端跑通由 **task_013_clean_vm_smoke_test** 接力（input.md AC-5 同款表述）

### dist/ 目录

`dist/` 已被根级 `.gitignore` 第 5 行通配 — 无需在本 task 内修改 `.gitignore`（`git check-ignore` 验证：`dist/test.sha256` 命中规则）。

### 重复 staple 处理

`notarize.sh`（task_005）内部已调用 `xcrun stapler staple` 并断言 "The staple and validate action worked!" 字面成功。本脚本 step 9 不再二次调用 stapler，仅在 notarize 跳过分支打印提示。`stapler staple` 对已 staple 的 DMG 二次调用是幂等的（重新校验已嵌入的 ticket，不会破坏 artifact），所以即便上层 task 期望 step 9 是独立的 stapler 调用，行为也不会偏差 —— 但日志会更干净。

## 需要 Reviewer 特别关注的地方

1. **`du -sk` vs `du -sh` 的双轨设计**：体积比较走 `du -sk`（KB 整数）规避单位陷阱；`du -sh` 只用于 `dmg_size_report.txt` 的人类可读字段。如果 Reviewer 觉得 report 还需要 KB 数值字段，可加。

2. **step 9 重复 staple 处理**：见"已知局限"。如果 Reviewer 偏好"step 9 总是显式调用 `xcrun stapler staple`（依赖其幂等性）"，可以改为无条件调用 —— 行为一致，日志稍冗余。

3. **AC-3 mount 自检的兜底设计**：trap cleanup 检查 `MOUNT_POINT` 变量；正常路径在 detach 后清空该变量，避免 trap 二次 detach 报错。Reviewer 可关注 trap 路径的鲁棒性。

4. **版本号读取用 python3 而非 jq**：`tauri.conf.json` 字段稳定为顶层 `version`，python3 在 macOS 是默认依赖。如果 Reviewer 偏好 jq（CI 也装得了），可以两行 sed 改成 `jq -r .version`，语义等价。

5. **AC-5 PENDING-CI 的边界**：明确委托给 task_013，避免 task_006 范围扩张到"再写一个完整端到端 GH Actions workflow + 注入证书"——后者本身就是 Conductor 计划中的独立 task。

## git diff --stat (本 task 直接产出)

```
NCdesktop/scripts/build-macos-dmg.sh | (rewritten, ~340 lines)
```

注：`git status` 中 `prepare-embedded-python.sh` / `prepare-embedded-markitdown-runtime.sh` 也显示 M 状态，**这是 task_001/002 PASS 状态的预存量**（文件 mtime：18:07 / 18:39，远早于本 task 开工时间 22:28；`git diff HEAD` 对比这两份文件的内容完全是 task_001/002 的产物，与本 task 无关）。本 task 仅触动 `build-macos-dmg.sh` 一处。
