# Task 输入 — task_010_dev_inspector_meta

## 目标
在 `InspectorExtraction.tsx` 展示转换元数据：是否走了 fallback、耗时、错误类别、转换器版本。让用户能区分"成功 / 已 fallback / 失败"三态。

## 前置条件
- 依赖 task：task_009（`getConversionMeta` 命令可用）

## 验收标准（AC）
1. **AC-1**：Inspector 在 `status === "extracted"` 区域底部新增一行"转换信息"：
   - 转换器名 + 版本（如 `MarkItDown 0.1.5` 或 `内置 PDF 文本提取 (builtin)`）
   - 若 `fallbackUsed === true` → 显示"已自动回退到 {内置提取器名}"（warning 文案色）
   - 耗时（`conversion_ms` 毫秒，>1000 ms 显示为秒）
2. **AC-2**：在 `status === "failed"` 区域新增 `errorClass` → 中文化文案映射（`markitdown_not_installed` → "未安装 MarkItDown"等）；不展示原始 stderr。
3. **AC-3**：新增 `extractionStore.fetchConversionMeta(assetId)` 在 fetchExtractedContent 完成后被调用；结果缓存到 `conversionMetaCache[assetId]`。
4. **AC-4**：用户主测：
   - 成功路径：UI 看到 `MarkItDown 0.1.5 · 1.2s`，无 fallback 提示
   - Fallback：UI 看到 `内置 PDF 文本提取 · 已自动回退 · 0.4s`
   - 失败：UI 看到 `未安装 MarkItDown`，下方"重试"按钮可点
5. **AC-5**：暗色模式下颜色对比度通过（warning 文案不刺眼也不太弱）。

## 技术约束
- 中文化映射集中在一个 `errorClassLabel(err)` / `extractorLabel(name)` 函数；不在 JSX 内分散三元。
- 复用现有 `extractorLabel`（line 20-33），追加 `materialized_markdown`、`source_markdown` 等已有 key 的覆盖检查。
- Tauri 调用经 `src/lib/tauri-commands.ts`，禁止在组件内直接 `invoke`。
- 不引入新 UI 库；颜色用 `var(--*)` token。

### ⚠️ 与 PM 手动改动的冲突 guard
PM 于 2026-05-12 手动改动了以下与本 task 相邻的文件，**本 task 禁止触碰**：
- `src/components/layout/Inspector.tsx`（108 行改动，仅作为父容器存在；本 task 只改子组件 `InspectorExtraction.tsx`）
- `src/components/layout/InspectorDetails.tsx`（80 行改动，与本 task 无关）
- `src/components/layout/Sidebar.tsx` / `Toolbar.tsx` / `SidebarFooter.tsx` / `SidebarItem.tsx`
- `src/components/features/AssetListView.tsx`
- `src/components/features/TagTree.tsx`
- `src/components/features/dropzone/DropzoneApp.tsx`、`DropzoneIdle.tsx`
- `src/components/features/today/TodayView.tsx`
- `src/components/features/calendar/CourseSection.tsx`
- `src/components/features/KnowledgeHubView/index.tsx`
- `src/components/features/knowledge/ConceptList.tsx`、`KnowledgeAssociationView.tsx`
- `src/components/features/PhotoViewer.tsx`、`ProjectCard.tsx`、`skills/*.tsx`
- `src/stores/uiStore.ts`
- `src/styles/glass.css`、`src/styles/globals.css`

**Dev 在开始前必须 `git status` 确认上述文件仍为 modified 但未提交；不可 stage、不可 commit、不可在它们里加任何 import/JSX**。
若必须从这些文件接收新 props/类型才能完成 AC，必须在 `output.md` 显式标注并征求 PM 确认。

## 参考文件
- `src/components/layout/InspectorExtraction.tsx:1-183`
- `src/stores/extractionStore.ts`
- `src/lib/tauri-commands.ts`
- 架构方案 §六、§十一 task_010

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src/components/layout/InspectorExtraction.tsx`
  - `src/stores/extractionStore.ts`（缓存 + fetch）
  - `src/lib/tauri-commands.ts`（如 task_009 未补全则补 TS 类型）
