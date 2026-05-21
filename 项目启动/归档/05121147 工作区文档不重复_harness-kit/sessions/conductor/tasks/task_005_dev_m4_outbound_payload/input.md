# Task 输入 — task_005_dev_m4_outbound_payload

## 目标
实现 `prepare_outbound_payload(asset_ids)` 命令：返回 done 态资产的 outbound MD 文件路径数组；非 done 态或混合态返回结构化错误，前端据此 toast 并禁用 startDrag。落盘走 hardlink → 跨卷 copy fallback，缓存在 `~/Library/Caches/NCdesktop/outbound/{asset_id}/`。

## 前置条件
- 依赖 task：task_003（WorkspaceAssetView + compute_asset_state 已稳定）
- 必须先存在的文件/接口：
  - `db::asset::resolve_asset_pair`（task_004 提供，但本 task 可并行：若未完成，本 task 用 `find_markdown_derivative` 自行解算）
  - `db::asset::compute_asset_state`（task_003）
  - `dirs_next` crate（已在依赖）

## 验收标准（AC）
1. **AC-1**：新建 `src-tauri/src/commands/outbound.rs`，导出：
   ```rust
   pub struct OutboundEntry { asset_id: String, path: String, display_name: String }
   pub async fn prepare_outbound_payload(database: State<Database>, asset_ids: Vec<String>) -> Result<Vec<OutboundEntry>, String>
   ```
   错误以 JSON 字符串返回（`serde_json::to_string(&OutboundError {...})`），前端解析为结构化错误。
2. **AC-2**：sanitize 规则严格按 PRD §4.4：`/`/`\` → `_`；删除 U+0000–U+001F + U+007F；保留 CJK / emoji；Windows 保留字 + 尾随 `.`/空格 → 追加 `_`；UTF-8 长度截断到 200 字节并对齐字符边界；若被截断追加 `_<asset_id 前 8 位>`。新增 `commands::outbound::sanitize_outbound_filename(display_name, asset_id) -> String`，单测覆盖 6 个规则分支。
3. **AC-3**：缓存目录策略（ADR-005）：
   - 路径：`dirs_next::cache_dir().unwrap().join("NCdesktop/outbound").join(asset_id)`
   - 每次调用本命令前 `remove_dir_all` 该 asset_id 子目录再 `create_dir_all`（幂等）
   - 文件名 = `sanitize_outbound_filename(display_name, asset_id)`
4. **AC-4**：落盘逻辑：`fs::hard_link(rendition_path, cache_path)` → 失败若 `kind == CrossesDevices` 走 `fs::copy`；其它 IO 错误返回 `IoFailed`。
5. **AC-5**：状态校验：
   - 单选非 done → `Err(StateNotDone { asset_id, state })`
   - 多选混合（至少一条非 done）→ `Err(MixedStates { offending: Vec<asset_id> })`
   - 任一 rendition 不存在 → `Err(RenditionMissing { asset_id })`
6. **AC-6**：在 `lib.rs` 注册 `prepare_outbound_payload`。前端 wrapper `lib/tauri-commands.ts` 新增 `prepareOutboundPayload(assetIds)`。
7. **AC-7**：`cargo test -p app_lib --lib commands::outbound` 通过；至少 8 个单测（sanitize 6 个 + happy path + 状态错误）。
8. **AC-8**：本任务**不**实现 NSStringPboardType 双 representation（按 ADR-008 列入 Phase 1 末 spike）；当前 plugin 写入 NSFilenamesPboardType 由文件名带 `.md` 后缀保证 ChatGPT/Claude 桌面端识别。

## 技术约束
- 不在 commands/ 中拼 SQL：状态判定走 task_003 的 `compute_asset_state`；asset 查询走 `db::asset::*`。
- 不裸 `tokio::spawn`。
- 该命令是 `async fn` 但 IO 是同步 fs；可包在 `tokio::task::spawn_blocking` 内或直接同步（IO 不重，预计 < 50ms / 文件）。
- 不引入新外部依赖（dirs_next 已就位）。
- 错误文案中文。

## 参考文件
- `src-tauri/src/commands/dropzone.rs::try_rename_or_copy_remove`（同源 EXDEV 处理模式）
- `src-tauri/src/utils/safe_name.rs`（参考 sanitize 实现，但需按 PRD §4.4 重写到 outbound 专用版）
- `src-tauri/src/extraction/scheduler.rs::write_derivative_md`（rendition 路径格式参考）
- `task_001_architect/output.md` §ADR-005 / §ADR-008

## 预估影响范围
- 新建文件：
  - `src-tauri/src/commands/outbound.rs`
- 修改文件：
  - `src-tauri/src/commands/mod.rs`（pub mod outbound;）
  - `src-tauri/src/lib.rs`（注册命令）
  - `src/lib/tauri-commands.ts`（+ prepareOutboundPayload）
- 估算变更：~450 行（含 ~200 行测试）
