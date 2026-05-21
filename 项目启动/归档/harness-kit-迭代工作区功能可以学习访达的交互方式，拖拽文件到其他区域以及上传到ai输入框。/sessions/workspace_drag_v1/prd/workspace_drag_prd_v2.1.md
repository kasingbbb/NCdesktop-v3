# 产品需求文档（PRD v2.1）

# NCdesktop 工作区文件操作迭代
## Finder 式文件调度能力

**版本**：v2.1  
**日期**：2026-04-26  
**复杂度**：L  
**关联 Debate**：`debate/session_001/`

---

## 1. 背景与版本变更说明

| 版本 | 核心变化 |
|------|---------|
| v1.0 | 初始版本，提出 Accordion 父子嵌套展示，后发现与现有两栏布局冲突而废弃 |
| v2.0 | 修正为两栏布局不变；提出方案 A 约束（原件+转化件强制整体移动） |
| **v2.1** | 撤销方案 A 约束；修复 startDrag 失效 bug；确立独立操作语义 |

### 三处断裂（当前 → 目标）

| 断裂 | 当前状态 | 目标 |
|------|---------|------|
| **拖拽能力** | 右栏可拖到外部（但有 bug）；左栏完全无 drag 能力 | 左右栏均可拖到 Finder/外部应用，且 bug 修复 |
| **内部文件夹操作** | 两栏均无法移动到同项目内子文件夹 | 右键"移到文件夹"，独立操作各文件 |
| **文本提取** | 无从转化文件快速获取文本内容的路径 | BatchToolbar"复制文本"一键写入剪贴板 |

### 不变的设计约束（硬约束）

- **两栏布局不重构**：左栏（导入原件）+ 右栏（工作区）分区不变
- **`startDrag` 保留**：OS 级文件拖拽能力不退步
- **独立操作语义**：拖/移哪个文件就是哪个，无隐式连带行为

---

## 2. 用户与核心场景

**目标用户**：在 NCdesktop 积累了大量原件和 AI 转化产出的个人知识工作者。

| 场景 | 操作路径 | 优先级 |
|------|---------|--------|
| A1：把原件拖到 macOS Finder | 左栏选中 → 拖拽到 Finder 窗口/桌面 | P0（bug 修复后可用） |
| A2：把转化件拖到外部应用 | 右栏选中 → 拖拽（修复现有 bug） | P0（bug 修复） |
| B：把任意文件移到工作区子文件夹 | 选中 → 右键"移到文件夹 ▶" → 点击目标 | P0 |
| C：把转化文本发给网页版 AI | 右栏多选 → BatchToolbar"复制文本" → 粘贴 | P1 |

**明确不支持（MVP 范围外）**：
- 拖拽方式移到 app 内文件夹（P1 Spike 评估中）
- 外部 AI 对话集成
- 文件夹创建/重命名

---

## 3. 功能需求

### P0 — MVP

---

#### F-01：修复 `startDrag` 失效

**根因**：`useDragAssets.ts` 中 `makeDragProps` 设置了 `draggable: true`，触发 HTML5 Web DnD，接管鼠标事件，导致 `onMouseMove` 不再触发，`startDrag` 从未被调用。用户看到的影子是 Web DnD 元素幽灵，Finder 无法识别。

**修复**：从 `makeDragProps` 返回值中移除 `draggable: true as const`。

**影响文件**：`src/hooks/useDragAssets.ts`（单行改动）

**验收标准**：
1. 右栏拖拽 Markdown 文件到 Finder 下载文件夹，松手后文件被复制
2. 多选 2 个右栏文件后拖拽，2 个文件均被复制

---

#### F-02：左栏原件补全拖拽能力

**描述**：左栏 rawAssets 卡片增加 `{...makeDragProps(a.id)}`。

**影响文件**：`src/components/features/AssetListView.tsx`（左栏 list 视图 ~L460、grid 视图 ~L496）

**行为**：拖拽一个原件，只复制这一个原件文件，不涉及其转化文件。

**验收标准**：
1. 左栏拖拽 PDF 到桌面，桌面生成该 PDF 副本
2. 不触发关联 Markdown 转化文件的任何操作

---

#### F-03：左栏 Cmd+Click 多选

**描述**：左栏卡片 onClick 增加 Cmd+Click 判断（与右栏逻辑对齐）；左栏卡片增加 `multiSelected` 高亮样式。

**Cmd+A 焦点区分**：新增 `leftPaneFocused` 状态（`onMouseEnter`/`onMouseLeave`）：
- 焦点在左栏 → Cmd+A 全选 `rawAssets`
- 焦点在右栏 → Cmd+A 全选 `processedAssets`（现有行为）

**多选拖拽**：多选后拖拽，`startDrag` 携带所有选中文件的 `filePath`，全部复制到目标位置。

**影响文件**：`src/components/features/AssetListView.tsx`

**验收标准**：
1. 左栏 Cmd+Click 选 3 个原件后拖拽，3 个文件均被复制
2. 左栏焦点时 Cmd+A 全选左栏；右栏焦点时 Cmd+A 全选右栏

---

#### F-04：AssetContextMenu 右键上下文菜单

**新建文件**：`src/components/features/AssetContextMenu.tsx`

**左栏菜单项**：

| 菜单项 | 行为 |
|--------|------|
| 移到文件夹 ▶ | 展开二级子菜单（见 F-05） |
| 在 Finder 中显示 | `revealProjectWorkspaceFolder` |
| 删除 | confirm 后删除该文件（仅删除选中文件，不连带） |

**右栏菜单项**：

| 菜单项 | 行为 |
|--------|------|
| 移到文件夹 ▶ | 展开二级子菜单（v2.1：右栏也支持独立移动） |
| 复制文本内容（P1） | 读取文本写剪贴板 |
| 在 Finder 中显示 | `revealProjectWorkspaceFolder` |
| 删除 | confirm 后删除该文件 |

**多选行为**：若选中了多个文件（`selectedAssetIds`），右键任意一个后触发的操作对整个选中集合生效。

**验收标准**：
1. 右键左栏 PDF，出现"移到文件夹"选项
2. 右键右栏 Markdown，出现"移到文件夹"选项
3. 删除操作只删除选中文件，不触及其关联文件

---

#### F-05："移到文件夹"二级子菜单

**描述**：列出当前 Project 的 WorkspaceFolder 子目录，点击执行移动。

**数据来源**：`listProjectWorkspaceFolders(activeProjectId)`（与 WorkspaceFolderStrip 共用）

**行为规格**：
- 当前已在目标文件夹的项灰显
- 点击目标文件夹 → 调用 Rust 命令 `move_asset_to_workspace_folder`
- **独立移动**：只移动触发右键菜单的文件（或选中集合），不带动关联文件
- 完成后刷新 assetStore + workspaceFolders，显示 Toast

**验收标准**：
1. 右键 Markdown → 移到"政策研究"子文件夹 → 只有该 Markdown 移动，原件 PDF 留原处
2. 右键 PDF → 移到子文件夹 → 只有该 PDF 移动，Markdown 留原处
3. 多选 2 个文件右键 → 两个均移到目标文件夹

---

#### F-06：Rust `move_asset_to_workspace_folder` 命令

**文件**：`src-tauri/src/commands/asset.rs`（新增函数）

**接口**：
```rust
#[tauri::command]
pub fn move_asset_to_workspace_folder(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_relative_path: String,  // WorkspaceFolderEntry.relativePath 或 "__ROOT__"
    project_id: String,
) -> Result<(), String>
```

**实现规格**：
1. 获取 `workspace_root = workspace::project_workspace_dir(&project_id)?`
2. 对每个 `asset_id`：读取 DB 中 `file_path`，计算目标路径
   - `__ROOT__` → `workspace_root / filename`
   - 其他 → `workspace_root / target_relative_path / filename`
3. `canonicalize` 目标目录并验证在 `workspace_root` 内（防路径越界）
4. 如目标目录不存在，`fs::create_dir_all`
5. `fs::rename(old_path, new_path)`
6. 成功后更新 DB `file_path`
7. 任一步骤失败：回滚已移动文件（`fs::rename` 反向），返回 `Err`
8. **不查询、不移动任何关联文件**

**TS 包装**（加入 `src/lib/tauri-commands.ts`）：
```ts
export async function moveAssetToWorkspaceFolder(
  assetIds: string[],
  targetRelativePath: string,
  projectId: string
): Promise<void> {
  return invoke<void>("move_asset_to_workspace_folder", {
    assetIds,
    targetRelativePath,
    projectId,
  });
}
```

**验收标准**：
1. 调用后文件物理存在于目标目录，DB filePath 已更新
2. 中途磁盘失败，文件和 DB 均回滚，原位置文件完整
3. `target_relative_path = "../../../etc"` 等越界路径返回 Err

---

### P1 — 迭代一

---

#### F-07：BatchToolbar "复制文本"按钮

**描述**：BatchToolbar 新增"复制文本"按钮，作用于选中的 processedAssets。

**内容优先级**：Markdown 文件内容 → `analysis.ocrText` → `analysis.summary` → 提示"尚无文本内容"

**实现**：新增 Rust 命令 `read_asset_text_content(asset_ids: Vec<String>) -> Result<Vec<String>, String>`，前端调用后合并（`\n---\n` 分隔）写入 `navigator.clipboard.writeText`。

**验收标准**：
1. 右栏选中 2 个 Markdown，点"复制文本"，剪贴板包含两个文件完整文本，中间有分隔线

---

#### F-08：拖拽到 app 内文件夹 Spike（子方案 3A）

**描述**：评估延迟 startDrag 方案的可行性。

**Spike 目标**：
- 验证 Tauri webview 中，Web DnD 的窗口级 `dragleave` 是否可靠触发
- 验证 WorkspaceFolderStrip 横条作为 Web DnD drop target 是否正常工作
- 验证 startDrag 在 app 内落下时 DropzoneApp **不被**误触发

**Spike 通过条件**：三项均满足 → F-08 升级为 P1 正式功能  
**Spike 失败处理**：维持 F-05 右键菜单，不引入拖拽到文件夹

---

## 4. 非功能需求

| 类别 | 要求 |
|------|------|
| 性能 | 右键菜单打开 ≤ 100ms；move 命令 ≤ 800ms/文件；复制文本（≤2MB）≤ 300ms |
| 错误处理 | Rust 命令失败 → Toast 显示具体错误；磁盘回滚保证无孤儿文件 |
| 路径安全 | target_relative_path 必须 canonicalize 后验证在 workspace root 内 |
| 兼容性 | 两栏布局、WorkspaceFolderStrip 横条、现有 startDrag 行为不被修改 |

---

## 5. 技术约束

| 约束 | 内容 |
|------|------|
| `draggable: true` 移除 | F-01 的核心修复，影响 useDragAssets.ts |
| `startDrag` 保留 | 外部拖拽机制不替换 |
| 两栏布局不重构 | AssetListView 双栏结构不变 |
| 独立操作语义 | Rust 命令只移动入参文件，不追加关联文件 |
| 磁盘+DB 原子性 | 任一步失败全部回滚 |
| 路径约束 | 所有文件操作限制在 `~/Downloads/NoteCaptWorkPlace/` 内 |

---

## 6. 分期计划

| 阶段 | 功能 | 预估 Task 数 |
|-----|------|------------|
| **P0 MVP** | F-01（修复bug）+ F-02（左栏drag）+ F-03（左栏多选）+ F-04（右键菜单）+ F-05（移到文件夹）+ F-06（Rust命令） | 4–5 个 Task |
| **P1 迭代一** | F-07（复制文本）+ F-08（拖拽Spike） | 2–3 个 Task |
| **P2 后续** | 拖拽正式版（Spike通过）、左栏框选、文件夹创建 | TBD |

---

## 7. Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 关键约束 |
|------|--------|------------|---------|
| 修复 startDrag 失效 | P0 | 右栏拖文件到 Finder | useDragAssets.ts 移除 draggable:true，单行改动 |
| 左栏补全 makeDragProps | P0 | 原件拖到 Finder | list + grid 两种 viewMode 都要加 |
| 左栏 Cmd+Click 多选 + Cmd+A 焦点区分 | P0 | 多选后批量拖拽/移动 | selectedAssetIds 共享，焦点状态新增 |
| AssetContextMenu（左右栏均有"移到文件夹"） | P0 | 右键菜单操作入口 | 独立操作，左右栏均支持 |
| "移到文件夹"二级子菜单 | P0 | 任意文件移到工作区子目录 | 复用 listProjectWorkspaceFolders 数据 |
| Rust move 命令（无连带，原子回滚） | P0 | 磁盘+DB 原子移动 | 路径越界检查，失败回滚 |
| BatchToolbar"复制文本" | P1 | 转化文本发给网页版 AI | 需新增 read_asset_text_content 命令 |
| 拖拽到文件夹 Spike | P1 | 升级为拖拽交互的前提 | Spike 失败不影响 P0 |

### 不可妥协的技术底线

1. **`draggable: true` 必须移除**：P0 最高优先级 bug 修复
2. **独立操作语义**：显式选中什么就操作什么，Rust 命令不查询关联文件
3. **两栏布局不变**：AssetListView 双栏结构不重构
4. **Rust 命令原子性**：失败时磁盘和 DB 同时回滚
5. **路径安全**：target_relative_path 必须 canonicalize 后验证不越界

### 已识别高风险项

| 风险 | 来源 | 状态 | 缓解策略 |
|------|------|------|---------|
| startDrag 与内部 drop target 机制冲突 | Debate Layer 1 | P1 Spike 评估中 | MVP 用右键菜单绕开 |
| DropzoneApp 被 startDrag 误触发 | Debate Layer 1 | 已识别为硬约束 | 是放弃 startDrag 内部路径的根本原因 |
| selectedAssetIds 跨栏混用副作用 | Debate Layer 3 | 已解决 | 独立语义下无副作用，无需 confirm |

### MVP 边界声明

**做什么**：
- 修复 startDrag 失效（移除 draggable:true）
- 左栏原件拖到 Finder（补全 makeDragProps）
- 右键菜单"移到文件夹"（左右栏均支持，独立操作）
- 左栏 Cmd+Click 多选
- Rust 原子 move 命令（无连带逻辑）

**不做什么**：
- 拖拽到 app 内文件夹（P1 Spike）
- 外部 AI 对话集成（P2）
- 文件夹创建/重命名（P2）
- 两栏布局重构（硬约束禁止）

### Debate 中已搁置的争议

1. **P1 Spike 结果未知**：子方案 3A（延迟 startDrag）在 Tauri webview 中的可行性需代码验证，Spike 通过后 P1 才能升级为正式拖拽功能。
