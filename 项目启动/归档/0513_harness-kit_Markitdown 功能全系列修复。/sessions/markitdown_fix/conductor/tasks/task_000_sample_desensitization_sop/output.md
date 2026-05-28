# Task 交付 — task_000_sample_desensitization_sop

## 实现摘要

按 ADR-009 在 NCdesktop 主仓内交付了真实样本脱敏 SOP + AES-256 加密链 + CI 解密 dry-run + 主仓 lint 阻断的全套基础设施。**未实际入库样本**（AC-4 依赖人工 + samples-private 仓建仓后由 PM 指派操作员完成，本 task 仅交付脚本与流程）。

核心设计决策：
- **脱敏脚本** `desensitize-sample.sh` 采用「正则 + 启发式」纯本地实现（数据不出境），规则版本 `v1.0`，覆盖 7 类格式；二进制格式（pdf/docx/pptx/xlsx/epub）当前以「拷贝 + 人工复核」兜底，避免越权动 dev-pack 的 embedded python；文本流（含 pdftotext 后输出）走完整 7 类 PII 替换。
- **加密链** 严格遵守 ADR-009 命令字 `openssl aes-256-cbc -pbkdf2 -salt -iter 100000 -pass env:MARKITDOWN_SAMPLES_KEY`；脚本三件套（desensitize / encrypt / decrypt）均 `set -euo pipefail` + 路径双引号 + mtime 幂等。
- **CI 双轨**：`decrypt-samples-dryrun.yml` 用 `workflow_dispatch` 演示 secret 注入 + 解密 + 文件数 ≥35 校验，**完全不上传明文 artifact**；`forbid-raw-samples.yml` 在 push/PR 时 lint 主仓阻断 5 类二进制后缀。
- **SOP 文档**齐 6 段（脱敏字段清单 + 工具链 + 双人复核 + 法律声明 + 撤回机制 + 应急处置）+ 入库 checklist 占位 + CHANGELOG 机制；MVP 期「单人 + subagent 职责分离」妥协写入文档，正式发布前须升级为真人双人复核。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/scripts/desensitize-sample.sh` | 新建 | 7 类 PII 脱敏 + meta.json 生成（v1.0 规则） |
| `NCdesktop/scripts/encrypt-samples.sh` | 新建 | AES-256-CBC + PBKDF2 加密；单文件 / 目录递归；mtime 幂等 |
| `NCdesktop/scripts/decrypt-samples.sh` | 新建 | 反向解密；同样支持目录递归 + 幂等 |
| `NCdesktop/docs/sample_desensitization_sop.md` | 新建 | SOP 主文档（6 段 + 入库 checklist + 附录） |
| `NCdesktop/.github/workflows/decrypt-samples-dryrun.yml` | 新建 | CI 解密 dry-run（workflow_dispatch） |
| `NCdesktop/.github/workflows/forbid-raw-samples.yml` | 新建 | 主仓 lint：阻断 pdf/docx/pptx/xlsx/epub |
| `NCdesktop/.gitignore` | 修改 | 显式排除 7 类样本后缀 + `samples/` + `*.enc` |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（脚本在 `NCdesktop/scripts/`、SOP 在 `NCdesktop/docs/`、CI 在 `NCdesktop/.github/workflows/`，与 task input §"预估影响范围" 完全一致）
- [x] API 路径 / 命名与 Architect 方案一致（脚本文件名、env var `MARKITDOWN_SAMPLES_KEY`、openssl 参数与 ADR-009 字面一致）
- [x] 数据模型与 Architect 方案一致（meta.json 仅含 `original_sha256 / sanitized_sha256 / rule_version / desensitized_by / timestamp`，不含 PII，符合技术约束 §24）
- [x] 未引入计划外的新依赖（脚本只依赖 bash / openssl / python3 / 可选 pdftotext / exiftool；未碰 dev-pack 的 embedded python）
- **方案 A 职责分离妥协**：本 worker（dev-desensitize）严格不读写 dev-pack 产物（`scripts/prepare-embedded-*.sh` / `requirements.lock` / `runtime-manifest.json` / `verify-rpath.sh` / `verify-manifest-consistency.sh`），全部新增文件与 dev-pack 输出零交集。
- **ADR-009 遵守**：openssl 命令字、私有 git-lfs 假设、CI secret 注入路径均与 ADR-009 字面对齐。

## 测试命令

```bash
# 1) bash -n 静态语法
bash -n NCdesktop/scripts/desensitize-sample.sh
bash -n NCdesktop/scripts/encrypt-samples.sh
bash -n NCdesktop/scripts/decrypt-samples.sh

# 2) YAML 语法
python3 -c "import yaml; yaml.safe_load(open('NCdesktop/.github/workflows/decrypt-samples-dryrun.yml')); yaml.safe_load(open('NCdesktop/.github/workflows/forbid-raw-samples.yml'))"

# 3) 脱敏功能（合成 PII 输入）
WORK=$(mktemp -d); cat >"$WORK/sample.txt" <<EOF
联系人：张伟，手机 13812345678，邮箱 zhangwei@example.com
身份证 11010519491231002X，银行卡 6222020200112233445
公司：北京晨曦科技有限公司 / Acme Corp
地址：北京市朝阳区建国路88号
国际号码 +1 415-555-0199
EOF
DESENSITIZER=test-operator bash NCdesktop/scripts/desensitize-sample.sh "$WORK/sample.txt"
grep -E '13812345678|zhangwei@example|11010519491231002X|6222020200112233445|\+1 415-555-0199' "$WORK/sample.sanitized.txt" && echo FAIL || echo PII_REDACTED_PASS

# 4) 加密 / 解密环回
WORK=$(mktemp -d); echo "hello task_000" > "$WORK/x.txt"
H1=$(shasum -a 256 "$WORK/x.txt" | awk '{print $1}')
export MARKITDOWN_SAMPLES_KEY="test-key-$(openssl rand -hex 16)"
bash NCdesktop/scripts/encrypt-samples.sh "$WORK/x.txt"
rm "$WORK/x.txt"
bash NCdesktop/scripts/decrypt-samples.sh "$WORK/x.txt.enc"
H2=$(shasum -a 256 "$WORK/x.txt" | awk '{print $1}')
[ "$H1" = "$H2" ] && echo ROUNDTRIP_PASS

# 5) 幂等 & 目录递归
bash NCdesktop/scripts/encrypt-samples.sh "$WORK/x.txt"  # 期望 [skip]

# 6) lint 逻辑（本地真跑）
git ls-files | grep -Ei '\.(pdf|docx|pptx|xlsx|epub)$' | wc -l   # 期望 0
```

## 测试结果

```
$ bash -n desensitize-sample.sh encrypt-samples.sh decrypt-samples.sh
ALL BASH -n PASS

$ python3 yaml-safe-load (2 workflows)
YAML OK

$ desensitize roundtrip:
[OK] sanitized → .../sample.sanitized.txt
[OK] meta      → .../sample.sanitized.txt.meta.json
联系人：[NAME_CN_REDACTED]，手机 [PHONE_CN_REDACTED]，邮箱 [EMAIL_REDACTED]
身份证 [IDCARD_REDACTED]，银行卡 [BANKCARD_REDACTED]
公司：[COMPANY_CN_REDACTED] / [COMPANY_EN_REDACTED]
地址：[ADDRESS_CN_REDACTED]
国际号码 [PHONE_E164_REDACTED]
ALL PII REDACTED: PASS
meta.json: {"original_sha256":"bf0b2f...","sanitized_sha256":"2f14f0...","rule_version":"v1.0",
            "desensitized_by":"test-operator","timestamp":"2026-05-13T10:46:28Z",...}
(无 PII 字段)

$ encrypt/decrypt roundtrip:
[enc]  .../x.txt → .../x.txt.enc
[dec]  .../x.txt.enc → .../x.txt
IDEMPOTENT ENC: PASS（第二次 encrypt 同文件输出 [skip]）
ROUND-TRIP: PASS（sha256 完全一致）
DIR MODE ENC: PASS（递归 a.txt + sub/b.txt）
DIR MODE DEC: PASS

$ git ls-files | grep -Ei '\.(pdf|docx|pptx|xlsx|epub)$' | wc -l
0   # 主仓干净，lint 当前不会误报
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | desensitize-sample.sh 处理含 7 类 PII 的合成 .txt | 已测 | PASS（全部替换为 `[*_REDACTED]` token） |
| ✅ 正常路径 | meta.json 字段齐全且不含 PII | 已测 | PASS（仅 sha256 / rule_version / desensitized_by / timestamp） |
| ✅ 正常路径 | encrypt → decrypt 环回 sha256 一致 | 已测 | PASS |
| ✅ 正常路径 | encrypt 幂等（已存在 .enc 且 mtime 新） | 已测 | PASS（输出 `[skip]`） |
| ✅ 正常路径 | 加密 / 解密 目录递归 | 已测 | PASS（含子目录） |
| ✅ 正常路径 | bash -n 三脚本静态语法 | 已测 | PASS |
| ✅ 正常路径 | 两个 yaml workflow yaml.safe_load | 已测 | PASS |
| ⚠️ 边界条件 | 缺失 MARKITDOWN_SAMPLES_KEY env | 已测 | PASS（明确 `[ERROR] ... not set` + exit 2） |
| ⚠️ 边界条件 | input file 不存在 | 已测 | PASS（exit 2 + 明确报错） |
| ⚠️ 边界条件 | 二进制格式（docx/pdf/epub）脱敏 | 部分覆盖 | 文本层（pdftotext 可用时）走完整规则；二进制本体仅拷贝 + 人工复核 — 已在 SOP 显式说明（已知局限） |
| ⚠️ 边界条件 | image EXIF 清除 | 设计已覆盖 | 依赖 exiftool；未在本机做端到端跑通（缺合成 jpg） — PENDING-USER-MACHINE |
| ⚠️ 边界条件 | 国际号码 `+1 415-555-0199` 含空格/横线 | 已测 | PASS（v1.0 修订正则后命中） |
| ❌ 异常路径 | forbid-raw-samples 在主仓存在样本时 fail | 未端到端跑 GH Actions | PENDING — 本地 `git ls-files | grep` 逻辑已验证；GH Actions runner 上的真实触发待 PM 提供 samples-private 后 dry-run |
| ❌ 异常路径 | decrypt-samples 用错误 key | 未单独测 | openssl 自身会以非 0 退出 + 错误 message；`set -e` 会终止脚本 |
| ❌ 异常路径 | 中文姓名误伤率（白名单姓氏 + 1-3 字） | 已设计未量化 | v1.0 启发式覆盖常见姓氏；正式上线前应在 ≥100 真实样本上抽样误伤率（依赖 AC-4 入库） |

## 浏览器/运行时验证

**N/A** — 本 task 交付物均为离线脚本 + CI workflow + 文档，无 UI 或可启动服务。

## 已知局限

1. **PENDING-PM**：样本仓 URL / Org / Repo 名 / Deploy Key 占位符（`<SAMPLES_REPO_URL>` / `<ORG>/<SAMPLES_REPO_NAME>` / `SAMPLES_DEPLOY_KEY`）分布于 `decrypt-samples-dryrun.yml` 与 SOP 文档顶部，需 PM 指派后全局替换。
2. **PENDING-OPERATOR（AC-4）**：实际样本入库（7 类 × ≥5 = ≥36 个）不在本 task scope；依赖 PM 指派操作员 + samples-private 仓建仓后，按 SOP §2 + 附录 A checklist 推进。本 task 仅交付流程与脚本。
3. **PENDING-USER-MACHINE**：image EXIF 清除依赖 `exiftool`、PDF 文本层提取依赖 `pdftotext`、docx/pptx/xlsx/epub 完整脱敏依赖 `python-docx / openpyxl / python-pptx / ebooklib`；本机未必全装。SOP 中以 `[WARN]` 显式提示用户安装。本 worker **禁止越权**修改 dev-pack embedded python 来安装这些库，符合方案 A 职责分离。
4. **MVP 双人复核妥协**：当前阶段单人 + subagent 模拟职责分离；正式发布前 PM 必须指派真人复核者补齐 SOP §3 正式流程，否则不可上线。
5. **中文姓名脱敏精度**：v1.0 仅用姓氏白名单 + 1-3 字启发式，存在误伤普通中文短语风险；待 AC-4 入库后做误伤率抽样，若 > 5% 则升 v1.1（NER 候选 / 更窄白名单 / 上下文判别）。
6. **二进制格式脱敏深度**：docx/pptx/xlsx/epub 当前是「拷贝 + 人工复核」，未做 unzip-modify-rezip 自动化。若 AC-4 入库后人工复核耗时不可接受，再升 v1.1 引入 ad-hoc python3 + python-docx 等（仍坚持不动 dev-pack embedded python）。
7. **forbid-raw-samples 的 allowlist 为空**：当前主仓不含合成 fixtures；若后续有合成样本（如 `NCdesktop/fixtures/synthetic/*.pdf`），需在 workflow 内 `ALLOWLIST_PREFIXES` 数组中显式加入。

## 需要 Reviewer 特别关注的地方

1. **`forbid-raw-samples.yml` 的覆盖广度**：当前 `FORBIDDEN_REGEX` 仅含 `pdf|docx|pptx|xlsx|epub`（与 AC-7 字面一致）。是否应额外扩展 `html|htm|jpg|jpeg|png` ？AC-7 字面未列这些，且 NCdesktop 主仓里 html/png 大量用作 UI 资源（app-icon.png 等），扩展会大面积误报。**建议保持当前清单 + 在 SOP 写明 html/image 类样本需 reviewer 人工把关**，但请 reviewer 拍板。
2. **`.gitignore` 中新加 `*.pdf / *.docx / *.pptx / *.xlsx / *.epub`**：这是「保险栏」防止误 add，但同时会让 `NoteCapt_0.1.0_aarch64.dmg` 这类合法构建产物的"同伴文件"（如发布说明的 pdf）也被默认忽略。请 reviewer 确认这与现有构建工件无冲突；若有特定合法 pdf/docx 需保留（如 `NCdesktop/README.pdf`），可用 `!path/to/file` 显式 unignore。
