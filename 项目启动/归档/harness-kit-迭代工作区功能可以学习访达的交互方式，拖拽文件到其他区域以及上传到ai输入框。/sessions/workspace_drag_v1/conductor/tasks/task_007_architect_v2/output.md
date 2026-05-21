# 技术方案 v2（补充迭代）

> 基于对 `项目启动/NCdesktop` 当前实现的 review，本架构补充修订仅针对 v2.1 PRD 中的"未完成 / 虚假完成"项。原 task_001_architect output.md 中 P0 主路径不变，本文件只补 delta。

## 项目概述

P0 MVP 已 SHIPPED 并打包，但 code review 暴露 3 个缺口：
1. F-03 Cmd+A 焦点分流：状态变量 `leftPaneFocused` 已埋点，但 keydown handler 未消费（全选源固定为 `displayAssets`）。
2. F-04 移动成功 Toast：`AssetContextMenu.handleMoveToFolder` 仅 `onMoved()`，未触达 `useUIStore.addNotification`。
3. F-07/F-08 整段 P1：复制文本（Rust 命令 + 入口）和 app 内拖拽 Spike 均未启动。

## 技术选型（继承）

- 前端：React + Zustand + Tauri webview（不变）
- 后端：Rust + tauri command + SQLite（不变）
- 剪贴板：`navigator.clipboard.writeText`（v2.1 PRD §F-07 已定）
- 通知：`useUIStore.addNotification`（项目内既有规范，BatchToolbar 已是消费方）

## ADR 增量

### ADR-007: Cmd+A 焦点分流采用 leftPaneFocused 状态读取，不重构 selectAll

- **状态**：已接受
- **上下文**：progress.md 标 task_004 DONE 但实际 handler 没分流，属于历史遗留。
- **决策**：在现有 keydown handler 内基于 `leftPaneFocused` 在 `rawAssets` / `processedAssets` 间分流；不引入新 store 字段。
- **被排除**：在 store 中维护 focusedPane（过度设计，仅 Cmd+A 一处消费）。
- **后果**：handler 依赖 `rawAssets`/`processedAssets`/`leftPaneFocused` 三个 ref，注意 deps 数组完整。

### ADR-008: 移动成功 Toast 复用 useUIStore.addNotification

- **状态**：已接受
- **上下文**：项目内通知统一走 `useUIStore`，BatchToolbar 已是消费方，无需新组件。
- **决策**：`AssetContextMenu.handleMoveToFolder` 成功路径调 `addNotification({type:'success', ...})`，失败路径调 `type:'error'`。
- **后果**：组件需注入 `addNotification` 或在内部 `useUIStore.getState()`。

### ADR-009: read_asset_text_content 内容优先级

- **状态**：已接受
- **上下文**：PRD §F-07 列了优先级链 — Markdown 文件全文 → `analysis.ocrText` → `analysis.summary` → 空提示。
- **决策**：Rust 命令按上述顺序读取，每个 asset 独立判定，返回 `Vec<String>`（与 asset_ids 同长度同序）。前端用 `\n---\n` 拼接后 `clipboard.writeText`。
- **被排除**：返回 `Option<String>` 由前端处理空值（前端逻辑会变重）。
- **后果**：Rust 端需要可访问 DB 中 `analysis` 字段；若 Markdown 文件读失败则回退到下一级，不抛错。

### ADR-010: F-08 Spike 走只读探针，不写实际落点逻辑

- **状态**：已接受
- **上下文**：PRD §F-08 列了 3 个 Spike 通过条件（webview dragleave / drop target 命中 / DropzoneApp 不误触）。
- **决策**：Spike Task 只写 instrumentation（console.log + 临时事件监听），跑通后写 Spike 报告，**不修改 startDrag 真实路径**。Spike 通过才发起独立 Task 做正式实现。
- **后果**：Spike Task 产出物是一份 `spike_report.md`，不是代码。

## 系统架构（增量）

```
AssetListView
  ├── keydown handler  ── 读 leftPaneFocused → 在 rawAssets/processedAssets 分流（task_008）
  └── AssetContextMenu ── handleMoveToFolder 成功后 addNotification (task_009)
                       └── 右键菜单新增"复制文本"项（右栏 only，task_011）

src-tauri/src/commands/asset.rs
  └── 新增 read_asset_text_content(asset_ids) -> Vec<String>  (task_010)

src/components/features/assets/BatchToolbar.tsx
  └── 新增"复制文本"按钮（task_011）

Spike 探针（task_012）
  └── 临时注入 window.addEventListener('dragleave'/'drop', ...) 验证 PRD §F-08 三条件
```

## 数据 / API 设计

### read_asset_text_content（新增 Rust 命令）

```rust
#[tauri::command]
pub fn read_asset_text_content(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
) -> Result<Vec<String>, String>
```

- 输入：asset_ids（按用户选择顺序）
- 输出：与输入等长的字符串数组；空内容用空串占位
- 优先级（每个 asset 独立判定）：
  1. 若 `file_path` 后缀为 `.md` 且可读 → 返回文件全文
  2. 否则若 DB `analysis.ocrText` 非空 → 返回 ocrText
  3. 否则若 DB `analysis.summary` 非空 → 返回 summary
  4. 否则返回 `""`

### TS 包装

```ts
// src/lib/tauri-commands.ts
export async function readAssetTextContent(assetIds: string[]): Promise<string[]> {
  return invoke<string[]>("read_asset_text_content", { assetIds });
}
```

## 目录结构（不新增模块）

仅修改：
- `src/hooks` / `src/components/features` 内既有文件
- `src-tauri/src/commands/asset.rs`（追加函数）+ `src-tauri/src/lib.rs`（注册）

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| F-03 实际已部分生效，重做引入回归 | 中 | 中 | task_008 先 verify，再决定改不改 |
| read_asset_text_content 大文件阻塞 | 低 | 中 | PRD §4 已定 ≤2MB / ≤300ms；首版同步读，超阈值再异步 |
| Markdown 文件已被移动 / 丢失 | 低 | 低 | 文件读失败 → 回退优先级链，不向用户报错（行为已在 ADR-009） |
| Spike 三条件中任一不满足 | 中 | 低 | Spike 失败已在 PRD §F-08 兜底（维持右键菜单） |

## Task 清单

```
- [ ] task_008_verify_cmda_focus     — 验证并修复 Cmd+A 焦点分流
- [ ] task_009_move_toast            — 移动成功/失败 Toast 反馈
- [ ] task_010_rust_read_text        — Rust read_asset_text_content + TS 包装
- [ ] task_011_copy_text_ui          — BatchToolbar/右键菜单"复制文本"入口
- [ ] task_012_drag_spike            — F-08 Spike 探针 + 报告
```

## Task 依赖拓扑

```
task_008  (独立)
task_009  (独立)
task_010 → task_011
task_012  (独立, Spike)

可并行：{task_008, task_009, task_010, task_012}
任意 P0 task 完成后即可单独发布，无需等齐
```

## Task 粒度自检

| Task | 单一目标 | 可独立测试 | 规模 | 依赖清晰 | AC 可验证 |
|------|---------|-----------|------|---------|----------|
| 008 | ✅ Cmd+A 分流 | ✅ 手动 + 单测 | <50 行 | 无 | ✅ |
| 009 | ✅ Toast | ✅ 手动观察 | <20 行 | 无 | ✅ |
| 010 | ✅ Rust 命令 | ✅ Rust 单测 | <100 行 | 无 | ✅ |
| 011 | ✅ UI 入口 | ✅ 手动 + e2e | <80 行 | task_010 | ✅ |
| 012 | ✅ Spike | ⚠️ 探针 | <30 行 instrumentation | 无 | ⚠️ 报告型 |
