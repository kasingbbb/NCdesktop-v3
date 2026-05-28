# Task 输入 — task_000_sample_desensitization_sop

## 目标
为 7 类格式（pdf-text / docx / pptx / xlsx / html / epub / image）真实生产样本制定脱敏 SOP，并建立 AES-256 加密的私有 git-lfs 样本仓 + CI secret 解密链，作为后续 task_012 真实样本矩阵的前置依赖。

## 前置条件
- 依赖 task：无（Sprint 0，整个项目的最前置）
- 必须先存在的文件/接口：无
- 组织前置：**脱敏负责人 ≠ 打包负责人**（PRD §7 / Debate R-⑥），由 PM 指派人员。

## 验收标准（Acceptance Criteria）
1. AC-1：`samples-private/` 独立私有 repo 创建完成，启用 git-lfs，包含 README 说明用途与解密流程。
2. AC-2：`scripts/desensitize-sample.sh` 完成；可读入任一格式样本，去除"姓名/手机号/邮箱/身份证/银行卡/公司名"等 PII（按 SOP 文档列表），输出脱敏后文件并附 `.meta.json` 记录原始 hash、脱敏规则版本、脱敏者。
3. AC-3：`scripts/encrypt-samples.sh` 用 `openssl aes-256-cbc -pbkdf2 -salt -iter 100000 -in <file> -out <file.enc> -pass env:MARKITDOWN_SAMPLES_KEY` 加密；`decrypt-samples.sh` 反向解密；脚本 `set -euo pipefail` + 幂等。
4. AC-4：每格式至少 5 个脱敏样本入库（共 ≥35 个），包含 PRD 边界用例：① epub 已知生产失效样本 ≥1；② 扫描型 pdf ≥3（用于 task_009 验证）；③ 文本型 pdf ≥3。
5. AC-5：GitHub Actions（或本地 CI）添加 `MARKITDOWN_SAMPLES_KEY` secret；提供一个 dry-run workflow 演示"checkout samples-private → decrypt → ls 文件数 = 35"。
6. AC-6：SOP 文档 `docs/sample_desensitization_sop.md` 含：脱敏字段清单 / 工具链 / 双人复核流程 / 法律风险声明 / 撤回机制。
7. AC-7：不得有任何样本（加密前或加密后）以明文形式提交到 NCdesktop 主仓；CI 增加 lint 阻断 `*.pdf|*.docx|*.epub|*.pptx|*.xlsx` 在主仓任意路径出现。

## 技术约束
- Bash：`set -euo pipefail`；所有路径变量双引号；幂等。
- 不得使用云端在线脱敏服务（数据出境风险）。
- AES key 长度 ≥ 256bit；不得硬编码在脚本；CI 与本地均通过环境变量注入。
- 样本元数据 `.meta.json` 不含 PII（仅 sha256 / 规则版本 / 时间戳）。

## 参考文件
- `sessions/markitdown_fix/prd/ncdesktop_markitdown_prd_v1.md` §3.1-F8、§4.4、§6 Sprint 0
- `sessions/markitdown_fix/debate/session_001/debate_conclusions.md` Layer 3 R-⑥
- ADR-009（`tasks/task_001_architect/output.md`）

## 预估影响范围
- 新建文件：
  - 独立仓 `samples-private/`（不在主仓）
  - `NCdesktop/scripts/desensitize-sample.sh`
  - `NCdesktop/scripts/encrypt-samples.sh` / `decrypt-samples.sh`
  - `NCdesktop/docs/sample_desensitization_sop.md`
  - CI workflow（dry-run 解密 + lint）
- 修改文件：`.gitignore`（保险显式排除样本格式）
