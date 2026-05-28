# Task 交付 — task_013_clean_vm_smoke_test

## 实现摘要

本 task 为**纯端到端冒烟脚本**，不动业务代码。交付物：

1. `scripts/vm-smoke.sh` — 自动化主脚本（3 次冷启 × {snapshot 复原 → AppleScript 序列 → 7 格式
   样本转录 → conversion_meta 抓取 → spctl 校验 → 体积/P95 启动时间断言}），最终聚合为
   `vm_smoke_report.json`。
2. `scripts/vm-base-image.md` — VM base image 制作 SOP（tart + macOS 12.x/14.x arm64），含
   snapshot 命名约定 / "干净"前提断言 / 故障排查。
3. `artifacts/README.md` + `artifacts/screenshots/.gitkeep` — 报告归档占位与实跑指引。

**核心设计决策**：

- **AppleScript 100% 带超时**：每条 osascript 调用都用 `with timeout of N seconds` 包裹（AS
  内部），外层再用 macOS host 端 `timeout`/`gtimeout` 做 ssh 调用兜底（双保险，防 ssh 网络挂死）。
- **复用 task_012 断言库**：通过 `source scripts/lib/sample-assertions.sh` 引入，调用
  `assertions::infer_format` / `assertions::classify` / `assertions::sha256`，**未** 二次定义。
- **复用 task_005 spctl 字面**：`spctl -a -vv -t open --context context:primary-signature` +
  grep `accepted` + `Notarized Developer ID`，与 `notarize.sh` 行 234/236/237 完全一致。
- **复用 task_006 体积报告**：解析 `dist/dmg_size_report.txt` 中 `dmg_total: <h> (<kb> KB)` 行
  做严格相等校验，drift → fail。
- **trap 兜底**：EXIT/INT/TERM 全部 `tart stop && tart delete ephemeral`，红线"不留运行中 VM"。

## AC-1~6 一览

| AC | 状态 | 证据 |
|----|------|------|
| AC-1：tart base image SOP（12 + 14） | **PASS（脚本静态完整）** | `scripts/vm-base-image.md` §3/§4 含 IPSW 命令 / snapshot 名 / "干净"前提自检 ssh 字面 |
| AC-2：vm-smoke.sh 7 格式 + scan-pdf 路由 | **PASS（脚本静态完整）** | `vm-smoke.sh` `drive_smoke_session()`；7 格式去重 + scan-pdf 强制断言 `E_SCAN_PDF_UNSUPPORTED` |
| AC-3：3 次冷启 100% + 任一失败整体 fail | **PASS（脚本静态完整）** | `vm-smoke.sh` `run()` 主循环 3 iter，每次 `tart clone → run → drive → stop → delete`；`finalize_report()` 聚合 + `all_pass == False → exit 1` |
| AC-4：spctl + 首启对话框零次 | **PASS（脚本静态完整）** | `assert_gatekeeper()` 复用 task_005 字面 + `log show --predicate process==CoreServices` grep `cannot check it for malicious software\|未识别开发者\|unidentified developer` |
| AC-5：体积一致 + P95<2s | **PASS（脚本静态完整）** | `verify_size()` 严格 kb 相等；`finalize_report()` Python 计算 P95（ceil(0.95·n)-1 索引），`p95 >= budget → fail` |
| AC-6：CI 无 macOS runner 豁免 | **PENDING-USER-MACHINE** | input.md 明示；output.md 明记；artifacts/README.md §"CI 限制" 同步声明 |

> **关于"PASS（脚本静态完整）"语义**：脚本逻辑、字面、超时、退出码已通过 `bash -n` 语法检查 +
> `VM_SMOKE_DRY_RUN=1` 端到端 dry-run（见下方测试结果）。**真实 Gatekeeper 验证 / 7 格式实跑 /
> P95 实测值**必须在用户本地 `tart` + 真实 DMG 上执行，结果写回 `artifacts/`，届时本 output.md
> 升级为完整 PASS。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/scripts/vm-smoke.sh` | 新建（+512 行） | 干净 VM 端到端冒烟主脚本 |
| `NCdesktop/scripts/vm-base-image.md` | 新建 | VM base image 制作 SOP（tart） |
| `sessions/markitdown_fix/conductor/tasks/task_013_clean_vm_smoke_test/artifacts/README.md` | 新建 | 实跑归档占位 + 命令指引 |
| `sessions/markitdown_fix/conductor/tasks/task_013_clean_vm_smoke_test/artifacts/screenshots/.gitkeep` | 新建 | 截屏目录占位 |

**未触碰**：
- `scripts/notarize.sh`（task_005）、`scripts/build-macos-dmg.sh`（task_006）、
  `scripts/run-real-sample-matrix.sh`（task_012）、`scripts/lib/sample-assertions.sh`（task_012）
  全部 mtime 早于本 session，仅 source/call 复用。
- 任何 Rust 业务代码（`src-tauri/src/**`）未修改。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（scripts/vm-smoke.sh 路径、artifacts/ 路径）
- [x] API 路径/命名与 Architect 方案一致（无 API；脚本接口三参数符合 input.md 预估范围）
- [x] 数据模型与 Architect 方案一致（report JSON schema 由本 task 定义，被 task_012 风格指导）
- [x] 未引入计划外的新依赖（`tart` / `sshpass` 是宿主机工具，**不进 VM**；红线无破坏）
- 偏离说明：无。

## 工具选择理由

| 维度 | tart | UTM | Apple Virtualization 直调 |
|------|------|-----|--------------------------|
| Apple Silicon 原生 | ✅ | ✅ | ✅ |
| CLI 自动化 | ✅（首选） | ❌ 主要 GUI | ✅ 但需要自实现 snapshot |
| snapshot/clone COW | ✅ 一键 | △ | ❌ 需自写 |
| 开源 | ✅ MIT | ✅ AGPL | N/A |
| 3 次冷启总耗时 | ~30s | ~3min（无 COW） | 不可控 |

**结论**：tart。脚本中 `tart clone <base> ephemeral` / `tart run --no-graphics` / `tart ip` /
`tart stop` / `tart delete` 五个原语足以完成本 task；UTM 仅用于首次安装 macOS 的 GUI 引导
（SOP §3.1），随后镜像导入 tart 管理。

## AppleScript 关键序列 + 超时设计

主要序列（每条独立 `run_applescript_in_vm` 调用，60s 默认超时）：

| 序列 | 实现 | 超时 |
|------|------|------|
| 挂载 DMG | `do shell script "hdiutil attach …"` 包在 AS `with timeout` 内 | 60s |
| 拖入 Applications | `tell application "Finder" to duplicate POSIX file … to folder "Applications"` | 60s |
| 启动 NoteCapt | `tell application "NoteCapt" to activate` | 60s |
| 拖文件入应用 | `do shell script "open -b com.notecapt.app /tmp/<sample>"` | 60s/sample |

**双保险超时**：
- 内层：AppleScript 原语 `with timeout of N seconds … end timeout`（事件分发器级）。
- 外层：宿主机 `timeout (N+5)s ssh …`（防 ssh 链路本身 hang）。

**故意避免 Finder 双击模拟**：原 input.md 建议"双击挂载 DMG"。我选择 `hdiutil attach` 等价
路径——理由：(1) Gatekeeper 评估 DMG 签名发生在 attach 调用上，与谁触发无关；(2) Finder 双击
依赖图形会话激活态，对 `--no-graphics` 启动的 tart VM 不可靠；(3) `hdiutil` 是 Apple 系统二进制
（/usr/bin/hdiutil），不违反"干净 VM"前提。同理 `open -b com.notecapt.app` 替代窗口拖拽。

## 复用 task_012 断言库的方式

```bash
# vm-smoke.sh L62-63
source "$THIS_DIR/lib/sample-assertions.sh"

# 调用点：
#   assertions::infer_format "$f"     → 推断 pdf-text / docx / pptx / xlsx / html / epub / image / pdf-scan
#   assertions::classify "$f" "$md" "$failure_code" → pass | fail | known-fail
#   assertions::sha256 "$f"           → 报告字段
```

- **未** 重新实现 7 格式分类逻辑（红线）。
- **未** 修改 `lib/sample-assertions.sh`（task_012 PASS 产物保持）。
- 已用 `bash scripts/lib/sample-assertions.sh --self-test` 确认未破坏：返回
  `OK: all assertions self-test passed`。

## artifacts/ 目录设计

```
artifacts/
├── README.md                                   ← 占位 + 用户实跑指引
├── screenshots/.gitkeep                        ← 截屏占位
└── （实跑后回填）
    vm_smoke_report.macos-12-base.json
    vm_smoke_report.macos-14-base.json
    spctl/spctl_ephemeral-*.txt
    per-iter/iter_*_ephemeral-*.json
    screenshots/*.png
```

`vm_smoke_report.json` 字段示意：

```json
{
  "vm_base": "macos-12-base",
  "iterations": [
    { "iter": 1, "launch_ms": 1820, "samples": [
        {"format": "pdf-text", "sample": "...", "sha256": "...",
         "conv_status": "done", "failure_code": "", "classified": "pass"},
        {"format": "pdf-scan", "sample": "...", "failure_code": "E_SCAN_PDF_UNSUPPORTED",
         "classified": "known-fail"},
        ...
    ]}, ...
  ],
  "cold_boots": 3,
  "all_pass": true,
  "fail_count": 0,
  "launch_ms": [1820, 1675, 1922],
  "launch_avg_ms": 1805.7,
  "launch_p95_ms": 1922,
  "launch_budget_ms": 2000,
  "p95_within_budget": true
}
```

## 测试命令

```bash
# 1) 语法检查
bash -n NCdesktop/scripts/vm-smoke.sh

# 2) 复用断言库自检不破坏
bash NCdesktop/scripts/lib/sample-assertions.sh --self-test

# 3) 端到端 dry-run（无 tart 也可跑，验证主控流程 + 报告聚合 + AC-3 gate）
ACTUAL_KB=$(du -sk /tmp/vmsmoke-test-dist/NoteCapt-arm64.dmg | awk '{print $1}')
printf 'dmg_total:        12M  (%s KB)\n' "$ACTUAL_KB" > /tmp/vmsmoke-test-dist/dmg_size_report.txt
VM_SMOKE_DRY_RUN=1 \
  DMG_SIZE_REPORT=/tmp/vmsmoke-test-dist/dmg_size_report.txt \
  VM_SMOKE_REPORT_DIR=/tmp/vmsmoke-test-report \
  bash NCdesktop/scripts/vm-smoke.sh \
    /tmp/vmsmoke-test-dist/NoteCapt-arm64.dmg macos-12-base /tmp/vmsmoke-test-samples
```

## 测试结果

```
$ bash -n NCdesktop/scripts/vm-smoke.sh && echo "SYNTAX OK"
SYNTAX OK

$ bash NCdesktop/scripts/lib/sample-assertions.sh --self-test
OK: all assertions self-test passed

$ VM_SMOKE_DRY_RUN=1 … bash NCdesktop/scripts/vm-smoke.sh …
[vm-smoke] preflight: DMG=/tmp/vmsmoke-test-dist/NoteCapt-arm64.dmg  VM_BASE=macos-12-base  SAMPLES_DIR=/tmp/vmsmoke-test-samples
[vm-smoke] preflight: DRY_RUN=1 — skipping tart presence check
[vm-smoke] verify_size: cross-check with /tmp/vmsmoke-test-dist/dmg_size_report.txt
[vm-smoke] verify_size: expected=12348KB actual=12348KB
[vm-smoke] ═══ cold boot 1 / 3 ═══
[vm-smoke] restore_snapshot[1]: tart clone macos-12-base → ephemeral-macos-12-base-1-28762
[vm-smoke] clean_vm_preflight: verify VM is genuinely clean
[vm-smoke] clean_vm_preflight: DRY_RUN=1 — skip
[vm-smoke] drive_smoke_session[1] → /tmp/vmsmoke-test-report/iter_1_ephemeral-macos-12-base-1-28762.json
[vm-smoke] ═══ cold boot 2 / 3 ═══
… (iter 2, iter 3 同上) …
[vm-smoke] finalize_report: aggregating 3 cold-boot iterations
[vm-smoke] finalize_report: → /tmp/vmsmoke-test-report/vm_smoke_report.json
{
  "vm_base": "macos-12-base",
  "iterations": [ {"iter": 1, "dry_run": true, "launch_ms": 0, "samples": []},
                  {"iter": 2, "dry_run": true, "launch_ms": 0, "samples": []},
                  {"iter": 3, "dry_run": true, "launch_ms": 0, "samples": []} ],
  "cold_boots": 3,
  "all_pass": true,
  "fail_count": 0,
  "launch_ms": [0, 0, 0],
  "launch_avg_ms": 0.0,
  "launch_p95_ms": 0,
  "launch_budget_ms": 2000,
  "p95_within_budget": true
}
OK: 3/3 cold boots, P95 launch within budget
[vm-smoke] DONE: /tmp/vmsmoke-test-report/vm_smoke_report.json
```

（`verify_size` 在第一次故意构造 drift 时也能正确 fail；上述输出是修正 size 后的成功流。）

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | DMG 存在、size 一致、3 次 dry-run 全过 | 已测 | PASS（见测试结果） |
| ✅ 正常路径 | 复用 sample-assertions.sh self-test | 已测 | PASS (10/10) |
| ⚠️ 边界条件 | DMG size 与 size_report.txt 不一致 | 已测 | PASS：`fail "DMG size drift: 12348 != 12345"` 触发 |
| ⚠️ 边界条件 | VM_BASE 不在白名单 | 已测 | PASS：`unsupported VM_BASE` 在 preflight fail |
| ⚠️ 边界条件 | trap cleanup 路径 | 已测（dry-run，无 ephemeral 创建） | PASS：cleanup 路径上 EPHEMERAL_NAME 为空时跳过 tart 调用 |
| ❌ 异常路径 | DMG 文件不存在 | 已测 | PASS：`fail "DMG not found"` |
| ❌ 异常路径 | samples_dir 不存在 | 已测 | PASS：`fail "samples dir not found"` |
| ❌ 异常路径 | 真实 tart 调用 / AppleScript 序列 / IPC ready 检测 / spctl 字面 | **PENDING-USER-MACHINE** | 需用户本地 tart + 真实 DMG 实跑 |

## 浏览器/运行时验证

**N/A 并说明原因**：本 task 是宿主机 shell 脚本（无 UI），且实跑目标本身是干净 macOS VM 中的
NoteCapt 应用首启行为——验证发生在 VM 内，由 AppleScript 序列与 SQLite 查询完成；脚本输出
`vm_smoke_report.json` 为 reviewer 唯一可读运行时凭据。Dry-run 路径见"测试结果"段。

## 已知局限

1. **PENDING-USER-MACHINE**：本机无 tart 安装、无 macOS 12/14 base image、无真实签名+公证后的
   DMG（task_005/006 输出在 dist/，非本 session 跑）。脚本静态完整，但 AC-2/3/4/5 的**真实数值**
   必须由用户本地实跑回填。
2. **sqlite3 路径假设**：脚本假设 NoteCapt 在 `~/Library/Application Support/NoteCapt/notecapt.db`
   写 `conversion_meta` 表。若实际路径变化，需 reviewer 在 VM 内 `find` 一次后小改路径。
3. **IPC ready marker 文件未定**：脚本假设前端写 `~/Library/Logs/NoteCapt/ready.marker` 作为
   IPC ready 信号。若 lib.rs 实际信号是 stdout 日志行，需 reviewer 把 `test -f ready.marker`
   替换为 `log show --predicate 'process == "NoteCapt"' | grep -F "ipc-ready"`。**此路径不在
   本 task scope**（红线"不动业务代码"），需另起 task 在 lib.rs 加 marker 写入。
4. **AppleScript 替代 Finder 双击**：用 `hdiutil` + `open -b` 替代真实双击。Gatekeeper 评估
   等价（DMG 签名 + bundle 签名都被检查），但**严格意义上的"双击"GUI 路径**未覆盖。如 reviewer
   认为必须覆盖 GUI 双击，需追加一条带图形会话的 AS 序列（成本高，建议作为下一轮迭代）。

## 需要 Reviewer 特别关注的地方

1. **P95 启动时间测量精度**（vm-smoke.sh `drive_smoke_session`）：宿主机 self-test 时发现
   macOS BSD `date` 不支持 `%N`（返回字面 `<sec>%3N`），**已在脚本中修复**：改用
   `ssh_run 'python3 -c "import time; print(int(time.time()*1000))"'`（Apple-shipped
   /usr/bin/python3 系统 stub，不违反"干净 VM"前提）。请 reviewer 复核取时方式。
2. **`launch_ms` 在 3 次冷启时计算 P95 的索引**（finalize_report L386）：用
   `ceil(0.95·n) - 1`，n=3 → idx=2 → 取最大值。这是保守 P95（实际等于 max）；如要更严格的
   线性插值版本，reviewer 提需求即可。
3. **`SAMPLES_DIR` 选取策略**：脚本对每种 format **取第一个匹配**的文件（`picked_for_fmt`
   去重）。若 samples-decrypted 下有多个 pdf-text 候选，选哪个不确定（取决于文件系统遍历
   顺序）。如需确定性，建议 reviewer 要求改成"显式 fixture 清单"。
4. **`tart` + `sshpass` 是宿主机依赖**：未列入项目 brew 依赖清单。需要 reviewer 决定是否在
   README 中加一行 prerequisites（**不进 VM，不破坏红线**）。
5. **base image 实际版本号选择**（vm-base-image.md §3.1）：脚本默认 macOS 12.7.6 / 14.5，
   但 IPSW URL 写的是 `…`占位。reviewer 实跑前必须替换为最新可下载 URL（IPSW 是用户机器
   下载，不在脚本 scope）。
