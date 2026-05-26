# Session Context — 项目上下文配置

> **Session**: custom_prompt_v1
> **创建时间**: 2026-05-15

---

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop — 用户自定义 Prompt 功能
- **一句话描述**：允许用户在 NCdesktop 内置 LLM Prompt（文件打标签、PARA 分组、知识概念提取、知识聚合）之上，自定义自己的 Prompt，实现个性化知识管理需求。
- **项目类型**：Desktop App
- **复杂度等级**：L（用户可见 UI 为核心产品体验，达到 L 级别）

---

## 2. 技术上下文 [必填]

- **主语言**：Rust (Tauri backend) + TypeScript (前端)
- **框架/运行时**：Tauri 2.x + React/Next.js 前端
- **数据库**：SQLite（本地优先架构）
- **关键外部依赖**：LLM API（用于文件打标签、PARA 分组、知识概念提取、知识聚合）
- **现有代码库**：改造现有代码（在已有内置 Prompt 系统之上扩展）
- **目标部署环境**：本地（macOS/Windows 桌面应用，默认离线，本地优先）

---

## 3. 关键约束 [必填]

- **安全性要求**：中 — 用户自定义 Prompt 需防止 Prompt 注入攻击；用户数据不离开本机
- **性能要求**：中 — 自定义 Prompt 合并/渲染不应显著增加 LLM 调用延迟
- **用户体验要求**：高 — 自定义 Prompt 必须对非技术用户友好，提供直观的编辑界面和即时预览
- **可维护性要求**：高 — Prompt 模板系统需支持内置 Prompt 版本升级时不破坏用户自定义内容
- **不可妥协的底线**：
  1. 内置 Prompt 始终作为 fallback 存在，用户自定义不可完全替代内置逻辑（防止系统不可用）
  2. 用户自定义 Prompt 数据必须持久化到本地，不可仅依赖内存
  3. 隐私优先：自定义 Prompt 内容不得上传至云端

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 25% | 自定义 Prompt 必须能正确影响 LLM 输出结果 |
| 安全性 | 20% | 防 Prompt 注入、隐私保护 |
| 代码质量 | 15% | 可读性、模块化 |
| 测试覆盖 | 10% | 关键路径测试 |
| 架构一致性 | 10% | 与现有 Prompt 系统一致 |
| 可维护性 | 20% | 内置 Prompt 升级时的兼容性 |

---

## 5. 领域特定代码规范 [按需填写]

```
- Prompt 模板使用结构化格式（如 Mustache/Handlebars 模板语法或自定义占位符）
- 用户自定义部分与系统内置部分必须有明确的分层边界
- LLM 调用时，合并后的 Prompt 需有 token 长度校验
- 所有 Prompt 操作需支持撤销/回退
```

---

## 6. 领域特定审查重点 [按需填写]

```
- 用户自定义 Prompt 是否能被正确注入到 LLM 调用链中
- 内置 Prompt 升级后，用户自定义内容的兼容性处理
- Prompt 合并策略的边界情况（空值、超长、特殊字符）
- 用户在不同功能模块（打标签/分组/提取/聚合）间的 Prompt 是否独立隔离
```

---

## 7. 角色专业背景补充 [按需填写]

- **Proposer 应具备的专业知识**：
  LLM Prompt Engineering、知识管理系统设计（PARA 方法论）、桌面应用 UX 设计、模板引擎设计模式
- **Reviewer 应重点关注的风险域**：
  Prompt 注入攻击、用户自定义与内置系统的耦合度、非技术用户的使用门槛、Prompt 版本迁移风险

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`sessions/custom_prompt_v1/prd/`
- **源码路径**：`（NCdesktop 主项目 src-tauri/ 及前端目录）`
- **Session 记录路径**：`sessions/custom_prompt_v1/`
- **进度文件**：`sessions/custom_prompt_v1/conductor/progress.md`
- **架构方案存放**：`sessions/custom_prompt_v1/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述（仅 M/L 复杂度）

- **核心辩题**：如何设计一套用户可自定义的 Prompt 系统，使其在内置 Prompt 之上实现个性化知识管理，同时保证系统稳定性、安全性和非技术用户的易用性？
- **辩论偏好**：
  - 重点辩论层：全部（4 层完整 Debate）
  - 最关心的维度：体验 > 安全 > 商业可行性 > 性能

---

## 10. 用户原始需求（PM 输入）

> 当前 NCdesktop 应用的文件打标签、文件分组保存（PARA）、知识概念提取逻辑、聚合知识的这几个关于 LLM 模型应用的 prompt 都是内置的。我发现有些用户有自己的打标签习惯、分组习惯、知识概念提取的偏好习惯。所以这个功能应该可以迭代为：用户可以在内置的 prompt 之上，自定义自己的 prompt，实现个性化知识管理需求。

### 涉及的四个核心 Prompt 功能模块：
1. **文件打标签 (Tagging)**：LLM 自动为文件生成标签
2. **文件分组保存 (PARA Grouping)**：LLM 按 PARA 方法论自动分类文件
3. **知识概念提取 (Concept Extraction)**：LLM 从内容中提取核心知识概念
4. **知识聚合 (Knowledge Aggregation)**：LLM 将多个知识碎片聚合为结构化知识
