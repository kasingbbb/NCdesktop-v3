# Task 输入 — task_010_rust_read_text

## 目标

新增 Rust 命令 `read_asset_text_content` 与 TS 包装 `readAssetTextContent`，按 v2.1 PRD §F-07 + ADR-009 的优先级链读取 asset 文本。

## 前置条件

- 既有：`src-tauri/src/commands/asset.rs`（参考其中 `move_asset_to_workspace_folder` 的 db / state 用法）
- 既有：`src-tauri/src/lib.rs` 命令注册中心
- 既有：`src/lib/tauri-commands.ts`（参考 `moveAssetToWorkspaceFolder` 风格）
- 数据访问：DB 中 `analysis` 字段（含 `ocrText`、`summary`）须已可读

## 验收标准

1. **AC-1**：接口 `read_asset_text_content(asset_ids: Vec<String>) -> Result<Vec<String>, String>`，返回数组长度与输入相等且顺序一致。
2. **AC-2**：对 `.md` 文件 → 返回 `fs::read_to_string` 的结果。
3. **AC-3**：非 `.md` 但 `analysis.ocrText` 非空 → 返回 ocrText。
4. **AC-4**：非 `.md` 且 ocrText 为空但 `analysis.summary` 非空 → 返回 summary。
5. **AC-5**：以上都不满足 → 返回空字符串 `""`（不抛 Err）。
6. **AC-6**：单个 asset 读 Markdown 文件失败（文件丢失/权限）→ 回退到 ocrText/summary，不污染其他 asset 的返回。
7. **AC-7**：`asset_ids` 中某 id 不存在 DB → 返回 Err（与现有命令风格一致）。
8. **AC-8**：在 `lib.rs` 注册 `read_asset_text_content`；`tauri-commands.ts` 导出 `readAssetTextContent(assetIds: string[]): Promise<string[]>`。

## 技术约束

- 同步读，单文件 ≤ 2MB 不强制限制（PRD §4 非功能要求 ≤300ms，作为运行时观察指标，不写阻断逻辑）。
- 不修改既有 DB schema。
- 复用 `database.conn.lock()` + `db::asset::get_by_id`。

## 参考文件

- `src-tauri/src/commands/asset.rs`（`move_asset_to_workspace_folder` 实现风格）
- `src-tauri/src/lib.rs`（命令注册位置）
- `src/lib/tauri-commands.ts` L128（`moveAssetToWorkspaceFolder` TS 包装风格）
- v2.1 PRD §F-07 + 本次 architect output §ADR-009

## 预估影响范围

- 新增：Rust 函数 ~60 行 + 注册 1 行 + TS 包装 ~8 行
