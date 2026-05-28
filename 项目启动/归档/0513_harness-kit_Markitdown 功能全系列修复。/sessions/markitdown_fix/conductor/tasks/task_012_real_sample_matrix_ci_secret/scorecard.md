# Review Scorecard — task_012_real_sample_matrix_ci_secret

## 审查思考过程

1. **Task 意图**：搭建 7 格式 × ≥5 真实样本端到端验收矩阵 —— 解密加密样本 → 调 markitdown → 共用断言库分类 → per-format & 整体通过率 → ≥95% 门禁。本 task 为"验收脚本 + CI workflow"层，零业务代码改动；真实样本入库由 task_000 PM 接力。
2. **AC 检查结果**：
   - AC-1 脚本结构：✅ `set -euo pipefail` + trap 清理 + 8 段流水线齐备
   - AC-2 ≥35 样本矩阵：✅ PENDING-OPERATOR 合理（脚本可遍历任意目录；38 总数分布在 `sample_distribution.md` 文档化；真实入库属 task_000 范围）
   - AC-3 ≥95% 门禁：✅ `PASS_THRESHOLD=95` + KPF/unauthorized 直接 exit 4，不静默
   - AC-4 CI workflow：✅ YAML 合法 + 注入 `MARKITDOWN_SAMPLES_KEY` + 上传 artifact + macOS-latest runner + PENDING 备注 `<ORG>/<SAMPLES_REPO_NAME>` 合理
   - AC-5 共用断言库：✅ `lib/sample-assertions.sh` 被本地脚本 `source` 引用，CI workflow 通过调本地脚本同源消费
   - AC-6 epub KPF：✅ 命名约定 + 强制 ESCALATE 编码 + self-test case 10 验证；真测 PENDING-SAMPLES 合理
3. **关键发现**：
   - 4 个关键断言全部满足（详见下方）
   - `known-fail-list.json` 解析逻辑有"comment 值被误认作样本路径"潜在缺陷（当前空 samples 数组下无危害；MINOR）
   - 红线零越界，无业务代码改动，无 task_000~011 PASS 产物改动

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 4.5 | AC-1/3/4/5 完整 PASS；2/6 真测 PENDING 但脚本逻辑全闭环；KPF 强制 ESCALATE 实现正确 |
| 安全性 | 25% | 4.5 | secret 错误路径明确；明文不入 log（仅 path/sha/lines/elapsed）；CI 末尾 scrub；明确禁止打印 markdown 内容；GH Actions 默认有 secret masking。known-fail-list 解析 grep 解 JSON 是脆弱点但当前样本为空 |
| 代码质量 | 15% | 4.5 | 函数化清晰、命名规范、注释充分；header 列了退出码语义 + 关键约束；macOS timeout fallback 透明披露 |
| 测试覆盖 | 15% | 4 | 断言库 10 case self-test 全通过；dry-run / missing-key / YAML / JSON 全部已测；真测 PENDING 已在 output.md 标注 |
| 架构一致性 | 10% | 5 | markitdown 调用方式与 `src-tauri/.../extractors/markitdown.rs` 一致；wall-clock=MARKITDOWN_TIMEOUT+10s 与 task_007 ETimeout90s 对齐；report.json 仅元数据字段，无外溢；ADR-009 完全对齐 |
| 可维护性 | 10% | 4.5 | `MARKITDOWN_RUN_CMD` hook 给 task_013 接 Tauri binary；`PASS_THRESHOLD` / `KNOWN_FAIL_LIST` env 可覆写；命名约定 SOP 化 |

**综合分**：0.25×4.5 + 0.25×4.5 + 0.15×4.5 + 0.15×4 + 0.10×5 + 0.10×4.5 = **4.475/5**

## 总体判断

- [x] **PASS**（接受 4 项 PENDING 合理性：AC-2/AC-4-CI 真跑/AC-6 真测 均为 task_000 PM + macOS runner 阻塞，非本 task 责任）

## 问题列表

### BLOCKER（必须修复）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **known-fail-list.json 解析脆弱**：`scripts/run-real-sample-matrix.sh` L119-123 用 `grep -oE '"[^"]+"'` 提取双引号字符串后只过滤了 `samples|comment|version` 这三个 key 名，但 `comment` 字段的**值**（一长串中文描述）也会被当作样本相对路径加入 `KNOWN_FAIL_REL` 数组。当前样本数组为空且 comment 内容不可能匹配真实相对路径，无功能危害；但属于"JSON 解析做对了类型/键、做错了值的来源"的潜在陷阱。建议：① 用 `python3 -c "import json,sys; [print(s) for s in json.load(open(sys.argv[1]))['samples']]"`（已有 python3 依赖）；② 或显式在 grep 后再过滤掉非 path-like 行（含空格/中文字符等）。
2. **bash -x 调试模式下会泄露 secret 值**：操作员手动 `bash -x scripts/run-real-sample-matrix.sh` 会在 trace 中打印 `+ [[ -z <KEY值> ]]`。CI workflow 未启用 `-x`，且 GH Actions 对 secrets 自动 mask，所以非"日志"威胁；但建议在 README 加一行"严禁 bash -x 跑本脚本（trace 会暴露 secret 比较时的值）"。
3. **macOS timeout fallback 极端 hang 时 wall-clock 可能略超 100s**：output.md 已披露；建议在 `.github/workflows/real-samples-matrix.yml` 的 macOS-latest step 显式 `brew install coreutils` 或使用 `gtimeout`，让生产 CI 永不走 fallback 路径。

## AC 一览（含 PENDING 合理性判定）

| AC | 状态 | PENDING 合理性 |
|---|---|---|
| AC-1 脚本结构 | PASS | — |
| AC-2 ≥35 样本矩阵 | PENDING-OPERATOR | 合理：脚本可遍历任意目录；阻塞点是 task_000 PM 入库，已文档化在 `sample_distribution.md` |
| AC-3 ≥95% 门禁 | PASS | — |
| AC-4 CI workflow | PASS（YAML）/ PENDING-CI（真跑） | 合理：YAML 完整，PENDING `<ORG>/<SAMPLES_REPO_NAME>` 占位 + macOS runner quota 属 PM 范围 |
| AC-5 共用断言库 | PASS | — |
| AC-6 epub KPF | PASS（脚本）/ PENDING-SAMPLES（真测） | 合理：命名约定 + 强制 ESCALATE 已编码 + self-test case 10 验证；真测样本待入库 |

## 红线全过

| 红线 | 通过 |
|---|---|
| 不改 Rust 业务代码 | YES |
| 不改 task_000~011 PASS 产物 | YES |
| 不把样本明文写入 build/log | YES |
| 不跳过 epub / scan-pdf | YES |
| 通过率门禁可被 silent 绕过 | YES（KPF 强制 exit 4 + unauthorized fail 强制 exit 4） |
| secret 写入日志 | YES（CI 路径不打印；末尾 scrub；GH Actions 自动 mask） |

## 4 关键断言核验

1. **secret 不入日志**：✅ CI workflow `Verify required secrets` step 仅打印 `Key length: ${#KEY}`；脚本主路径无任何 echo `$MARKITDOWN_SAMPLES_KEY`。注意：本地 `bash -x` debug 模式下会在 `[[ -z $KEY ]]` 比较时 trace 出值（已在 MINOR-2 标注），属于操作员手动诊断场景，CI 不触发。
2. **样本明文不写 build/log**：✅ 日志只输出 `fmt / status / sha:0:12 / lines / elapsed / failure_code / rel`，明确"绝不打印 markdown 内容主体"；markdown 输出落在 `$WORK_DIR/.out/`（mktemp 临时目录）trap EXIT 时一并 `rm -rf`。report.json 字段集合 = `sample/format/status/failure_code/markdown_lines/elapsed_ms/sha256/exit_code`，全是元数据。
3. **单样本 wall-clock 上限 = 100s**：✅ `WALL_CLOCK_BUDGET_SEC=$(( MARKITDOWN_TIMEOUT_SEC + 10 ))` = 100；`timeout 100s python -m markitdown` 包裹；macOS 无 timeout 时用 background+watchdog（best-effort，已披露）。
4. **不跳过 epub / scan-pdf**：✅ `find` 一律列出所有非 `.enc/.meta.json/README/.git` 文件，主循环对所有样本调用 `run_markitdown_one`；脚本无 `continue|skip` 分支基于格式跳过。`pdf-scan` 格式失败被识别为 `E_SCAN_PDF_UNSUPPORTED` 并落入 `known-fail`（而不是跳过，仍跑 markitdown 留痕）；`epub` 全跑，`_known_production_failure.epub` 必须 PASS 否则 ESCALATE。

## 4 关注点结论

1. **dry-run 模式**：✅ `DRY_RUN=1` 早返回（L70-83），仅跑断言库 self-test + 必需脚本探测，不调 markitdown，不要求 key/samples。本机已实测 `exit=0`。
2. **per-format 通过率统计**：✅ 用 bash associative array（`declare -A`）不依赖 jq/python，CI 解析友好。公式 `(pass + known-fail) * 100 / total`；output.md "关注点 3" 已主动暴露分子是否含 known-fail 的策略选择，留给 PM 决策。当前实现符合"产品已声明的 Out 类不计入分母"语义。
3. **known-fail-list.json 格式**：⚠️ 格式 OK（`{comment, version, samples[]}`），但解析逻辑脆弱（见 MINOR-1）。
4. **Tauri command 调用方式**：✅ 偏离 input.md "通过 Tauri command" 文字但已在 output.md "对 Architect 方案的遵守声明 / 偏离说明"主动声明 —— 选 `python -m markitdown` 与 `extractors/markitdown.rs` 一致是更稳健选择，避免 task_012 引入 Tauri test binary 构建复杂度；`MARKITDOWN_RUN_CMD` env hook 给 task_013（cleaned VM smoke）接 DMG 内嵌 python 入口。偏离合理。

## 是否触非授权区

**否**。本次新增文件全部在 input.md 预估影响范围内：
- `NCdesktop/scripts/run-real-sample-matrix.sh`（新）
- `NCdesktop/scripts/lib/sample-assertions.sh`（新）
- `NCdesktop/scripts/known-fail-list.json`（新）
- `NCdesktop/tests/real_samples/{README.md, sample_distribution.md}`（新）
- `NCdesktop/.github/workflows/real-samples-matrix.yml`（新）

业务代码（`src-tauri/src/**`）零改动；task_000~011 PASS 产物（含 `decrypt-samples.sh` / `failure_code.rs` / `scheduler.rs` / `audio_asr_iflytek.rs`）零改动。

## 给 Dev 的修复指引

无 BLOCKER / MAJOR；MINOR 3 项为非阻塞优化建议，可在 task_013 验收期一并处理或在真实样本入库后由 PM 触发改进。
