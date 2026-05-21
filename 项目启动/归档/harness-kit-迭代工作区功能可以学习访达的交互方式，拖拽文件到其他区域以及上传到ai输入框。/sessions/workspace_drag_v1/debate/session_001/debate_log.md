# Debate 记录 — workspace_drag_v1

**日期**：2026-04-26  
**复杂度**：L（4 层完整 Debate）  
**版本迭代**：v1（问题定义）→ v2（修正两栏约束）→ v2.1（修复 startDrag bug + 撤销连带约束）

---

## 前置约束（进入 Layer 1 前 PM 确认）

| 约束 | 来源 |
|------|------|
| 两栏布局（原件左栏 + 工作区右栏）不改变 | 用户明确要求 |
| `startDrag` 现有能力不退步（右栏拖到外部已工作） | 代码观察 + 用户确认 |
| 原件左栏目前无 drag 能力（gap） | 代码观察：makeDragProps 只挂载在右栏 |
| 用户操作独立语义：拖什么就是什么，无连带 | 用户明确要求（v2.1） |

---

## Layer 1：问题定义

### Round 1

**Proposer 主张**：用户目标拆解为三件事：
- 事件 A：拖到外部（Finder/外部应用）— 右栏已有，左栏缺失
- 事件 B：拖到 app 内文件夹 — 两栏均缺失，存在 startDrag 与 Web DnD 技术冲突
- 事件 C：文本提取发给外部 AI — 当前右栏拖出是文件路径，网页版 AI 需要文本内容

**Reviewer 挑战（L3）**：事件 C 中"拖到外部 AI"和"app 内 AI 入口"混淆，价值差距未定义。startDrag 已可把文件拖到 Claude.app，内部 AI 入口的增量价值是什么？

### Round 2

**Proposer 修正**：事件 C 重新定义为"快速提取转化文本到剪贴板"（而非内部 AI 组件），实现为 BatchToolbar 按钮，绕开拖拽冲突。

**Reviewer 挑战（L2）**：事件 B 的技术冲突需要明确路径：
- 路径 1：放弃拖拽，改 BatchToolbar UX
- 路径 2：放弃 startDrag（违反约束，禁止）
- 路径 3：双模式拖拽（Web DnD 内部 + startDrag 外部）

### Round 3

**Proposer 提出路径 3 两个子方案**：
- 3A：延迟 startDrag — 先走 Web DnD，窗口级 dragleave 时触发 startDrag
- 3B：坐标检测 fallback — startDrag 调用后通过 mousemove 坐标检测内部落点

**Reviewer 挑战（L2）**：3B 有确定性 bug 风险：项目存在 DropzoneApp 组件，startDrag 在 app 内落下会触发重复导入。

### Round 4（裁定）

**Host 强制决策**：  
- **事件 B MVP = 右键菜单"移到文件夹"**（绕开所有拖拽冲突）  
- 路径 3A 列为 P1 Spike，Spike 通过后升级为拖拽交互

**Layer 1 最终共识**：
1. 事件 A：左栏补全 makeDragProps（P0）
2. 事件 B：右键菜单 MVP，拖拽路径为 P1 Spike（P0/P1）
3. 事件 C：BatchToolbar"复制文本"按钮（P1）
4. 两栏布局不变，startDrag 保留

---

## Layer 2：理想态

### Round 5

**Proposer 描述理想工作流**：
- 左栏多选 → 拖到 Finder（startDrag 补全）
- 左栏/右栏右键 → 移到文件夹子菜单
- BatchToolbar 多选转化件 → 复制文本 → 粘贴到网页版 AI

**技术模型**：新增 AssetContextMenu 组件、WorkspaceFolderPanel（后改为维持横条）

### Round 6

**Reviewer 挑战（L2）**：方案 A 约束（原件+转化件强制整体移动）在两栏架构下产生隐式副作用 — 用户在右栏操作转化件，系统悄悄移动左栏原件，违反透明度原则。

**Proposer 修正**：右栏禁止直接"移到文件夹"，移动始终以左栏原件为锚点。

> **注**：此约束在 v2.1 中由于 PM 明确"独立操作语义"而被完全撤销。

**Reviewer 挑战（L2）**：左栏多选策略未定义。

**Proposer**：Cmd+Click 多选，共享 selectedAssetIds，Cmd+A 按焦点区分作用域。

---

## Layer 3：差距分析

### Round 7

**Proposer Gap 清单**：

| Gap | 类型 | MVP？ |
|----|------|------|
| 左栏缺 makeDragProps | UI/交互 | P0 |
| 左栏缺 Cmd+Click 多选 | 交互/状态 | P0 |
| Cmd+A 未按焦点区分 | 状态 | P0 |
| AssetContextMenu 组件缺失 | UI | P0 |
| "移到文件夹"二级子菜单缺失 | UI | P0 |
| Rust move_asset_to_workspace_folder 命令缺失 | 后端 | P0 |
| BatchToolbar"复制文本"缺失 | UI | P1 |

**Reviewer 挑战（L2）**：selectedAssetIds 混入左栏原件后，BatchToolbar 现有"移到项目"会对原件生效，可能破坏 sourceAssetId 关联关系。

**Proposer 解决**：BatchToolbar 含原件时显示副作用 confirm，说明将同时操作 N 个关联转化文件。

---

## Layer 4：策略

### Round 8

**Proposer MVP 清单**：
- F-01：左栏 makeDragProps 补全
- F-02：左栏 Cmd+Click 多选 + Cmd+A 焦点区分
- F-03：AssetContextMenu（左/右栏语义分工）
- F-04："移到文件夹"二级子菜单
- F-05：Rust move_asset_to_workspace_folder（方案 A，原子回滚）
- F-06：BatchToolbar 副作用 confirm

**Reviewer 挑战（L2）**：WorkspaceFolderStrip（横条）与新面板的共存状态同步问题。

**Proposer**：横条保留，共享 uiStore.workspaceFolderRelativePath 单一状态源，横条退化为辅助控件。

**Reviewer 挑战（L2）**：事件 C（复制文本）被放到 P1 的理由需明确。

**Proposer**：BatchToolbar 复制文本需新增 read_asset_text_content Rust 命令，为减少单次后端变更范围放 P1；如 PM 认为高优先可提升到 P0。

---

## v2.1 关键修正（PM 追加需求）

### 新增 Bug 报告
用户反馈：右栏拖拽到 Finder 下载文件夹，有拖拽影子但松手后文件未被复制。

**根因分析**（通过代码观察）：
`makeDragProps` 同时设置了 `draggable: true`（Web DnD）和 `onMouseMove → startDrag`。
浏览器在 mousemove 1px 后启动 Web DnD，接管鼠标事件，导致 `onMouseMove` 不再触发，`startDrag` 从未被调用。
用户看到的是 Web DnD 的元素幽灵，落到 Finder 时 dataTransfer 只有 app 内部 JSON，Finder 不认。

**修复**：移除 `draggable: true as const`（一行改动）。

### 撤销方案 A 约束
PM 明确：用户操作哪个文件就移动哪个文件，不存在连带行为。
- 右栏也支持独立"移到文件夹"
- Rust 命令不查询关联文件，只移动入参 asset_ids
- BatchToolbar 副作用 confirm 简化（无连带则无副作用警告）
