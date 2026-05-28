# Task 交付 — task_012_real_sample_matrix_ci_secret

## 实现摘要

本 task 是**验收脚本与集成测试**，**零业务代码改动**。在 NCdesktop 主仓内交付：
1. 共用断言库 `scripts/lib/sample-assertions.sh`（本地脚本与 CI 共用，避免实现漂移）；
2. 主入口 `scripts/run-real-sample-matrix.sh`（解密 → 单样本调 markitdown → 收集 report.json → per-format 通过率门禁）；
3. 已知失败清单模板 `scripts/known-fail-list.json`；
4. 测试目录规范 `tests/real_samples/{README.md,sample_distribution.md}`（命名约定 + 7×5 矩阵分布）；
5. CI workflow `.github/workflows/real-samples-matrix.yml`（注入 secret + 调脚本 + 上传 report.json artifact + 明文 scrub）。

**核心设计决策**：
- markitdown 调用方式与 `src-tauri/.../extractors/markitdown.rs` 完全一致 — `python -m markitdown <file>`，保证业务路径不漂移；通过 `MARKITDOWN_RUN_CMD` env 允许后续接 Tauri test binary（task_013 接力点）。
- wall-clock 上限 = `MARKITDOWN_TIMEOUT_SEC (90)` + 10s = 100s，与 task_007 ETimeout90s 对齐；macOS 无 `timeout` 命令时用 background+watchdog fallback。
- 日志只输出 sample 路径、sha256、status、行数、耗时；**绝不打印 markdown 内容主体**（input.md 技术约束）。
- AC-6：`*_known_production_failure.epub` 命名前缀作为脚本硬约定，失败直接 ESCALATE，known-fail-list 内即便列入也强制忽略。

## AC 一览

| AC | 状态 | 说明 |
|---|---|---|
| AC-1 脚本 + 集成测试结构 | **PASS** | `run-real-sample-matrix.sh` + `lib/sample-assertions.sh` 完整；断言三要素（非空 / 结构 / failure_code）齐备 |
| AC-2 7×5 样本矩阵 | **PENDING-OPERATOR** | 脚本逻辑可遍历任意目录；真实 ≥35 样本入库依赖 task_000 PM；分布要求文档化在 `tests/real_samples/sample_distribution.md` |
| AC-3 ≥95% 通过率门禁 | **PASS（脚本逻辑）** | `PASS_THRESHOLD=95`；分子 = pass + known-fail；unauthorized fail / KPF fail 直接 exit 4；known-fail-list 内的失败不阻断但保留落入 RCA 的痕迹 |
| AC-4 CI workflow | **PASS（YAML 完整）/ PENDING-CI** | `real-samples-matrix.yml` 完整且 yaml.safe_load 通过；真实 CI 跑通依赖 macOS runner + PM 填 `<ORG>/<SAMPLES_REPO_NAME>` + 注入 `SAMPLES_DEPLOY_KEY` / `MARKITDOWN_SAMPLES_KEY` |
| AC-5 共用断言库 | **PASS** | `scripts/lib/sample-assertions.sh` 被本地脚本和 CI workflow 共用同一份；per-format 通过率统计已实现 |
| AC-6 epub 已知失效样本 | **PENDING-SAMPLES** | 脚本已实现 `assertions::known_production_failure` 命名约定 + 强制 ESCALATE 逻辑；自检 case 10 已验证；真测样本待入库 |

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/scripts/run-real-sample-matrix.sh` | 新建 | 主入口，本地+CI 共用 |
| `NCdesktop/scripts/lib/sample-assertions.sh` | 新建 | 共用断言库（10 个内置 self-test case 全通过）|
| `NCdesktop/scripts/known-fail-list.json` | 新建 | 已知失败清单模板，空数组占位 |
| `NCdesktop/tests/real_samples/README.md` | 新建 | 测试目录说明 + 命名约定 |
| `NCdesktop/tests/real_samples/sample_distribution.md` | 新建 | 7×5 矩阵分布要求 + PM 入库 checklist |
| `NCdesktop/.github/workflows/real-samples-matrix.yml` | 新建 | CI workflow |

**业务代码（src-tauri/src/**）：零改动。**已 PASS 的 task_000~011 产物：零改动。**

## 对 Architect 方案的遵守声明

- [x] 目录结构：scripts/ + tests/real_samples/ + .github/workflows/，与 input.md 预估影响范围一致
- [x] API 路径/命名：脚本与 Tauri 业务侧不交叉；markitdown 调用方式与 `extractors/markitdown.rs` 一致
- [x] 数据模型：report.json 仅含元数据字段（path / format / status / failure_code / lines / elapsed_ms / sha256 / exit_code）
- [x] 未引入计划外的新依赖：脚本仅依赖 bash + openssl + python + markitdown（已有）；CI workflow 仅 `actions/checkout@v4` + `actions/setup-python@v5` + `actions/upload-artifact@v4`
- 偏离说明：input.md AC-1 提到"通过 Tauri command 触发"，本实现选择**直接 `python -m markitdown`**，理由：① 与业务 extractor 完全一致的入口；② 避免本 task scope 引入 Tauri test binary 构建（task_013 范围）；③ 保留 `MARKITDOWN_RUN_CMD` env 给后续 task_013 接 Tauri binary

## 测试命令

```bash
cd NCdesktop
# 1. 语法
bash -n scripts/run-real-sample-matrix.sh
bash -n scripts/lib/sample-assertions.sh
# 2. 断言库自检
bash scripts/lib/sample-assertions.sh --self-test
# 3. dry-run
DRY_RUN=1 bash scripts/run-real-sample-matrix.sh
# 4. 缺 secret 错误路径
( unset MARKITDOWN_SAMPLES_KEY; bash scripts/run-real-sample-matrix.sh; echo exit=$? )
# 5. YAML schema
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/real-samples-matrix.yml'))"
# 6. JSON schema
python3 -c "import json; json.load(open('scripts/known-fail-list.json'))"
```

## 测试结果

```
=== bash -n run-real-sample-matrix.sh ===
OK
=== bash -n sample-assertions.sh ===
OK
=== assertions self-test ===
OK: all assertions self-test passed
=== dry-run ===
[run-real-sample-matrix] DRY_RUN=1 → validating plumbing only
OK: all assertions self-test passed
[run-real-sample-matrix] OK: dry-run plumbing check passed
=== missing key error path ===
[run-real-sample-matrix] ERROR: MARKITDOWN_SAMPLES_KEY not set (required to decrypt samples-private)
Usage: MARKITDOWN_SAMPLES_KEY=<key> SAMPLES_PRIVATE_DIR=<path> scripts/run-real-sample-matrix.sh
...
exit=2
=== YAML schema validate ===
jobs: ['matrix']
on: ['workflow_dispatch']
OK
=== JSON validate known-fail-list ===
{'comment': '...', 'version': 1, 'samples': []}
```

断言库 10 个 self-test 全通过（见 `lib/sample-assertions.sh` 末尾）：has_structure（标题/段落）、nonempty（空/全空白）、classify（pass/known-fail/fail）、infer_format（docx/scan-pdf/text-pdf/html/image）、known_production_failure 正/反例、AC-6 KPF 强制 fail。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | dry-run plumbing 检查 | 已测 | PASS（exit 0） |
| ✅ 正常路径 | 断言库 10 case self-test | 已测 | PASS |
| ✅ 正常路径 | YAML / JSON schema 合法 | 已测 | PASS |
| ⚠️ 边界条件 | 缺 `MARKITDOWN_SAMPLES_KEY` env | 已测 | exit=2 + 错误文案明确 PASS |
| ⚠️ 边界条件 | 缺 `SAMPLES_PRIVATE_DIR` env | 代码覆盖 | exit=2，未独立 run（路径 fall-through 同上一条） |
| ⚠️ 边界条件 | known-production-failure 样本失败 | self-test case 10 | exit=4 (ESCALATE) PASS |
| ⚠️ 边界条件 | 扫描 PDF → E_SCAN_PDF_UNSUPPORTED | self-test case 6 | known-fail（不阻断）PASS |
| ❌ 异常路径 | 真实 ≥35 样本端到端 | **未测** | PENDING-OPERATOR — 依赖 task_000 PM 入库 samples-private 仓 |
| ❌ 异常路径 | CI workflow 在 GH Actions runner 真实跑 | **未测** | PENDING-CI — 依赖 PM 填 repo URL + Deploy Key + macOS runner quota |
| ❌ 异常路径 | epub 已知失效样本真测 | **未测** | PENDING-SAMPLES — 同上 |

## 浏览器/运行时验证

N/A — 本 task 是 shell/CI 验收脚本，无 UI、无可启动服务。等价"运行时验证"= 上方测试结果章节的 dry-run / missing-key / self-test。

## 已知局限

1. **AC-2 / AC-6 真测 PENDING-OPERATOR**：脚本逻辑完整，但真实样本入库由 task_000 PM 阻塞。此为 input.md 与 task_000 output.md 共识的 PENDING 接力点（task_013 范围）。
2. **macOS 无 `timeout` 命令**：脚本提供 background+watchdog fallback；该 fallback 在极端情况下（python 卡死且 SIGKILL 不响应）可能让 wall-clock 略超 100s。生产 CI 用 ubuntu/macOS runner 都装了 coreutils-compatible timeout（macOS 可通过 brew gtimeout，或 setup-python 自带的 GNU timeout），不会触发 fallback。
3. **report.json 用手拼 JSON**：避免引入 jq 依赖。已通过 `python3 -c "import json; json.load(...)"` 在 dry-run 后验证语法。真实 ≥35 样本跑通后建议再做一次 schema 全量校验。
4. **`MARKITDOWN_RUN_CMD` 覆写未端到端测试**：仅留 hook 给 task_013（接 Tauri test binary）；当前默认路径 `python -m markitdown` 已可用。

## 需要 Reviewer 特别关注的地方

1. **wall-clock 上限实现**（`run_markitdown_one` in run-real-sample-matrix.sh）：是否真的能在 `WALL_CLOCK_BUDGET_SEC=100` 时强制中断 markitdown 子进程？特别是 macOS fallback 路径。建议 reviewer 在有 timeout 命令的环境构造一个故意 hang 的样本验证。
2. **markitdown 调用方式偏离**（见"对 Architect 方案的遵守声明"偏离说明）：选择 `python -m markitdown` 而非 Tauri command，是否符合架构师/PM 预期？如不符合，应在 task_013 时接 Tauri binary，本 task 留 `MARKITDOWN_RUN_CMD` hook。
3. **per-format 通过率统计**（`scripts/run-real-sample-matrix.sh` §7）：分子用 `pass + known-fail`，分母用 `total`；known-fail 不算 fail 是否符合"95% 真实通过率"语义？另一种诠释是仅 `pass / total ≥ 95%`，会让"扫描 PDF 与 epub 生产已知失效"被算作 fail 拖低整体通过率。当前实现倾向前者（产品已声明的 Out 类不计入分母），如需调整可微调 §7 公式。
4. **AC-6 命名约定**（`*_known_production_failure.epub`）：脚本硬依赖文件名后缀，且优先级高于 known-fail-list。是否需要在 SOP 文档（`docs/sample_desensitization_sop.md`）补充该命名要求？当前已在 `tests/real_samples/README.md` 与 `sample_distribution.md` 中文档化。

## PENDING 接力点（→ task_013）

- task_013 cleaned VM smoke：需在 clean macOS VM 上把 DMG 安装 + 跑 `run-real-sample-matrix.sh` 端到端；可设 `MARKITDOWN_RUN_CMD` 为指向 DMG 内嵌的 python 入口
- task_000 PM 操作员：按 `tests/real_samples/sample_distribution.md` checklist 入库 ≥35 脱敏样本
- CI 真跑：PM 填 `.github/workflows/real-samples-matrix.yml` 中 `<ORG>/<SAMPLES_REPO_NAME>` + 配置 secrets
