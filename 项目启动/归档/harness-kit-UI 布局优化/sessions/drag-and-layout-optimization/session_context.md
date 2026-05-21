# Session Context — 拖拽分享与中栏布局优化

> 基于 harness-kit session_context 模板填写

---

## 1. 项目信息 [必填]

- **项目名称**：NCdesktop 拖拽外发与中栏布局优化
- **一句话描述**：为 NCdesktop 的文件管理功能增加「拖拽到外部应用/网页」的多模态素材外发能力，并优化三栏布局中栏的文件呈现密度，让用户能在一屏内看到更多文件、一次性多选更多素材丢给外部 AI 工具消化
- **项目类型**：Desktop App（Tauri v2 + React）
- **复杂度等级**：M（中等）— 预估 5-7 个 task，UI 是产品核心，有 1-2 个待验证技术点（Tauri 拖出文件协议）

---

## 2. 技术上下文 [必填]

- **主语言**：TypeScript（前端）+ Rust（后端 Tauri Core）
- **框架/运行时**：React 19 + Vite 6 + Tauri v2
- **数据库**：SQLite（via rusqlite）
- **关键外部依赖**：
  - `@tauri-apps/api` — 前端 IPC
  - `tauri-plugin-liquid-glass` — macOS Liquid Glass 窗口效果
  - `zustand` — 状态管理
  - `lucide-react` — 图标
  - `tailwindcss v4` — 样式
- **现有代码库**：改造现有代码（项目已运行，文件管理功能基本完善）
- **目标部署环境**：macOS 本地桌面应用

---

## 3. 关键约束 [必填]

- **安全性要求**：中 — 拖出文件时必须使用实际本地文件路径，不可暴露内部 DB 结构
- **性能要求**：高 — 拖拽启动延迟 < 100ms，大量文件（100+）列表滚动 60fps
- **用户体验要求**：高 — 拖拽操作必须符合 macOS 原生体验（拖拽预览、多文件计数徽章、松手即完成）
- **可维护性要求**：中 — 代码结构遵循现有项目约定（hooks + stores + components 分层）
- **不可妥协的底线**：
  1. 拖出操作必须传递**真实文件路径**（不是临时文件），确保外部应用能直接消费
  2. 不破坏现有的「拖入导入」功能和内部素材拖拽关联功能
  3. 中栏布局优化不能牺牲信息可读性——文件名、类型、标签仍需可见
  4. 必须支持多选拖出（框选 / Cmd+Click → 拖出全部选中项）

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 30% | 拖出功能必须真实工作，文件路径正确 |
| 安全性 | 10% | 风险较低，但不可暴露敏感路径 |
| 代码质量 | 15% | 遵循项目现有编码规范 |
| 测试覆盖 | 10% | 拖拽 E2E 难以自动化，手工验证为主 |
| 架构一致性 | 15% | hooks/stores/components 分层一致 |
| 可维护性 | 20% | 拖拽协议需清晰定义，未来可扩展 |

---

## 5. 领域特定代码规范

- TypeScript 严格模式，禁止 `any`
- React 函数式组件，hooks 在顶部
- Zustand store 按功能域拆分
- Tailwind CSS v4 + CSS 自定义属性（设计令牌）
- Rust 端所有 IPC 命令返回 `Result` 类型
- 拖拽相关 MIME 类型统一以 `application/notecapt-*` 为前缀

---

## 6. 领域特定审查重点

- Tauri v2 的 `startDrag` API 是否正确使用（macOS NSPasteboard 协议）
- `dataTransfer.setData()` 的 MIME 类型是否与系统文件拖放兼容
- 多选状态管理（selectedAssetIds）与拖拽触发的一致性
- 中栏布局的信息密度 vs 可读性平衡
- 框选与拖拽的冲突处理（何时启动框选 vs 何时启动拖拽）

---

## 7. 角色专业背景补充

- **Proposer 应具备的专业知识**：
  - macOS 原生拖放协议（NSPasteboard / NSFilenamesPboardType）
  - Tauri v2 的 `drag-and-drop` 插件与 `startDrag` API
  - Web `DataTransfer` API 与系统交互的限制
  - 高信息密度 UI 设计（Finder 列表模式、VSCode 资源管理器）
- **Reviewer 应重点关注的风险域**：
  - Web → 系统文件拖放的技术可行性边界（WebView 安全沙箱限制）
  - 拖拽与框选手势的冲突
  - 中栏紧凑布局在不同窗口尺寸下的响应式表现
  - 与现有 `useDragAssets` / `useRubberBandSelect` hooks 的兼容性

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`sessions/drag-and-layout-optimization/prd/`
- **源码路径**：`../../NCdesktop/src/`（相对于 harness-kit 根目录）
- **Session 记录路径**：`sessions/drag-and-layout-optimization/`
- **进度文件**：`sessions/drag-and-layout-optimization/conductor/progress.md`
- **架构方案存放**：`sessions/drag-and-layout-optimization/conductor/tasks/task_001_architect/output.md`

---

## 9. 辩题概述

- **核心辩题**：如何为 NCdesktop 的多模态素材管理增加「拖拽外发到系统和外部应用」能力，并优化三栏布局中栏的文件呈现密度，使其成为真正的「一键抓取、一次性丢给 AI 工具」的操作中枢？
- **辩论偏好**：
  - 重点辩论层：全部（4 层完整辩论，因为涉及 UX + 技术可行性双重不确定性）
  - 最关心的维度：体验 > 性能 > 安全

---

## 10. 当前项目状态快照

### 已实现的文件管理功能
1. **双栏呈现**：左「导入原件」+ 右「工作区」，右栏显示 AI 标签、归类目录、文件信息
2. **筛选**：标签筛选、工作区子文件夹筛选
3. **选择**：框选（useRubberBandSelect）、Cmd+Click 多选、Cmd+A 全选
4. **批量操作**：BatchToolbar 支持移动/复制到其他项目、批量删除
5. **查看**：双击 / Space / Enter 打开 DocumentViewer 全屏阅读器
6. **Inspector**：右侧第三栏展示素材详情、AI 分析、标签、提取结果

### 当前拖拽能力
1. **拖入导入**：已完善，支持 dropzone 悬浮窗 + 主窗口拖入 → 复制到 NoteCaptWorkPlace → LLM 分类
2. **内部拖拽**：`useDragAssets` 设置 `application/notecapt-assets` MIME，仅应用内部识别
3. **时间轴关联**：`useKeyframeDrop` 使用 `application/x-asset-id`，与 useDragAssets 的 MIME 不一致（技术债）
4. **拖出到外部**：❌ 完全不支持。当前 dataTransfer 仅设置自定义 MIME，不含 `Files` 或 `text/uri-list`

### 当前 UI 布局痛点
1. 三栏布局中栏的列表项卡片太大（py-2.5, gap-2），每屏可视素材数量有限
2. 右侧工作区网格为 2 列（sm:3 列），缩略图 56×56px
3. 列表模式下信息行间距过宽，不利于「扫一眼选多个」的效率操作
