# Task 输出 — task_010_dev_inspector_meta

## 1. 摘要
在 Inspector 的"提取内容"面板新增"转换信息三态展示"：成功路径显示转换器名 + 版本 + 耗时；fallback 场景额外显示 warning 文案"已自动回退到 …"；失败态用中文化 `errorClassLabel` 取代原始 stderr。`extractionStore` 增加 `conversionMetaCache` 与 `fetchConversionMeta`，在 `fetchExtractedContent` 成功后自动拉取最新元数据。

## 2. 实际变更
- 修改文件（3 个，全部在白名单内）：
  - `src/stores/extractionStore.ts`（+17 行）
    - 新增 `import type { ConversionMetaRow }`
    - state 新增 `conversionMetaCache: Record<string, ConversionMetaRow[]>`
    - 新增 action `fetchConversionMeta(assetId)`，失败仅 `console.warn`
    - 在 `fetchExtractedContent` 成功路径调用 `fetchConversionMeta`
  - `src/components/layout/InspectorExtraction.tsx`（+60/-4 行）
    - 顶部新增 `errorClassLabel(cls)`（8 个错误类映射 → 中文）
    - 顶部新增 `formatConversionMs(ms)`（>1000 ms → `Xs` 保留 1 位小数）
    - `extractorLabel` 补充 `source_markdown` / `builtin` 中文映射
    - 组件 useEffect 兜底调用 `fetchConversionMeta(asset.id)`
    - `status === "failed"` 改为 `errorClassLabel(latestMeta?.errorClass)`
    - `status === "extracted"` 区域底部追加"转换信息"行 + fallback warning 行
- **未修改** `src/lib/tauri-commands.ts`（task_009 已加 `ConversionMetaRow` 与 `getConversionMeta`，本 task 只 `import type` 引用）。

## 3. 接口契约
- 前端 ↔ store：
  - `useExtractionStore().conversionMetaCache[assetId]: ConversionMetaRow[] | undefined`
  - `useExtractionStore().fetchConversionMeta(assetId): Promise<void>`
- store ↔ tauri：仍走 `cmd.getConversionMeta(assetId)`（task_009 提供）。
- 无新增对外类型，复用 `ConversionMetaRow`。

## 4. 测试结果

### 4.1 TypeScript typecheck（仅本 task 修改文件，根目录全量）
```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && npx tsc --noEmit 2>&1 | tail -30
```
输出：**空**（即 0 error，0 warning）。全工程 `tsc --noEmit` 无任何报错；本 task 新增/修改代码无类型问题，PM 手改文件也未引入 TS error。

### 4.2 手测脚本（无 GUI 环境无法实跑，作为可执行 QA 步骤登记）
| 步骤 | 操作 | 期望 UI |
|---|---|---|
| 1. 成功路径 | 拖入正常 PDF → 等状态变为 extracted | 「转换信息：MarkItDown 0.1.5 · 1.2s」，无 fallback 行 |
| 2. Fallback | 临时卸载 markitdown（或环境 `python -m markitdown` 报错），拖入同样 PDF | 「转换信息：内置 PDF 文本提取 builtin · 0.4s」+ warning 行「已自动回退到 内置 PDF 文本提取」 |
| 3. 失败 | 拖入 0 字节或损坏 PDF | 红色文案「转换出错」/「转换输出为空」（根据 errorClass），"重试"按钮可点 |
| 4. 暗色模式 AC-5 | 切换系统暗色 | warning `#FF9500`（或 `--color-warning`）可读不刺眼，tertiary 灰可见 |

## 5. 已知局限 / 风险
- **无 GUI 主测**：本环境无 Tauri/桌面运行能力，AC-4 三态实际渲染需用户在本地 dev server (`pnpm tauri dev`) 验证。
- `--color-warning` token 当前未在 `globals.css` 定义；本 task 用 CSS fallback 语法 `var(--color-warning, #FF9500)` 兼容，并在代码注释中标注后续应集中提取到 token（不能改 globals.css，PM 已改）。
- `latestMeta` 取数组 `[0]`，依赖后端 `get_conversion_meta` 返回按 `converted_at DESC` 排序（task_009 已实现）。若后端排序改变，需同步调整。
- 失败态原 `content?.errorMessage` 不再展示；若 errorClass 为 null（极端老数据未补 meta），fallback 文案为"提取失败"，可能比原 stderr 信息少。这是 AC-2 显式要求"不展示原始 stderr"的必要取舍。

## 6. 需要 Reviewer 特别关注
- **未触碰 PM modified 文件**清单（已 `git status` 确认仍为 M、未被本 task 二次修改）：
  - `src/components/layout/Inspector.tsx`
  - `src/components/layout/InspectorDetails.tsx`
  - `src/components/layout/Sidebar.tsx` / `Toolbar.tsx` / `SidebarFooter.tsx` / `SidebarItem.tsx`
  - `src/components/features/AssetListView.tsx` / `TagTree.tsx`
  - `src/components/features/dropzone/DropzoneApp.tsx` / `DropzoneIdle.tsx`
  - `src/components/features/today/TodayView.tsx` / `calendar/CourseSection.tsx`
  - `src/components/features/KnowledgeHubView/index.tsx`
  - `src/components/features/knowledge/ConceptList.tsx` / `KnowledgeAssociationView.tsx`
  - `src/components/features/PhotoViewer.tsx` / `ProjectCard.tsx` / `skills/*.tsx`
  - `src/stores/uiStore.ts`
  - `src/styles/glass.css` / `globals.css`
- 验证命令：
  ```bash
  cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop && git diff --stat 项目启动/NCdesktop/src/lib/tauri-commands.ts 项目启动/NCdesktop/src/components/layout/InspectorExtraction.tsx 项目启动/NCdesktop/src/stores/extractionStore.ts
  ```
  实际输出：3 files changed, 96 insertions(+), 4 deletions(-)。

## 7. 后续 / Open items
- 提取 `--color-warning` 到 `globals.css` 时，需要 PM 协调（globals.css 在 PM 改动里）。
- `formatConversionMs` 与 `errorClassLabel` 若多处复用，可下沉到 `src/utils/extraction.ts`；本 task 暂保留在 `InspectorExtraction.tsx`，避免引入新文件。
- 暗色对比度（AC-5）需用户主测确认。

## 8. 交付清单
- 代码：见 §2
- 测试：见 §4
- 文档：本 output.md
- 无新增依赖，无 schema 迁移，无后端改动
