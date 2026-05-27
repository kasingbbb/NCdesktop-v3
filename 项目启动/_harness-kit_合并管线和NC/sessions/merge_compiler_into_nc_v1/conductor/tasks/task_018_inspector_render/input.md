# Task 输入 — task_018_inspector_render

## 目标
改造 `InspectorExtraction.tsx`：将原 `<pre>` 纯文本展示升级为 markdown 渲染 + frontmatter 解析展示（ai_tags / ai_summary 两个核心字段），保留原 `<pre>` 模式作为 fallback。

## 前置条件
- 依赖 task：task_017（parseFrontmatter util + 两个展示组件 + react-markdown 依赖已就绪）
- 必须先存在的文件/接口：
  - `src/utils/parseFrontmatter.ts`
  - `src/components/features/extraction/FrontmatterTagsView.tsx`
  - `src/components/features/extraction/FrontmatterSummaryView.tsx`

## 验收标准（Acceptance Criteria）
1. **AC-1**：修改 `src/components/layout/InspectorExtraction.tsx`：
   - status === 'extracted' 且 content.structuredMd 存在时：
     - 调 `parseFrontmatter(content.structuredMd)` 拿 `{ frontmatter, body, parseError }`
     - 如果 `frontmatter` 不为 null：
       - 顶部展示 `<FrontmatterSummaryView summary={frontmatter.aiSummary} isAi={true} />`
       - 中间展示 `<FrontmatterTagsView aiTags={frontmatter.aiTags} ruleTags={frontmatter.ruleTags} />`
       - 下方展示 body（用 react-markdown + remark-gfm 渲染表格 + GFM 元素），max-height 240px overflow-y-auto
     - 如果 `frontmatter` 为 null 或 `parseError` 存在 → 回退到原 `<pre>` 模式（保持现有行为）
2. **AC-2**：在 KC 元数据展示行追加显示（现有 quality / extractor 信息保留）：
   - `kc_enriched` = "true" → 显示 "AI 增强：完整"
   - `kc_enriched` = "partial" → 显示 "AI 增强：仅规则标签（LLM 不可用）"
   - `kc_enriched` = "false" → 显示 "未启用 AI 增强"
   - `kc_enriched` 为 null（历史数据） → 不显示该行
3. **AC-3**：复制按钮逻辑不变（仍复制完整 structuredMd 含 frontmatter）
4. **AC-4**：单元测试（vitest）：
   - `inspector_renders_summary_and_tags_for_kc_enriched_md`
   - `inspector_falls_back_to_pre_when_no_frontmatter`
   - `inspector_falls_back_to_pre_on_parse_error`
   - `inspector_displays_kc_enriched_partial_label`
   - `inspector_displays_kc_enriched_none_for_history`
5. **AC-5（TD-2 补齐，task_017 reviewer 上抛）**：在本 task 一并补齐 `FrontmatterTagsView` + `FrontmatterSummaryView`（task_017 已落地组件）的 a11y：
   - `FrontmatterTagsView`：根元素 `role="list"` + 每个 tag chip `role="listitem"` + `aria-label="AI 标签 / 规则标签"`（区分两个 tag 来源）
   - `FrontmatterSummaryView`：根元素 `aria-label="AI 摘要"` + 若 `isAi=true` 加 visible badge "AI"（已存在则确认正确）
   - 追加测试：`frontmatter_tags_view_uses_role_list` / `frontmatter_summary_view_has_aria_label`
6. **AC-6（TD-4 落地，task_021 reviewer 上抛）**：本 task AC-2 中的 `kc_enriched`字面 "true"/"partial"/"false" 映射 → 用户可见文案 "完整 / 仅规则标签 / 未启用"，**这一翻译层落在 InspectorExtraction.tsx 而非 KcStatusBadge**（task_021 仅保留 UX 状态 "success"/"failed"/"loading"/"idle"，与 YAML 字面解耦）。dev 在 output.md 明确说明该翻译层归属。

## 技术约束
- react-markdown 渲染时禁用 raw HTML（`allowDangerousHtml: false`）
- markdown 渲染区限制最大高度 240px（与原 `<pre>` 一致），有滚动
- 图表 GFM 表格用 remark-gfm 支持
- 不引入 KaTeX / mermaid 等大插件（本期不需要数学公式）
- 失败回退原 `<pre>` 必须工作（向后兼容）

## 参考文件
- Architect output.md §"ADR-012 V1 分支 B 后的执行计划" + §"目录结构 src/"
- `src/components/layout/InspectorExtraction.tsx:174-234` 现有 extracted 状态渲染
- task_017 input.md（FrontmatterTagsView / FrontmatterSummaryView 签名）

## 预估影响范围
- 新建文件：
  - `src/components/layout/__tests__/InspectorExtraction.test.tsx`（如未存在）
- 修改文件：
  - `src/components/layout/InspectorExtraction.tsx`：注入 frontmatter 解析 + 展示组件 + react-markdown

## Reviewer 重点关注项
- 原 `<pre>` fallback 仍能工作（历史无 frontmatter 的 MD）
- react-markdown 安全配置（不渲染 raw HTML）
- 性能：每次 Asset 切换都重新 parseFrontmatter，是否影响响应（用 useMemo）
- 视觉：标签 + 摘要展示与现有 Inspector 设计语言一致

## 复杂度
S（1d 工作量，~500 行含测试）
