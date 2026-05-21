# Task 交付 — task_011

`Breadcrumb.tsx`（三段：library > project > category）+ `EmptyImportCTAStub`（嵌入 FolderListView）。

## 偏离声明
- `EmptyImportCTA` 独立组件 + dropzone 触发暂用文案提示；实际触发导入逻辑（与 PR-2 task_006 联动）留 task_017。
- URL hash / state 同步：MVP 无路由库，留 v2。

## 文件
`src/components/features/Breadcrumb.tsx`（新）

**PASS** 3.8/5
