# Task 输入 — task_011_inspector_content

## 目标
在 Inspector 面板中显示素材的提取内容预览（Markdown 渲染），以及在 AssetListView 中添加提取状态角标。

## 前置条件
- 依赖 task：task_010（前端类型和 store）
- 必须先存在的文件/接口：`extractionStore.ts`、`extraction.ts` 类型

## 验收标准（Acceptance Criteria）
1. AC-1：Inspector 面板新增"提取内容"标签页/区域，渲染 `structured_md` 的 Markdown 内容
2. AC-2：提取状态显示：pending（灰色）、extracting（蓝色动画）、extracted（绿色）、failed（红色）、unsupported（灰色划线）
3. AC-3：failed 状态下显示"重试"按钮，点击调用 `retryExtraction`
4. AC-4：extracted 状态下显示"复制文本"按钮，复制 raw_text 到剪贴板
5. AC-5：AssetListView 卡片右上角添加提取状态角标组件 `ExtractionBadge`
6. AC-6：选中素材时 Inspector 自动加载提取内容（调用 `getExtractedContent`）
7. AC-7：提取中的素材显示进度百分比（来自 Tauri Event）

## 技术约束
- Markdown 渲染可使用简单的 HTML 转换（或 `dangerouslySetInnerHTML` + 简易 md-to-html）
- 遵循现有 Inspector 的标签页/区域结构
- 角标组件须轻量（`memo` 优化），不引起列表重渲染
- 使用 Tailwind CSS 样式

## 参考文件
- `src/components/layout/Inspector.tsx` — 现有 Inspector 实现
- `src/components/features/AssetListView.tsx` — 素材列表视图
- `src/stores/extractionStore.ts` — 提取状态 store
- PRD §3.2 F06, F07 — Inspector 预览 + 状态角标

## 预估影响范围
- 新建文件：`src/components/features/extraction/ExtractionBadge.tsx`
- 修改文件：`src/components/layout/Inspector.tsx`（添加提取内容区域）、`src/components/features/AssetListView.tsx`（添加角标）
