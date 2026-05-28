# UX Review Report — task_009_ux_review

## 审查信息

- 审查时间：2026-05-15
- 审查方式：静态代码评审（Tauri 桌面应用，subagent 无法启动 GUI，由 `PromptCustomizationPanel.tsx` + `PromptCustomizationPanel.test.tsx` + PRD § 3.1 三方对照反推 UI 行为）
- 覆盖的核心旅程：
  - 旅程 A：专家用户编辑 tagging Prompt 并保存
  - 旅程 B：单条恢复默认（已自定义条目）
  - 旅程 C：全部恢复默认
  - 旅程 D：触发字节超限（>16 KiB）
  - 旅程 E：触发占位符校验（concept 删除 `{content}`）
- 项目类型：Desktop App（Tauri 2.x，session_context § 1）
- UX 优先级：**高**（session_context § 4 用户体验权重最高，特别关注非技术用户友好度与错误提示清晰度）
- 审查文件：
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/PromptCustomizationPanel.tsx`（400 行）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/SettingsPanel.tsx`（366 行）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/userPromptStore.ts`（201 行）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/types/user-prompt.ts`（66 行）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`（463 行 / 23 用例）

---

## 启发式评估结果（Nielsen 10 项 + 项目适配）

| # | 启发式原则 | 检查要点 | 评分 (1–5) | 关键发现 |
|---|----------|----------|-----------|---------|
| 1 | **系统状态可见性** | loading / saving / 状态点 / 字节计数 | **3** | 折叠态可看见状态点（设计增强）；字节计数实时；**缺 saving spinner**：保存中按钮仅 disabled，无 spinner 或"保存中…"文案；loadAll 失败时无骨架 / 重试 UI |
| 2 | **系统与真实世界匹配** | 术语、文案 | **4** | "已自定义 / 默认 / 文件打标签 / PARA 分组" 与 PRD 1:1；`{content}` 占位符 chip 直观；专家用户应当能理解 |
| 3 | **用户控制与自由** | 撤销 / 回退 / 退出 | **4** | "恢复默认"（单条 + 全部）双层 confirm；编辑过程中可关闭面板（drafts 不持久化但状态点告知未保存）；**唯一缺陷**：edit 中切换 Tab 离开后 drafts 丢失，无未保存提醒 |
| 4 | **一致性与标准** | 平台惯例 / 相同操作表现 | **3** | `ChevronDown/Right size=14` 与 NCdesktop 其他折叠（`InspectorExtraction.tsx` 用 size=12）略有偏差；按钮颜色规范用 `var(--color-accent)` 一致；`window.confirm` 在 macOS Tauri webview 中显示为系统级 modal 与"打开设置后再次弹 modal"略割裂 |
| 5 | **错误预防** | 输入校验 / 确认对话框 | **5** | 三层预防：占位符 chip 提示 → 实时缺失警告 + save disabled → confirm 二次确认；恢复默认前 confirm；字节超限 save disabled；做得很好 |
| 6 | **识别而非回忆** | 关键信息可见 | **3** | 字节上限 `16 KiB / 16384 字节` 与占位符 chip 都可见，识别成本低；**唯一缺陷**：当 textarea 滚动到底部时，顶部的占位符 chip 不可见 — 用户改长 prompt 时可能忘记必含哪个占位符（不致命，警告会拦截） |
| 7 | **灵活性与效率** | 快捷键 / 路径 | **2** | **无快捷键**：保存无 Cmd/Ctrl+S；恢复无快捷键；Tab 在 textarea 内会输入 `\t` 而非跳转焦点（HTML 默认）；专家用户编辑长 prompt 时反复切按钮成本高 |
| 8 | **美学与简约设计** | 信息噪比 / 视觉层次 | **4** | 视觉层次清晰（折叠头 / chip / textarea / 计数 / 按钮区 / 错误条 自上而下）；空间留白合理；字节计数三色阶语义明确；**轻度噪比**：tagging/para 折叠头无 R4 副标题，用户看不到"合并到同一次分类调用"线索 |
| 9 | **帮助用户识别 / 诊断错误** | 错误信息可操作 | **4** | 占位符警告："缺少必含占位符：{content}（保存按钮已禁用）" — 明确诊断 + 可执行；字节超限："已超过 16 KB 上限" + 红色计数；store.error 原样透传后端中文错误（如"自定义 Prompt 过长（17234 字节，上限 16384 字节），请精简"）— 良好；**缺陷**：store.error 是单值，多个 module 同时展开时同一条错误会在每条展开的子项下方重复出现 |
| 10 | **帮助与文档** | 引导 | **3** | 顶部 3 行说明（PRD § 3.1 1:1）；占位符 chip 起引导作用；**缺陷**：用户首次打开如不知 Prompt 是什么 / 删除 `{content}` 为何不可保存，没有 tooltip 解释；属于"高级功能"定位下的可接受范围 |

**启发式平均分：3.5 / 5**

---

## 用户旅程扫描结果

### 旅程 A：专家用户编辑 tagging Prompt 并保存（核心旅程）

**路径**：设置入口 → 切到 "Prompt 自定义" Tab → 展开"文件打标签"折叠 → 编辑 textarea → 字节计数刷新 → 保存按钮变 active → 点保存 → dirty 归零 + 状态点变 ● 已自定义

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|---------|---------|---------|------|------|
| 入设置 → Tab | OK | OK | Tab 可切回 | 5 | `FileText` 图标合理，在 ai 与 privacy 之间合理 |
| 展开折叠 | OK（ChevronDown 翻转 + 边框延展） | — | 再点收起 | 4 | 折叠头同时显示状态点是设计增强 |
| 编辑文本 | OK（实时计数） | OK（占位符 chip 已展示） | 撤销靠 textarea 原生 undo（Cmd+Z） | 3 | **无快捷键 Cmd+S**；**无未保存离开提示** |
| 字节计数 | OK 三色阶 | OK 红色 + "已超过 16 KB 上限" | — | 5 | 实时反馈 |
| 点保存 | **❌ 缺 spinner** | OK（错误横条） | 失败时 drafts 保留可重试 | 3 | 网络慢时用户无法判断是否在 IPC 中 |
| 保存成功反馈 | **❌ 无 toast/绿对勾** | — | 状态点变色 + dirty 归零 | 3 | 反馈隐式；用户可能没注意到"已自定义"标志切换 |

**旅程整体评分：3.5 / 5**
**阻断点**：无
**摩擦点**：① 保存中无 spinner；② 保存成功无显式 toast；③ 编辑期切 Tab 离开会丢草稿无提示

---

### 旅程 B：单条恢复默认

**路径**：折叠头 → 看到"● 已自定义" → 展开 → 点底部"恢复默认"按钮 → confirm 弹"将恢复「文件打标签」为内置默认值。继续？" → 确认 → textarea 自动重置为默认 + 状态点变灰

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|---------|---------|---------|------|------|
| 找到入口 | OK（折叠头状态点） | — | 折叠态可见 | 5 | |
| 点恢复 | OK confirm | OK 拒绝路径已测 | confirm 取消 = 安全退出 | 5 | |
| textarea 重置 | OK（drafts 同步 defaultText） | — | reset 后状态点变灰、按钮 disabled | 4 | **缺**：textarea 重置没有过渡动画 / 高亮（input.md AC 第 4 维度提到希望"瞬时高亮"） |
| 按钮态切换 | OK（reset 按钮变 disabled） | — | — | 5 | |

**旅程整体评分：4.5 / 5**
**阻断点**：无
**摩擦点**：textarea 重置后无瞬时视觉高亮提示（AC-1 第 4 维度暗示但未硬性要求）

---

### 旅程 C：全部恢复默认

**路径**：底部右下角"全部恢复默认"按钮 → confirm 弹"将恢复全部 4 条 Prompt 为内置默认值，已有自定义会丢失。继续？" → 确认 → 触发 reset(null) → loadAll → 4 个状态点全部变灰

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|---------|---------|---------|------|------|
| 按钮位置 | OK（右下角） | — | — | 5 | PRD 1:1 |
| confirm 文案 | OK（"已有自定义会丢失"明确警告） | OK 拒绝路径已测 | 取消 = 安全 | 5 | 文案防误点 |
| 批量重置 | OK（loadAll 触发） | OK（store.error 原样透传） | — | 4 | **缺**：批量重置后无 toast 告知"已重置 X 条" |
| 4 状态点同步 | OK | — | — | 5 | |

**旅程整体评分：4.5 / 5**
**阻断点**：无
**摩擦点**：批量重置成功后无显式总结反馈（如 toast）

---

### 旅程 D：触发字节超限（>16 KiB）

**路径**：粘贴 17 KB 文本 → 字节计数变红 → "已超过 16 KB 上限"出现 → save disabled → 删字缩短 → 计数变橙 / 灰 → save 恢复可用

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|---------|---------|---------|------|------|
| 粘贴超长 | OK（实时计数 17000 / 16384 字节） | OK 红色 + 文案 | 删字可恢复 | 5 | |
| 警告位置 | 字节计数左侧"已超过 16 KB 上限" | OK | — | 4 | **小细节**：警告与计数行同行左对齐，视觉上不够醒目（红色计数本身已明显，但警告文案较短可能被忽略） |
| save disabled | OK（cursor: not-allowed + opacity 0.6） | — | — | 5 | |
| 恢复路径 | OK | — | OK 实时刷新 | 5 | |

**旅程整体评分：4.5 / 5**
**阻断点**：无
**摩擦点**：警告"已超过 16 KB 上限"位置可更醒目（如独立行或带感叹图标）

---

### 旅程 E：触发占位符校验（concept 删除 `{content}`）

**路径**：展开"知识概念提取" → 看到顶部 chip `{content}` → 删除文本中的 `{content}` → 实时出现红色警告"缺少必含占位符：{content}（保存按钮已禁用）" → save disabled → 重新输入 `{content}` → 警告消失

| 步骤 | 状态反馈 | 错误处理 | 可返回性 | 评分 | 发现 |
|------|---------|---------|---------|------|------|
| chip 提示可见 | OK（折叠展开时即显示） | — | — | 5 | 极佳的预防式提示 |
| 删除后警告 | OK 红色 + 解释 + 已禁用 | OK 即时反馈 | 重新输入即可恢复 | 5 | 错误诊断 + 可执行明确 |
| save disabled | OK | — | — | 5 | |
| 恢复路径 | OK | — | OK | 5 | |

**旅程整体评分：5 / 5**
**阻断点**：无
**摩擦点**：无

---

## 技术性 UX 检查结果

### 核心交互
- [x] 表单提交有 loading 状态 — **部分**：save 按钮在 IPC 中 disabled，但**无 spinner / 文案变化**（MINOR）
- [x] 成功操作有明确视觉反馈 — **部分**：状态点变色 + dirty 归零，**无显式 toast / 绿对勾**（MINOR）
- [ ] 键盘可操作（Tab 导航、Enter 提交）— **未实现**：textarea 内 Tab 输入 `\t`；保存无 Cmd+S（MINOR）
- [x] 关键操作有确认机制 — 恢复默认（单条 + 全部）均 `window.confirm`

### 错误处理体验
- [x] 验证错误在表单内提示 — 占位符警告 + 字节计数 + 错误横条均内联，非 alert
- [x] 网络异常有友好提示 — store.error 原样透传后端中文消息（如"自定义 Prompt 过长（17234 字节，上限 16384 字节），请精简"）；不暴露技术堆栈
- [x] 错误信息对用户友好 — **MAJOR 注意**：store.error 是单值，多个展开子项会**同时**显示同一条错误，视觉上"重复 3 次"（task_007 output.md "Reviewer 关注点 4" 已自标）

### 安全体验
- [x] 敏感输入框使用正确 input type — textarea 合理
- [x] 敏感信息不在 URL 中暴露 — 本地 Tauri 应用，N/A
- N/A 会话失效 — 本地应用

### 基础可用性
- [ ] ARIA label / 语义化 HTML — **部分**：仅有 `aria-expanded`、`aria-hidden`，按钮 `disabled` 时**缺 `aria-disabled`**；textarea 无 `aria-label` 或 `aria-labelledby`（MAJOR for screen reader 用户）
- [x] 响应式布局基本可用 — SettingsPanel 固定 640px，桌面尺寸下合理
- [x] 文字可读性 — text-sm/text-xs 是 13/12px（var 定义），略小但桌面应用 OK；对比度使用 `var(--text-primary/secondary/tertiary)` 主题变量，合规

---

## 发现的问题

### BLOCKER（必须修复，影响核心流程不可用）

**无 BLOCKER。** 5 个核心旅程全部畅通。

---

### MAJOR（强烈建议修复，影响体验但不阻塞核心功能）

#### MAJOR-1：错误横条在多个展开子项中重复显示

- **问题**：`store.error` 是单值字符串，当用户同时展开 2+ 个 module 折叠后任一操作失败时，**同一条错误**会出现在所有展开的子项下方，视觉上"重复 N 次"
- **影响旅程**：旅程 A（保存失败时）+ 旅程 B/C（恢复失败时）
- **复现路径**（静态推导）：
  1. 同时展开 tagging + concept 折叠
  2. 在 tagging 中输入超长 prompt 触发 save
  3. 后端返回字节超限错误
  4. `error-banner-tagging` 与 `error-banner-concept` 同时显示同一条错误
- **建议修复方向**：在 `userPromptStore` 中把 error 改为 `Record<PromptModule | "global", string | null>`，错误归属到具体 module；或将错误条改为顶部全局横条 + 当前操作子项高亮
- **位置**：`PromptCustomizationPanel.tsx:382-394`（每个展开子项都渲染 `error` 条）
- **task_007 output 自标**：已在 "Reviewer / UX 评审关注的地方 #4" 注明

#### MAJOR-2：缺 `aria-disabled` 与 textarea `aria-label`，screen reader 不友好

- **问题**：保存 / 恢复按钮使用原生 `disabled` 属性，但配色 + 不可点的状态信息只通过视觉传达（opacity 0.5/0.6 + cursor not-allowed）；textarea 完全没有 `aria-label`，screen reader 用户无法定位"这是哪个 module 的 textarea"
- **影响旅程**：可访问性，所有旅程对依赖屏幕阅读器的用户都将无法独立完成
- **建议修复方向**：
  - 按钮加 `aria-disabled={saveDisabled}`、`title={...禁用原因}` tooltip 解释禁用原因（占位符未满足 / 字节超限 / 未修改）
  - textarea 加 `aria-label={`${title} Prompt 编辑区`}` 或 `aria-labelledby` 引用标题
- **位置**：`PromptCustomizationPanel.tsx:301-314 / 346-378`
- **session_context UX 优先级=高**：明确要求"对非技术用户友好"，无障碍属于此范围

#### MAJOR-3：保存中 / 保存成功无显式反馈

- **问题**：点击保存后按钮变 disabled（因 dirty 归零），但**无 spinner、无"保存中…"文案、无成功 toast**；用户在慢网络 / 异常时不知 IPC 是否在进行，可能反复点击或误以为没保存
- **影响旅程**：旅程 A 核心场景
- **复现路径**（静态推导）：
  1. 用户编辑 prompt 后点保存
  2. IPC 在异步状态（200ms~2s）
  3. 视觉无任何变化（按钮原本 active，变 disabled 后看上去与"不能点"同色），状态点仍灰
  4. IPC 完成后状态点跳变为彩色"已自定义"，dirty 归零；用户可能漏看
- **建议修复方向**：组件内自管 `saving` useState（task_006 store.loading 是 loadAll 整体态，单条 save 无 store 态）；保存中显示 spinner + "保存中…"；成功后短暂高亮状态点或弹一个 1.5s 自动消失的 toast"已保存"
- **位置**：`PromptCustomizationPanel.tsx:138-145 / 363-378`
- **task_006 store 明确说明**："单条 save/reset 不影响 loading 字段；如 UI 需要单条 saving 态，下游 task_007 在组件内自管 useState"。task_007 未实现此 saving 态，应在二轮修复中补上

---

### MINOR（可选修复，打磨体验）

#### MINOR-1：顶部说明文案"如不确定,请保持默认值。"使用半角逗号

- **问题**：`PromptCustomizationPanel.tsx:121` 字面 `如不确定,请保持默认值。` 用了**半角逗号 `,`**，而 PRD § 3.1 第 65 行原文是全角 `如不确定，请保持默认值。`；中文行文中半角逗号视觉上略生硬
- **影响旅程**：旅程 A 入口体验（首次打开看到的说明）
- **建议**：改为全角 `，`
- **位置**：`PromptCustomizationPanel.tsx:121`

#### MINOR-2：恢复默认后 textarea 内容无瞬时高亮 / 过渡

- **问题**：reset 后 textarea 内容瞬间替换为 defaultText，无淡入 / 高亮过渡，用户可能误以为是自己删的
- **影响旅程**：旅程 B / 旅程 C
- **建议**：reset 后给 textarea 加一个 200ms 背景高亮淡出（如 `transition: background-color 0.3s; background-color: var(--color-accent)/10` 短暂触发），或在按钮区附近显示一个 1.5s 自动消失的"已恢复默认"文案
- **位置**：`PromptCustomizationPanel.tsx:301-314`

#### MINOR-3：字节超限警告位置不够醒目

- **问题**：警告"已超过 16 KB 上限"与字节计数同行左对齐，文案较短，视觉上易被红色计数遮盖
- **影响旅程**：旅程 D
- **建议**：警告独立一行，加 `⚠`（或 lucide AlertTriangle）图标；或直接合并到计数右侧成"17234 / 16384 字节（超限）"
- **位置**：`PromptCustomizationPanel.tsx:329-343`

#### MINOR-4：批量恢复 / 单条恢复成功无显式总结反馈

- **问题**：reset 成功后状态点变灰是隐式反馈；如用户折叠了未展开的子项，无法第一时间确认重置范围
- **影响旅程**：旅程 B / 旅程 C
- **建议**：可与 MAJOR-3 合并成 toast 系统：reset 成功后弹"已恢复「文件打标签」为默认 / 已恢复全部 4 条"
- **位置**：`PromptCustomizationPanel.tsx:87-99 / 146-157`

#### MINOR-5：折叠图标 size=14 与 NCdesktop 其他折叠组件不一致

- **问题**：`InspectorExtraction.tsx` 使用 `size=12`，本组件用 `size=14`，平台一致性轻微割裂
- **影响旅程**：跨页面视觉一致性
- **建议**：统一为 size=12 或更新 InspectorExtraction
- **位置**：`PromptCustomizationPanel.tsx:246-249`（task_007 自标 "Reviewer 关注点 #6"）

#### MINOR-6：未保存编辑离开 Tab 无提示

- **问题**：用户编辑某 module 后未保存切到其他 Tab / 关闭面板，drafts 丢失（store 不持久化），下次回来 textarea 回到 effectiveText（loadAll 重置）
- **影响旅程**：旅程 A 副情景（用户中途离开）
- **建议**：onClose / Tab 切换时如 `Object.values(dirty).some(Boolean)` 则 confirm "有未保存修改，确认离开？"；或全局保留 drafts 但提示状态点为"● 未保存"
- **位置**：`SettingsPanel.tsx:53` onClick={onClose} + `:89` setActiveTab

#### MINOR-7：textarea 滚动到底部后顶部 chip 不可见

- **问题**：当 prompt 很长（如接近 16 KiB），textarea 滚动后顶部 `{content}` chip 不在视野内；用户编辑底部时可能忘记必含的占位符是哪个
- **影响旅程**：旅程 E（长 prompt 场景）
- **建议**：把 chip 行放在 textarea 下方（与字节计数同区域），或在警告中追加"（必含：{content}、{concept_name}）"
- **位置**：`PromptCustomizationPanel.tsx:275-298`

---

## R4 文案决议建议

**背景**：PRD 视角 4 module（tagging / para / concept / aggregation），后端实际 3 调用链（tagging+para 合并到一次 `classify_prompt_v2` 调用；concept、aggregation 各自独立）。task_007 当前实现采取"维持 PRD 视角"，不向用户揭示后端合并细节。

**评估**：

- **方案 A（维持现状，不揭示）**：
  - 优点：UI 简洁；专家用户不需要理解后端调用图；遵循"信息分层"原则
  - 缺点：用户编辑 tagging 与 para 时无线索表明两者一起送 LLM，可能在调优 tagging 时不理解为何 PARA 分组结果也变了；R4 风险点未在 UI 层缓解
- **方案 B（在 tagging / para 折叠头加一行 muted 副标题）**：
  - 优点：揭示一次让用户知情；与 PRD 桥接摘要 R4 缓解策略"UI 文案明确指出"对齐
  - 缺点：副标题文字需仔细斟酌（避免吓到非技术用户），增加约 8-10 行代码

**建议：方案 B**

理由：
1. session_context UX 优先级 = 高，要求"非技术用户友好"，但 PRD § 2.1 明确目标用户是**专家用户**；专家用户知情更重要
2. R4 风险点（task_001 ADR R4）明确"通过 UI 文案缓解"，方案 A 实际上未缓解
3. 实现成本极低（< 10 行），无回归风险
4. 副标题措辞建议（muted, text-xs, text-tertiary）：
   - tagging 折叠头副标题：`与「PARA 分组」共用同一次分类调用，两者同时生效`
   - para 折叠头副标题：`与「文件打标签」共用同一次分类调用，两者同时生效`

实现路径：在 `PromptModuleSection` props 加 `subtitle?: string`，在 `PromptCustomizationPanel` 主组件按 module 注入。

---

## 总体评分：3.7 / 5

**优点**：错误预防层级清晰（5/5）、占位符校验体验顶级（旅程 E 5/5）、PRD 文案 1:1 落地、ADR-005 命名隔离严格、测试覆盖 23 用例。
**改进空间**：状态可见性（特别是 saving 反馈）、可访问性（aria-disabled / aria-label）、错误条多子项重复、R4 文案缺失。

## 最终判断

- [x] **PASS（可以进入验收）** — 5 个核心旅程畅通，无 BLOCKER
- [ ] ESCALATE

**建议**：3 项 MAJOR + 7 项 MINOR 列为 task_007 二轮微调修复（预估总改动 **< 50 行**），不阻塞 task_008 e2e 验收。R4 文案决议（方案 B）建议同时落地。task_010 Architecture Guard 阶段前完成。

---

## 可选 fix list（< 50 行改动）

| # | 文件:行号 | 当前 | 期望 | 理由 | 严重度 |
|---|----------|------|------|------|-------|
| 1 | `PromptCustomizationPanel.tsx:121` | `如不确定,请保持默认值。` | `如不确定，请保持默认值。` | 改全角逗号，与 PRD § 3.1 一致 | MINOR-1 |
| 2 | `PromptCustomizationPanel.tsx:138-145` | save 无 saving useState 与 spinner | 组件内 `const [saving, setSaving] = useState(false)`; onSave 时 setSaving(true)/false; 按钮态 `disabled={saveDisabled \|\| saving}` + spinner | 补 saving 反馈 | MAJOR-3 |
| 3 | `PromptCustomizationPanel.tsx:363-378` | 仅 `disabled={saveDisabled}` | 追加 `aria-disabled={saveDisabled}` + `title={saveDisabled ? 禁用原因 : undefined}` | 无障碍 + 鼠标悬停解释 | MAJOR-2 |
| 4 | `PromptCustomizationPanel.tsx:301-314` | textarea 无 `aria-label` | 加 `aria-label={`${title} 的 Prompt 编辑区`}` | screen reader 支持 | MAJOR-2 |
| 5 | `PromptCustomizationPanel.tsx:382-394` | 错误横条在每个展开子项都渲染 | 仅在 store 中记录失败模块（如 `error: { module, message } \| null`），仅在该 module 子项下方渲染；或改为顶部全局横条 | 去重 | MAJOR-1（依赖 store 改动 ~10 行） |
| 6 | `PromptCustomizationPanel.tsx:228-267` | 折叠头无副标题 | 加 `subtitle?: string` props，tagging/para 传入"与 X 共用同一次分类调用" | R4 文案揭示 | R4 方案 B |
| 7 | `PromptCustomizationPanel.tsx:329-343` | 警告文案与计数同行 | 把"已超过 16 KB 上限"独立一行 + ⚠ 图标 | 视觉醒目 | MINOR-3 |
| 8 | `PromptCustomizationPanel.tsx:246-249` | `<ChevronDown size={14} ...>` | `<ChevronDown size={12} ...>` | 与 NCdesktop 其他折叠组件一致 | MINOR-5 |
| 9 | `SettingsPanel.tsx:53` onClose / `:89` setActiveTab | 直接执行 | 加 dirty 守卫：`if (Object.values(useUserPromptStore.getState().dirty).some(Boolean)) confirm("有未保存修改…")` | 防误丢草稿 | MINOR-6 |
| 10 | `userPromptStore.ts:99-104` | `error: string \| null` | `error: { module: PromptModule \| null, message: string } \| null` | 配合 fix #5 | MAJOR-1 |

**预估代码改动总量**：~40-45 行（fix #2 内 useState/spinner ~10 行；fix #5 + fix #10 配合 ~15 行；其他每条 1-5 行）

---

> 本 output.md 严格按 UX Evaluator prompt § 输出格式撰写；不修改任何代码；不修改 progress.md（按 input.md 关键约束 AC-4）。fix list 由 task_007 二轮（或独立的 task_007_round2）承接执行。
