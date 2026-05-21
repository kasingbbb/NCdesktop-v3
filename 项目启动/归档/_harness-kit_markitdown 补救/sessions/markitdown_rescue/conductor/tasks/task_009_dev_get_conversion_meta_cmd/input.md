# Task 输入 — task_009_dev_get_conversion_meta_cmd

## 目标
新增 Tauri 命令 `get_conversion_meta(asset_id)` 返回该资产的转换尝试历史（按时间倒序）。

## 前置条件
- 依赖 task：task_006（`conversion_meta` 表 + CRUD）、task_008（已有数据可查）

## 验收标准（AC）
1. **AC-1**：`commands/conversion.rs` 新增 `pub fn get_conversion_meta(database, asset_id) -> Result<Vec<ConversionMetaRow>, String>`，内部调用 `db::conversion_meta::list_by_source`。
2. **AC-2**：响应字段 camelCase（见 architect §六.1）。
3. **AC-3**：在 `lib.rs::invoke_handler` 中注册。
4. **AC-4**：`src/lib/tauri-commands.ts` 新增 `getConversionMeta(assetId)` 与 TS 类型 `ConversionMetaRow`。
5. **AC-5**：手测：拖入 PDF 等待提取完成，调用 `getConversionMeta` 至少返回 1 行；触发失败→fallback 场景，返回至少 2 行。

## 技术约束
- 命令命名风格与既有 `check_markitdown_status` / `convert_asset_to_markdown` 一致（snake_case 注册，camelCase 暴露）。
- 不允许在命令内做业务过滤；纯 DB 查询。
- 错误返回字符串，由前端展示。

## 参考文件
- `src-tauri/src/commands/conversion.rs`
- `src-tauri/src/lib.rs:126-127`（既有命令注册）
- `src/lib/tauri-commands.ts:142-162`

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/commands/conversion.rs`
  - `src-tauri/src/lib.rs`
  - `src/lib/tauri-commands.ts`
