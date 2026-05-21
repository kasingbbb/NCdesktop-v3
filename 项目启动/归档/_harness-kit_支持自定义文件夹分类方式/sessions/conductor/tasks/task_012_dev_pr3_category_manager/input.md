# Task 输入 — task_012_dev_pr3_category_manager

## 目标
F11 `CategoryManager` 平铺 CRUD（新增 / 重命名 / 启停 / 删除）+ 后端 `commands/categories.rs`；删除按钮仅在 `is_builtin=0` 且引用计数=0 时显式可见。

## 前置条件
- 依赖 task：task_002（categories 表）+ task_009（categoryStore 已存在）
- 必须先存在的文件/接口：`categories` 表 / `category_aliases` 表 / `categoryStore`

## 验收标准（AC）
1. 新建 `src-tauri/src/commands/categories.rs`，实现：`list_categories`、`create_category`、`rename_category`、`set_category_disabled`、`delete_category`、`add_category_alias`
2. slug 白名单 `[a-z0-9一-龥_-]`，长度 1-32；保留字 `__uncategorized__`、`__archived__`、`other` 拒绝
3. `delete_category` 前置：`is_builtin=0` AND `assets WHERE category_slug=? COUNT=0`；否则返回中文错误
4. `rename_category` 仅改 label；slug 不动；rename slug 走"新 row + alias 历史"路径（v2）
5. `CategoryManager` UI：表格 + 新增按钮 + 行内编辑 label + 启停 toggle + 删除（条件可见）
6. 自定义分类 vs 内置 PARA 视觉区分
7. 单测：(a) 白名单拒绝 (b) 保留字拒绝 (c) 引用计数=0 才能删 (d) UNIQUE 冲突
8. 与 categoryStore 双向同步（变更后 fetch refresh）

## 技术约束
- TS 严格 + Zustand 副作用集中
- 错误中文友好

## 参考文件
- task_001 output.md §API 设计 + §数据模型
- task_009（categoryStore）

## 预估影响范围
- 新建：`commands/categories.rs`（~250）、`components/settings/CategoryManager.tsx`（~250）
- 修改：`lib.rs`、`tauri-commands.ts`、`categoryStore.ts`（加 mutation actions）
- 测试：`src-tauri/tests/categories.rs`（~100）

## Reviewer 重点关注
- 删除条件的双重校验（前端禁用 + 后端拒绝）
- slug 白名单中 CJK 范围 `一-龥` 是否过宽（建议同时限长度）
- rename 与 alias 表的未来升级路径不被本 task 阻断
