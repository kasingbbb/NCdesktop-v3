# Task 输入 — task_008_error_codes_and_failure_code_migration

## 目标
落地 8 类错误码枚举（`FailureCode`）+ `conversion_meta.failure_code` 字段 migration + "失效四元定义"判定函数，替换当前 `exit==0 && stdout==''` 误判成功的逻辑。

## 前置条件
- 依赖 task：无（可与打包链并行；建议先于 task_007）
- 必须先存在的文件/接口：现有 `conversion_meta` 表

## 验收标准（Acceptance Criteria）
1. AC-1：在 `src-tauri/src/extraction/models.rs`（或新建 `failure_code.rs`）定义：
   ```rust
   pub enum FailureCode {
     ERuntimeMissing, EExtraMissingEpub, EScanPdfUnsupported,
     EAudioWrongRoute, EOutputEmpty, EOutputGibberish,
     EOutputNoStructure, ETimeout90s,
   }
   ```
   含 `as_str()`（`SCREAMING_SNAKE_CASE`）与 `Display`。
2. AC-2：`db/migration.rs` 新增 migration：
   ```
   ALTER TABLE conversion_meta ADD COLUMN failure_code TEXT NULL;
   CREATE INDEX idx_conversion_meta_failure_code ON conversion_meta(failure_code);
   ```
   migration 幂等（已存在列不报错）。
3. AC-3：`db/conversion_meta.rs` 增 `update_failure_code(asset_id, code: Option<FailureCode>)`；写入 `success` 时 failure_code 必须为 None。
4. AC-4：实现 `fn classify_output(stdout: &str, exit_code: Option<i32>, elapsed: Duration) -> Result<(), FailureCode>`：
   - 非 0 退出 → `ETimeout90s`（若 elapsed ≥ 90s）或对应运行时码；
   - 退出 0 但 trim 后为空 → `EOutputEmpty`；
   - 非 UTF-8 或可打印字符 < 50% → `EOutputGibberish`；
   - 无标题且无段落 → `EOutputNoStructure`；
   - 否则 `Ok(())`。
   单测覆盖每条分支。
5. AC-5：`markitdown.rs` 调用 `classify_output` 替换现有"空字符串=成功"分支；image 空回退保留（task_011 保留矩阵），但走 `extractor_type="markitdown_image_fallback"` 而非误标 success。
6. AC-6：前端错误码 → 文案映射表（i18n）含 8 条 + `legacy_unverified`（task_014 用）。

## 技术约束
- migration 必须前向兼容：旧 DB 升级后旧记录 failure_code = NULL（未判定），task_014 再回填。
- `FailureCode` 序列化为字符串而非整数（forward compat）。
- 严禁修改 `audio_asr_iflytek.rs`（PRD 底线 #4）。

## 参考文件
- `src-tauri/src/extraction/extractors/markitdown.rs:120-160`（现有判定逻辑）
- `src-tauri/src/db/conversion_meta.rs`、`db/migration.rs`
- Debate Layer 2 共识：8 错误码 + 四元定义

## 预估影响范围
- 新建：`src-tauri/src/extraction/failure_code.rs`
- 修改：`models.rs`、`extractors/markitdown.rs`、`db/conversion_meta.rs`、`db/migration.rs`、前端 i18n
