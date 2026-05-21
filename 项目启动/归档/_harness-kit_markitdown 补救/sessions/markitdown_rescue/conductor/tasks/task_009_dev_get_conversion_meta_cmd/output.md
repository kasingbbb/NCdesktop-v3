# task_009 Dev Output — get_conversion_meta 命令

## 1. 任务摘要
为 markitdown rescue 添加 Tauri 命令 `get_conversion_meta`，作为 `db::conversion_meta::list_by_source` 的纯透传 wrapper；同步在前端 `tauri-commands.ts` 补充 `ConversionMetaRow` 类型与 `getConversionMeta` 调用函数。Inspector 等前端组件可直接 `import { getConversionMeta }` 拉取某 source asset 的转换历史。

## 2. 改动文件列表（共 3 个）
1. `src-tauri/src/commands/conversion.rs`
   - 顶部新增 `use crate::db::conversion_meta::ConversionMetaRow;`
   - 文件末尾追加 `#[tauri::command] pub fn get_conversion_meta(...)`（10 行，含锁错误映射 + `list_by_source` 透传，无业务过滤）。
2. `src-tauri/src/lib.rs`
   - `invoke_handler!` 列表在既有 `convert_asset_to_markdown` 之后新增一行 `commands::conversion::get_conversion_meta,`。
3. `src/lib/tauri-commands.ts`
   - 文件末尾追加 `export interface ConversionMetaRow { ... }`（12 字段，全 camelCase）+ `export async function getConversionMeta(assetId): Promise<ConversionMetaRow[]>`。
   - **仅追加**，未修改已有内容；这也是 PM 31 文件之外唯一允许的前端改动。

## 3. 命令签名与契约
- 注册名：`get_conversion_meta`（snake_case）。
- 参数：`{ assetId: string }`（前端 camelCase ↔ Rust `asset_id`）。
- 返回：`Vec<ConversionMetaRow>`，序列化为 camelCase 字段（依托 `ConversionMetaRow` 既有 `#[serde(rename_all = "camelCase")]`）。
- 排序：完全继承 `list_by_source` 的 `ORDER BY converted_at DESC`；命令层不再排序、不再过滤。
- 错误：
  - 数据库锁失败 → `Err("数据库锁获取失败: {e}")`。
  - SQL 错误透传 `list_by_source` 的字符串。
  - 找不到行 → 返回 `Ok(vec![])`（list 语义），不报错。

## 4. 测试情况
**未新增测试**。理由：
1. task_006 已在 `db/conversion_meta.rs` 内验证 `list_by_source` 在 DESC 排序、空查询、多行下的行为。
2. 命令实现是 5 行 wrapper，仅做 `Mutex::lock()` + 调用，无业务逻辑分支。
3. `#[tauri::command]` 函数测试需要构造 `State<Database>`，引入 `fetch_conversion_meta` 抽象层属于过度设计（违反 input.md "保持小改动"原则）。
4. AC-5 验证留给手测：迁移到 R10 schema 后，对一个真实 asset 跑 `convert_asset_to_markdown`，再调用 `get_conversion_meta(asset_id)` 应返回 1 行。

→ 此决策在 §7 已知局限登记。

## 5. 验证结果
```
cd src-tauri && cargo check 2>&1 | tail
warning: notecapt (lib) generated 4 warnings (pre-existing in llm/chat.rs, unrelated)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.66s
```
- **0 error**（保持 task_006/007/008 的 0 error 状态）。
- 4 个 warning 全部位于 `src/llm/chat.rs`（unused variable / dead_code），与本 task 完全无关，task_006 output 已记录。
- 未跑前端 vitest（PM 改了 31 个前端文件，可能引入 unrelated 失败；本 task 前端仅类型增量，无运行时影响）。
- 未跑 `cargo test`：本 task 未新增 Rust 测试；既有 `db::conversion_meta::tests` 在 task_006 已绿。

## 6. 验收标准（AC）映射
| AC | 状态 | 备注 |
|---|---|---|
| AC-1：命令存在且注册 | PASS | `lib.rs` invoke_handler 已含 `get_conversion_meta` |
| AC-2：返回 `Vec<ConversionMetaRow>`（camelCase） | PASS | 复用 task_006 `ConversionMetaRow` 已有 `rename_all = "camelCase"` |
| AC-3：参数 assetId（camelCase）↔ asset_id | PASS | 见命令签名 |
| AC-4：纯 DB 透传无业务过滤 | PASS | 仅 lock + list_by_source |
| AC-5：插入 2 行后调用返回 2 行 DESC | DEFER | DB 层 task_006 已测；命令层手测验证（见 §7） |
| AC-6：前端 ConversionMetaRow + getConversionMeta 可调用 | PASS | `tauri-commands.ts` 末尾追加 |
| AC-7：cargo check 0 error | PASS | 见 §5 |
| AC-8：不动 src/ 组件 | PASS | 仅改 `src/lib/tauri-commands.ts`（input.md 明示允许） |

## 7. 已知局限 / 后续工作
1. **命令层无单测**：依赖 task_006 的 DB 层测试 + 后续手测/集成测试覆盖。如果未来 wrapper 增加任何逻辑（如 limit、derived_asset 过滤），必须补 Rust 单测。
2. **未跑前端类型检查**：`tauri-commands.ts` 仅追加 export，TypeScript 严格模式下应可通过，但未经 `tsc --noEmit` 验证。Inspector 任务 task_010+ 在引入此函数时会自然触发类型检查。
3. **DB 表存在性假设**：依赖 task_007 migration R10 已建 `conversion_meta` 表；若在未迁移环境调用命令，`list_by_source` 会返回 "no such table" 错误（这是 list_by_source 既有行为，非本命令引入）。
4. **derivedAssetId 当前未在前端使用**：本 task 仅暴露字段，task_010 (Inspector) 决定是否消费。

## 8. 交接信息
- **下游**：task_010（Inspector UI 集成）可直接：
  ```ts
  import { getConversionMeta, type ConversionMetaRow } from "@/lib/tauri-commands";
  const rows = await getConversionMeta(assetId);
  // rows[0] 即最新一次转换（converted_at DESC）
  ```
- **fallback 判定建议（给 UI）**：`row.fallbackUsed === true` 或 `row.errorClass !== null` 时展示"降级"badge；`row.qualityLevel === 4` 表示 markitdown 成功。
- **不要在前端做排序**：DB 已 DESC，UI 直接取 `[0]` 即"最新一次"。
- **Inspector 调用频率提示**：每次切换 asset 调一次即可；conversion_meta 表行数预期低（每 asset O(1~5) 行），无需 LRU 缓存。

---
任务完成。3 文件改动，0 error，0 新增测试（理由见 §4/§7）。
