# Review Scorecard — task_004_dev_m3_asset_id_commands

## 审查前验证

- [x] 测试结果存在且非空（cargo test db::asset 22 PASS / commands::asset 13 PASS / npm run check 0 error）
- [x] 自测验证矩阵存在，正常 + 边界 + 异常路径全部 PASS
- [x] 架构遵守声明已填写，含 2 处偏离说明

→ 交付完整，进入实质审查。

## 审查思考过程

### 1. Task 意图
落地 ADR-007 "命令链 asset_id 化"在 M3 rename 路径：
- 新增 `db::asset::resolve_asset_pair`（双向解算 root / derivative）
- 新增命令 `rename_asset(asset_id, new_display_name) -> WorkspaceAssetView`，双写 root.name + derivative.name，**不动磁盘文件名**
- 前端 `renameAsset` wrapper + store action 替代旧 `updateAsset`

### 2. AC 逐条检查

| AC | 状态 | 核查证据 |
|---|---|---|
| AC-1 `resolve_asset_pair` 双向 + Err("素材不存在") | ✅ | `db/asset.rs:132-152`；4 个单测覆盖 root 输入 / derivative 输入 / 无 derivative / 缺失 |
| AC-2 rename 校验 + 双写 + 不动磁盘 + 返回 WorkspaceAssetView | ✅ | `commands/asset.rs:317-344`；`validate_display_name` 用 `trimmed.len()`（Rust str::len 是字节数，UTF-8 安全）；测试断言 `file_path` 不变 |
| AC-3 rename(root_id, "新名.pdf") → derivative.name = "新名.md" | ✅ | `rename_double_writes_root_and_derivative` |
| AC-4 rename(derivative.id) 反解 root 后双写 | ✅ | `rename_via_derivative_id_resolves_to_root` 断言 view.id == root.id |
| AC-5 旧 update_asset 保留 + 前端 rename 切换 | ✅ | grep 全局：仅 `assetStore.updateAsset` 一处保留（带 @deprecated），无 UI 真实调用面；rename 唯一入口走 `renameAsset` |
| AC-6 lib.rs 注册 `rename_asset` | ✅ | `lib.rs:110`，紧跟 update_asset 之后单行追加，未重排其他命令 |
| AC-7 cargo test 全过 + ≥ 4 新单测 | ✅ | db::asset +4，commands::asset +7，共 +11 |

### 3. 领域审查重点核对

- **Asset ↔ Derivative 关系**：`resolve_asset_pair` 严格按 source_asset_id 解算；rename 双写两行 .name，磁盘 `{id}_{stem}.{ext}` 命名不动 → 守住 ADR-006 / ADR-001。
- **rename 作用于逻辑资产**：root.name 写用户原始输入（保留 `/`），derivative.name 走 `sanitize_stem` + `.md`；列表查询经 `list_root_assets`，不会复出双条目。
- **跨 await 持锁**：`rename_asset` 是同步 `#[tauri::command]`（非 async），MutexGuard 在显式作用域内释放，无 cross-await 风险。
- **outbound 落盘文件名一致性（task_005 接口预留）**：display_name 仅活在 DB；rendition file_path 字段不变 → task_005 在 `prepare_outbound_payload` 中读 WorkspaceAssetView.name 做 sanitize 即可产生与 derivative.name 一致的 cache 文件名。接口面对齐充分。

### 4. 关键设计验证

- **`derivative_name_from_root` 不用 `Path::file_stem`**：作者把 `sanitize_stem` 提到 `rfind('.')` 之前，确保 `a/b.pdf` → `a_b` → `a_b.md`，规避 Path 把 `/` 当分隔符切前缀的陷阱（`rename_derivative_name_uses_sanitize_stem` 单测精确锁住）。`idx > 0` 保护 `.env` 类首位 dot 输入。判定：选择合理。
- **AppHandle 入参偏离**：与 task_003 `get_assets` 一致，用于读 `SourceMissingSet`（task_007 注册前 try_state 返回 None 走兼容路径）。前端 invoke 参数未变。判定：合理。
- **UTF-8 字节校验**：`trimmed.len()` 在 Rust 中即 UTF-8 字节数，非字符数；`rename_rejects_over_200_bytes` 用 201 个 ASCII（=201 字节）失败 + 200 字节通过验证边界。判定：正确。

### 5. 关键发现

1. **rename 视图重建经 `list_root_assets` 整项目扫一遍**：作者自述"低频可接受 + 守 ADR-002 单查询入口"，权衡合理。万级 asset 项目极端情况下单次 rename 可能 ≥ 50ms，列入已知 trade-off，不阻塞 PASS。
2. **`update_markdown_derivative` 复用方式的可读性**：rename 调用时传 `d.file_size` 与 `&d.imported_at` 原值，"看起来在改但其实不改"。建议加 1 行注释或后续抽 `rename_markdown_derivative_only_name` 函数（MINOR）。

## 评分

权重来自 session_context.md §4：

| 维度 | 权重 | 分数 | 说明 |
|---|---|---|---|
| 功能正确性 | 25% | 5 | AC 全过，双向解算、双写、不动磁盘、错误文案全部通过单测锁定 |
| 用户体验 | 25% | 4 | rename 返回 WorkspaceAssetView 让前端就地 patch，避免整列表重拉；错误文案中文；唯一遗憾是 UI 暂无 rename 入口（task_008 接线） |
| 架构一致性 | 20% | 5 | 完全遵守 ADR-001/002/007，零新增 migration，db 不做 IO，命令仅 asset_id；AppHandle 偏离与 task_003 同模式 |
| 代码质量 | 10% | 4 | `rename_asset_inner` 抽离便于测试是优点；`update_markdown_derivative(... d.file_size, &d.imported_at)` 传原值语义略隐晦 |
| 测试覆盖 | 10% | 5 | 11 个新单测覆盖正常 / derivative 输入 / sanitize / 空白 / 超长 / 缺失；边界 200 字节通过 + 201 拒绝 |
| 可维护性 | 10% | 4 | sanitize 选型有清晰注释；deprecated 标注引导迁移；命名一致 |

**加权综合分：4.65 / 5**

## 总体判断

- [x] **PASS**

无 BLOCKER，无 MAJOR。偏离 (a) AppHandle 入参 / (b) sanitize_stem + rfind 替代 Path::file_stem 均判定合理。

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR

1. **`update_markdown_derivative` 在 rename 中传原值"假更新"语义不直观**
   - 位置：`commands/asset.rs:271-277`
   - 建议：在调用处加注释明确 `file_size` / `imported_at` 是占位原值；或后续抽 `db::asset::update_markdown_derivative_name_only(conn, id, new_name)` 专函（与 task_006 删除级联可一并考虑）。
2. **`derivative_name_from_root` 先 sanitize_stem（含 120 char 截断）后切扩展**
   - 位置：`commands/asset.rs:239-247`
   - 现象：极长 root.name 会先被截到 120 char，可能把 `.pdf` 截掉；最终结果仍是合法 `.md` 文件名（≤ 123 byte），不构成 bug，但与"先剥扩展再 sanitize"的直觉相反。建议加注释说明顺序由 sanitize_stem 对 `/` 的替换需求驱动。
3. **rename 视图重建走 `list_root_assets` 整项目**
   - 位置：`commands/asset.rs:283-287`
   - 已知 trade-off，作者已在"已知局限 1"标注。无需修复，但未来若工作区达到万级且 rename 成为热点，需补 `load_root_view_by_id`。

## 给 Dev 的修复指引

无需修复，直接 PASS。MINOR 项可作为后续 task（task_005/006）顺手优化时合并处理。

## 偏离判定

| 偏离 | 判定 |
|---|---|
| (a) `rename_asset` 增 `AppHandle` 入参（同 task_003 模式，前端 invoke 不变） | **接受** —— 与 ADR-007 / task_007 SourceMissingSet 注入预留对齐 |
| (b) `derivative_name_from_root` 用 `sanitize_stem + rfind('.')` 替代 `Path::file_stem` | **接受** —— 实测单测锁住 `a/b.pdf → a_b.md`，规避 Path 切前缀陷阱；`idx > 0` 保护 `.env` 类输入 |
