# Review Scorecard — task_013_clean_vm_smoke_test

## 审查思考过程

1. **Task 意图**：在干净 macOS 12/14 arm64 VM 上对最终签名+公证 DMG 做端到端冒烟（Gatekeeper / 7 格式样本 / 3 次冷启 100% / P95 < 2s），交付 `scripts/vm-smoke.sh`（主控）+ `scripts/vm-base-image.md`（VM 制作 SOP）+ artifacts/ 占位。
2. **AC 检查结果**：见下表，6/6 PASS（AC-6 合法 PENDING-USER-MACHINE）。
3. **关键发现**：脚本静态完整、字面与 task_005/006/012 严格一致、双保险超时、复用断言库未二次定义。AC-1~5 真实数值依赖用户本地 tart + 真实 DMG 实跑回填，符合 input.md AC-6 carve-out 语义。

## AC 一览

| AC | 状态 | 证据 |
|----|------|------|
| AC-1 tart base image SOP（12+14）| PASS | `vm-base-image.md` §3/§4 含 IPSW + snapshot 名 + §3.5 严禁清单 + §3.5 宿主自检 ssh 字面；vm-smoke.sh L191-201 `clean_vm_preflight` 复用同字面 |
| AC-2 vm-smoke.sh 7 格式 + scan-pdf | PASS | `drive_smoke_session` L412 `needed_fmts="pdf-text docx pptx xlsx html epub image pdf-scan"`；L470 强制 `failure_code != E_SCAN_PDF_UNSUPPORTED → fail` |
| AC-3 3 次冷启 100% + 任一 fail 整体 fail | PASS | `run()` L586 `for i in 1 2 3`；每 iter 完整 `restore_snapshot → drive → stop → delete`；`finalize_report` L550 `all_pass = all_pass and len(iters)==3`；L100 `rm -f iter_*.json` 防 stale 串扰 |
| AC-4 Gatekeeper（spctl + 首启对话框零次）| PASS | L294 `spctl -a -vv -t open --context context:primary-signature`（与 notarize.sh 字面一致）；L297-300 `grep accepted` + `Notarized Developer ID`；L305 三语 grep `cannot check it for malicious software\|未识别开发者\|unidentified developer` |
| AC-5 体积一致 + P95 < 2s | PASS | `verify_size` L321 解析 `(<kb> KB)` 严格 kb 相等；L527 P95 索引 `ceil(0.95·n)-1`；L572 `p95 < budget` 双 gate |
| AC-6 CI 无 macOS runner 豁免 | PASS（PENDING-USER-MACHINE 合理）| input.md AC-6 明示；output.md L26-41 标注；artifacts/README.md §CI 限制同步声明 |

## 红线 6 项

| 红线 | 状态 |
|------|------|
| 修改 Rust 业务代码 | PASS（vm-smoke.sh 是宿主 shell；零 Rust 改动）|
| 修改 task_005/006/012 scripts | PASS（mtime 检查：vm-smoke.sh 23:24 晚于 notarize.sh / build-macos-dmg.sh / sample-assertions.sh / run-real-sample-matrix.sh；仅 `source` 复用） |
| 修改 task_000~011 PASS 产物 | PASS（artifacts/ 全新建，scripts/ 仅新增 2 文件） |
| VM 内预装依赖 | PASS（grep `brew install\|pip install\|curl.*install\|wget.*install` 在脚本可执行段 0 命中，仅注释提及红线本身）|
| AppleScript 无超时 | PASS（双保险：L249 `with timeout of ${secs} seconds … end timeout` 内层 + L262/L272 `$timeout_bin $outer ssh …` 外层 secs+5） |
| 静默吃错 | PASS（`set -euo pipefail` + `fail()` 立即 exit；每 iter 落 `iter_*.json`；`assert_gatekeeper` 失败 → `fail`；`finalize_report` AC-3 gate 非零退出）|

## 4 关键关注点结论

1. **AppleScript 双保险超时**：YES，内层 AS `with timeout of N seconds … end timeout`（L249-251）+ 外层 `$timeout_bin $outer ssh`（L272-277），且无 `timeout` bin 时降级到内层并 log 警告（L265-269）—— 容灾完备。
2. **AC-5 P95 计算（n=3 弱意义）**：dev 在 L526-527 显式注释"P95 of 3 samples is just the max — 但 documents the computation explicitly so reviewers can see we didn't fudge"。诚实但 n=3 统计意义弱；output.md §"需要 Reviewer 特别关注的地方"#2 主动提需求路径（线性插值），可作为 follow-up。**接受当前实现**——AC-5 budget 是"P95 < 2s"，max=保守上界，等价于"3 次中最坏 < 2s"，比插值更严格，不违背 PRD §4.1。
3. **复用 task_012 断言库方式**：**source**（L63 `source "$THIS_DIR/lib/sample-assertions.sh"`）；仅调用 `assertions::infer_format` / `assertions::classify` / `assertions::sha256`，未重定义；self-test 通过（`OK: all assertions self-test passed`）；sample-assertions.sh mtime 早于 vm-smoke.sh，未被改动。
4. **artifacts/ 目录设计**：README.md 清晰列出 `vm_smoke_report.macos-{12,14}-base.json` / `spctl/` / `per-iter/` / `screenshots/` 四类，附实跑命令 + CI 限制声明。结构便于 PM/QA 检索。

## macOS BSD date `%N` bug fix

**YES**：L378-381 注释明确说明 BSD date 不支持 `%N`，改用 `ssh_run 'python3 -c "import time; print(int(time.time()*1000))"'`；L397 取 `t_now` 同方式；用 `/usr/bin/python3`（Apple CLT stub）不违反"干净 VM"前提。grep 命中 2 处，dev 自报属实。

## 触非授权区

**否**。仅新建 `scripts/vm-smoke.sh`、`scripts/vm-base-image.md`、`artifacts/README.md`、`artifacts/screenshots/.gitkeep`。task_005/006/012 脚本 mtime 早于 vm-smoke.sh，未被覆写。零 Rust 改动。

## 评分

| 维度 | 权重 | 分数 | 说明 |
|------|------|-----|------|
| 功能正确性 | 25% | 4.5 | AC-1~5 静态完整 + dry-run 端到端通过；真实数值依赖用户机器（已 input.md 授权 PENDING）|
| 安全性 | 25% | 4.5 | `set -euo pipefail` + `fail()` + trap cleanup；sshpass 密码字面属性可接受（host-only NAT，干净 VM 一次性）|
| 代码质量 | 15% | 5 | 标头注释 53 行说明 why/inputs/AC mapping/red lines；函数命名一致；DRY 复用 task_012 断言库 |
| 测试覆盖 | 15% | 4 | bash -n + assertions self-test + dry-run 三场景已测；真实流程依赖用户机器，但 dev 已自报矩阵 |
| 架构一致性 | 10% | 5 | spctl 字面 100% 复用 task_005；体积报告字段复用 task_006；分类逻辑复用 task_012；ADR-005 字面 1:1 |
| 可维护性 | 10% | 5 | output.md §"需要 Reviewer 特别关注"列 5 项 follow-up；vm-base-image.md SOP 完备；故障排查表清晰 |

**综合分**：(4.5·0.25 + 4.5·0.25 + 5·0.15 + 4·0.15 + 5·0.10 + 5·0.10) = 1.125 + 1.125 + 0.75 + 0.60 + 0.50 + 0.50 = **4.60 / 5**

## 总体判断

- [x] **PASS**（接受 AC-6 PENDING-USER-MACHINE 合理；脚本静态完整、字面正确、未触红线）

## 问题列表

### BLOCKER

无。

### MAJOR

无。

### MINOR（可选改进，不阻塞 PASS）

1. **n=3 的 P95 统计意义弱**：当前取 max（保守上界），dev 已在脚本与 output.md 主动提示。若后续追求线性插值版（如 7-th percentile of 3 samples 用 (n-1)·p 索引），可作下一轮迭代。
2. **`SAMPLES_DIR` 取第一个匹配的非确定性**：output.md §"需要 Reviewer 特别关注"#3 提及；建议后续提供"显式 fixture 清单"。
3. **`tart` / `sshpass` 宿主依赖未列入项目 brew 依赖清单**：建议在 README prerequisites 段补一行（宿主侧，不进 VM，不破坏红线）。
4. **IPC ready marker（`~/Library/Logs/NoteCapt/ready.marker`）依赖前端写入**：output.md §已知局限#3 已说明此假设，需另起 task 在 `lib.rs` 加 marker 写入。当前实现等待路径合理（poll 100ms × deadline_ms*3）。

## 给 Dev 的修复指引

无（PASS）。

## 落盘状态

scorecard 写入：`sessions/markitdown_fix/conductor/tasks/task_013_clean_vm_smoke_test/scorecard.md` — **YES**。
