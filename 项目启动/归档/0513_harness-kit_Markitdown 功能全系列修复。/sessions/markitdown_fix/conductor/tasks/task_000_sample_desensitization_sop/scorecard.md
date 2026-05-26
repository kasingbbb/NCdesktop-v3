# Review Scorecard — task_000_sample_desensitization_sop

> **落盘说明**：本 scorecard 由 Conductor 代 Reviewer 子代理落盘。Reviewer 实例在评审完成阶段遭遇 Write/Bash 工具权限拒绝，无法直接持久化结果。完整评审结论由 Reviewer 通过 task-notification 回报给 Conductor，本文件为该回报的结构化落盘版本。

## 审查思考过程

1. **Task 意图**：建立 NCdesktop 项目"真实样本"的合规管理基线 — 脱敏 SOP（7 类 PII）+ AES-256-CBC + PBKDF2 加密链 + GitHub Actions secret + forbid-raw-samples CI gate + 私有样本仓占位（Sprint 0 范围内不实际入库 PII）。架构契约：ADR-009。

2. **AC 检查结果**：
   - AC-1（samples-private repo 占位）：✅ PENDING-PM 标识合理，placeholder + README 已说明。
   - AC-2（desensitize-sample.sh 覆盖 7 类 PII + meta.json 不含 PII）：✅ 脚本覆盖姓名/手机/邮箱/身份证/银行卡/公司名/物理地址；meta.json 仅含 sha256/规则版本/时间戳，无 PII 字段。
   - AC-3（openssl 加密链 + key 通过 env 注入禁止硬编码）：✅ `encrypt-samples.sh:10` / `decrypt-samples.sh:10` 缺失 `MARKITDOWN_SAMPLES_KEY` 即 `exit 2`；openssl `-pass env:MARKITDOWN_SAMPLES_KEY` 字面合规；`set -euo pipefail` 完备。
   - AC-4（≥35 样本 PENDING-OPERATOR 标注）：✅ Sprint 0 dev 不该实际入库 PII，标注合理。
   - AC-5（dryrun workflow yaml）：✅ checkout + decrypt + ls 计数 三步齐全。
   - AC-6（SOP 文档 6 段）：✅ 脱敏清单/工具链/双人复核/法律声明/撤回机制/应急处置 齐全；MVP 期单人 + subagent 妥协说明明示。
   - AC-7（forbid-raw-samples CI workflow）：✅ workflow 存在；本地 grep 真跑过。

3. **关键发现**：
   - Dev 主动声明的 2 处 Reviewer 关注点均不阻塞 PASS。
   - 三项红线（key 硬编码 / 主仓含 raw 样本 / 越权 dev-pack）实地核验全部合规。
   - 5 项 PENDING 全部合理推迟。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 35% | 4 | AC-1~7 全部 PASS（PENDING 部分按 input.md 字面授权）；扣 1 分因 forbid-raw-samples regex 范围与 .gitignore 广度存在合理边界争议。 |
| 安全性 | 10% | 5 | key 严格通过 env 注入；`set -euo pipefail`；openssl `-pbkdf2 -iter 100000` 字面正确；meta.json 零 PII；主仓零 raw 样本。 |
| 代码质量 | 10% | 4 | 脚本头部注释清晰；缺失环境变量即 exit 行为一致。 |
| 测试覆盖 | 25% | 4 | 9 PASS + 5 PENDING；PENDING 主要因 AC-4 真实入库前置不可执行（合理）。 |
| 架构一致性 | 10% | 5 | 显式遵循 ADR-009；脱敏链条 + 加密链 + CI gate 三件套齐全。 |
| 可维护性 | 10% | 4 | SOP 文档 6 段结构清晰；MVP 期妥协明示便于后期升级。 |

**综合分：4.20 / 5**（加权：0.35·4 + 0.10·5 + 0.10·4 + 0.25·4 + 0.10·5 + 0.10·4 = 4.20）

## 总体判断

- [x] **PASS**

理由：核心 AC 全部 PASS，三项红线（key 硬编码 / 主仓 raw 样本 / 越权 dev-pack）实地核验全部合规，5 项 PENDING 全部合理推迟。Dev 主动声明的 2 处关注点均不阻塞，其中关注 2 仅为 MINOR 改进建议（不强制）。

## Dev 主动关注点裁决

### 关注 1（forbid-raw-samples 仅含 5 类二进制后缀，未含 html/jpg/png）：PASS

实地核验主仓已 tracked 大量合法 `.png`（`NCdesktop/src-tauri/icons/` 30+ 图标 + `NCdesktop/app-icon*.png`）与 `.html`（`NCdesktop/index.html`），扩展 regex 必致大面积误报；AC-7 字面也只列 5 类二进制，当前实现与 AC 字面一致，html/image 类样本由 SOP §3 双人复核 checklist 兜底，合理。

### 关注 2（.gitignore 新增全仓 `*.pdf / *.docx / *.pptx / *.xlsx / *.epub`）：MINOR（PASS with caveat，不强制 FIX）

当前主仓零 tracked `.pdf/.docx/.pptx/.xlsx/.epub`，今天无任何文件被误遮蔽；未来若加 `docs/release-notes.pdf` 会被静默忽略。

**建议（非阻塞）**：下个 PR 改为前缀限定 `samples/**/*.pdf` 等形式。

**如未来 FIX 的验证标准**：
1. 把 `.gitignore` 第 36-40 行改为 `samples/**/*.{pdf,docx,pptx,xlsx,epub}` + `**/raw-samples/**/...`
2. `git check-ignore -v docs/release-notes.pdf` 不命中、`samples/test.pdf` 命中
3. `git ls-files '*.pdf' '*.docx' '*.pptx' '*.xlsx' '*.epub'` 仍返回空

## 红线三项

| 红线 | 结果 | 证据 |
|---|---|---|
| key 硬编码 | **PASS** | `encrypt-samples.sh:10` / `decrypt-samples.sh:10` 缺失 env 即 exit 2；openssl `-pass env:MARKITDOWN_SAMPLES_KEY` 字面合规；CI 通过 secrets 注入 |
| 主仓含 raw 样本 | **PASS** | `git ls-files '*.pdf' '*.docx' '*.pptx' '*.xlsx' '*.epub'` 实地返回空 |
| 越权改 dev-pack | **PASS** | 本 task 新增/修改清单零交集 `prepare-embedded-*.sh` / `runtime-manifest.json` / `requirements.lock` / `verify-rpath.sh` / `verify-manifest-consistency.sh`；git status 中 dev-pack 脚本的 ` M` 状态来自先前 task_001/002（脚本头部明示 `task_001_prepare_embedded_python_rpath_verify`），与 task_000 无关 |

## 5 项 PENDING 推迟合理性

| PENDING | 合理性 |
|---|---|
| PENDING-PM（样本仓 URL/Deploy Key） | YES — 需 PM 决策外部资源 |
| PENDING-OPERATOR（AC-4 ≥35 样本实际入库） | YES — Sprint 0 dev 不该实际入库 PII |
| PENDING-USER-MACHINE（pdftotext/exiftool/python-docx 等本机依赖） | YES — 工具链环境前置 |
| forbid-raw-samples GH Actions 真触发 | YES — 需远端 push 触发 |
| decrypt 错误 key 异常路径单独测 | YES — 可在 task_012 消费阶段补 |

## MINOR 改进项（不阻塞 PASS，记入未来轮次）

1. `.gitignore` 前缀限定（见关注 2 验证标准）
2. forbid-raw-samples regex 可考虑加 `samples/` 前缀约束后再扩展 html/jpg/png
3. SOP §法律声明 段落可在后续 sprint 由法务复核
4. decrypt 错误 key 路径补单测（task_012 消费时一并完成）

## 工具权限补充说明

Reviewer 子代理在评审执行期 Write 与 Bash（touch/redirect）工具被拒，无法直接落盘 scorecard.md。完整评审分析由 Reviewer 通过 task-notification 完整回报，Conductor 代为落盘以维持任务流转完整性。
