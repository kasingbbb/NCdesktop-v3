# Task 输出 — task_016_settings_form

## 状态
**完成（含 PM ESCALATE 2026-05-27 补丁 AC-7 全量落地）**

## 工作目录验证

```
$ cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop && git rev-parse --show-toplevel
/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop
$ git branch --show-current
feat/windows-unit-13-cloud-ai
$ git rev-parse HEAD (commit 前)
acd5cffbc07739e5adc245546811aeeab295420f
```

## 改动文件清单（4 个）

| 文件 | 类型 | 说明 |
|------|------|------|
| `项目启动/NCdesktop/src/components/features/bridge/KcSettingsForm.tsx` | 新建 | F11 Settings UI 主体（~640 行，含 KeyInputBlock / Toggle / SubToggleRow 内部子组件） |
| `项目启动/NCdesktop/src/components/features/bridge/__tests__/KcSettingsForm.test.tsx` | 新建 | 9 单元测试（AC-5 七项 + 2 额外守护） |
| `项目启动/NCdesktop/src/components/features/SettingsPanel.tsx` | 修改 | 新增 "知识增强 (KC)" tab，挂载 `<KcSettingsForm />`；位置紧跟 "AI / LLM" tab（与 AC-6 一致） |
| `项目启动/NCdesktop/src/lib/tauri-commands.ts` | 修改 | `KcHealthStatus` interface 追加 `aiEnabled?: boolean \| null`（PM 补丁字段，后端 task_020 DTO 当前未透传时取 null/undefined 走"未知"显示） |

## AC 逐项落地

### AC-1：KcSettingsForm 主体结构
- [x] 标题 "知识增强（KC）"
- [x] 总开关 `kcEnabled` toggle（aria-label="启用知识增强"）
- [x] 子区块 "AI 增强能力"：双 Key（智谱 AI / OpenAI）输入框，type=password + autoComplete=off + mask 显示当前 Key（`前4****后4` 或全 `*`）+ 清除按钮（toggle clearPending 状态，保存时走 `clear` 语义）
- [x] 子区块 "功能开关"：`useAi` / `enableQa` / `enableLinks` 三 toggle；当 `kcEnabled=false` 或两个 Key 均未配置时 disabled
- [x] 状态行：KC 服务状态（ready/starting/stopped/unavailable + reason）+ **AI 能力（aiEnabled true/false/未知，PM 补丁）** + 重启按钮（仅 unavailable / stopped 显示）

### AC-2：实时事件订阅
- [x] `useEffect` mount 时 `listen("notecapt/kc-status-changed", () => refreshHealth())`；event 回调内调 `getKcHealth()` 刷新 `health` state（含 aiEnabled）
- [x] unmount 时 `unlisten()` 清理，且对"unmount-before-listen-resolved"的竞态做了 mounted flag 防御（fn() 立即清理）

### AC-3：保存按钮
- [x] 点击 → 计算 `zhipuKeyAction` / `openaiKeyAction`：`clearPending=true → clear`，`draft.trim() 非空 → set`，否则 `keep`
- [x] 调 `setKcSettings({ enabled, useAi, enableQa, enableLinks, zhipuKeyAction, zhipuKeyValue?, openaiKeyAction, openaiKeyValue? })`
- [x] 成功后 reset draft + clearPending，重新 `loadInitial()` + `refreshHealth()`，toast "已保存。Key 若有变化将在后台重启 KC（数秒）"
- [x] 失败时 toast "保存失败：{error}"，不清 draft

### AC-4：useState 局部状态
- [x] 全部表单字段（6 bool + 2 key draft + 2 clearPending + 4 configured/mask）走 `useState`，无 Zustand 引入

### AC-5：单元测试（**9 PASS / 9 RUN**，覆盖 7 强制 AC + 2 额外）
| # | 测试名 | 状态 |
|---|--------|------|
| 1 | `renders_with_default_settings` | PASS |
| 2 | `toggle_kcEnabled_disables_sub_toggles` | PASS |
| 3 | `restart_button_only_shown_when_unavailable` | PASS |
| 4 | `key_input_masks_value` | PASS |
| 5 | `kc_use_ai_disabled_when_no_key` | PASS |
| 6 | **`test_key_connectivity_button_calls_health_endpoint`**（PM 补丁） | PASS |
| 7 | **`ai_enabled_status_renders_from_health_dto`**（PM 补丁） | PASS |
| 8 | `save_button_uses_keep_when_draft_empty_and_key_configured`（额外） | PASS |
| 9 | `restart_button_shown_when_stopped`（额外） | PASS |

### AC-6：Settings 主页挂载
- [x] `SettingsPanel.tsx`：新增 `Sparkles` icon import + `kc` tab id + `<KcSettingsForm />` 渲染分支
- [x] tab 顺序：…/ AI / LLM / **知识增强 (KC)** / Prompt / Privacy（与 input.md AC-6 "AI / LLM 设置区块之后"对齐）

### AC-7（PM ESCALATE 2026-05-27 补丁）：测试连通性按钮
- [x] 每个 Key 输入框旁的 [测试连通性] 按钮（智谱 + OpenAI 各一）
- [x] 点击调 `getKcHealth()`（不需要新 endpoint）
- [x] 显示结果：
  - `status=ready && aiEnabled=true` → ok：`"AI 已就绪（ai_enabled=true）"`
  - `status=ready && aiEnabled=false` → err：`"Key 配置但 AI 未启用（检查 KC 后端 ai_provider 配置）"`
  - `status!=ready` → err：`"KC 服务不可用（{status}）"`
  - `aiEnabled=null/undefined` → err：`"KC 已就绪，但 ai_enabled 字段未知（请检查 KC 后端版本）"`（守护后端 DTO 未透传 case）
- [x] 不阻塞保存（仅 setConnectivityState，无表单状态污染）
- [x] 双语 title 属性 `"测试连通性 / Test connectivity"`

## 关键技术决策

### 1. `KcHealthStatus.aiEnabled` 字段处理（与 input.md 表述的不一致解决）

task_016 input.md 写"task_020 已落地 ... `KcHealthStatus` interface 含 `aiEnabled` 字段"，但实际 `src-tauri/src/commands/kc.rs::KcHealthStatusDto` 与 `src-tauri/src/kc/process.rs::KcHealthStatus` 当前均**不含** `ai_enabled` 字段（task_020 commit `100e66f6` 未涉及）。

**处理方式**：把 `aiEnabled?: boolean | null` 加为前端 TS interface 可选字段，UI 按 `null/undefined → "未知"` 防御渲染。这样：
- 后端**未来扩展**时（task_020 补丁 / task_027 等），DTO 加 `ai_enabled: bool` 字段，前端自动捕获展示；
- 后端**当前**不返回时，前端不会崩溃 / 不会误显示，仅 "未知" 提示；
- 测试用例显式 mock `aiEnabled=true/false/null` 三种情况，证实分支正确。

不直接改后端 DTO 是因为本 task 是前端 Dev 范畴，且后端 task_020 已 commit 固化；后续应由后端 PR 补字段（建议 spawn 子任务，但本 task 通过可选字段先解耦不阻塞）。

### 2. 初值读取走 `getAllSettings()`
后端无 `get_kc_settings` command，故走通用 `getAllSettings()` 拿 `Record<string, string>` 后按 `kc.*` 7 个常量解析。Key 字段直接判 `trim().length > 0` 等价于"已配置"，与后端 `KcSettings::load` 的 `load_opt_string` 语义一致。

### 3. 健康轮询的 visibility-gated 实现
- mount 时检查 `document.visibilityState`，仅在 `"visible"` 时 `setInterval(refreshHealth, 5000)`；
- 监听 `visibilitychange` 事件，切到 hidden → `clearInterval`；切回 visible → 立即 refresh + 重启 interval；
- unmount 时**两道清理**：`clearInterval` + `removeEventListener` + `unlisten`（listen 订阅独立 effect）。
- 守护"组件已卸载但 listen.then 才解析"的竞态：`mounted` flag + 同步 `fn()` 兜底。

### 4. Key 输入"清除"模式（双态而非破坏性单次按钮）
点击 "清除" 后切换 `clearPending=true`，输入框 border 变红 + 按钮文案变 "撤销清除"；保存时走 `clear` 语义；用户改主意可点 "撤销清除" 还原为 `keep`。避免"误点立即丢 Key"的操作风险（reviewer 重点关注项）。

## 测试 / 类型检查

```
$ cd 项目启动/NCdesktop && pnpm vitest run KcSettingsForm
Test Files  1 passed (1)
     Tests  9 passed (9)
Duration  834ms

$ pnpm tsc --noEmit
(exit 0, 0 errors)
```

**0 退化**：master baseline `9 failed files / 44 failed tests / 414 passed` → 含本 task 改动 `9 failed files / 44 failed tests / 434 passed`（新增 +20 PASS，含本 task 9 + 同时跑通的稳定测试）；失败列表完全相同，未引入新失败。

## 与其他 task 的边界

- **task_019**（DocumentViewer）：本 task 与之零交集，仅共享 master HEAD；并跑无冲突。
- **task_020**（KcHealthStatusDto.ai_enabled 后端透传）：建议作为后续小 task 落地；当前前端已 forward-compat 兼容。
- **task_021**（KcStatusBadge）：本 task 不复用 KcStatusBadge（语义不同：KcStatusBadge 是衍生件 4 态 `success/partial/failed/none`；本 task 状态行是 KC 服务进程态 `ready/starting/stopped/unavailable`）。

## 提交

commit message 草稿：
```
feat(frontend): task_016 — KcSettingsForm 完整实装（含 PM ESCALATE 补丁 AC-7 测试 Key 连通性）

新增 src/components/features/bridge/KcSettingsForm.tsx（F11 Settings UI）：
- 总开关 kcEnabled + 双 Key 输入（智谱/OpenAI，mask + keep/clear/set）+ 3 子开关
- 实时 KC 服务状态行（订阅 notecapt/kc-status-changed + visibility-gated 5s 轮询）
- PM ESCALATE 2026-05-27 补丁 AC-7：每个 Key 旁的 [测试连通性] 按钮（调 getKcHealth
  按 ai_enabled 判定 ✓/✗）+ 状态行额外显示 AI 能力（aiEnabled true/false/未知）

挂载到 SettingsPanel.tsx 新增 "知识增强 (KC)" tab；
tauri-commands.ts KcHealthStatus 追加 aiEnabled?: boolean | null（后端 task_020 DTO
未透传时按"未知"防御渲染，forward-compat）。

9 单元测试全 PASS（7 AC + 2 额外）；pnpm tsc --noEmit 0 error；0 退化。

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```
