# Task 输出 — task_015b_close_td3_parse_failure_code

## 实现摘要

**决策**：关闭 TD-3 技术债（task_003 Reviewer 抛出、task_012 Reviewer 复识为 MAJOR-1），把 `parse_failure_code` 单源化到 `db/conversion_meta.rs`，删除 scheduler.rs 内的 workaround mini-parser。

**实际做了什么**：
1. **canonical parser 补全 5 KC 分支**（`db/conversion_meta.rs:263`）。沿用现有 8 markitdown match-arm 风格追加 5 个 `"E_KC_*" => Some(FailureCode::EKc*)` 字面分支；可见性从 `fn` 改为 `pub(crate) fn` 让 scheduler 可调用。字面与 `FailureCode::EKc*::as_str()` 严格 round-trip——**守护测试用 `as_str()` 取字面**而非硬编码字符串，从而真正消除"第二份字面源"风险。
2. **删除 scheduler 内 workaround**（原 scheduler.rs:1444-1457）。15 行 local mini-parser 全部删掉，调用点（1424 行）改为 `db_conv_meta::parse_failure_code(...)`（`db_conv_meta` 别名在 scheduler.rs:8 已存在，无需新增 use）。
3. **守护测试双地点**：
   - scheduler.rs:2045 的 `parse_failure_code_recognises_all_five_kc_variants` 保留并改 `use crate::db::conversion_meta::parse_failure_code;`，从 scheduler-local 守护变为"调用 path 守护"——防止未来某次回退复制一份 mini-parser。
   - `conversion_meta.rs` tests mod 新增：
     - `parse_failure_code_recognises_all_five_kc_variants`：用 `as_str()` round-trip 验证 5 KC（直接拿 `FailureCode::EKc*::as_str()` 喂回 parser，**绝不硬编码字面字符串**）。
     - `parse_failure_code_round_trips_all_thirteen_variants`：覆盖完整性哨兵，遍历全部 13 枚举变体做 round-trip。未来若 `FailureCode` 新增变体而忘记同步 parser，此测试 fail。

## 修改的文件

| 文件 | 改动 | 净 LOC（非测试） |
|---|---|---|
| `src-tauri/src/db/conversion_meta.rs` | parser 加 5 KC match arm + `pub(crate)` + 文档；测试 mod 加 2 守护测试 | +7 行（非测试） |
| `src-tauri/src/extraction/scheduler.rs` | 删 15 行 local mini-parser + 1424 行调用方改 path；保留并迁移守护测试（`use crate::db::conversion_meta::parse_failure_code;`） | -14 行（净删） |

`git diff --stat` 总计：2 files, +76 / -19（含注释与测试）；**非测试 src 净 -7 行**，远低于 30 行上限。

## 测试命令 / 测试结果

```
$ cd src-tauri && cargo test --lib parse_failure_code
running 3 tests
test db::conversion_meta::tests::parse_failure_code_recognises_all_five_kc_variants ... ok
test db::conversion_meta::tests::parse_failure_code_round_trips_all_thirteen_variants ... ok
test extraction::scheduler::tests::parse_failure_code_recognises_all_five_kc_variants ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 511 filtered out

$ cargo test --lib
test result: ok. 514 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.60s
```

- baseline 512 PASS（task_012 末）+ 2 个新增守护测试 = **514 PASS / 0 FAIL**。
- 0 测试退化。
- 3 个 `parse_failure_code` 守护测试全部 PASS。

## TD-3 关闭判定

**已关闭，不再有第二份字面源。**

- ✅ scheduler.rs 中 local `fn parse_failure_code(s: &str) -> Option<FailureCode>` 已物理删除（grep `parse_failure_code` 在 scheduler.rs 中现仅 2 处：1424 行调用点 `db_conv_meta::parse_failure_code(fc)` + 2045 行守护测试 `use crate::db::conversion_meta::parse_failure_code;`）。
- ✅ canonical `db/conversion_meta.rs::parse_failure_code` 现含全部 13 字面（8 markitdown + 5 KC），是 KC 失败码 → 枚举映射的**唯一源头**。
- ✅ 守护测试用 `FailureCode::*.as_str()` 取字面（不再硬编码字符串），与 `failure_code.rs` 枚举源同步——若 task_003 字面被改动，本测试会因 round-trip 失败而 fail。
- ✅ `parse_failure_code_round_trips_all_thirteen_variants` 兜底覆盖完整性：未来新增枚举变体若忘同步 parser，此测试立刻 fail。

任何后续 KC 失败码（包括未来 task 可能追加的新枚举）若想被 `kc_persist_resolved_with_conn` 识别，**唯一接入点**是 `db/conversion_meta.rs::parse_failure_code` 一处——TD-3 在结构上闭合。

## Reviewer 特别关注

1. **canonical parser 字面与 `FailureCode::EKc*::as_str()` 一致性**：
   - 直接对比 `db/conversion_meta.rs:263+` 5 KC match arm 字面 vs `extraction/failure_code.rs:64+` `as_str()` 返回值——逐字符一致（`E_KC_UNAVAILABLE` / `E_KC_ENRICH_FAILED` / `E_KC_LLM_UNAVAILABLE` / `E_KC_TIMEOUT` / `E_KC_INPUT_TOO_LARGE`）。
   - `parse_failure_code_recognises_all_five_kc_variants`（conversion_meta.rs tests mod）直接用 `code.as_str()` 喂回 parser 验证 round-trip——这是字面一致性的"机器守护"。
   - `parse_failure_code_round_trips_all_thirteen_variants` 覆盖全部 13 变体，作为未来扩展的兜底哨兵。

2. **scheduler 调用 path 全部改完**：
   - `grep -n "parse_failure_code" scheduler.rs` 结果：
     - L1424：`if let Some(code) = db_conv_meta::parse_failure_code(fc)` ← canonical 调用
     - L2030（守护测试内）：`use crate::db::conversion_meta::parse_failure_code;` ← canonical use
   - 已无任何 local `parse_failure_code` 函数定义残留。
   - `db_conv_meta` 别名沿用 scheduler.rs:8 既有 `use crate::db::conversion_meta::{self as db_conv_meta, ConversionMetaRow};`，**未新增 import**。

3. **task_012 注入逻辑零改动**：
   - `kc_persist_resolved` / `kc_persist_resolved_with_conn` 主体逻辑、调用点、签名全部未动（验证：`git diff scheduler.rs` 仅 1424 行函数调用改名 + 删 1444-1457 函数定义）。
   - 不动 `FailureCode` 枚举本身（task_003 固化锁）。
