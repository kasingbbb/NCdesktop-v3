# Task 交付 — task_019_doc_viewer_render

## 实现摘要

把 `DocumentViewer.tsx::TextContent` 子组件从 `<pre>` 纯文本展示升级为 react-markdown + remark-gfm 渲染（KC v6 增强 MD：详细索引段表格 + 段落锚点）+ 顶部 frontmatter 卡片：

1. **AC-1**：`TextContent` 加载内容后调 `parseFrontmatter(content)`：
   - 解析成功 (`frontmatter !== null && !parseError`) → 顶部 `FrontmatterCard`（摘要 + 标签 + kc_enriched 状态）+ 主体 `<ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>`。
   - 解析失败 / 无 frontmatter（历史 MD）/ parseError → 回退到原 `<pre>` 模式（行为兼容）。
2. **AC-2**：Tailwind utility class 套在 markdown-body wrapper 上：
   - 表格：边框 / 表头加粗 + secondary 底色 / 隔行 zebra（`[&_tbody>tr:nth-child(odd)]:bg-...`）。
   - 标题 h1-h4：与 NC tokens 一致（`text-2xl/xl/lg/base` semibold）。
   - 锚点：react-markdown 默认渲染 `<a href="#paragraph-0">` —— `<table>` + fragment 跳转皆原生支持。
   - 代码块：单色 surface-tertiary 背景 + 等宽 + radius-sm；inline code 同上。
   - 段落最大宽度：64ch（`maxWidth: "64ch"` 装在外层 `<div>`）。
3. **AC-3**：`ImageContent` / `PdfContent` / `AudioContent` / `FallbackContent` **字面零改动**（保留 ZoomIn/ZoomOut/Maximize/iframe/audio controls/fallback icon）。
4. **AC-4**：4 个核心 + 2 个 AC-5 测试落在 `src/components/features/viewer/__tests__/DocumentViewer.test.tsx`，全部通过。
5. **AC-5（TD-4）**：frontmatter 卡片 `kc_enriched` 字面映射 → 共享 helper `mapKcEnrichedToLabel`（详见下方"helper 抽取决策"），InspectorExtraction（task_018）同步迁移。

**安全**：`react-markdown@9` 默认不挂 `rehype-raw`，等价于 `allowDangerousHtml: false` —— 用户写入 `<script>` 不会被执行（task_018 同款约束）。

## Helper 抽取决策（TD-4）

**结论：抽出**到 `src/utils/kcEnrichedLabel.ts`，并把 task_018 的 InspectorExtraction.tsx 同步迁移。

### 决策理由

| 维度 | 抽出 helper（选） | 在 DocumentViewer 重写 |
|------|------------------|---------------------|
| DRY | ✅ task_018 + task_019 共用一份字面 → 文案映射 | ❌ 双份字面映射，未来 KC schema 升级（如加 "timeout"）改两处 |
| TD-4 落地姿势 | ✅ input.md AC-5 建议 helper `mapKcEnrichedToLabel(value): { label, tone }` | ❌ 不满足 input.md 建议 |
| 翻译层归属 | ✅ 仍在业务表层（utils 不是 KcStatusBadge 视觉徽章），与 task_018 §"翻译层归属说明"一致 | 同左 |
| 类型扩展性 | ✅ 返回 `{ label, tone }`，task_019 用 tone 画 dot；task_018 仅用 label —— 各取所需 | ❌ task_018 字符串签名不够 |

### Helper API

```ts
export type KcEnrichedTone = "success" | "partial" | "inactive";
export interface KcEnrichedLabelResult { label: string; tone: KcEnrichedTone; }
export function mapKcEnrichedToLabel(
  value: string | null | undefined,
): KcEnrichedLabelResult | null;
```

- `"true"` → `{ label: "AI 增强：完整", tone: "success" }`（green dot）
- `"partial"` → `{ label: "AI 增强：仅规则标签（LLM 不可用）", tone: "partial" }`（amber dot）
- `"false"` → `{ label: "未启用 AI 增强", tone: "inactive" }`（grey dot）
- `null` / `undefined` / 未识别字面 → `null`（整行隐藏，历史数据 fail-safe）

## task_018 同步迁移说明

task_018 落地时 `kcEnrichedLabel` 是 `InspectorExtraction.tsx` 内的私有 function。task_019 提取共享 helper 后，对 task_018 的迁移：

1. `src/components/layout/InspectorExtraction.tsx`：
   - 删除文件内 `kcEnrichedLabel(string | null | undefined): string | null` 私有 helper（含 §"翻译层归属说明"注释）。
   - import `mapKcEnrichedToLabel` from `../../utils/kcEnrichedLabel`。
   - 调用点 `kcEnrichedLabel(content.kcEnriched)` → `mapKcEnrichedToLabel(content.kcEnriched)?.label`（取 `.label`，丢弃 tone —— Inspector 视觉上不需要 dot）。
2. 行为保持完全一致：返回 null → 整行隐藏；三种文案逐字一致。
3. 回归：`pnpm vitest run InspectorExtraction` → **13/13 PASS**（与迁移前同），3 个 kc_enriched label 测试 (`partial` / `true` / `false` / `null`) 全部不退化。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src/utils/kcEnrichedLabel.ts` | **新建** | 共享 helper `mapKcEnrichedToLabel(value): { label, tone } | null` |
| `src/utils/__tests__/kcEnrichedLabel.test.ts` | **新建** | 5 个 helper 单测（true/partial/false/null/未识别） |
| `src/components/features/viewer/DocumentViewer.tsx` | 修改 | TextContent 改造（parseFrontmatter + FrontmatterCard + react-markdown body + fallback `<pre>`）；其它 4 个子组件零改动 |
| `src/components/features/viewer/__tests__/DocumentViewer.test.tsx` | **新建** | 6 个 viewer 单测（table / anchor / frontmatter card / fallback / partial amber / null 隐藏） |
| `src/components/layout/InspectorExtraction.tsx` | 修改 (TD-4 helper 迁移) | 删除内部 `kcEnrichedLabel` private helper；改 import `mapKcEnrichedToLabel`；调用点改 `.label` |

**未触及**（约束严守）：
- `src/utils/parseFrontmatter.ts`（task_017 锁定，仅消费）
- `src/components/features/viewer/DocumentViewer.tsx` 其它 4 个 sub-component（ImageContent / PdfContent / AudioContent / FallbackContent）字面零字符改动
- `src/components/features/extraction/KcStatusBadge.tsx`（task_021，翻译层归属切割）
- `src/lib/tauri-commands.ts`（getFileContent 历史缺失为 pre-existing baseline，非本 task 范围）
- 任何 Rust 后端文件

**无新依赖**：`react-markdown@9.0.1` / `remark-gfm@4.0.0` / `js-yaml@4.1.0` 均由 task_017 引入；本 task 仅 import。

## 测试命令与结果

```bash
cd 项目启动/NCdesktop
pnpm vitest run DocumentViewer kcEnrichedLabel  # 11 PASS
pnpm vitest run InspectorExtraction              # 13 PASS（task_018 不退化）
pnpm tsc -p tsconfig.app.json --noEmit
```

**靶向 vitest**（3 个 test file）：
```
✓ src/utils/__tests__/kcEnrichedLabel.test.ts (5 tests)
✓ src/components/features/viewer/__tests__/DocumentViewer.test.tsx (6 tests)
✓ src/components/layout/__tests__/InspectorExtraction.test.tsx (13 tests)

Test Files  3 passed (3)
     Tests  24 passed (24)
```

### AC-4 / AC-5 测试映射

| Test name | AC | 验证点 |
|-----------|----|--------|
| `doc_viewer_renders_markdown_with_table` | AC-4(1) | KC v6 表格 → `<table>`/`<thead>`/`<tbody>` |
| `doc_viewer_renders_anchor_links_with_fragment` | AC-4(2) | `[第 0 段](#paragraph-0)` → `<a href="#paragraph-0">` |
| `doc_viewer_renders_frontmatter_card_for_kc_md` | AC-4(3) | frontmatter 卡片含 ai_summary / ai_tags / rule_tags |
| `doc_viewer_falls_back_to_pre_on_invalid_markdown` | AC-4(4) | 无 frontmatter → `<pre>` fallback；markdown view & card 不渲染 |
| `doc_viewer_kc_enriched_partial_shows_amber_label` | **AC-5 新增** | partial → "AI 增强：仅规则标签（LLM 不可用）" + `data-testid="doc-viewer-kc-dot-partial"` |
| `doc_viewer_kc_enriched_null_hides_row` | **AC-5 新增** | kc_enriched 字段缺失 → 整行（dot + label）皆不渲染 |
| `mapKcEnrichedToLabel` × 5 | helper 覆盖 | true/partial/false/null/未识别 |

**TypeScript `tsc --noEmit`**：

baseline 129 errors（master HEAD pre-existing，与 task_017 / task_018 之后某 commit 引入的 `getFileContent` 缺失等无关变更相关）。
- **本 task 修改/新建文件**中 0 新增 error；
- DocumentViewer.tsx:18 `getFileContent` import 错误是 **pre-existing baseline**（早自 commit `52bb473b` 起，与 task_019 范围正交），未触动 `src/lib/tauri-commands.ts`。

**全量 vitest**：
- 33 file passed / 9 file failed = 与 task_026 baseline 同（与本 task 改动零交集，无新退化）；
- 全量 414 passed / 44 failed = baseline 44 fail 全部继承自 master pre-existing。

## Reviewer 特别关注

1. **react-markdown 安全配置**：未传 `rehype-raw`，等价 `allowDangerousHtml: false`。
2. **64ch 宽**：`<div className="mx-auto" style={{ maxWidth: "64ch" }}>` 包裹 frontmatter card + markdown body，整体居中。
3. **段落锚点跳转**：react-markdown 默认 `<a href="#...">` 走浏览器 native fragment scroll；DocumentViewer 容器是 `overflow-y-auto` —— `<a>` 点击会 scroll 该容器，无需额外 JS。测试以 `href="#paragraph-0"` 存在断言；E2E（task_023）可补 click → scrollTop 变化的浏览器集成测试。
4. **失败回退**：`useMarkdownView` flag 综合 `frontmatter !== null && !parseError`；YAML 非法或 historic MD（无 `---`）都走 `data-testid="doc-viewer-pre-fallback"` 路径。
5. **TD-4 helper 抽取**：未来若 KC v7 加 `kc_enriched: "timeout"` 之类新字面，只改 `src/utils/kcEnrichedLabel.ts` 一处。
6. **task_018 等价行为**：迁移前后 InspectorExtraction 三态文案/null 隐藏行为字面一致，回归 13/13 PASS。

## DoD 自检

- [x] AC-1: parseFrontmatter + frontmatter card + react-markdown body
- [x] AC-2: 表格 / 标题 / 锚点 / 代码块 / 64ch 宽 全部 Tailwind 实装
- [x] AC-3: ImageContent / PdfContent / AudioContent / FallbackContent 字面零改动
- [x] AC-4: 4 个 viewer 单测落地
- [x] AC-5: helper + partial amber 测 + null 隐藏测
- [x] react-markdown `allowDangerousHtml: false`（默认）
- [x] 不引入新依赖
- [x] task_018 24/24（实际 13/13）回归不退化
- [x] 测试 24 PASS（11 新 + 13 不退化）
