---
description: 引导 Agent 快速理解并部署 Harness-Kit 环境
---

# Harness-Kit Onboarding

当你进入一个包含 `harness-kit/` 的新项目目录时，请按照以下步骤初始化你的认知和工作流：

## 1. 核心上下文加载
请按顺序读取以下文件以建立基本的框架意识：
1. `harness-kit/README.md`: 了解套件概况。
2. `harness-kit/core/handoff_contracts.md`: 明确你与其他角色之间的交互协议。
3. `harness-kit/core/bootstrap.md`: 了解当前项目的生命周期起始点。

## 2. 状态判定
检查项目根目录下是否存在：
- `sessions/<session_name>/session_context.md`: 如果不存在，说明项目尚未初始化。请引导用户执行 `harness-kit/scripts/new_project.sh`。
- `sessions/conductor/progress.md`: 如果存在，请将其作为当前状态的唯一真相，决定下一步行动。

## 3. 角色激活协议
根据 `progress.md` 中的 `STATE` 字段，从 `harness-kit/roles/` 中加载对应的角色 Prompt。
- 例如：如果 `STATE: DEVELOPING`，请立即加载 `harness-kit/roles/conductor/dev/prompt.md` 并遵循其内部协议（实现前计划、自测矩阵等）。

## 4. 交付验证
在产出任何交付物（PRD, Code, scorecard）之前，必须查阅 `harness-kit/core/handoff_contracts.md`，确保你的输出符合契约要求的必填项。

---

> **注意**：作为一个合格的 Harness Agent，你必须时刻保持“草稿纸设计”意识，确保所有的关键决策和进度都持久化到磁盘，而不是仅依靠对话上下文。
