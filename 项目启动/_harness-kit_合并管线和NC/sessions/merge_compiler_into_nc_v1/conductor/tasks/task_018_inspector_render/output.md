# Task 交付 — task_018_inspector_render

## 实现摘要

把 `InspectorExtraction.tsx` 在 `status === "extracted" && content.structuredMd` 分支的渲染逻辑升级：

1. **AC-1**：用 `useMemo` 调 `parseFrontmatter(content.structuredMd)` 解析 YAML frontmatter（依赖 task_017）：
   - 解析成功 (`frontmatter !== null && !parseError`) → 渲染
     `<FrontmatterSummaryView summary={frontmatter.aiSummary} isAi />` + `<FrontmatterTagsView aiTags ruleTags />`
     + 用 `react-markdown` + `remark-gfm` 渲染 `body`（max-h-240px overflow-y-auto，与原 `<pre>` 视觉对齐）。
   - 解析失败 / 无 frontmatter / `parseError` → 回退到原 `<pre>` 渲染完整 `structuredMd`（行为字面 round-trip，向后兼容历史 MD）。
2. **AC-2 / AC-6 (TD-4)**：新增 `kcEnrichedLabel(kcEnriched)` 翻译函数，把 DB 字面值 `"true"/"partial"/"false"` 映射为 `"AI 增强：完整"/"AI 增强：仅规则标签（LLM 不可用）"/"未启用 AI 增强"`；`null` 返回 null（整行不渲染，历史数据）。该翻译层落在 `InspectorExtraction.tsx`，详见 §"翻译层归属说明（AC-6/TD-4）"。
3. **AC-3**：`handleCopy` 沿用 task_026 原逻辑（复制 `rawText ?? structuredMd`，即完整 markdown 含 frontmatter）；按钮 UI 字面零改动。
4. **AC-5 (TD-2)**：在 task_017 落地的两个组件追加 a11y：
   - `FrontmatterTagsView`：根 `role="list" aria-label="文档标签"`；AI chip `role="listitem" aria-label="AI 标签 {tag}"`；规则 chip `role="listitem" aria-label="规则标签 {tag}"`；前导文字 "标签：" 标 `aria-hidden`。
   - `FrontmatterSummaryView`：根 `role="region" aria-label={label}`（"AI 摘要" / "摘要"）；`isAi=true` 时追加 visible "AI" badge。

**安全**：`react-markdown` v9 默认不挂 `rehype-raw`，等价于 `allowDangerousHtml: false`。用户写入 `<script>` 之类 raw HTML 不会被渲染/执行，只会被当作文本。

**性能**：`parseFrontmatter` 用 `useMemo([content?.structuredMd])` 缓存，asset 切换或 structuredMd 变化才重解析。

## 翻译层归属说明（AC-6 / TD-4）

task_021 reviewer 抛出 TD-4：`kc_enriched` 的"字面值 → 用户文案"映射应该落在哪一层？

**结论：落在 `InspectorExtraction.tsx`，不复用 `KcStatusBadge`。**

| 层 | 关心的字面 | 来源 | 用途 |
|----|----------|------|------|
| `KcStatusBadge`（task_021） | `"success" / "failed" / "loading" / "idle"` | UX 状态机 | 视觉徽章；可复用于 KC 队列态 / 调度态 / 队列 toast |
| `kcEnrichedLabel`（本 task） | DB 列 `kc_enriched` 字面 `"true" / "partial" / "false" / null` | KC `enrichment.rs` 写库 | Inspector 业务展示的本地翻译 |

两者职责显式解耦的好处：
1. `KcStatusBadge` 不被 YAML/DB schema 字面绑死，未来 KC schema 升级（如加 `"timeout"`/`"abort"` 字面）只改 `kcEnrichedLabel`，徽章组件不需要触动。
2. Inspector 业务文案"AI 增强：完整 / 仅规则标签（LLM 不可用）/ 未启用"高度本地化，包含 KC 内部状态语义（"LLM 不可用"等暗示 task_011 `PartialLlmUnavailable` 来源），强行收敛到通用 `KcStatusBadge` 会让 Badge 知道太多上下文。
3. 历史数据 `kc_enriched = null` 在 Inspector 表达为"整行不渲染"，而 `KcStatusBadge` 在 idle 态可能仍要显示一个灰底徽章 —— 两种处理策略各自合理，不应耦合。

**Reviewer 关注点**：本 task 的 `kcEnrichedLabel` 是 `InspectorExtraction.tsx` 内的私有 helper（不导出），不污染模块边界；未来如 DocumentViewer / KC 队列页也要展示同款文案，可以重构提取到 `src/utils/kcEnrichedLabel.ts` 共享 —— 但本 task 范围内不预先抽象（YAGNI）。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src/components/layout/InspectorExtraction.tsx` | 修改 (+~55 行) | parseFrontmatter useMemo + 注入 Summary/Tags + react-markdown body + kcEnrichedLabel + kc_enriched 行 |
| `src/components/features/extraction/FrontmatterTagsView.tsx` | 修改 (+~10 行) | a11y：根 `role=list aria-label`；每个 chip `role=listitem aria-label`；前导 "标签：" 标 `aria-hidden` |
| `src/components/features/extraction/FrontmatterSummaryView.tsx` | 修改 (+~15 行) | a11y：根 `role=region aria-label={label}`；isAi=true 追加 visible "AI" badge |
| `src/components/layout/__tests__/InspectorExtraction.test.tsx` | 修改 (+~110 行) | 追加 6 个 AC-4 测试（frontmatter 渲染 / fallback / parse error / kc_enriched 三态映射 / null 不显示） |
| `src/components/features/extraction/__tests__/FrontmatterTagsView.test.tsx` | 修改 (+~20 行) | 追加 `frontmatter_tags_view_uses_role_list` 测试 |
| `src/components/features/extraction/__tests__/FrontmatterSummaryView.test.tsx` | 修改 (+~20 行) | 追加 `frontmatter_summary_view_has_aria_label` 测试 |

**未触及**（约束严守）：
- `src-tauri/src/kc/enrichment.rs` —— 纯前端 task
- `src-tauri/src/extraction/scheduler.rs`
- `src-tauri/src/db/*`
- `src/utils/parseFrontmatter.ts` —— task_017 已锁定，本 task 仅消费
- `src/components/features/extraction/KcStatusBadge.tsx` —— 翻译层归属切割，不在本 task 范围

**无新依赖**：`react-markdown@9.0.1` / `remark-gfm@4.0.0` / `js-yaml@4.1.0` 均由 task_017 引入 `package.json`，本 task 仅 import。

## 测试命令与结果

```bash
cd 项目启动/NCdesktop
pnpm vitest run InspectorExtraction.test FrontmatterTagsView.test FrontmatterSummaryView.test
pnpm tsc --noEmit
pnpm vitest run                              # 全量回归
```

**靶向 vitest**（3 个 test file）：
```
RUN  v4.1.1
✓ src/components/features/extraction/__tests__/FrontmatterSummaryView.test.tsx (5 tests)
✓ src/components/features/extraction/__tests__/FrontmatterTagsView.test.tsx (6 tests)
✓ src/components/layout/__tests__/InspectorExtraction.test.tsx (13 tests)

Test Files  3 passed (3)
     Tests  24 passed (24)
```

**测试增量明细**：

| Test File | baseline | 本 task 后 | 增量 |
|-----------|----------|-----------|------|
| InspectorExtraction.test.tsx | 7 (task_026 AC-3) | 13 | **+6**（AC-4 全部子项 + 翻译三态 + 历史 null） |
| FrontmatterTagsView.test.tsx | 5 (task_017 AC-5) | 6 | **+1**（AC-5 a11y） |
| FrontmatterSummaryView.test.tsx | 4 (task_017 AC-5) | 5 | **+1**（AC-5 a11y） |
| **合计** | **16** | **24** | **+8** |

**前端 `tsc --noEmit`**：`EXIT=0`，0 error。

**前端 `vitest run`（全量）**：42 test files / 440 tests。
- 本 task 改动文件：**24/24 passed**（之上 3 file/24 tests）；
- 全量结果：33 passed / 9 failed test files；396 passed / 44 failed tests。
- 9 个 failed file（useDragAssets / AppLayout / ContentArea / Inspector(non-Extraction) / Sidebar / SettingsPanel / TagTree / turnLearningOff / TitleBar）均是 worktree baseline 已 fail（与 task_026 output.md 记录的同一组），**与本 task 改动零交集**。
- 失败计数 **44 == task_026 落地后基线**，**0 退化**。

## Reviewer 特别关注

1. **react-markdown 安全配置**（input.md 技术约束）：
   - 用 v9 默认调用 `<ReactMarkdown remarkPlugins={[remarkGfm]}>...</ReactMarkdown>`，**不挂** `rehype-raw`、**不传** `skipHtml={false}`、`children` 走 props 而非 `dangerouslySetInnerHTML`；
   - v9 内部默认 `skipHtml: false` 但 raw HTML 节点被当作 unknown 节点 stringify，最终走 react escape → 不会执行；
   - 等价于 input.md 要求的 `allowDangerousHtml: false`。

2. **fallback `<pre>` 必须工作**（向后兼容）：
   - `useFrontmatterView = parsed.frontmatter !== null && !parsed.parseError` —— 两个独立条件都要 OK 才走新路径；
   - 测试 `inspector_falls_back_to_pre_when_no_frontmatter` + `inspector_falls_back_to_pre_on_parse_error` 显式断言 `<pre data-testid="pre-fallback">` 出现且 `data-testid="frontmatter-view"` 不存在；
   - 解析失败时 `parsed.body` 是 markdown 原文（含 `---` 头），与原始 `<pre>` 显示完全一致；

3. **`useMemo` 解析缓存键**（性能 reviewer 关注项）：
   - `useMemo` 依赖数组用 `[content?.structuredMd]`（字符串字面），不是 `[content]`（引用）；
   - asset 切换时 `content` 引用变 + `structuredMd` 字面变 → 重新解析（符合预期）；
   - 同一 asset 因父组件 re-render 触发的 `content` 引用相等情况下，依赖数组字面相等 → 不重解析（符合预期）。

4. **`kcEnrichedLabel` 默认分支处理**：
   - `switch` 用 `case "true"/"partial"/"false"`，所有未识别值（null / undefined / 脏数据如 `"unknown"`）走 `default` 返回 `null`；
   - 调用方 `kcLabel === null` 时 `return null` 整行不渲染 —— 防御脏 DB 行；
   - 测试 `inspector_displays_kc_enriched_none_for_history` 显式断言 `kc_enriched=null` 时整行不存在。

5. **a11y 选择**（TD-2）：
   - `FrontmatterTagsView` 选 `role="list"` 而非 `<ul>` —— 现有 DOM 结构（`<div>` + `<span>` chip）的 visual layout 用 flex/gap 已成型，改 `<ul>` 会破坏 visual；用 `role` 显式声明语义，最小侵入；
   - `FrontmatterSummaryView` visible "AI" badge 是新增 `<span data-testid="ai-badge">`（task_017 原本只有图标 + 文字 "AI 摘要"，无单独 badge）—— task_017 reviewer TD-2 要求"若 isAi=true 加 visible badge 'AI'（已存在则确认正确）"，这里按"未存在 → 补"处理；
   - badge 加 `aria-hidden="true"` 避免与根 `aria-label="AI 摘要"` 双重朗读。

6. **`(无标签)` 分支不需要 a11y**（边界）：
   - `FrontmatterTagsView` 当 `ai.length === 0 && rule.length === 0` 时 early return 渲染纯文本 "（无标签）"，**不需要** `role="list"`（没有 item），保持原貌；
   - 测试 `FrontmatterTagsView_handles_empty` 仍能通过。

## 对约束的遵守声明

- [x] 不改 `enrichment.rs` / `scheduler.rs` / `db/*`（纯前端 task）
- [x] 不引入新依赖（`react-markdown` / `remark-gfm` / `js-yaml` 由 task_017 落地）
- [x] react-markdown 禁用 raw HTML（v9 默认配置，不挂 rehype-raw）
- [x] markdown 渲染区最大高度 240px（与原 `<pre>` 一致，class `max-h-[240px] overflow-y-auto`）
- [x] remark-gfm 支持 GFM 表格（测试 `inspector_renders_summary_and_tags_for_kc_enriched_md` 用 GFM table 断言 `<table>` 出现）
- [x] 不引入 KaTeX / mermaid
- [x] fallback `<pre>` 字面保留（测试覆盖）
- [x] 复制按钮逻辑零改动（task_026 行为字面 round-trip）
- [x] TypeScript `tsc --noEmit` 0 error
- [x] 0 退化（44 fail = baseline 44 fail，9 个 failed file 与本 task 文件零交集）
- [x] 与 task_025（DropzoneApp，并行 task）零文件交集
