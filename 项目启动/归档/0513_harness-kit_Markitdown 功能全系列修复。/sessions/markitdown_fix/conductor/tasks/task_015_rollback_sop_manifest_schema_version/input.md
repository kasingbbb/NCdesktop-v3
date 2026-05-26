# Task 输入 — task_015_rollback_sop_manifest_schema_version

## 目标
落地回滚 SOP：hotfix 周期文档 + N-1 DMG 镜像归档机制 + `runtime-manifest.json.schema_version` 演进策略，使故障日有可执行预案。

## 前置条件
- 依赖 task：task_006（DMG 流水线已稳定）、task_012、task_013（验收通过的产物即将归档）
- 必须先存在的文件/接口：DMG 产物 + SHA256 公示

## 验收标准（Acceptance Criteria）
1. AC-1：`docs/rollback_sop.md` 含：
   - 回滚触发条件（如 ≥3 用户冷启失败 / Gatekeeper 阻断 / epub 矩阵通过率 < 80%）；
   - hotfix 周期目标（发现到回滚镜像上线 ≤ 4h）；
   - 通信模板（用户公告、内部 incident 时间线）；
   - 责任人（on-call 名单占位）。
2. AC-2：`scripts/archive-dmg.sh`：每次发布成功后把 DMG + 签名链 + manifest + sha256 归档到 `dist/archive/<version>/`，保留至少 N-1（最近上一版本）+ N-2 双层；自动清理 N-3 及更早。
3. AC-3：`runtime-manifest.json.schema_version` 字段在 task_002 已写入；本 task 增 `docs/manifest_schema_versioning.md`，定义：
   - 何时 bump（新增字段 = minor；删除/语义变 = major）；
   - 老 manifest 在新应用启动时的兼容策略（major 不兼容 → 退到自检失败 + 引导重装；minor → 按缺省值降级解析）。
4. AC-4：CI 增加 `verify-archive-presence.yml`：合并到 main 必须存在 N-1 归档；缺失则失败。
5. AC-5：演练：从 archive 取 N-1 DMG → 在干净 VM 安装 → Gatekeeper 通过 + 转录 1 个真实样本成功；演练报告 `task_015/.../artifacts/drill_report.md` 归档。
6. AC-6：SOP 文档必须经 PM 与负责人双签字（PR review 中标注 Reviewer 名单），不可单人合入。

## 技术约束
- N-1 DMG 必须与 N 同样 notarized + stapled（即历史镜像不得失效）。
- Manifest schema bump 与 DMG version bump 强解耦（不同 versioning）。

## 参考文件
- Debate Layer 4 共识：回滚 SOP 是 P0
- ADR-010 / ADR-005
- PRD §3.1 F12

## 预估影响范围
- 新建：`docs/rollback_sop.md`、`docs/manifest_schema_versioning.md`、`scripts/archive-dmg.sh`、CI workflow
- 修改：`scripts/build-macos-dmg.sh`（末尾调用 archive-dmg.sh）
