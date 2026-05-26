# Session Context — concept_rescan_perf_v1

> **Session**: concept_rescan_perf_v1
> **创建时间**: 2026-05-16
> **复杂度**: S+（性能优化 + 增量扫描；2 文件交集；无 UI 新功能；无数据迁移）

---

## 1. 项目信息

- **项目名称**: NCdesktop — 知识概念重新扫描性能优化
- **一句话描述**: 优化"知识关联 → 重新扫描"功能，将 87 文档全量扫描从 ~84 分钟压缩到 < 10 分钟，并消除"0/87 卡死"的用户感知问题
- **项目类型**: NCdesktop 现有功能优化
- **目标用户**: 所有 NCdesktop 用户（典型工作区 50-200 文档）

---

## 2. 技术上下文

- **主语言**: Rust (Tauri backend) + TypeScript (前端)
- **目标改造位置**:
  - 后端：`src-tauri/src/commands/knowledge.rs::extract_concepts_for_library` 或同等函数
  - 前端：`src/components/features/knowledge/KnowledgeAssociationView.tsx`
- **数据库**: SQLite（可能需新增字段标记 concept_extracted_at）
- **关键外部依赖**: LLM API（同 custom_prompt_v1 session）+ tokio::stream / futures::stream::buffer_unordered

---

## 3. 关键约束

- **不破坏既有功能**: extract_concepts_for_library 仍可被其他 task 单独调用
- **不绕过 task_004 的 assemble_messages_for_concept**: 自定义 prompt 注入路径保持
- **错误隔离**: 单文档失败不能终止整个 batch；失败文档进入"待重试"状态
- **进度推送实时性**: 每文档完成后立即 emit 进度，不积压
- **内存安全**: 并发 4 路时内存峰值可控（不应一次性载入所有文档全文）

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 25% | 全量 + 增量都能正确产出 concept |
| 性能 | 25% | 全量从 84min → < 10min；增量秒级 |
| 错误隔离 | 15% | 单文档失败不污染 batch |
| 进度反馈 | 15% | UI 实时显示进度，无"卡死"感 |
| 代码质量 | 10% | tokio idiom；可读性 |
| 测试覆盖 | 10% | 并发路径 + 错误隔离 + 增量跳过 |

---

## 5. 诊断报告（task 输入的真相来源）

`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/docs/diagnose_concept_rescan_perf.md`

关键结论：
- 单文档 LLM 调用 ~58 秒，87 文档串行 ~84 分钟
- 前端进度推送机制本身实现正确，只是首文档完成前没有 progress 可 emit
- 平均 content 62 KB（最大 970 KB），存在大量冗余 token

---

## 6. 优化范围（PM 已确认全套）

### P0-1 并发 buffer_unordered(4)
- 改造 `extract_concepts_for_library` 主循环为 `Stream::buffer_unordered(4)`
- 错误隔离：单文档失败转为 `Result<ConceptResult, FailedDoc>`，batch 继续
- 84min → ~22min

### P0-2 content 截断到 8 KB
- 在拼 prompt 前 `content.chars().take(N).collect::<String>()` 或 byte-safe 截断
- 84min → ~22min
- P0-1+P0-2 叠加 → ~7-10min

### P0-3 UI 文案 + 脉冲条
- "正在扫描文档..." → "正在处理首批文档（每篇约 60 秒），预估全量 X 分钟..."
- 进度条改为脉冲动画（首文档完成前）

### P1 增量扫描
- DB 字段：`assets.concept_extracted_at` 或 `knowledge_concepts.last_extracted_at`
- "重新扫描"按钮分为两种：① 增量（默认，跳过已扫描）② 全量重扫（清空标记）
- 首次全量 ~10min；后续增量秒级

---

## 7. 不可妥协的底线

1. 不破坏 task_004 的自定义 prompt 注入链路
2. 不引入新依赖（tokio / futures 已在依赖中）
3. 失败的单文档必须可重试（不能 silent drop）
4. 增量扫描必须能"强制全量重扫"覆盖（用户 escape hatch）

---

## 8. 文件路径约定

- **Session 根**: `sessions/concept_rescan_perf_v1/`
- **Conductor 进度**: `sessions/concept_rescan_perf_v1/conductor/progress.md`
- **task 输入/产出**: `sessions/concept_rescan_perf_v1/conductor/tasks/task_perf_NN_*/`
