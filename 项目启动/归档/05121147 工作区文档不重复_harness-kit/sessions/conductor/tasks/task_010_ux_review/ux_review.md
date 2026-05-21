# UX Review Report — task_010_ux_review

## 审查信息

- 审查时间：2026-05-13
- 审查方式：静态代码审查（不启动应用 — layout/* 有 pre-existing 合并冲突）
- 项目类型：Desktop App（Tauri + React + Rust）— UX 优先级"高"（session_context §3 / §4：UX 权重 25%）
- 覆盖的核心旅程（PRD §2.2）：
  1. 批量导入（悬浮窗 5 文件 → 5 条 MD）
  2. 整理（rename / 打标签）
  3. 拖出消费（多选 done 拖 ChatGPT — outbound 准备阶段）
  4. 失败 / 离线降级
  5. 回看源（"查看原文件"）
- 关键代码：
  - `NCdesktop/src/components/features/AssetListView.tsx`
  - `NCdesktop/src/components/features/AssetContextMenu.tsx`
  - `NCdesktop/src/hooks/useDragAssets.ts`
  - `NCdesktop/src/lib/asset-state.tsx`
  - `NCdesktop/src/stores/assetStore.ts` / `stores/uiStore.ts`
  - `NCdesktop/src/components/features/dropzone/DropzoneIdle.tsx` / `DropzoneApp.tsx`
  - `NCdesktop/src/types/workspaceAsset.ts` / `types/asset.ts`

---

## 启发式评估结果（Nielsen 10 项）

| 启发式原则 | 检查要点 | 评分(1-5) | 发现 |
|-----------|----------|-----------|------|
| 系统状态可见性 | 转化中、失败、离线等状态是否随时可见 | 4 | `AssetStateBadge` 四态徽章 + 图标 + 文案齐全，`Loader2` 旋转动画给到 converting；但是 converting 态没有"已耗时 N 秒"/"队列位置"等进度暗示，长任务（音频 ASR 数十秒到分钟级）用户会怀疑卡死 |
| 系统与真实世界匹配 | 中文文案、图标直觉 | 5 | "已就绪 / 转化中 / 失败 / 离线待转化"中文准确；图标语义清晰（✓ / 🔄 / ⚠ / 📶✗） |
| 用户控制与自由 | 撤销 / 取消 | 2 | 删除无撤销（仅 `window.confirm`）；converting 态无取消按钮（PRD 标记 M8 P1）；重命名通过原生 `window.prompt`，体验粗糙、不支持二级撤销 |
| 一致性与标准 | 同操作一致表现 | 4 | 状态文案通过 `assetStateLabel` 单一来源映射，符合规范；但删除用 `window.confirm`，重命名用 `window.prompt`，重试失败用 toast — 三种反馈媒介不一致 |
| 错误预防 | 出错前预防 | 3 | 删除有 `window.confirm`（含中文 + 计数）；非 done 态拖拽在后端兜底报错，但 UI 无"hover 即提示"前置（cursor 仍是 default，用户先拖才知失败） |
| 识别而非回忆 | 关键操作可见 | 3 | 右键菜单需要用户记得"右键唤起重命名/删除/移动"；列表行无明显"⋯ 操作"按钮；快捷键提示缺失 |
| 灵活性与效率 | 快捷键 | 2 | 只实现了 Cmd/Ctrl+A 多选；Enter 重命名、Backspace/Delete 删除均未实现；Esc 关闭菜单 ✓ |
| 美学与简约 | 视觉层次 | 4 | 双栏布局清晰、徽章/标签/分组日期信息密度合理 |
| 帮助用户诊断错误 | 错误信息友好 | 4 | `OutboundError` 6 个 kind 分别映射中文 toast 文案，参数化 state / offending count；但 `ioFailed.detail` 直接透传原始消息，可能含 Rust 路径 |
| 帮助与文档 | 引导 | 3 | 空态文案给到"拖入会复制到 NoteCaptWorkPlace"，但未直接指引"启动悬浮窗"或键盘快捷键 |

**启发式平均分：3.4 / 5**

---

## 用户旅程扫描结果

### 旅程 1：批量导入（悬浮窗 → 5 条 MD）

**路径**：悬浮窗按钮 → 展开 → 系统级拖入 → onDragDropEvent → import_files → 列表占位 N 条 → 转化完成

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|----------|----------|----------|------|------|
| 唤起悬浮窗 | ✅ | n/a | ✅ | 4 | DropzoneIdle 有 attract pulse，但"Drop" 缩写非首次用户直觉 |
| 拖入 5 文件 | ✅ | ✅（路径为空时 fileName 提示） | ✅ | 4 | onDragDropEvent 兜底文案"未获取到文件路径"友好 |
| 看到 5 条占位 | ✅ | ✅ | n/a | 5 | M0 原子导入保证 3s 内可见；徽章显示 converting |
| 等待 ASR/markitdown | ⚠ | ✅ | ✅ | 3 | converting 态无进度/耗时；长任务用户焦虑 |
| 完成转化 | ✅ | n/a | n/a | 5 | done 徽章 + 图标切换清晰 |

**整体评分**：4 / 5  
**摩擦点**：converting 态没有耗时指示

### 旅程 2：整理（rename / 打标签）

**路径**：单选 → 右键 → 重命名 → window.prompt → 提交

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|----------|----------|----------|------|------|
| 单选资产 | ✅（高亮 + outline） | n/a | ✅ | 5 | 选择反馈完整 |
| 右键菜单 | ✅（aria-label / role=menu） | ✅（Esc 关闭） | ✅ | 4 | 但多选时重命名按钮 disabled 灰显，文案"（多选不可用）"清晰 |
| 输入新名 | ❌（原生 prompt） | ⚠（trim/同名静默关闭） | ✅ | 2 | `window.prompt` 体验古朴、无字符 sanitize 提示、长度截断规则用户不可见 |
| 提交 | ✅ | ⚠ | ✅ | 3 | 失败用 `window.alert` 弹原始错误（含技术堆栈风险） |

**整体评分**：3 / 5  
**摩擦点**：原生 prompt/alert 与应用风格强烈不符

### 旅程 3：拖出消费（多选 done → ChatGPT，仅准备阶段）

**路径**：多选 → mousedown → kick off prepare_outbound_payload → mousemove 阈值 → startDrag

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|----------|----------|----------|------|------|
| 多选 done 资产 | ✅ | n/a | ✅ | 5 | brand-navy outline + 背景 |
| mousedown kick off | ❌（无视觉） | ✅（payload 失败有 toast） | ✅ | 4 | user gesture 时序设计合理；但用户无感知 IPC 在进行 |
| mousemove 触发拖拽 | ✅（系统拖影） | ✅ | ✅ | 5 | startDrag 提供系统级反馈 |
| 非 done 拖拽 | ❌（cursor 仍 default） | ✅（toast 4 变体） | ✅ | 3 | **关键摩擦**：hover 时 cursor 无 `not-allowed` 提示；用户必须先拖才知失败 |
| toast 提示 | ✅ | ⚠（堆积） | ✅ | 3 | 多次失败 → toast 堆积（addNotification 直接 push 数组无 dedupe） |

**整体评分**：4 / 5  
**摩擦点**：非 done 态拖拽缺前置 cursor 提示；toast 可堆积

### 旅程 4：失败 / 离线降级

**路径**：网络断 → 导入 → offline 占位 → 网络恢复 → 重试 → done

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|----------|----------|----------|------|------|
| offline 占位 | ✅（WifiOff 徽章） | ✅ | ✅ | 5 | 文案"离线待转化"准确 |
| failed 态 | ✅（AlertCircle 黄） | ✅ | ✅ | 5 | hover title 显示 `失败原因：reason`，体验良好 |
| 重试按钮 | ✅（行内徽章右侧） | ✅（toast） | ✅ | 4 | 位置在徽章右侧（行内）；**但无 loading 状态**：连击会重复触发 retryAssetConversion，按钮无 disabled/spinner，视觉抖动 |
| 重试后刷新 | ✅ | ✅ | ✅ | 4 | onRetry 触发 fetchAssets + addNotification |

**整体评分**：4 / 5  
**摩擦点**：重试按钮缺 loading 态（AC-3 部分未达）

### 旅程 5：回看源（"查看原文件"）

**路径**：选中资产 → 找到"查看原文件" → 点击 → Finder 打开

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|----------|----------|----------|------|------|
| 找到入口 | ❌ | n/a | n/a | 1 | **完全没有"查看原文件"按钮/菜单项**。AssetContextMenu 只有"在 Finder 中显示"（指向工作区目录，非 source）；AssetListView 仅在 title 属性 tooltip 显示原件路径 |
| source-missing 视觉 | ❌ | ❌ | n/a | 1 | 后端 task_007 已 wire `sourceMissing: boolean`（types/asset.ts:46 / types/workspaceAsset.ts:56），但 grep 整个 src/components 无任何消费——既无角标，也无按钮置灰 |

**整体评分**：1 / 5  
**阻断点**：PRD §2.2 场景 5 与 PRD §3.1 M7 在 UI 层未落地

---

## 技术性 UX 检查结果

### 核心交互
- [x] 表单提交 loading 状态防重复 — **部分**：moveAssetToWorkspaceFolder 有 `moving` flag；重命名/删除/重试 **无** loading
- [x] 成功操作视觉反馈 — addNotification info toast
- [⚠] 键盘可操作 — 仅 Cmd+A；Enter / Backspace / Delete / F2 均未绑定
- [x] 关键操作确认 — 删除有 `window.confirm`（中文 + 计数）

### 错误处理体验
- [x] 验证错误不用 alert — toast 大部分；但 rename 失败用 `window.alert`
- [x] 网络异常有友好提示 — offline 徽章
- [⚠] 错误信息对用户友好 — `ioFailed.detail` 可能透传 Rust 错误堆栈；rename 失败 `String(err)` 直传

### 基础可用性
- [x] ARIA / 语义化 — AssetContextMenu 有 `role=menu` / `aria-label`；AssetStateBadge 有 `data-testid` / `aria-label`
- [n/a] 响应式 — Desktop 固定窗口
- [x] 字号 / 对比度 — 11px 标签略小但在二级信息可接受；主信息 14px+

---

## 发现的问题

### BLOCKER（必须修复，影响核心流程不可用）

1. **"查看原文件"入口完全缺失（PRD §2.2 场景 5）**
   - 影响旅程：旅程 5 回看源
   - 复现：选中任意资产 → 右键 → 菜单仅有"移到文件夹 / 重命名 / 在 Finder 中显示（指向工作区目录）/ 删除"。无"查看原文件"或等价项；列表行也无按钮。`sourcePathHint` 仅作为 button `title` 属性 tooltip 展示。
   - 影响：PRD 明列 5 个核心场景之一无法完成；session_context §3 不可妥协底线 #5"源文件可访问可恢复"未在 UI 兑现
   - 建议修复方向：AssetContextMenu 增加"查看原文件"项；调用 `revealItemInDir(sourcePath)`；当 `sourceMissing === true` 时 disabled + 文案改为"原文件已不存在"

2. **`sourceMissing` 字段后端已 wire 但前端 0 消费（AC-4 全部未达）**
   - 影响旅程：旅程 5 回看源
   - 复现：`grep -rn sourceMissing src/components` 仅返回类型定义文件，无任何组件使用。即使后端报告 source 文件已被外部删除，列表行无角标、无视觉差异、无按钮置灰。
   - 影响：M7 启动期 source 扫描的产品价值未交付到用户面前
   - 建议修复方向：AssetListView 行渲染时检查 `a.sourceMissing`，在文件名后渲染 `⚠ 原件丢失` 徽章；配合 BLOCKER #1 的"查看原文件"按钮 disabled

### MAJOR（强烈建议修复）

3. **重试按钮无 loading 状态，连击会视觉抖动（AC-3 部分未达）**
   - 影响旅程：旅程 4 失败/离线降级
   - 复现：failed 态 → 快速连点"重试"按钮 → 多次 IPC 同时 in-flight，按钮 UI 无 disabled / spinner；handleRetried 触发 fetchAssets 也会让 badge 短时间在 failed↔converting 间闪烁
   - 建议修复方向：AssetStateBadge 增加 `retrying` 内部状态，handleRetry 期间按钮 disabled + 文案"重试中…"；onRetry 完成后由父级 fetch 自然更新

4. **非 done 态拖拽无 hover 前置反馈（AC-2 部分未达）**
   - 影响旅程：旅程 3 拖出消费
   - 复现：hover converting/failed/offline 行 → cursor 仍 default → 用户拖拽后才看到 toast 提示
   - 建议修复方向：list item style 中按 state 切换 `cursor: not-allowed`（state !== 'done'）；徽章 title 增加"无法拖出"前缀

5. **Toast 堆积（AC-2 (c) 未达）**
   - 影响旅程：旅程 3 拖出消费 / 旅程 4 重试
   - 复现：uiStore.addNotification 直接 `[...s.notifications, notification]` 推入，无去重/折叠；用户对 5 条非 done 资产逐一尝试拖拽 → 5 条几乎相同的 warning toast 堆积
   - 建议修复方向：addNotification 增加 `dedupeKey` 参数，相同 key 在 N 秒窗口内合并/替换；或拖拽 toast 切换为 inline ghost layer，限单条

6. **重命名用 `window.prompt`、失败用 `window.alert`，体验古朴**
   - 影响旅程：旅程 2 整理
   - 复现：右键 → 重命名 → 浏览器原生 prompt 弹出（无 sanitize 规则提示、无 200 字节长度提醒）；失败 → `window.alert` 直显 `String(err)`
   - 建议修复方向：使用应用内 Modal 输入框（既有 uiStore.openModal 体系），inline 显示 sanitize 规则、字符计数；失败改用 toast

7. **键盘可达性不足（AC-5 仅部分达成）**
   - 影响旅程：旅程 2 / 旅程 4
   - 复现：选中资产后按 Enter / F2 无反应（无法触发重命名）；按 Backspace / Delete 无反应（无法触发删除）
   - 建议修复方向：AssetListView useEffect 内 keydown 增加 Enter→重命名 / Backspace→删除 / F2→重命名（macOS 与 Win 习惯各兼顾）

### MINOR（可选修复）

8. **converting 态无进度/耗时提示**
   - 影响旅程：旅程 1
   - 复现：音频文件导入 → converting 旋转动画 → 数十秒至分钟无变化 → 用户怀疑卡死
   - 建议修复方向：badge title 显示"已等待 Ns"（由 `importedAt` 推算）；P1 引入 M10 详情面板时联动

9. **空态文案未引导悬浮窗 / 快捷键**
   - 影响旅程：旅程 1
   - 复现：新项目 → 空态文案"该项目暂无素材 / 拖入文件会复制到 NoteCaptWorkPlace…"——未提示"也可点击悬浮窗 Sparkles 按钮"或"Cmd+A 全选已有素材"
   - 建议修复方向：空态文案增加一行"点击右下悬浮窗 ✨ 也可手动上传"

10. **`ioFailed.detail` 可能暴露技术细节**
   - 影响旅程：旅程 3
   - 复现：跨卷 copy 失败 / 权限拒绝 → `err.detail` 透传 Rust IO 错误（含路径）
   - 建议修复方向：tauri-commands.ts parseOutboundError 对 ioFailed 文案做白名单 map，detail 仅保留在 console.error

---

## 总体评分：3.2 / 5

- 状态机 / 文案规范化（AC-1）做得优秀（5/5），单一来源映射 `assetStateLabel` + `AssetStateBadge` 是亮点
- 拖拽时序（mousedown kick off）技术设计合理（AC-2 a 时序逻辑），但 hover 前置反馈 / toast 堆积有摩擦
- 重试机制可达，但缺 loading（AC-3 部分）
- **AC-4 source-missing 整条链路在 UI 层未落地（BLOCKER）**
- AC-5 键盘可达性仅 Cmd+A，Enter/Backspace 未实现
- AC-6 空态文案区分了"无资产"vs"筛选无匹配"，但缺悬浮窗引导（MINOR）
- AC-7 本报告含 10 条改进建议（≥5），满足

---

## 改进建议汇总（含追踪标记）

| # | 建议 | 状态 |
|---|------|------|
| 1 | 增加"查看原文件"右键菜单项 + 调用 reveal source | 需 P1 跟进（BLOCKER 但 PRD §3.2 未列于 P1；建议新增 task / 升级到 P0 hotfix） |
| 2 | 列表行消费 `sourceMissing` 渲染角标 + 置灰按钮 | 需 P1 跟进（M7 收尾，建议合并入 #1） |
| 3 | 重试按钮 retrying 内部状态 + disabled + 文案 | 需 P1 跟进 |
| 4 | 非 done 行 cursor: not-allowed + title 前缀 | 需 P1 跟进 |
| 5 | uiStore.addNotification 增加 dedupeKey | 需 P1 跟进 |
| 6 | 重命名/失败迁移到应用内 Modal/Toast | 接受现状（P1 体验打磨） |
| 7 | 键盘绑定 Enter / Backspace / F2 | 需 P1 跟进 |
| 8 | converting 已等待 Ns 提示 | 接受现状（等 M10 详情面板） |
| 9 | 空态文案补悬浮窗引导 | 接受现状 |
| 10 | ioFailed.detail 文案 sanitize | 接受现状 |

---

## 最终判断

- [ ] PASS（可以进入验收）
- [x] **ESCALATE（有 BLOCKER，需要修复后重审）**

**理由**：PRD §2.2 五个核心场景中场景 5"回看源"在 UI 层无入口；session_context §3 不可妥协底线 #5（"源文件可访问可恢复"）未兑现到用户面前；AC-4 完全未达。P0 M7 后端实现存在，但产品价值断在前端最后一公里。建议新建 follow-up task（建议命名 `task_011_source_view_ui`）至少修复 BLOCKER #1 + #2，并顺手 fix MAJOR #3 #4 #5（成本低、收益高），通过后重审。
