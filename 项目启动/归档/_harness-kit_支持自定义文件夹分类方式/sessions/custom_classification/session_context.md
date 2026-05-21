# Session Context — 自定义分类与工作区视图迭代

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop / NoteCapt — 自定义文件夹分类方式
- **一句话描述**：在已有 PARA（项目/领域/资源/其他）四类分类、文件转录、重命名、打标签、归档的基础上，修复工作区映射缺陷、新增 Finder 风格的文件夹视图、并允许用户自定义分类与三类核心 Prompt。
- **项目类型**：Desktop App（Tauri + React + Rust 后端）
- **复杂度等级**：L（用户可见 UI 为产品核心；新增设置面板 + Prompt 编辑器 + 文件夹视图；分类目录变化涉及磁盘 IO 与已分类资产迁移）

---

## 2. 技术上下文 [必填]

- **主语言**：TypeScript（前端）+ Rust（Tauri 后端）
- **框架/运行时**：React + Zustand + Tauri 2 + Vite
- **数据库**：SQLite（src-tauri/src/db/*）
- **关键外部依赖**：LLM Provider（通过 `src-tauri/src/llm/*`）、MarkItDown（文件转录）
- **现有代码库**：改造现有代码（已有 `src/stores/uiStore.ts`、`projectStore.ts`、`assetStore.ts`、`src-tauri/src/llm/prompts.rs`、`src-tauri/src/workspace.rs`、`src-tauri/src/commands/dropzone.rs`）
- **目标部署环境**：macOS Tauri 应用（用户本地）

---

## 3. 关键约束 [必填]

- **安全性要求**：中
  - 用户自定义 Prompt 必须在前端**纯文本编辑**，不得在执行前进行模板字符串求值（避免 RCE/注入到上层 system prompt）
  - 自定义分类名必须做文件名安全字符校验（禁止 `/ \ : * ? " < > |`、保留字、长度上限）
  - 改名 / 改分类不得**静默删除**或丢失既有资产；所有迁移要原子 + 可撤销
- **性能要求**：中
  - 工作区文件夹视图首屏 <300ms（懒加载子目录），单目录文件数千级别仍可流畅滚动
- **用户体验要求**：高
  - 必须像 Finder：列表/图标双视图、双击进入、面包屑回退、可拖拽
  - 用户在子项目内点击"导入"必须导入到**当前所在子项目**而非工作区根
- **可维护性要求**：中
  - Prompt 改为"内置默认 + 用户覆盖（diff 存 settings）"模式，方便未来升级内置 Prompt 不覆盖用户改动
- **不可妥协的底线**：
  1. 自定义分类生效后，已有的 PARA 分类资产必须保持可用（向后兼容映射 / 迁移确认弹窗）
  2. 用户改 Prompt 后，**关键变量占位符**（如 `{content}`）必须由系统校验存在；否则禁止保存
  3. 工作区映射修复不得引入跨项目文件错串

---

## 4. 质量偏好

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 30% | 工作区映射 bug 是 P0，必须 100% 修好 |
| 用户体验 | 25% | 文件夹视图是核心新交互 |
| 安全性 | 15% | Prompt 注入 + 分类名校验 |
| 架构一致性 | 10% | 与现有 store/Tauri 边界保持一致 |
| 测试覆盖 | 10% | 至少覆盖：导入路由、分类迁移、Prompt 占位符校验 |
| 可维护性 | 10% | Prompt 默认值与覆盖层分离 |

---

## 5. 领域特定代码规范

```
- TypeScript：严格模式；所有跨进程数据走 src/lib/tauri-commands.ts 统一封装
- Zustand store：副作用集中在 store action，组件只 dispatch / 读取
- Rust：Tauri command 必须返回 Result<T, String>；错误信息中文友好
- 文件 IO：在 src-tauri/src/workspace.rs 内集中；磁盘写入前做路径合法性校验
- Prompt：默认值仍在 src-tauri/src/llm/prompts.rs；覆盖层从 settings 读取，渲染时注入
```

---

## 6. 领域特定审查重点

```
- 导入路由：dropzone 的 target 是否正确反映"当前工作区上下文"（活动 projectId + workspaceFolderRelativePath）
- PARA 分类迁移：自定义分类启用 / 重命名 / 删除时，已有资产指向的分类不变性
- Prompt 占位符校验：`{content}` 等关键变量缺失检测、长度上限、禁用代码块嵌套
- 文件夹视图：N+1 IPC 调用风险、长目录滚动性能
- 设置面板：Prompt 编辑器是否与正在跑的 LLM 任务隔离（保存即生效 vs 下次任务生效）
```

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：
  桌面应用 UX（macOS Finder 模型、Files-app 模型）、Tauri/IPC 架构、Prompt 工程（变量化模板）、PARA 方法论
- **Reviewer 应重点关注的风险域**：
  数据丢失 / 跨项目串扰 / Prompt 注入 / 自定义分类与已有数据兼容 / 性能（大量小文件）

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`sessions/custom_classification/prd/`
- **源码路径**：`项目启动/NCdesktop/src/`、`项目启动/NCdesktop/src-tauri/src/`
- **Session 记录路径**：`sessions/custom_classification/`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述

- **核心辩题**：
  在不破坏既有 PARA 数据的前提下，如何同时落地三件事——
  (a) 修复"子项目内导入未映射回工作区"的 Bug；
  (b) 实现 Finder 风格文件夹视图；
  (c) 让用户在设置中自定义分类体系并编辑命名/分类/打标签三个 Prompt。

- **辩论偏好**：
  - 重点辩论层：差距分析 + 策略（MVP 取舍）
  - 最关心的维度：UX、数据兼容性、Prompt 安全

---

## 10. 用户原始需求（逐字记录）

> 目前项目具有：文件转录、文件重命名、文件打标签、文件整理（项目/领域/资源/其他）四类归类。
>
> **需求 1 — 修复工作区对应问题**：在工作区文件夹里导入原件可呈现；但点进具体已归类项目文件夹后再导入原件，工作区里**没有出现**对应文档。怀疑是上下文映射没对上，需修复。
>
> **需求 2 — 增加文件夹显示格式**：希望工作区可像 Finder 一样以列表/图标方式呈现文件所在文件夹，可点开进入子目录查看内容。
>
> **需求 3 — 支持自定义分类与 Prompt 修改**：除内置分类外，设置中新增"自定义分类格式"功能；同时把当前的命名 Prompt、分类 Prompt、打标签 Prompt 列出，允许用户自行编辑，实现个性化。
