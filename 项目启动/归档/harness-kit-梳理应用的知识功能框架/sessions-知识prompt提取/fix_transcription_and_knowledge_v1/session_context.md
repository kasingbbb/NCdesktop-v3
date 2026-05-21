# Session Context — fix_transcription_and_knowledge_v1

## 1. 项目信息
- 项目名称：NCdesktop / NoteCapt
- 一句话描述：Tauri + React 桌面应用，本地化「工作区 → 转录 → 知识抽取 → 知识关联」流水线
- 项目类型：Desktop App (Tauri)
- 复杂度等级：M

## 2. 技术上下文
- 主语言：Rust（后端）+ TypeScript / React（前端）
- 框架：Tauri v2 + Vite + React 18
- 数据库：SQLite（本地）
- 关键外部依赖：MarkItDown、Whisper、PDF/OCR extractor、LLM provider
- 现有代码库：改造现有
- 部署：本地（macOS .dmg）

## 3. 关键约束
- 安全性：低（本地数据）
- 性能：中（大文件抽取需异步）
- 用户体验：中（工作区视图 + 知识页是核心）
- 可维护性：中
- 不可妥协的底线：
  1. 用户编辑过的概念（`user_edited=true`）任何情况下不得被自动覆盖
  2. 重抽取不得删除/孤立旧派生文件，必须版本化保留
  3. 工作区中每个原文件必须有可点击的 `.md` 邻居（哪怕是占位）

## 4. 质量偏好
| 维度 | 权重 |
|---|---|
| 功能正确性 | 35% |
| 测试覆盖 | 25% |
| 架构一致性 | 15% |
| 代码质量 | 10% |
| 安全性 | 5% |
| 可维护性 | 10% |

## 5. 领域代码规范
- Rust：错误用 `Result`，跨边界用 `anyhow::Error`
- 前端：Zustand store + 函数组件
- DB schema 变更必须可向前迁移（无 destructive drop）
- 工作区路径硬编码：`~/Downloads/NoteCaptWorkPlace/<project_id>/`

## 6. 领域审查重点
- `source_asset_should_materialize` 的所有 false 路径必须有占位文件回填
- 所有重抽取入口必须走版本化（`_versions/<asset_id>/v{N}.md`）
- 概念 upsert 必须先检查 `user_edited` 标记
- 转录完成事件 → 抽取队列的链路必须有失败重试 + 不重复入队

## 7. 角色专业背景
- Proposer：本地优先（local-first）应用、文件系统抽象、LLM pipeline
- Reviewer 关注：数据丢失风险、孤儿文件、重入幂等性、用户编辑保护

## 8. 文件路径约定
- PRD：`sessions/fix_transcription_and_knowledge_v1/prd/`
- 源码：`src/`（前端）+ `src-tauri/src/`（后端）
- Session 记录：`sessions/fix_transcription_and_knowledge_v1/`
- 进度：`sessions/fix_transcription_and_knowledge_v1/conductor/progress.md`

## 9. 辩题概述
- 核心辩题：如何修复转录→工作区不全 + 提升知识关联体验
- 辩论偏好：问题定义 + 策略
- 最关心的维度：用户体验 + 数据安全（不丢失/不覆盖）
