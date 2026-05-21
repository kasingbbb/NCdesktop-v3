# UX Review Report — Round 2（task_011_ux_blocker_fix 之后）

## Round 2 重审结论

- task_010 BLOCKER #1（"查看原文件"入口缺失）: **已解除**
- task_010 BLOCKER #2（sourceMissing UI 0 消费）: **已解除**
- task_010 MAJOR #3（重试 loading + 防抖）: **已解除**
- task_010 MAJOR #4（非 done cursor: not-allowed + 徽章 title 前缀）: **已解除**
- task_010 MAJOR #5（toast dedupe 3s）: **已解除**
- task_010 MAJOR #6（rename Modal 替代 prompt + 字节计数 + sanitize + 失败 toast）: **已解除**
- task_010 MAJOR #7（键盘 Enter / F2 / Backspace / Delete）: **已解除**
- ESCALATE 解除: **是**

---

## 审查信息

- 审查时间：2026-05-13
- 审查方式：静态代码审查（沿 task_010 同方法；layout/* pre-existing 合并冲突仍未解，不启动应用）
- 项目类型：Desktop App（Tauri + React + Rust）— UX 优先级"高"
- 覆盖的核心旅程（PRD §2.2）：5 个核心旅程全部重扫
- 实际代码核对清单（全部已读取）：
  - `NCdesktop/src/components/features/AssetListView.tsx`
  - `NCdesktop/src/components/features/AssetContextMenu.tsx`
  - `NCdesktop/src/components/features/RenameAssetModal.tsx`（新建）
  - `NCdesktop/src/hooks/useDragAssets.ts`
  - `NCdesktop/src/stores/uiStore.ts`
  - `NCdesktop/src/lib/asset-state.tsx`
  - `NCdesktop/src-tauri/src/commands/source_view.rs`（新建）

---

## 启发式评估结果（Nielsen 10 项）— 轮次 2

| 启发式原则 | 检查要点 | R1 分 | R2 分 | 关键变化 |
|-----------|----------|-------|-------|----------|
| 系统状态可见性 | converting/failed/offline/retrying 反馈 | 4 | 4 | retrying 增加"重试中…" + disabled；converting 仍无进度（MINOR 仍存在） |
| 系统与真实世界匹配 | 中文文案、图标 | 5 | 5 | "原件丢失"、"无法拖出"、"原文件已不存在"等新文案准确直觉 |
| 用户控制与自由 | 撤销 / 取消 | 2 | 3.5 | Rename / Delete 改应用内 Modal 可点取消 / 点遮罩取消；删除仍无 undo（产品决定） |
| 一致性与标准 | 同操作一致表现 | 4 | 4 | rename 全路径统一应用内 Modal；**但右键删除仍 `window.confirm`**，与键盘 Delete 的中文 Modal 不一致（Code Reviewer 已 MINOR 标记） |
| 错误预防 | 出错前预防 | 3 | 4.5 | non-done 行 `cursor: not-allowed` + title「无法拖出」前置反馈到位；rename 实时字节计数 + sanitize 提示防错 |
| 识别而非回忆 | 关键操作可见 | 3 | 4 | source-missing 角标可见；"查看原文件"显式菜单项；键盘快捷键仍未在 UI 暴露提示（MINOR） |
| 灵活性与效率 | 快捷键 | 2 | 4 | Enter/F2 → rename；Backspace/Delete → 删除；Cmd+A 仍在；Modal 打开时 keydown 早返回防误触 |
| 美学与简约 | 视觉层次 | 4 | 4 | AlertTriangle 角标尺寸 10px、配色 amber 与既有徽章和谐 |
| 帮助用户诊断错误 | 错误信息友好 | 4 | 4 | rename 失败改 toast（dedupeKey="rename_asset:err"）；ioFailed.detail 仍未 sanitize（MINOR 8 未做） |
| 帮助与文档 | 引导 | 3 | 3 | 空态文案、快捷键提示仍未补（MINOR 9 未做） |

**启发式平均分**：R1 = 3.4 / 5 → **R2 = 4.0 / 5**

---

## 用户旅程扫描结果（R2）

### 旅程 1：批量导入（悬浮窗 → 5 条 MD）
- 路径与 R1 一致，未触及。
- 整体评分：4 / 5（未变）
- 摩擦点：converting 态无耗时指示（MINOR 8，沿用）

### 旅程 2：整理（rename / 打标签）
- 路径：选中 → 右键「重命名」**或** 单选按 Enter / F2 → 应用内 Modal → 字节计数（UTF-8）+ sanitize 实时提示 → 同名 / 空 / 含分隔符时按钮 disabled → 提交 → 失败 toast（dedupeKey）。
- 评分：R1 3 → **R2 4.5 / 5**
- 关键改善：`window.prompt` / `window.alert` 全数清除；中文友好；UTF-8 字节计数实时（中文"你好"=6 字节正确）。
- 残留摩擦：右键删除仍 `window.confirm`（见下方"新发现"）。

### 旅程 3：拖出消费（多选 done → ChatGPT）
- 路径：hover non-done → cursor `not-allowed` + title「无法拖出：当前状态非 done」前置警告；多次失败同 errorKind → toast 3s 窗口合并为 1 条。
- 评分：R1 4 → **R2 4.5 / 5**
- 关键改善：AC-2 / AC-5 全部到位；用户在拖之前即可见拒绝信号。

### 旅程 4：失败 / 离线降级
- 路径：failed 徽章 → 点重试 → 按钮即刻进入"重试中…" + disabled（1s 防抖窗口） → 完成后 onRetry 触发 fetchAssets。
- 评分：R1 4 → **R2 4.5 / 5**
- 关键改善：连点不再重复触发 IPC；视觉抖动消除（AC-3 达成）。

### 旅程 5：回看源（本次重点）
- 路径：右键资产 → 菜单内「查看原文件」 → 调 `reveal_source_file(source_path)` → macOS `open -R` 在 Finder 高亮；`sourceMissing === true` 时菜单项 disabled + 文案改为「原文件已不存在」，列表行同时显示 amber「原件丢失」徽章（≥ 2 处可感）。
- 评分：R1 1 → **R2 5 / 5**
- 关键改善：
  - PRD §2.2 场景 5 在 UI 层完整闭环；session_context §3 不可妥协底线 #5 已兑现。
  - source-missing 信号 UI 双触点（行内角标 + 菜单项 disabled 文案改），满足 BLOCKER #2 "≥ 2 处可感"。
  - 失败路径加 toast（dedupeKey="reveal_source:err"），不会与 task_011 后端错误产生堆积。
- 残留摩擦（MINOR，不阻断）：`reveal_source_file` 仅 macOS（与既有 `reveal_project_workspace_folder` 一致，PRD 限定 macOS DMG）；前端无白名单（Reviewer 已标 MINOR）。

---

## 技术性 UX 检查结果（R2 增量）

- [x] 表单提交 loading：rename Modal busy / delete Modal busy / retry 按钮 isRetrying 全数 disabled + 中文文案
- [x] 成功反馈：renameAsset / deleteAsset 成功后自然刷新；info toast 与既有路径一致
- [x] 键盘可操作：Cmd+A / Enter / F2 / Backspace / Delete / Esc 全数覆盖；INPUT/TEXTAREA 内不拦截；Modal 打开时早返回
- [x] 关键操作确认：键盘 Delete 走中文 Modal（×️ 右键删除仍 `window.confirm`）
- [x] 错误信息：rename 失败改 toast，不再 `window.alert`；reveal_source 失败有中文 toast
- [x] ARIA：rename Modal 有 `role="dialog" aria-modal="true" aria-label="重命名资产"`；删除 Modal 同等
- [x] data-* 测试钩子：`data-source-missing` / `data-cursor` / `data-retrying` / `data-disabled` / `data-testid` 完备

---

## 发现的问题（R2）

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **右键菜单删除路径仍使用 `window.confirm`，与键盘 Delete 的中文 Modal 不一致**
   - 位置：`AssetContextMenu.tsx:174`（`handleDelete` 内）
   - 影响旅程：旅程 2 整理
   - 触感：键盘 Backspace/Delete 触发应用内中文 Modal；右键菜单"删除"触发浏览器原生 confirm，两个媒介风格不一致。
   - UX 视角判定：**保持 MINOR，不升级到 MAJOR**。理由：核心流程可完成、文案中文、双重确认即使风格不同也不构成"无法完成任务"或"严重困惑"；启发式 #4 一致性扣 0.5 分已反映在 R2 评分中。
   - 建议修复方向：把 `AssetContextMenu` 删除按钮改为父级回调 `onRequestDelete?.(targetIds)`，复用 AssetListView 已有的删除 Modal（与 rename 同模式）。

2. **converting 态无耗时指示**（沿用 task_010 MINOR 8）
   - 位置：`AssetStateBadge`
   - 影响旅程：旅程 1
   - 建议：title 加"已等待 Ns"；P1 详情面板时联动。

3. **空态文案未引导悬浮窗 / 快捷键**（沿用 task_010 MINOR 9）

4. **`ioFailed.detail` 可能透传 Rust 错误细节**（沿用 task_010 MINOR 10）

5. **`reveal_source_file` 不限白名单**（Code Reviewer 已标）
   - 位置：`src-tauri/src/commands/source_view.rs`
   - 当前仅校验 trim 非空 + exists。本地 IPC 风险低；若未来开放给非 webview 通道，需要白名单校验（路径属于 DB 中 Asset.sourceData 集合）。

### 关于"多选场景 Enter / Backspace 行为是否合理"

- **Enter / F2**：实现限定 `ids.length === 1` 时才弹 rename Modal；多选时按 Enter 无任何反应。**合理**（与右键菜单"多选不可用"灰显的语义一致）。
- **Backspace / Delete**：`ids.length > 0` 即弹中文确认 Modal，文案明确"删除选中的 N 个文件？此操作不可撤销"。**合理**，与右键菜单"删除 N 个文件"语义一致。
- **唯一遗憾**：键盘 Delete 走中文 Modal、右键删除走 `window.confirm` 的不一致（已在 MINOR 1 标注）。

### 关于"rename Modal 字节计数边界"

- 验证规则：`utf8ByteLength(trimmed) > MAX_NAME_BYTES`（即 > 200，**= 200 允许**）。
- 输入框红边 + 字节计数红色：`byteLen > MAX_NAME_BYTES`。
- "确认"按钮 disabled：`!validation.ok || sameAsInitial || busy`。
- **边界正确**：200B 允许，201B 起 disabled + 中文错误"名称超过 200 字节上限"。

---

## 总体评分

- R1：3.2 / 5
- **R2：4.4 / 5**

加权要点：
- 功能正确性（AC-1~AC-9 全数兑现，边界齐全） + 0.6
- 用户体验（启发式平均 3.4 → 4.0） + 0.6
- 残留 5 个 MINOR 中 4 个为 R1 已存在的接受现状项；新增 1 个右键删除路径一致性问题 −0.0（MINOR 级权重低）

---

## 最终判断

- [x] **PASS（可以进入验收）**
- [ ] ESCALATE

**ESCALATE 解除理由**：

1. **PRD §2.2 场景 5"回看源"完整落地**：`AssetContextMenu` 新增「查看原文件」菜单项，调用新 Tauri 命令 `reveal_source_file`（macOS `open -R` 路径存在校验），并按 `sourceMissing` 切换 disabled + 中文文案。session_context §3 不可妥协底线 #5（"源文件可访问可恢复"）真正兑现到用户面前。
2. **source-missing 信号 UI ≥ 2 处可感**：列表行 amber AlertTriangle「原件丢失」徽章 + 菜单项 disabled 文案改 + 行 li 上 `data-source-missing` 测试钩子，覆盖三个表面。
3. **5 个 MAJOR 全数解除**：重试 1s 防抖 + isRetrying disabled；non-done `cursor: not-allowed` + title 前缀；uiStore dedupeKey 3s 滑动窗口；rename 应用内 Modal（UTF-8 字节计数 + sanitize 提示 + 同名禁用 + 失败 toast）；键盘 Enter / F2 / Backspace / Delete 全数绑定。
4. **未引入新 BLOCKER / MAJOR**：仅 1 个新 MINOR（右键删除路径一致性），UX 视角不升级；rename 字节边界、多选键盘行为均合理。
5. **验收维度**：tsc 0 / cargo 通过 / 本 task 新增 31 个测试 PASS / pre-existing 42 fail 与本 task 文件零交集。

建议在 P1 体验打磨阶段一并处理 5 个 MINOR（右键删除统一为中文 Modal、converting 耗时提示、空态文案补悬浮窗引导、ioFailed.detail sanitize、reveal_source_file 白名单）。
