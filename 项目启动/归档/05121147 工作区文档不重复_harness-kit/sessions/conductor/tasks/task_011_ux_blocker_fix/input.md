# Task 输入 — task_011_ux_blocker_fix

> 由 Conductor 应对 task_010 UX 评审 ESCALATE 创建。范围：修 2 BLOCKER + 5 MAJOR。

## 目标

修复 task_010 ux_review.md 列出的 2 个 BLOCKER 与 5 个 MAJOR，使前端真正承接 P0 M7 的 source-missing 信号与 PRD §2.2 场景 5"查看原文件"旅程，并打磨重试 / 拖拽 / 键盘 / rename Modal 等高频交互。

## 前置条件

- 依赖 task：task_002–009 + FIX_001 全部完成（后端 source_missing 已 wire；前端 WorkspaceAssetView 已切流）
- 必须先存在：
  - `WorkspaceAssetView.sourceMissing: boolean`（task_003 DTO）
  - `revealItemInDir` 或等价 Tauri opener（grep 仓库已有；如无，新增最小命令包装 `open::that(source_path)`）
  - `uiStore.openModal` / `uiStore.addNotification`（既有体系）

## 验收标准（AC）

### BLOCKER 修复（必须）

1. **AC-1（BLOCKER #1：查看原文件入口）**：
   - `AssetContextMenu` 新增菜单项「查看原文件」：
     - `sourceMissing === false` → enabled → 点击调 `revealItemInDir(sourcePath)`（或 Tauri opener 等价命令）
     - `sourceMissing === true` → disabled + 文案改为「原文件已不存在」
   - 文案统一中文；菜单项位置建议放在「在 Finder 中显示」之上或之下，保持视觉相邻。

2. **AC-2（BLOCKER #2：source-missing 列表角标）**：
   - `AssetListView` 行渲染时检查 `asset.sourceMissing`，在文件名右侧 / 状态徽章旁渲染 `⚠ 原件丢失` 徽章（lucide `AlertTriangle` + 黄/橙色），徽章本身 title 提示「源文件不在原位置，rendition 仍可拖出」。
   - 行 `data-source-missing={sourceMissing}` 用于测试。

### MAJOR 修复（强烈建议）

3. **AC-3（MAJOR #3：重试按钮 loading）**：
   - `AssetStateBadge`（或重试按钮所在组件）加内部 `isRetrying` 状态，`handleRetry` 期间按钮 disabled + 文案改"重试中…"；完成后由父级 fetch 或 store invalidate 自然刷新。
   - 同一行 1 秒内重复点击应被无视（防抖）。

4. **AC-4（MAJOR #4：非 done 行 cursor 反馈）**：
   - AssetListView 行 style 按 state 切换 `cursor: not-allowed`（`state !== 'done'`），徽章 `title` 增加「无法拖出」前缀。

5. **AC-5（MAJOR #5：toast dedupe）**：
   - `uiStore.addNotification` 增加可选 `dedupeKey` 参数，相同 key 在 3 秒窗口内合并/替换（保留最新一条）。
   - `useDragAssets` toast 4 变体使用 `dedupeKey = "outbound:<errorKind>"`，避免快速重复拖拽时堆积。

6. **AC-6（MAJOR #6：rename Modal 替代 window.prompt）**：
   - 用应用内既有 `uiStore.openModal`（或同等 Modal 体系）实现 rename 表单：
     - 输入框 + 字符 / 字节计数（≤ 200 字节，超出红色提示）
     - 显示 sanitize 规则（一行简介）
     - 失败用 toast（不再 window.alert）
   - rename 命令仍走 `renameAsset(assetId, newName)`。

7. **AC-7（MAJOR #7：键盘可达性 Enter / Backspace）**：
   - AssetListView 中已有的 keydown 处理（Cmd+A 等）扩展：
     - `Enter` → 当前选中单条 → 触发 rename Modal
     - `Backspace` / `Delete` → 触发删除确认（既有删除流，确认对话框中文）
     - `F2` → 与 Enter 等价（macOS / Windows 双兼容）

### 通用要求

8. **AC-8**：所有可见文案中文；所有改动覆盖 vitest 测试：
   - `AssetListView.test.tsx` 追加：source-missing 角标渲染；data-source-missing 属性；non-done 行 cursor 类
   - `AssetContextMenu.test.tsx`（如无则新建）：查看原文件项 enabled/disabled 切换
   - `useDragAssets.test.tsx`：toast dedupeKey 合并行为
   - rename Modal：基础渲染 + 字节计数 + 校验失败 toast

9. **AC-9**：`npx vitest run` 全 PASS（不包括 pre-existing layout/* 失败，需排除或文档化）；`npm run check`（tsc）0 错；`cargo build -p notecapt` 通过（如需新增 Tauri command，必须编译过）。

## 技术约束

- 前端 DTO 形状仅在 `src/types/`，组件不重塑。
- 文案统一中文。
- 不引入新 UI 库；图标用现有 lucide-react。
- 不修改 dropzone 内部 / 后端 commands（rename / delete / retry 命令签名不变；如需 reveal-source 命令缺失则在 `src-tauri/src/commands/` 内新增最小命令并注册到 `lib.rs`）。
- 不绕过 `tauri-commands.ts` wrapper。

## 参考文件

- `sessions/conductor/tasks/task_010_ux_review/ux_review.md`（问题详情 + 修复方向）
- `src/components/features/AssetListView.tsx`
- `src/components/features/AssetContextMenu.tsx`
- `src/hooks/useDragAssets.ts`
- `src/stores/uiStore.ts`（Modal / Notification 体系）
- `src/lib/asset-state.tsx`
- `src/lib/tauri-commands.ts`

## 预估影响范围

- 新建文件：
  - `src/components/features/RenameAssetModal.tsx`（如现有 Modal 体系需要独立组件）
  - 若 Tauri 缺 reveal-source 命令：`src-tauri/src/commands/source_view.rs`（+ `mod.rs` + `lib.rs` 注册）
- 修改文件：
  - `src/components/features/AssetListView.tsx`
  - `src/components/features/AssetContextMenu.tsx`
  - `src/hooks/useDragAssets.ts`
  - `src/stores/uiStore.ts`（dedupeKey）
  - `src/lib/asset-state.tsx`（重试 loading）
  - `src/lib/tauri-commands.ts`（如新增 reveal-source）
  - 相关测试文件
- 估算变更：~600 行（含 ~250 行测试）

## 注意

- **本 task 不修复 layout/* 5 个 pre-existing 合并冲突**（与本期 P0 改动零交集，由用户独立处理）。
- 不引入 P1 范围（M8 取消转化 / M9 网络自愈 / M11 多选混合 toast P1 升级 / M12 批量进度等）。
- 不重审 task_002–009 已通过的部分；只在必要时为兼容新交互调整测试。
