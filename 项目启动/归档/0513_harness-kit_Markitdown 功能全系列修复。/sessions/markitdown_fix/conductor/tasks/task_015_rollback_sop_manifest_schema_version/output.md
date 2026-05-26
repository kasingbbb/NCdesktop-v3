# Task 交付 — task_015_rollback_sop_manifest_schema_version

## 实现摘要

落地 4 件文档/脚本 + 1 个 CI workflow + 1 个演练报告骨架，构成回滚 SOP 完整闭环：
- **rollback_sop.md**：触发条件 / 4h hotfix 周期 / 中英双语用户公告 / 内部 incident 时间线 / on-call 名单占位 / 双签字 SOP。
- **archive-dmg.sh**：N-1+N-2 双层归档 + N-3 自动清理 + 当前版本强保护（防止 hotfix backport 误删自己）。
- **manifest_schema_versioning.md**：major/minor bump 规则 + 老 manifest 兼容策略（major 退到 ERuntimeMissing；minor 用 `serde(default)` 降级）+ 与 DMG version 强解耦。
- **verify-archive-presence.yml**：用 `gh release list` 验证 ≥2 个 release 且 N-1 含 DMG+sha256，PR to main 触发。
- **drill_report.md**：演练计划 + 8 步预期步骤 + 实测占位（PENDING-USER-MACHINE）。

**关键设计决策**：archive-dmg.sh 采用**独立后续步骤**方案，**不修改 `scripts/build-macos-dmg.sh`**。理由见下方"Reviewer 关注点"。

## AC 完成度一览

| AC | 状态 | 备注 |
|----|------|------|
| AC-1 rollback_sop.md | PASS | 4 节齐全（触发条件 / 4h 周期 / 通信模板中英双语 / on-call 占位） |
| AC-2 archive-dmg.sh | PASS | N-1+N-2 双层保留 + N-3 清理 + 当前版本保护 + sha256/manifest/entitlements/report 完整 |
| AC-3 manifest_schema_versioning.md | PASS | bump 规则 + 兼容策略 + 当前 schema_version=1 + 与 DMG version 解耦说明 |
| AC-4 verify-archive-presence.yml | PASS（YAML 校验通过，待真实 PR 验证） | 用 GitHub Releases 而非 git 内 dist/archive，避免大文件入库 |
| AC-5 演练报告 | PENDING-USER-MACHINE | 骨架完成，实测需干净 macOS VM 上执行 |
| AC-6 双签字 SOP | PASS（PENDING-PM 填名单） | rollback_sop.md 末尾"文档变更签字"节 + manifest_schema_versioning.md §4.3 一致 |

## 修改/新增的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `NCdesktop/docs/rollback_sop.md` | 新建 | AC-1 + AC-6 |
| `NCdesktop/docs/manifest_schema_versioning.md` | 新建 | AC-3 |
| `NCdesktop/scripts/archive-dmg.sh` | 新建（已 chmod +x） | AC-2 |
| `NCdesktop/.github/workflows/verify-archive-presence.yml` | 新建 | AC-4 |
| `sessions/.../task_015/artifacts/drill_report.md` | 新建 | AC-5 骨架 |

**未触及**：`build-macos-dmg.sh`、`notarize.sh`、`sign-bundle.sh`、`vm-smoke.sh`、`run-real-sample-matrix.sh`、任何 Rust 业务代码、task_000~014 PASS 产物。

## 对 Architect 方案的遵守声明

- [x] 目录结构与方案一致（docs/ + scripts/ + .github/workflows/）
- [x] 未引入计划外新依赖（python3 + shasum + jq + gh，均已是 task_005/006/012 既有依赖）
- [x] 数据模型与 task_002 manifest schema_version 字段对齐
- 偏离说明：input.md "预估影响范围" 写"修改 build-macos-dmg.sh 末尾调用"，本实现选择**不修改**，采用独立步骤方案。理由见下方 Reviewer 关注点 #1。Prompt 明确允许此二选一。

## 测试命令

```bash
# 1. 文档 markdown 静态检查（依赖 python3 可读）
ls -l NCdesktop/docs/rollback_sop.md NCdesktop/docs/manifest_schema_versioning.md

# 2. archive-dmg.sh 语法
bash -n NCdesktop/scripts/archive-dmg.sh

# 3. YAML 语法
python3 -c "import yaml; yaml.safe_load(open('NCdesktop/.github/workflows/verify-archive-presence.yml'))"

# 4. dry-run 矩阵（mock 1.0.0/1.1.0/1.2.0 + 当前 1.3.0 + hotfix 回填 1.0.5）
# 见下方"测试结果"段
```

## 测试结果

```
syntax OK
yaml OK

=== Test 1: N (=1.3.0) 归档 + prune ===
[archive-dmg] version: 1.3.0
[archive-dmg] sha256: 6c240cfa419ebb56972a77b813075eb7f6a7b323668c634ae242f8c38d3b54b9
[archive-dmg] prune: 删除 N-3 及更早归档 .../archive/1.0.0
[archive-dmg] DONE
expected: 1.1.0 1.2.0 1.3.0
actual:   1.1.0 1.2.0 1.3.0
PASS: N + N-1 + N-2 保留，N-3=1.0.0 已清理

=== Test 2: archive_report.txt ===
version:        1.3.0
files: 1.3.0.sha256 / NCdesktop_1.3.0_aarch64.dmg / archive_report.txt /
       entitlements.plist / runtime-manifest.json
dmg_sha256:         6c240cfa419ebb56972a77b813075eb7f6a7b323668c634ae242f8c38d3b54b9
entitlements_sha256: 41a056da66c0de781266487ec2f15634c095dd4b1b656669192e7ffa105f6005

=== Test 4: sha256 校验 ===
NCdesktop_1.3.0_aarch64.dmg: OK

=== Test 5: hotfix backport 当前版本保护（1.0.5 语义 < 现存 1.1/1.2/1.3） ===
PASS: 当前版本 1.0.5 即使语义低也被保护
ALL TESTS PASS
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|----------|---------|------|------|
| ✅ 正常路径 | N 归档 + 清理 N-3 | 已测 | PASS（1.0.0 被删，1.1/1.2/1.3 保留） |
| ✅ 正常路径 | sha256 与原始 dist/<v>.sha256 交叉校验 | 已测 | PASS（一致路径走通） |
| ✅ 正常路径 | archive_report.txt 含 git rev + 文件清单 | 已测 | PASS |
| ⚠️ 边界条件 | hotfix backport：当前版本语义低于现存归档 | 已测 | PASS（当前版本强保护，不被 prune 误删） |
| ⚠️ 边界条件 | dry-run 模式（ARCHIVE_DRY_RUN=1） | 已测 | PASS（跳过 prune） |
| ⚠️ 边界条件 | manifest 缺失 | 已测 | WARN 后继续（不阻塞归档） |
| ❌ 异常路径 | DMG 路径不存在 | 已测 | FAIL 退出 1 |
| ❌ 异常路径 | tauri.conf.json version 为空 | 已测（代码路径） | FAIL 退出 1 |
| ❌ 异常路径 | sha256 与 dist 原始不一致 | 已测（代码路径） | FAIL 退出 1 |

## 浏览器/运行时验证

**N/A**：本 task 仅交付文档 + Shell 脚本 + CI workflow + 演练骨架，无 UI、无可启动服务。
- AC-5 演练对应的 macOS Gatekeeper / 冷启动验证 = PENDING-USER-MACHINE（需干净 VM）。

## 范围 Gate

`git status` 仅含本 task 5 个新建文件 + 1 个 artifacts/drill_report.md，未触：
- Rust 业务代码：未触
- task_005/006/012/013 核心 scripts：未触
- task_000~014 PASS 产物：未触
- `build-macos-dmg.sh`：未触（独立步骤方案，见下）

## 已知局限

1. **AC-5 演练实测 PENDING-USER-MACHINE**：本机无干净 macOS VM，仅交付演练骨架。预期由 Tech Lead 在 vm-base-image.md 描述的环境中执行后回填 drill_report.md 的"实测执行"段。
2. **on-call 名单 PENDING-PM**：rollback_sop.md §4 + 文档变更签字节占位待 PM 在 PR review 时填入实名。
3. **schema_version 一旦 bump 到 minor 需要 Rust 端配套**：本 task 仅写策略文档，未给 Rust 结构体添加 `#[serde(default)]`（红线：不动 Rust 业务代码）。下次真实 bump 时由对应 dev 同步落地。
4. **verify-archive-presence.yml 尚未在真实 PR 上 run 过**：需 ≥2 个 GitHub Release 才能正向验证。当前 release 数若不足，workflow 会 fail-fast 报错——这是预期行为而非 bug。

## 需要 Reviewer 特别关注的地方

**1. archive-dmg.sh 设计为独立步骤，未嵌入 build-macos-dmg.sh**
- 理由：task_006 已 PASS，其 10 步流程是完整测试覆盖的核心路径。在末尾追加一行虽然"最小侵入"，但仍是修改已 PASS 产物，触发回归风险（例如归档失败导致整个 build 流程 exit ≠ 0，破坏 task_006 PASS 边界）。
- 独立步骤的代价：CI 流水线需显式调用两次（先 build-macos-dmg.sh，再 archive-dmg.sh），多一行编排。
- Prompt 明确允许此二选一，请 Reviewer 确认接受。

**2. 归档存储位置：本地 `dist/archive/` vs GitHub Releases vs S3**
- archive-dmg.sh 写本地 `dist/archive/<version>/`（开发机/构建机）；
- verify-archive-presence.yml 读 GitHub Releases；
- 两者**未自动同步**——预期 release 流程中由人/CI 把 `dist/archive/<v>/` 上传到 Release assets。
- 是否需要在本 task 增 `upload-archive-to-release.sh`？我判断**不需要**（input.md 未要求且会扩大 scope）。请 Reviewer 确认。

**3. manifest schema 当前版本**
- task_002 写的是整数 `1`，本文档兼容整数→字符串 "1.0"（§4.1 `SchemaVersion` 枚举）。Reviewer 请验证 task_007 自检逻辑确实可以处理这两种类型，或要求 task_007 dev 复核。

**4. 双签字 SOP 的 CI 强制方式**
- 当前是文档约束 + 文化约定，未通过 GitHub Branch Protection 强制 ≥2 approvals。
- 是否需要在本 task 增 branch protection 配置？我判断**不需要**（涉及仓库管理员权限，超出 dev scope），但建议 Reviewer 提醒 PM 在 GitHub Settings → Branches 启用"Require 2 approvals before merging"。

**5. drill_report.md PENDING 字段的回填责任**
- 演练实测后由 Tech Lead 直接编辑此文件填实测结果，无需新 PR 走双签字流程（因为只填实测数据不改 SOP）。Reviewer 确认此理解。
