# Debate 共识结论 — workspace_drag_v1

**最终版本**：v2.1（含 PM 追加的 bug 修复和独立操作语义）

---

## Layer 1 共识

| 结论 | 内容 |
|------|------|
| 问题定义 | 三处断裂：左栏无拖拽能力、两栏无内部文件夹移动、转化文本无快速提取路径 |
| 文件夹定义 | 同项目内 WorkspaceFolder 子目录（`~/Downloads/NoteCaptWorkPlace/<project_id>/...`） |
| 事件 B 路径 | MVP = 右键菜单；P1 = 拖拽 Spike（子方案 3A：延迟 startDrag） |
| 事件 C 定义 | BatchToolbar"复制文本"按钮，不建内部 AI 组件 |
| 操作语义 | 独立操作（v2.1 修正）：拖/移哪个文件就是哪个，无连带 |

## Layer 2 共识

| 结论 | 内容 |
|------|------|
| 左栏多选 | Cmd+Click，共享 selectedAssetIds；Cmd+A 按焦点区分左/右栏 |
| 右键菜单分工 | 左栏 + 右栏均有"移到文件夹"（v2.1，独立语义） |
| 横条保留 | WorkspaceFolderStrip 横条不废除，与右键菜单共享 uiStore 状态 |

## Layer 3 共识

| 结论 | 内容 |
|------|------|
| P0 Gap 清单 | 左栏 drag、左栏多选、AssetContextMenu、移到文件夹子菜单、Rust move 命令 |
| P1 Gap | 复制文本按钮、拖拽 Spike |
| startDrag bug | draggable:true 导致 Web DnD 截断 onMouseMove，startDrag 从未执行；修复：移除该属性 |

## Layer 4 共识

| 结论 | 内容 |
|------|------|
| MVP = P0 | F-01 bug修复 + F-02 左栏drag + F-03 左栏多选 + F-04 右键菜单 + F-05 移到文件夹 + F-06 Rust命令 |
| P1 | F-07 复制文本 + F-08 拖拽Spike |
| 不做 | 拖拽到内部文件夹（P1 Spike）、外部AI集成、文件夹创建、两栏重构 |

---

## 论证追踪表（最终状态）

| 论点 | 提出方 | 状态 | 备注 |
|------|--------|------|------|
| 左栏缺 makeDragProps | Proposer | ✅ 已验证 | P0 补全 |
| 事件 B 用右键菜单（MVP） | Reviewer 建议 | ✅ 已验证 | DropzoneApp 冲突是硬约束 |
| 拖拽到内部文件夹 = P1 Spike | 双方共识 | ✅ 已验证 | 目标：子方案 3A |
| 事件 C = BatchToolbar 复制文本 | Proposer | ✅ 已验证 | P1 |
| DropzoneApp 重复导入 bug 风险 | Reviewer | ✅ 已验证 | 是放弃 startDrag 路径的根本原因 |
| 方案 A 约束（连带移动） | Proposer（v2.0） | ❌ 已撤销 | PM v2.1 明确独立操作语义 |
| 右栏禁止"移到文件夹" | Reviewer（v2.0） | ❌ 已撤销 | 连带约束撤销后右栏也支持 |
| draggable:true 导致 startDrag 失效 | 代码观察 | ✅ 已验证 | 修复：移除该属性 |
| 独立操作语义（拖/移哪个是哪个） | PM 追加 | ✅ 已验证 | v2.1 核心约束 |
| Cmd+A 按焦点区分左/右栏 | Proposer | ✅ 已验证 | |
| 横条保留 + 共享 uiStore 状态 | Proposer | ✅ 已验证 | |
