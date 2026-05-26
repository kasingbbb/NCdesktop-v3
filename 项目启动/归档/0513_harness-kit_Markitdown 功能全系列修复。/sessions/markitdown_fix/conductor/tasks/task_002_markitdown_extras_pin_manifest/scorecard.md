# Review Scorecard — task_002_markitdown_extras_pin_manifest (T-B)

> Reviewer：fresh instance（无先前上下文）
> 评审日期：2026-05-13
> 评审对象：output.md + 实际产物文件（含 E-1/E-2 修订）

---

## 审查思考过程

### 1. Task 意图复述
改造 `prepare-embedded-markitdown-runtime.sh`，把 markitdown 0.1.5 + extras（含 ebooklib、beautifulsoup4、mammoth）按 lock 安装到嵌入 PBS Python 的 `site-packages`，并生成符合 ADR-010 schema_version=1 的 `runtime-manifest.json`，配 `verify-manifest-consistency.sh` 做 CI 校验。

### 2. 交接契约 §3 接收方检查
- [x] 测试结果存在且非空（首装/幂等/--force/异常路径四段输出均粘贴）
- [x] 自测验证矩阵存在且正常路径全部 PASS
- [x] 架构遵守声明已填写（ADR-002 / ADR-010 双勾，含偏离说明）
- [x] 浏览器/运行时验证段已填（N/A + 说明：纯 shell + 构建期产物）

→ 进入实质性审查。

### 3. AC 逐条核验

| AC | 验收要点 | 实证 | 结果 |
|----|----------|------|------|
| AC-1 | lock 顶层 3 条 pin（markitdown[extras]==0.1.5 / bs4==4.12.3 / ebooklib==0.18）+ 传递依赖补齐 | requirements.lock 第 34–36 行字面命中；mammoth/pdfminer.six/python-pptx/openpyxl/pillow/lxml/charset-normalizer/ebooklib/beautifulsoup4 grep 全 OK；--no-deps 安装路径 | PASS |
| AC-2 | 7 imports 全过 | IMPORT_PROBES 数组 = `(ebooklib bs4 pdfminer pptx mammoth openpyxl PIL)`；prepare 脚本 line 131–137 实跑；output.md 输出 "all 7 imports OK"；修订记录回归实跑成功 | PASS |
| AC-3 | manifest 8 字段全 + imports 精确 7 项（修订后顺序） | runtime-manifest.json 字段全集已检；imports = `["ebooklib","bs4","pdfminer","pptx","mammoth","openpyxl","PIL"]` 字面与顺序均与 input.md AC-3 / ADR-010 补强段一致 | PASS |
| AC-4 | lock 与 manifest 字段同源 + verify 脚本兜底 | prepare 脚本顶部 readonly 常量 = manifest 单一事实源；verify-manifest-consistency.sh 用 grep 提取 lock 顶层 3 条版本，PYEOF 内逐字段对照 manifest，含 imports/extras/schema_version/runtime_id/build_timestamp/arch/python.{source,version,build} 全检查 | PASS |
| AC-5 | 幂等 + `set -euo pipefail` + `--force` 重装 | prepare 脚本 line 24 `set -euo pipefail`；line 78–94 manifest 版本比对实现 skip；line 97–111 --force 走 pip uninstall 安全清理（不递归 rm 防误删 pip/setuptools） | PASS |
| AC-6 | site-packages < 300MB（E-1 修订阈值） | du -sh 报告 289M < 300M | PASS |

### 4. 红线检查（用户特别点名）

| 红线 | 检查方法 | 结果 |
|------|----------|------|
| **python-docx 残留** | `grep -E "python-docx\|python_docx" scripts/requirements.lock` | **清除** — 仅在注释中提到"已从本 lock 移除"。lock 第 33–72 行的所有 pin 中无 `python-docx==` |
| **imports 列表中 `docx` 字面残留** | grep manifest/prepare/verify 三处 | **清除** — manifest.imports / IMPORT_PROBES / verify 的 expected_imports 均为 `mammoth`，无 `docx`。仅 `markitdown[pdf,docx,pptx,xlsx]` extras 标识符内含 `docx`（合法 pip extras 名，非 import 探针） |
| **越权写 task_007/011 内容** | 通读三脚本 + manifest | **未越权** — 脚本无任何启动期 self-check 调用逻辑（task_007 范畴）、无 route guard（task_011 范畴）、无 asr 相关。manifest 仅产出数据；消费由 task_007 负责，已在 output.md "需要 Reviewer 特别关注的地方"中明示边界 |
| **imports ≠ 7 项** | manifest.imports 数组长度 + verify 脚本 expected_imports | **= 7** — 严格 7 项，顺序匹配 input.md AC-3 修订字面 |

### 5. 修订记录核查（output.md 末尾 M-1 ~ M-4）
- **M-1** python-docx 删除：lock 中确认无 `python-docx==`；lxml==6.1.0 / typing_extensions==4.15.0 保留（被 ebooklib/python-pptx 共用，说明合理）
- **M-2** manifest imports `docx → mammoth`：runtime-manifest.json line 17 字面一致
- **M-3** prepare 脚本 IMPORT_PROBES：line 43 字面 `mammoth`，无 `docx`
- **M-4** verify expected_imports：line 88 字面 `mammoth`，无 `docx`
- 顶层 3 条 pin（markitdown==0.1.5 / bs4==4.12.3 / ebooklib==0.18）lock 第 34–36 行原样保留

四处同步无遗漏，修订记录与实际产物字节一致。

### 6. 关键发现
1. **AC-4 单一事实源实现到位**：prepare 脚本顶部 readonly 常量同时驱动 lock 顶层（人工同步 + verify 兜底）与 manifest 生成，verify 脚本反向校验，闭环成立。
2. **--force 清理实现安全**：通过 `pip uninstall` 而非递归 `rm -rf site-packages`，避免把 pip/setuptools 一起删掉的常见误操作。

---

## 评分

> 权重采用 Reviewer prompt 默认（session_context.md 未指定特殊权重）。

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 全部 6 条 AC 满足（含 E-1 阈值 / E-2 imports 修订）；红线四项全清；实跑回归 PASS |
| 安全性 | 25% | 5 | 安装路径限定嵌入 python（无 `--user` / 无系统污染）；--force 用 `pip uninstall` 安全清理；未引入 hash 校验但已在已知局限说明（PyPI 信任，ADR-009 未要求 hash pin），不构成红线 |
| 代码质量 | 15% | 5 | 脚本注释充分（含上下文/ADR 引用 / 红线声明）；readonly 常量分组清晰；变量命名一致；comment header 标明 task ID 与目的 |
| 测试覆盖 | 15% | 5 | 正常 + 幂等 + --force + 异常路径（manifest 篡改 → exit 1）+ 体积报告全部实跑；修订后回归再次实跑 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-002 / ADR-010（含 2026-05-13 补强 imports=mammoth）；目录/字段/schema 全对齐；未引入计划外依赖 |
| 可维护性 | 10% | 5 | lock 顶部含 SOP 重生成说明；脚本含 `-h/--help` 出口；修订记录章节给出明确的字段定位（行号/文件名），3 个月后接手 Agent 可快速理解 |

**综合分：5.00 / 5**（加权 = 0.25·5 + 0.25·5 + 0.15·5 + 0.15·5 + 0.10·5 + 0.10·5 = 5.00）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] NOT_PASS

判决理由：
1. 全部 AC（含 E-1/E-2 修订后契约）通过实证核验；
2. 红线四项（python-docx 残留 / docx 字面残留 / 越权 task_007-011 / imports 项数）全部清除；
3. AC-4 单一事实源闭环成立，verify-manifest-consistency.sh 真实校验 lock ↔ manifest 8 字段（含 imports 字面 7 项、schema_version=1、markitdown.extras、python.{source,version,build}、runtime_id、build_timestamp、arch）；
4. 自测矩阵实跑，修订记录与产物字节一致，无伪报。

---

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选优化，不影响 PASS）
1. **lock 重生成 SOP 可补充 hash 校验路径**
   - 文件：`scripts/requirements.lock` 顶部注释（line 22–28）
   - 现状：通过本机临时 venv `pip freeze` 解析，无 `--require-hashes`。
   - 建议：在 task_006 / task_011 决策"是否启用包 hash 校验"前，可不动；若 ADR-009 后续扩展到包供应链，可在 SOP 补 `pip freeze --all` + `pip hash` 流程。
   - 验证标准：仅文档增量，无功能变更。

2. **`--help` 输出范围切片可优化**
   - 文件：`scripts/prepare-embedded-markitdown-runtime.sh` line 61
   - 现状：`sed -n '2,25p'` 硬编码行号；脚本顶部注释如果未来扩展，--help 输出可能截断。
   - 建议：可改为定位到第二个 `# ---` 分隔符。非阻塞，可在 task_006 整批维护脚本时统一改。

---

## 给 Dev 的修复指引

不适用（PASS）。

---

## 自检清单
- [x] 我是否逐条检查了 AC 满足情况？是（含 E-1/E-2 修订）
- [x] 我是否检查了 session_context.md 的领域审查重点？是（红线四项）
- [x] 我的每个 BLOCKER/MAJOR 是否给出了具体的修复方向和验证标准？无 BLOCKER/MAJOR
- [x] 我的评分是否诚实？是 — 任务边界清晰、产物与契约完全对齐、实测全过、修订执行精确，给 5/5 有据
- [x] PASS 判决是否得到产物字节级核验支撑？是 — 四份产物文件均已读毕
