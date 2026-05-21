# Harness-Kit 标准套件

这是一个通用化的 AI 编程 Harness 框架套件，遵循 Bink 的一人代码（One-man Code）实践理念。你可以将此套件直接复制到任何项目中，以启动高质量的 AI 协作流程。

## 目录结构

- `core/`: 核心框架逻辑，包括启动协议、会话模板和角色间契约。
- `roles/`: 所有 Agent 角色的定义。每个角色有独立的文件夹，未来可扩展示例（examples/）和自动化测试。
- `.agent/workflows/`: 提供给 Agent 的自动化指令。
- `scripts/`: 用于初始化新项目的辅助脚本。

## 快速开始

1. **部署套件**：将 `harness-kit/` 文件夹整体复制到你的新项目根目录。
2. **初始化项目**：
   ```bash
   bash harness-kit/scripts/new_project.sh <project_name>
   ```
3. **启动引导**：在那之后，让 AI 读取 `harness-kit/.agent/workflows/onboarding.md`，它将自动引导你完成后续步骤。

## 核心理念
- **框架与业务解耦**：所有项目特定信息通过 `session_context.md` 注入。
- **任务原子化**：通过增量执行循环（Incremental Loop）确保每步质量。
- **状态持久化**：使用 `progress.md` 作为唯一真相来源，支持跨 Session 断点续传。
