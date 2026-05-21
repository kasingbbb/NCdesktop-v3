# Task 输入 — task_006_dev_pr2_subdir_direct_import

## 目标
F4 子目录直接归类：前端 dropzone 透传当前 `workspaceFolderRelativePath`；后端解析为有效 `category_slug`（含 alias）则跳过 LLM 路径决策，绑定该 slug；否则 fallback LLM。AI 摘要 / 标签后台并行不变。

## 前置条件
- 依赖 task：task_005
- 必须先存在的文件/接口：`workspace.rs::assert_scope`、`categories` 表、`category_aliases` 表

## 验收标准（AC）
1. `import_files` 命令新增可选入参 `workspace_folder_relative_path: Option<String>`（兼容旧调用方）
2. 后端解析逻辑：
   - 若 path 为 `__ROOT__` 哨兵 → 走 LLM 路径决策（旧行为）
   - 若 path 第一段命中 `categories.slug` 或 `category_aliases.alias_slug` → `category_slug = 解析结果`，跳过 LLM 路径决策
   - 否则 → fallback LLM
3. AI 摘要 / 标签仍后台并行执行
4. 前端 `Dropzone` 组件读取 `uiStore.workspaceFolderRelativePath` 并透传
5. feature flag `subdir_direct_import` 默认 on（PR 合并即生效，可关闭）
6. 资产经 Tauri event `workspace:asset-changed` 推送，前端 1 秒内可见
7. 单测：(a) `__ROOT__` 走 LLM (b) 已知 slug 跳过 (c) alias 解析 (d) 未知 slug fallback (e) 跨项目拒绝

## 技术约束
- 命令向后兼容（可选参数）
- alias 查询走 `category_aliases` 索引
- 跳过 LLM 路径决策不等于跳过整个 LLM；摘要 / 标签仍调

## 参考文件
- `commands/dropzone.rs::resolve_import_project_id` + L347 topics 写入
- `src/stores/uiStore.ts:32-33`（workspaceFolderRelativePath, "__ROOT__" 哨兵）
- `src/components/features/Dropzone.tsx`
- task_001 output.md §dropzone 接口扩展
- PRD §10 Conductor 桥接摘要 §未达成共识 §2

## 预估影响范围
- 修改：`commands/dropzone.rs`（+200）、`src/lib/tauri-commands.ts`（命令签名）、`src/components/features/Dropzone.tsx`（透传）
- 测试：`src-tauri/tests/dropzone_subdir.rs`（新 ~120）、前端 e2e 测试

## Reviewer 重点关注
- alias 解析后是否更新 alias 命中计数（v2，本 task 不做但留 hook）
- feature flag off 时是否完全退化为旧行为
- AI 摘要并发时的写写冲突（应共用 task_003 的 single-writer queue）
