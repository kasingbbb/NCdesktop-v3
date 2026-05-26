# Task 输入 — task_014_legacy_unverified_migration

## 目标
对存量 `conversion_meta` 中 `status='success' AND (content IS NULL OR trim(content)='')` 的旧记录回填 `failure_code='legacy_unverified'`（与 `failed`/8 错误码并列），保证老用户升级后感知不退步，并向下游知识进化系统提供明确的"未验证"信号。

## 前置条件
- 依赖 task：task_008（`failure_code` 字段已存在）
- 必须先存在的文件/接口：`conversion_meta.failure_code` 列

## 验收标准（Acceptance Criteria）
1. AC-1：`db/migration.rs` 新增 migration（在 task_008 migration 之后）：
   ```sql
   UPDATE conversion_meta
   SET failure_code = 'legacy_unverified'
   WHERE failure_code IS NULL
     AND status = 'success'
     AND (content IS NULL OR length(trim(content)) = 0);
   ```
2. AC-2：migration 是幂等的（再次运行无 row 影响）；通过"affected_rows + 二次执行 = 0"作为单测。
3. AC-3：`db/conversion_meta.rs` 查询接口区分三态：`Success(content)` / `LegacyUnverified` / `Failed(FailureCode)`，避免下游误把 `legacy_unverified` 当 `success` 喂入知识进化系统。
4. AC-4：前端 UI：列表项显示三态独立 badge + 文案：
   - success：✅
   - legacy_unverified：⚠️ "升级前未验证，建议重转"（带"重新转录"按钮）
   - failed：❌ + 8 错误码文案
5. AC-5：单测：mock 一个 status=success/content='' 的旧记录 → 运行 migration → failure_code='legacy_unverified'；status=success/content='正常' 的记录不动。
6. AC-6：知识进化系统消费侧（如有）必须 filter 掉 `legacy_unverified`，由 task 调用方在 output.md 中标注消费侧已知点。

## 技术约束
- 严禁把 `legacy_unverified` 标为 `failed`（PRD R-④：老用户感知不退步）。
- migration 不得阻塞应用启动（大表用分批 UPDATE，或在 background thread）。

## 参考文件
- ADR-007
- Debate Layer 3 R-④
- PRD §3.1 F11

## 预估影响范围
- 修改：`db/migration.rs`、`db/conversion_meta.rs`、`models/asset.rs`（如查询返回类型变化）、前端列表组件

---

## AC-1 字面修订（Conductor 裁决 2026-05-13）

**背景**：dev 执行时 ESCALATE 发现 `conversion_meta` 表无 `status` / `content` 列（V6/V11/V12 字面追溯确认），"成功+空内容"语义实际归属 `extracted_content` 表（V8 建：status='extracted' + raw_text + structured_md）。

**裁决**：AC-1 SQL 修订为方案 A + 最新一行约束（与 task_008 `update_failure_code` 锚定策略一致）：

```sql
UPDATE conversion_meta
SET failure_code = 'legacy_unverified'
WHERE failure_code IS NULL
  AND id IN (
    SELECT cm.id
    FROM conversion_meta cm
    JOIN extracted_content ec
      ON ec.asset_id = cm.source_asset_id
    WHERE ec.status = 'extracted'
      AND (ec.raw_text       IS NULL OR length(trim(ec.raw_text))      = 0)
      AND (ec.structured_md  IS NULL OR length(trim(ec.structured_md)) = 0)
      AND cm.id = (
        SELECT id FROM conversion_meta
        WHERE source_asset_id = cm.source_asset_id
        ORDER BY converted_at DESC LIMIT 1
      )
  );
```

**AC-6 scope 修订**：消费侧 filter（`commands/knowledge.rs` + `commands/knowledge_unit_learning.rs` + `db/asset.rs:1118` LEFT JOIN）**不归本 task 实现**。本 task 仅完成 AC-6 字面"标注消费侧已知点"（dev 已在 output.md 列 3 处）；实际 filter 改造作为独立 follow-up（Conductor 用 spawn_task 提单）。

**未变更**：AC-2~5 + 三态枚举 + 前端 badge + 幂等单测 仍按原文执行。
