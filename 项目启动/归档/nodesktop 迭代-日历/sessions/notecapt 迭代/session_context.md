# Session Context — 项目上下文配置

> **使用方法**：复制本文件到 `sessions/<session_name>/session_context.md`，填写所有标注为 `[必填]` 的字段。所有角色 Prompt 将从此文件读取项目特定信息。

---

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop（NoteCapt Desktop）迭代
- **一句话描述**：基于 Tauri v2 的多模态知识采集桌面终端，将现场录音/拍照/扫描转化为 LLM 可消费的结构化知识
- **项目类型**：Desktop App
- **复杂度等级**：L（参照 bootstrap.md 评估 — 预估 task >8、UI 是产品核心、涉及音频处理/AI 集成等技术不确定性）

---

## 2. 技术上下文 [必填]

- **主语言**：TypeScript (前端) + Rust (后端/Tauri)
- **框架/运行时**：Tauri v2, React 19, Vite 6, Zustand
- **数据库**：SQLite (FTS5 全文搜索)
- **关键外部依赖**：Tailwind CSS 4, LLM API (NotebookLM/ChatGPT/Claude 导出桥), 音频波形渲染库
- **现有代码库**：改造现有代码（NCdesktop 已有基础实现，含 55+ IPC 命令）
- **目标部署环境**：本地 macOS 桌面应用

---

## 3. 关键约束 [必填]

- **安全性要求**：中 — 处理用户个人录音和照片数据，需确保本地存储安全，LLM 导出时脱敏
- **性能要求**：高 — 音频波形需 100 peaks/sec 渲染精度，Timeline 播放必须零延迟同步
- **用户体验要求**：高 — "Liquid Glass" 设计系统，macOS 原生质感，流畅物理动画
- **可维护性要求**：中 — 模块化组件架构，Zustand 状态管理
- **不可妥协的底线**：
  1. Timeline 同步精度：音频播放与 Magic Moment 关键帧的时间对齐误差 < 100ms
  2. Liquid Glass 设计一致性：所有面板必须遵循 L1-L5 层级的毛玻璃深度系统
  3. 数据完整性：TF 卡导入流程不得丢失或损坏原始文件

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 25% | 核心采集和同步链路必须可靠 |
| 安全性 | 10% | 本地应用，安全性需求较低但不可忽视 |
| 代码质量 | 15% | TypeScript 类型安全，Rust 内存安全 |
| 测试覆盖 | 10% | 关键 IPC 通道和状态转换需要测试 |
| 架构一致性 | 15% | 前后端 IPC 契约一致性关键 |
| 可维护性 | 10% | 组件模块化，方便后续迭代 |
| 用户体验 | 15% | Liquid Glass 视觉一致性和动画流畅度 |

> 权重总和为 100%。因产品核心为 UI 体验，UX 权重提升至 15%，安全性相应下调。

---

## 5. 领域特定代码规范 [按需填写]

```
- TypeScript：严格类型注解，组件 Props 使用 interface 定义
- React：函数式组件 + Hooks，禁止 class 组件
- 状态管理：所有全局状态通过 Zustand store，禁止 prop drilling 超过 2 层
- CSS：Tailwind CSS 4 + CSS Variables 实现 Liquid Glass 设计 token
- Rust/Tauri：IPC 命令使用 snake_case 命名，返回值统一使用 Result<T, E>
- 错误处理：前端使用 Error Boundary，Rust 端使用 thiserror
- 文件路径：使用 Tauri 的 path API，禁止硬编码路径
```

---

## 6. 领域特定审查重点 [按需填写]

```
- Tauri IPC 命令的序列化/反序列化是否正确处理
- 音频播放状态与波形渲染的同步机制
- Magic Moment 关键帧时间戳的精度和存储格式
- Liquid Glass 层级（L1-L5）的 blur/opacity 值是否一致
- SQLite FTS5 查询性能和索引策略
- TF 卡热插拔的事件监听和错误恢复
- 大文件（长时间录音）的内存管理
```

---

## 7. 角色专业背景补充 [按需填写]

- **Proposer 应具备的专业知识**：
  （Tauri v2 IPC 架构、React 19 并发特性、Web Audio API、音频波形可视化、macOS 设计规范 HIG）
- **Reviewer 应重点关注的风险域**：
  （音频/UI 同步死锁、大文件内存溢出、IPC 序列化性能瓶颈、CSS 动画性能（GPU 合成层管理））

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`product/prd/`
- **源码路径**：`product/src/`
- **Session 记录路径**：`sessions/`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述（仅 M/L 复杂度） [按需填写]

- **核心辩题**：NCdesktop 迭代的核心问题是什么？是 UI 视觉打磨、功能完善、还是架构重构？如何在有限迭代中最大化产品价值？
- **辩论偏好**：
  - 重点辩论层：策略（聚焦迭代优先级排序）
  - 最关心的维度：体验 + 商业可行性
