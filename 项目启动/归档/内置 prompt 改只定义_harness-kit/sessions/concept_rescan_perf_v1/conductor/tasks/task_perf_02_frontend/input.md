# Task 输入 — task_perf_02_frontend

## 目标

改进"知识关联 → 重新扫描"UI 反馈：① 首文档完成前显示脉冲条 + 温和提示（消除"卡死"错觉）；② 文案明确预期耗时；③ 为 task_perf_01 即将上线的 force_full 参数预留入口（本期不暴露完整双按钮，仅传 force_full=true 让"重新扫描"按钮做"强制全量重扫"，保持既有行为不破坏）。

## 前置条件

- 依赖 task：与 task_perf_01_backend **并行**（你不需要等它完成；但需要假定后端的 IPC 签名）
- 必须先存在的文件：
  - `src/components/features/knowledge/KnowledgeAssociationView.tsx`（主战场）
  - `src/lib/tauri-commands.ts` 或同等位置（IPC 封装；grep 找现状）

## 验收标准（Acceptance Criteria）

### AC-1（首文档完成前的脉冲条）

- 现状：`ExtractionProgressBar`（line 345）基于 `progress.processed / progress.totalAssets` 计算 percent；首文档完成前 processed=0，percent=0%，进度条 100% 空白 → 用户误以为卡死
- 改造：增加"未启动 / 启动中（processed=0 且 totalAssets > 0）/ 进行中 / 完成 / 错误"5 个状态
  - **启动中状态**：percent = 0 但显示**脉冲动画**（用 Tailwind `animate-pulse` 或自定义 CSS keyframes；不引入新动画库）
  - **进行中**：percent 真值显示
- 文案分支（中文，温和）：
  - 启动中：`正在处理首批文档（每篇约 60 秒）…` + 副标题 `预估全量约 {Math.ceil(totalAssets * 60 / 4 / 60)} 分钟（4 路并发）`
  - 进行中：`已处理 {processed}/{totalAssets} 个文档 · 发现 {conceptsFound} 个概念`
  - 完成：`扫描完成 · 共发现 {conceptsFound} 个概念`
  - 错误：`扫描出错：{progress.error || "未知错误"}` + 红色提示
- 验证：vitest 加测试覆盖 4 个状态的渲染

### AC-2（按钮态：扫描中禁用 + 文案变化）

- "重新扫描"按钮（line 213）在 `progress.state === "running"` 时：
  - `disabled={true}` + `aria-disabled={true}`
  - 文案改为 `扫描中…`
  - title `已有扫描任务在执行，请等待完成`
- 完成 / 错误 / 未启动状态恢复 "重新扫描" 文案 + 可点击
- 验证：vitest 加测试

### AC-3（IPC 调用参数对齐 task_perf_01）

- task_perf_01 backend 的 IPC 入口签名将是 `start_concept_extraction(library_id: String, force_full: bool)`
- 在 `lib/tauri-commands.ts`（或既有 invoke 封装位置）追加/修改：
  ```ts
  export async function startConceptExtraction(libraryId: string, forceFull: boolean): Promise<void> {
    return invoke("start_concept_extraction", { libraryId, forceFull });
  }
  ```
- 在 `KnowledgeAssociationView.tsx::handleStartScan`（line 213 附近）：
  - 当前 `handleStartScan(true)` 中的 `true` 含义不明 — grep 看其实际语义，若与"forceFull"语义一致就保留；若是其他参数（如 `incremental` 反义），改 IPC 调用为传 `forceFull: true`（**本期重新扫描按钮总是强制全量重扫，保持既有用户体验**）
- 增量逻辑（force_full=false）**本期前端不暴露**（task_perf_01 后端落地后，UI 改造 P2 才加双按钮"增量扫描" / "强制全量"）
- 验证：vitest mock invoke 验证 forceFull=true 透传

### AC-4（既有事件监听不破坏）

- `listen("notecapt/concept-extraction-progress", ...)` 保持不变
- payload 类型 `ExtractionProgress`（含 processed / totalAssets / conceptsFound / state）保持
- task_perf_01 后端的 emit_progress 不改 payload 结构

### AC-5（tsc / vitest 全绿）

- `pnpm tsc --noEmit` 0 error
- `pnpm test src/components/features/knowledge/` 全绿（如有既有测试）
- 新增的脉冲条 / 按钮态测试覆盖 AC-1 + AC-2

## 技术约束

- **不引入新依赖**（Tailwind animate-pulse 已在；如要自定义 keyframes 加到既有 `tailwind.config.js`，~3 行）
- **不修改 PR-4 半成品** / **不修改 task_007 / task_007_round2 产物**（PromptCustomizationPanel.tsx / SettingsPanel.tsx 等）
- **不修改 Rust 文件**（task_perf_01 范围）
- **文案中文，温和**
- **不修改 progress.md**

## 参考文件

**必读**：
- session_context：`sessions/concept_rescan_perf_v1/session_context.md`
- handoff_contracts § 3：`core/handoff_contracts.md`
- Dev 系统提示词：`roles/conductor/dev/prompt.md`

**代码参考**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/knowledge/KnowledgeAssociationView.tsx`（主战场，全文 ~400 行）
  - line 63-96：既有 listen 与 unlisten 逻辑
  - line 209-224：重新扫描按钮
  - line 345-381：`ExtractionProgressBar`
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/lib/tauri-commands.ts`（IPC 封装层，task_005 改过；grep `startExtraction` 或 `concept` 看现状）

## 预估影响范围

- **修改文件**：
  - `src/components/features/knowledge/KnowledgeAssociationView.tsx`（~50 行：5 状态文案 + 按钮态 + 脉冲条 + IPC 调用更新）
  - `src/lib/tauri-commands.ts`（~5 行：startConceptExtraction 函数）
  - 测试（如已有）：~30 行
- **预估总变更**：~50-80 行

## 并行约束

⚠️ task_perf_01_backend 正在并行进行（改 Rust）。
- 你**只改 TS**，零 Rust 改动
- 你必须假定后端 IPC 签名 `start_concept_extraction(library_id, force_full)` — 如果 task_perf_01 改了命名，由 Conductor 在 Review 阶段做最终一致性检查
- 现有事件 `"notecapt/concept-extraction-progress"` 名称保持
