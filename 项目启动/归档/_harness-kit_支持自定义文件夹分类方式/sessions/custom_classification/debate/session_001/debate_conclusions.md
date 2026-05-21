# Debate Conclusions — custom_classification / session_001

> 用于 Architect 直接消费的"决策摘要"，PRD 已基于本结论展开。

## 1. 问题定义
- Bug 1 = 复合缺陷：
  1. `resolve_import_project_id` 未消费 `workspaceFolderRelativePath`（ProjectFolderScope 未注入）
  2. `commands/dropzone.rs:347` 把裸字符串写入 `ai_analyses.topics`（schema 是 JSON array）
  3. LLM 返回 category 含分隔符/空格 + `sanitize_path_segment::take(48)` → display↔slug 不可逆，产生"幽灵分类"
- 三术语：**ProjectFolderRoot**（磁盘）/ **ProjectFolderScope**（导入上下文）/ **WorkspaceView**（UI）

## 2. 理想态
- 分类作用域：**library 级**（project 级 override 留 v2）
- WorkspaceView：**DB 权威**，IPC 暴露聚合 + 子路径分页；不走 FS `read_dir`
- Prompt 编辑器三态 dry-run：在线必过 / 离线 schema-only + `validated_offline=true` / 用户跳过 + 二次确认 + `user_skipped_validation=true`

## 3. 差距 → 设计
- Schema：`categories(id, library_id, slug, label, parent_id, icon, sort_order, is_disabled, is_builtin)` + `category_aliases(slug, alias_label, library_id)` + `assets.category_slug`
- Prompt 持久化：复用 V1 既有 `settings` KV 表，**零新 schema**
- 命令面新增：`list_workspace_assets(project_id, category_slug, sub_path?)` + `commands/prompts.rs` 四件套（list/get/save/restore_default 含 dry_run）
- 前端：`WorkspaceFolderStrip` → `WorkspaceCategorySidebar`（~60% 复用）+ Finder 列表/图标/面包屑

## 4. 策略 / MVP 拆分
| PR | 内容 | 依赖 | 可并行 |
|----|------|------|--------|
| PR-1 | V10 migration + categories 表族 + 内置 PARA 种子 + slug 稳定化 | — | — |
| PR-2 | Bug 1 三连修：ProjectFolderScope 注入 / topics 写入修正 / 启动期自愈扫描 | 不依赖 PR-1 | ✓ 与 PR-1 并行 |
| PR-3 | `list_workspace_assets` + WorkspaceCategorySidebar + Finder 视图 | PR-1 | — |
| PR-4 | Prompt 编辑器（设置面板）+ 三态 dry-run + 占位符校验 + restore-default | settings KV（已有） | ✓ 与 PR-1 并行 |

## 5. 红线 / 不可妥协
- PARA 既有资产 100% 向后兼容（categories_v9_backup 30 天）
- Prompt 占位符（如 `{content}`）缺失即禁止保存
- 工作区映射修复不得引入跨项目串扰
- 自定义分类名禁字符 `/ \ : * ? " < > |` + 长度上限

## 6. 三档降级启动
1. 迁移成功 → 正常
2. 部分失败 → 残留资产归 `__uncategorized__` + 顶部横幅
3. DB 损坏 → 只读安全模式（仅查阅，不允许导入/改类）

## 7. 子目录导入策略
- 在已知 category 视图导入 → **不调 LLM**，直接绑定当前视图 slug
- 本地启发式 mismatch（文件名/MIME 与当前 category 明显冲突）→ toast 让用户确认是否仍导入到当前位置

## 8. 已锁定的待命名约定
- 内部保留 slug：`__uncategorized__`、`__archived__`
- 内置 PARA slug 不可改（只能改 label / 加 alias）：`1-项目` `2-领域` `3-资源` `4-存档`
- `other` 仅作为 LLM 兜底落点，不显示为正式分类（视图归到 `__uncategorized__`）

## 9. 移交 Architect 的关键问题
1. PR-1 的 `categories` 主键：自增 INTEGER vs (library_id, slug) 复合？
2. PR-2 启动期自愈扫描放在 `db/migration.rs` 还是独立 `repair.rs`？
3. PR-3 的 `list_workspace_assets` 分页策略：cursor vs offset？长目录 1k+ 时的内存峰值？
4. PR-4 dry-run 在线探活：复用 `llm/client.rs::ping` 还是新增 `validate_prompt`？
5. WorkspaceCategorySidebar 与现有 `WorkspaceFolderStrip` 的共存/替换策略（feature flag？）

> 这些问题留给 Architect 在 task_001 输出技术方案时回答。
