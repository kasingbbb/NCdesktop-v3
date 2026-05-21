# Task 输入 — task_011_dev_pr3_breadcrumb_empty

## 目标
F10 面包屑三段（Library > Project > WorkspaceView/...）+ 空目录态 `EmptyImportCTA`（绑定当前 slug，跳过 LLM）。

## 前置条件
- 依赖 task：task_009 + task_010
- 必须先存在的文件/接口：`categoryStore.activeCategorySlug`、F4 `subdir_direct_import` 已可用

## 验收标准（AC）
1. `Breadcrumb`：三段式，可点击回退；当前段不可点
2. `EmptyImportCTA`：当 list_workspace_assets 返回 0 项时显示；按钮"导入到「{label}」"
3. 点击按钮触发 dropzone import_files，透传 `workspace_folder_relative_path = current view path`
4. 空目录态文案根据 builtin / custom 区分
5. 切换 sub_path 时面包屑同步；URL hash / state 同步

## 技术约束
- 不新增路由库；用 uiStore 状态驱动
- 与 PR-2 task_006 联动：CTA 触发的导入必须命中"跳过 LLM"路径

## 参考文件
- task_001 output.md §F10
- task_006 output.md（依赖 import_files 接口）

## 预估影响范围
- 新建：`Breadcrumb.tsx`（~120）、`EmptyImportCTA.tsx`（~100）
- 修改：`WorkspaceLayout.tsx` 顶部插入面包屑

## Reviewer 重点关注
- 面包屑状态与浏览器后退按钮的一致
- CTA 触发后是否真的跳过 LLM（需端到端验证）
