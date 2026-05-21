# Session Context — 工作区文件拖拽与文件夹操作迭代

---

## 1. 项目信息

- **项目名称**：NCdesktop 工作区文件操作迭代（Finder 式文件调度）
- **一句话描述**：为 NCdesktop 的两栏文件视图补全 Finder 式多选、拖拽到外部、右键移到文件夹的交互能力
- **项目类型**：Desktop App
- **复杂度等级**：L

---

## 2. 技术上下文

- **主语言**：TypeScript（前端）/ Rust（后端）
- **框架/运行时**：Tauri 2.x + React 18 + Vite；Rust tokio 异步
- **数据库**：SQLite（通过 rusqlite，State<Database> 注入）
- **关键外部依赖**：
  - `@crabnebula/tauri-plugin-drag` v2.1.0 — OS 级文件拖拽（`startDrag`）
  - `@tauri-apps/api` — Tauri invoke / 事件系统
  - Zustand — 前端状态管理（assetStore、uiStore、projectStore）
- **现有代码库**：改造现有代码
- **目标部署环境**：本地（macOS 桌面应用）

---

## 3. 关键约束

- **安全性要求**：中 — 文件操作限制在 `~/Downloads/NoteCaptWorkPlace/` 内，防止路径越界
- **性能要求**：中 — 右键菜单打开 ≤ 100ms，move 命令 ≤ 800ms/文件
- **用户体验要求**：高 — 交互行为必须符合 macOS Finder 心理模型
- **可维护性要求**：中 — 与现有 hooks/store 模式保持一致

- **不可妥协的底线**：
  1. 两栏布局（原件左栏 + 工作区右栏）不重构
  2. `startDrag` 现有能力（右栏拖到外部应用）不退步
  3. Rust move 命令必须原子性：磁盘和 DB 同步回滚，不产生孤儿文件
  4. 文件操作只作用于用户显式选中的文件，无隐式连带行为
  5. 路径计算必须 canonicalize 后检查是否在项目 workspace root 内（防越界）

---

## 4. 质量偏好

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 35% | 拖拽/移动行为必须精确 |
| 用户体验 | 25% | Finder 心理模型对齐 |
| 代码质量 | 15% | 与现有 hooks/store 模式一致 |
| 架构一致性 | 15% | 不引入新的状态层或架构层 |
| 测试覆盖 | 5% | 核心 Rust 命令有单元测试 |
| 安全性 | 5% | 路径越界防护 |

---

## 5. 领域特定代码规范

```
前端：
- Hooks 遵循现有 useDragAssets / useRubberBandSelect 模式
- 组件 props 用 interface 定义，不用 type
- Tauri invoke 调用统一通过 src/lib/tauri-commands.ts 封装
- Store 操作通过 useAssetStore / useUIStore，不直接操作 DOM 状态
- 新组件放在 src/components/features/ 或 src/components/features/assets/

Rust：
- 命令函数统一放在 src-tauri/src/commands/ 对应文件
- 新命令在 src-tauri/src/lib.rs 的 invoke_handler 中注册
- 数据库操作通过 State<'_, Database> 注入，conn.lock() 获取连接
- 错误返回 Result<T, String>，不 panic
- 文件路径操作后必须 canonicalize() 并验证在 workspace root 内
```

---

## 6. 领域特定审查重点

```
- startDrag 调用时机：必须在 mousemove 阈值后触发，不能被 Web DnD 抢先截断
- move 命令回滚：如果第 N 个文件失败，前 N-1 个已移动的文件是否都回滚
- 路径越界检查：target_relative_path 是否可以通过 ".." 跳出 workspace root
- selectedAssetIds 跨栏混用：左栏多选时，BatchToolbar 原有"移到项目"操作是否产生异常
- Cmd+A 全选作用域：左栏焦点时 Cmd+A 不能意外选中右栏文件
```

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：macOS Finder 交互模型、Tauri 2.x 插件体系（plugin:drag）、React DnD 与 OS DnD 的区别
- **Reviewer 应重点关注的风险域**：路径越界、startDrag 与 Web DnD 的时序冲突、Rust 文件操作原子性、selectedAssetIds 跨栏混用副作用

---

## 8. 文件路径约定

- **PRD 路径**：`sessions/workspace_drag_v1/prd/`
- **Session 记录路径**：`sessions/workspace_drag_v1/`
- **进度文件**：`sessions/workspace_drag_v1/conductor/progress.md`
- **架构方案存放**：`sessions/workspace_drag_v1/conductor/tasks/task_001_architect/output.md`
- **源码路径（主项目）**：`项目启动/NCdesktop/src/`

---

## 9. 辩题概述

- **核心辩题**：在保留 `startDrag`（OS 级文件拖出）能力的前提下，如何为 NCdesktop 两栏布局补全 Finder 式多选和文件夹调度能力
- **重点辩论层**：全部（L 复杂度，4 层完整 Debate）
- **最关心的维度**：用户体验 + 技术可行性（startDrag vs Web DnD 兼容性）
