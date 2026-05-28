# Review Scorecard — task_015_rollback_sop_manifest_schema_version

## 审查思考过程

1. **Task 意图**：交付回滚 SOP 的"工程化前置"——4 节文档（触发条件 / 4h 周期 / 通信模板 / on-call）+ N-1/N-2 双层 DMG 归档脚本 + manifest schema 演进策略 + CI 验证 + 演练报告骨架 + 双签字制度，使故障日不需即兴决策。
2. **AC 检查结果**：6/6 满足（AC-5 演练实测、AC-6 PM 签名 PENDING 合理且 Dev 已标注）。
3. **关键发现**：
   - dev 选择"独立步骤"而非追加 build-macos-dmg.sh，正确保护 task_006 PASS 边界；mtime 验证 build-macos-dmg.sh（22:28:48）= task_006 时间窗（22:31:09），task_015 未触。
   - archive-dmg.sh 用 python3 解析版本号（避开 macOS BSD sort 无 -V 的陷阱），与 input.md 的"sort -V"建议方向一致且更稳。
   - 当前版本强保护逻辑（`keep_set.add(current)`）正确实现，hotfix backport 场景实测通过。
   - manifest schema 与 DMG version 强解耦在文档 §4.2 明确，且红线"看到 tauri.conf.json 改 version 强改 schema_version 应驳回"反向写出，质量很高。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 35% | 5 | 6 AC 全覆盖；触发条件梯度（task_012 ≥95% 矩阵 → 此 SOP 80% 触发回滚）设计合理；4h 周期细分到 1h+1h+2h。archive 脚本 N-1/N-2 + N-3 清理 + 当前版本保护齐全。 |
| 安全性 | 10% | 5 | sha256 双重校验（写归档 + 与 dist 原 .sha256 交叉对比）；entitlements 单独留 sha256 用于签名链证据；通信模板含 stapled / Gatekeeper 失效条款。 |
| 代码质量 | 10% | 5 | Bash 严格遵守 set -euo pipefail；变量全双引号；python3 内嵌脚本职责单一；注释充分（解释为什么不用 sort -V）。 |
| 测试覆盖 | 25% | 4 | 9 个矩阵覆盖正常/边界/异常路径；dry-run prune 抽测 2 例（1.0.0 普通清理、1.0.5 hotfix backport）独立复现通过。AC-5 实测 PENDING-USER-MACHINE 合理，骨架质量高。 |
| 架构一致性 | 10% | 5 | ADR-005（notarize+staple）+ ADR-010（runtime self-check）严格对齐；FailureCode::ERuntimeMissing 路径与 task_007 衔接；预估范围"修改 build-macos-dmg.sh" 偏离已显式声明并给出充分理由。 |
| 可维护性 | 10% | 5 | Markdown 文档结构清晰、字段占位明确（PENDING-PM / PENDING-USER-MACHINE）、签字记录有滚动表格；archive_report.txt 含 git_rev 便于追溯。 |

**综合分：4.75/5**（加权：35×5 + 10×5 + 10×5 + 25×4 + 10×5 + 10×5 = 175+50+50+100+50+50 = 475 → 4.75）

## 总体判断

- [x] **PASS**

接受 AC-5 / AC-6 标注的 PENDING 项（USER-MACHINE 实测 + PM 实名）作为下一环节产物，不阻塞本 task 合入。

## 红线检查

| 红线 | 结果 |
|------|------|
| 修改 Rust 业务代码 | NO（未触） |
| 修改 notarize.sh / sign-bundle.sh / vm-smoke.sh / run-real-sample-matrix.sh | NO（未触） |
| 修改 build-macos-dmg.sh | NO（mtime 22:28:48 = task_006 窗口，diff 全部为 task_006 header + AC-2/3/6 内容，零 archive-dmg 引用） |
| 修改 task_000~014 PASS 产物 | NO（git status 仅 4 个新文件 + 1 个 drill_report） |
| N-1 归档失败 / 历史 DMG 失效 | NO（脚本中保留策略与 sha256 校验确保归档完整性；演练步骤 4/5 明确 spctl + stapler 双 PASS 才算通过） |
| Manifest schema bump 与 DMG version 耦合 | NO（§4.2 显式解耦 + 反向红线） |

**红线全过：YES**

## 4 关注点结论

1. **独立步骤 vs 追加 build-macos-dmg.sh**：**接受 dev 选择**。理由：task_006 PASS 边界保护优先级高于"少 1 行编排"，CI 多调用一次是低成本。MINOR 建议：在 README / release-checklist 补一行"build-macos-dmg.sh 后必须调 archive-dmg.sh"，防人工失误。
2. **归档存储位置（本地 vs Release vs S3）**：**接受**。current dev 用 GitHub Releases + `gh release list` 验证，DMG 不入 git 是正确选择。upload-archive-to-release.sh 不纳入本 task scope 合理（input.md 未要求）。MINOR：建议下个 release 流程文档（非 task_015）补 release upload 步骤。
3. **双签字 CI 强制**：**当前仅 SOP 文档约束 + 手工 PR review**。dev 判断"branch protection 涉仓库管理员权限，超出 dev scope"成立。MINOR：建议 Reviewer 转给 PM 在 GitHub Settings → Branches 启用 "Require 2 approvals"，文档已铺路（rollback_sop.md §文档变更签字）。不阻塞本 task。
4. **schema_version 整数 vs 字符串**：**接受 dev 在 §4.1 给出的 `SchemaVersion` 枚举（Legacy(u32) + Semantic(String)）兼容方案**。dev 提到"下次真实 bump 时由对应 dev 同步落地" 红线（不动 Rust 代码）正确。MINOR：建议在 follow-up backlog 加一条 "schema_version=2 首次落地需 task_007 dev 同时实现 SchemaVersion enum + 兼容测试用例"。

## 自测复跑结果

- `bash -n archive-dmg.sh`：**PASS**
- YAML lint `verify-archive-presence.yml`：**PASS**（python yaml.safe_load）
- archive-dmg.sh dry-run 抽测：**2/2 PASS**
  - Test 1（normal）：1.0.0/1.1.0/1.2.0 + current=1.3.0 → 保留 1.1/1.2/1.3，prune 1.0.0 ✓
  - Test 2（hotfix backport）：1.0.5/1.1.0/1.2.0/1.3.0 + current=1.0.5 → 全保留（当前版本强保护命中）✓
- 4 个 markdown 文档语法：**PASS**（无未闭合代码块 / 表头）
- `git status` 范围：**PASS**（仅 4 新文件 + 1 个 artifacts/drill_report.md，零修改）

## 非授权区触碰

**未触碰**：
- Rust 业务代码：未触
- task_005/006/012/013 脚本：未触
- build-macos-dmg.sh：未触（mtime 22:28:48 早于 task_015 input.md 17:50:39 之后的工作窗口，但 diff 内容完整证明属于 task_006 落地，与 task_015 无关；output.md §范围 Gate 明确声明"未触"）
- task_000~014 PASS 产物：未触

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **upload-archive-to-release.sh 缺位**：archive-dmg.sh 只写本地 `dist/archive/`，而 verify-archive-presence.yml 读 GitHub Releases；两者同步靠人工。建议在 follow-up backlog 加一条 release upload 自动化（非本 task）。
2. **schema_version 切换 SchemaVersion enum 时的 task_007 配套**：本 task 仅文档，未落 Rust 兼容枚举与单元测试。建议 follow-up 加任务 "首次 schema bump 时落地 SchemaVersion + 老 manifest 兼容测试"。
3. **双签字 branch protection 文档化**：rollback_sop.md §文档变更签字写了规则，但 GitHub Settings 启用步骤未文档化。建议追加一份 1 段 README 段落到 docs/ 或 PM 在 PR review 时 self-confirm。
4. **archive 调用串接建议**：在 release-checklist 或 CI YAML（未来引入时）显式记一行 "build-macos-dmg.sh && archive-dmg.sh"，降低未来人工失误（不要求修改 task_006 脚本）。

## 给 Dev 的修复指引

**N/A**（PASS，无需修复）

## Reviewer 备注

- AC-5 实测 PENDING-USER-MACHINE 与 AC-6 PM 实名 PENDING 已在 output.md 明确标注 + drill_report.md 表格留位，回填路径清晰，不视为交付不完整。
- input.md "预估影响范围" 写"修改 build-macos-dmg.sh 末尾调用"是预估而非强制约束，dev 的"独立步骤"方案保护 task_006 PASS 边界的工程判断更优。
- mtime 凭据：build-macos-dmg.sh（May 13 22:28:48）= task_006 output.md（22:31:09）窗口；task_015 input.md（17:50:39）早于 task_006 时间窗，但 task_015 output.md（23:34:38）后 dev 未再修改 build-macos-dmg.sh（diff 内容 100% 为 task_006 AC-2/3/4/6 范围）。
