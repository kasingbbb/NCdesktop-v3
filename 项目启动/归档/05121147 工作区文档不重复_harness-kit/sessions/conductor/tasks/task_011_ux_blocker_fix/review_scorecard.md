# Review Scorecard — task_011_ux_blocker_fix

## 审查思考过程

### 1. Task 意图复述
修复 task_010 UX 评审的 2 BLOCKER（"查看原文件"入口缺失 / `sourceMissing` UI 0 消费）+ 5 MAJOR（重试 loading / 非 done cursor / toast dedupe / rename Modal / 键盘 Enter+Backspace+F2），把 P0 M7 后端 source-missing 信号与 PRD §2.2 场景 5 真正落到 UI。

### 2. AC 检查结果

| AC | 内容 | 验证 |
|----|------|------|
| AC-1 | 「查看原文件」菜单项 + sourceMissing disabled + 文案改 | ✅ `AssetContextMenu.tsx:147-167, 373-398` + `source_view.rs` + wrapper `tauri-commands.ts:85`；3 个测试 case 覆盖 |
| AC-2 | sourceMissing 角标 + `data-source-missing` | ✅ `AssetListView.tsx:673, 681, 710-724`（AlertTriangle 角标 + title 提示） |
| AC-3 | 重试 loading + 1s 防抖 | ✅ `asset-state.tsx:87-110`（`isRetrying` + `lastClickRef` + `data-retrying` + `disabled` + 文案）；测试断言 disabled / data-retrying / 单次调用 |
| AC-4 | 非 done cursor: not-allowed + title | ✅ `AssetListView.tsx:688-701`（cursor inline + `data-cursor` + title 「无法拖出」）；it.each 覆盖 converting/failed/offline |
| AC-5 | uiStore dedupeKey 3s 滑动窗口 | ✅ `uiStore.ts:128-220`（`dedupeLastSeen` Map + 替换语义）；`useDragAssets.ts:109-115` 使用 `outbound:<kind>`；测试断言连续 3 次 → notifs.length === 1 |
| AC-6 | Rename Modal（字节计数 UTF-8 + sanitize + 失败 toast） | ✅ `RenameAssetModal.tsx` + `AssetListView.tsx:372-393` 提交+失败 toast (dedupeKey `rename_asset:err`)；9 个测试覆盖（含「你好」=6 字节 / 201>200 / 路径分隔符 / 空 / busy） |
| AC-7 | Enter/F2 → rename, Backspace/Delete → 删除确认 | ✅ `AssetListView.tsx:327-369`（仅单选 Enter/F2 触发 rename，N 选 Backspace/Delete 弹中文确认 Modal）；Modal 打开时 keydown 早返回；INPUT/TEXTAREA 内不拦截 |
| AC-8 | 中文 + 4 测试文件覆盖 | ✅ 用户可见文案均中文，console.error 才是英文；31/31 PASS |
| AC-9 | tsc 0 / cargo / vitest（不含 pre-existing） | ✅ |

### 3. 关键发现
1. **pre-existing 失败零交集已验证**：`git status` 显示 task_010 引述的 42 fail 全部命中 `layout/Sidebar*.tsx`、`Toolbar.tsx`、`Inspector.tsx`（标 UU 合并冲突）以及 `TagTree.test.tsx` / `SettingsPanel.test.tsx`（学习模式重构 M）—— 与本 task 修改的 `AssetListView/AssetContextMenu/RenameAssetModal/useDragAssets/asset-state/uiStore/source_view.rs` **文件名零重合**。
2. **`AssetContextMenu.handleDelete` 仍用 `window.confirm`**：键盘 Backspace 路径走新的中文 Modal，右键删除仍走 `window.confirm`（line 174）。task_011 input.md 未显式要求改右键删除，但 ux_review.md 启发式 4「一致性」指出原生对话框风格不符；属轻度遗漏。
3. **`reveal_source_file` 跨平台仅 macOS**：`#[cfg(target_os = "macos")]` + 非 macOS 返回中文错误「当前平台不支持在文件管理器中打开」。与既有 `reveal_project_workspace_folder` 一致，符合 PRD §3.1 macOS DMG 限定；路径仅校验非空 + exists。后端不限制白名单（source 可能在任意路径），输出已是 `Result<(), String>`，shell 注入风险被 `Command::arg` 单参数化挡住。

---

## 评分

权重来自 session_context §4（功能正确性 25 / 用户体验 25 / 架构一致性 20 / 代码质量 10 / 测试覆盖 10 / 可维护性 10）。

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 25% | 5 | AC-1~AC-9 全数满足；边界（空 source/UTF-8 中文字节/防抖/dedupe 合并/disabled 切换）均有断言；rename 同名提交禁用、busy 下双 Modal 状态阻止重入处理得当 |
| 用户体验 | 25% | 4.5 | source-missing 在 UI 2 处可感（角标 + 菜单项 disabled 文案改）；非 done cursor 反馈到位；rename Modal 字节计数实时、sanitize 提示明确；唯一遗憾：右键删除仍 `window.confirm`，与键盘路径风格不一致 |
| 架构一致性 | 20% | 5 | 不绕 `tauri-commands.ts`；新命令对齐既有 `reveal_project_workspace_folder` 命名/实现；仅在 `Notification` 加可选 `dedupeKey`，未改 `WorkspaceAssetView`；组件继续消费已有字段；命令注册在 `commands/mod.rs:20` + `lib.rs:193`，干净 |
| 代码质量 | 10% | 4.5 | RenameAssetModal validator 单一职责、`__test__` 命名空间暴露 helper；AssetListView 内联删除 Modal 代码量略大（~70 行）但与 WorkspaceFolderListView 视觉一致；keydown handler 依赖列表完备 |
| 测试覆盖 | 10% | 4 | 31 条 case 全 PASS；AC-2/4/5 行属性以 `FixtureRow` 复刻而非整页集成（已明示限制）；AC-7 键盘未直接 unit-test（依赖 openRenameModal/openDeleteModal 回调间接覆盖）—— Dev 已在 known limits 标注，可接受 |
| 可维护性 | 10% | 4 | `dedupeLastSeen` 是模块级 Map（Dev 已提示 store reset 不清，PRD 当前无此路径，合理）；Rename Modal 行为独立、易于复用；删除 Modal 内联未抽公共组件（Dev 标注未来可统一） |

**综合分**：5×0.25 + 4.5×0.25 + 5×0.20 + 4.5×0.10 + 4×0.10 + 4×0.10 = **4.68 / 5**

---

## 总体判断

- [x] **PASS**

判定理由：2 个 BLOCKER 完全闭环（场景 5 回看源真正可触达 + sourceMissing 信号 UI 2 处可感）；5 个 MAJOR 全数兑现；测试 31/31 PASS、tsc 0、cargo 通过；pre-existing 42 fail 经 `git status` 文件名级核对与本 task 修改文件零交集；架构无偏差。

---

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR（可选修复，不影响 PASS）

1. **`AssetContextMenu.handleDelete` 仍使用 `window.confirm`**
   - 代码位置：`NCdesktop/src/components/features/AssetContextMenu.tsx:174`
   - 现象：键盘 Backspace/Delete 已走中文 Modal，右键菜单删除仍弹原生 `window.confirm`，两路径反馈媒介不一致（呼应 ux_review.md 启发式 #4）。
   - 修复方向：把右键 onClick 也改为调父级 `onRequestDelete?.(targetIds)` 回调，复用 `AssetListView` 已有的删除确认 Modal。
   - 验证标准：右键 → 删除 → 弹应用内中文 Modal（与键盘 Delete 同样路径）。

2. **`dedupeLastSeen` 是模块级 Map，单测之间需手动清理**
   - 代码位置：`NCdesktop/src/stores/uiStore.ts:135`
   - 现象：跨 test 文件如果两组测试都使用同一 dedupeKey 且时间窗口重叠，可能互相干扰；当前 vitest 隔离文件级模块，影响有限，但若未来引入 watch mode 串联，需提供 reset helper。
   - 修复方向：导出 `__resetDedupeForTests()`，或在 store reset 路径里 `dedupeLastSeen.clear()`。
   - 验证标准：测试 helper 调用一次即可消除残余 key。

3. **`reveal_source_file` 不限白名单**
   - 代码位置：`NCdesktop/src-tauri/src/commands/source_view.rs:18-41`
   - 现象：source_path 仅校验非空 + exists，理论上前端可传任意磁盘路径。当前 IPC 仅本地、无外部入口，风险低；若未来 IPC 暴露给 webview 之外通道，需要白名单防"打开任意敏感路径"。
   - 修复方向：可加可选「路径必须属于已知 Asset.sourceData 集合」校验（需 DB 查询）。
   - 验证标准：传入 DB 中不存在的 sourceData 路径 → 拒绝。

---

## 给 Dev 的修复指引

PASS，无强制修复。MINOR 项可在下一迭代或 P1 体验打磨阶段一并处理。

## 自检清单
- [x] 逐条 AC 核对
- [x] 领域审查重点（PRD §2.2 场景 5、不可妥协底线 #5）已验证
- [x] pre-existing 失败与本 task 文件名级零交集已用 `git status` 验证
- [x] 评分诚实（5/5 仅在 AC 全数无遗漏 + 边界齐全时给）
