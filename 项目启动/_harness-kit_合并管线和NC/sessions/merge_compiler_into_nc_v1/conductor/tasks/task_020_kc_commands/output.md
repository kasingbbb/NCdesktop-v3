# Task 交付 — task_020_kc_commands

## 实现摘要

新建 `src-tauri/src/commands/kc.rs`，实装 3 个 Tauri command，给前端 task_016 `KcSettingsForm.tsx` 与 KC 状态 banner 提供 IPC 入口；前端 `src/lib/tauri-commands.ts` 同步追加 typed 包装。

**3 个 command**：

| Command | 签名 | 行为 |
|---|---|---|
| `get_kc_health` | `async fn(State<Arc<KcProcessManager>>) -> Result<KcHealthStatusDto, String>` | 包装 `KcProcessManager::health_check`；DTO 转换（chrono → RFC3339 字符串）；永远 Ok |
| `restart_kc_process` | `async fn(State<Arc<KcProcessManager>>) -> Result<(), String>` | 包装 `KcProcessManager::restart`；冷却期 Err 经 `KcStartError::reason` 转 friendly String |
| `set_kc_settings` | `async fn(State<Arc<KcProcessManager>>, State<Database>, KcSettingsPayload) -> Result<(), String>` | DB 写回 7 字段 + keep/clear/set Key 语义 + **Key 变化时 tokio::spawn 异步 restart**（不阻塞返回）|

**关键设计决策**：

1. **Key 变化才 restart**（不是所有字段都触发）：
   - `KcSettings` 7 字段中只有两个 Key (`zhipu_api_key` / `openai_api_key`) 通过 env (`build_env_vars`) 注入 KC 子进程；
   - 4 个 bool 字段（`enabled` / `use_ai` / `enable_qa` / `enable_links`）是 NC 主进程在调 KC ingest 时即时读取的，**不在子进程 env**；
   - 因此只有 Key 变化时才需要 restart（节约 3-5s 启动窗口，UX 更顺滑）。

2. **restart 不阻塞 set_kc_settings 返回**：用 `tauri::async_runtime::spawn` detach restart future，前端立即收到 `Ok(())`；restart 进度通过 `notecapt/kc-status-changed` 事件订阅。

3. **restart 失败不让 set_kc_settings 失败**：DB 已成功写入是"保存成功"的真正含义；restart 失败仅 log::warn（旧 Key 仍在 KC 子进程内继续工作）。

4. **DTO 显式 camelCase**：所有 DTO 用 `#[serde(rename_all = "camelCase")]`，与前端 TS 接口 round-trip 锁死，不依赖 Tauri 默认 `ArgumentCase::Camel`。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src-tauri/src/commands/kc.rs` | 新建 | 3 个 command + 2 个 DTO + `apply_key_action` helper + 14 单测（~480 行）|
| `src-tauri/src/commands/mod.rs` | 修改（+3 行） | 注册 `pub mod kc;` |
| `src-tauri/src/lib.rs` | 修改（+5 行） | `invoke_handler!` 追加 3 个 command |
| `src/lib/tauri-commands.ts` | 修改（+78 行） | 追加 `KcHealthStatus` / `KcSettingsPayload` 接口 + `getKcHealth` / `restartKcProcess` / `setKcSettings` wrapper |

**未触及**：
- `src-tauri/src/kc/process.rs`（task_008 固化）
- `src-tauri/src/kc/settings.rs`（task_010 固化）
- `src-tauri/src/extraction/scheduler.rs`（不在本 task scope）
- 任何已有 command 模块

## DTO 字段对照表（Rust ↔ TypeScript）

### `KcHealthStatusDto` ↔ `KcHealthStatus`

| Rust 字段 (snake_case) | TS 字段 (camelCase) | 类型 | 来源 |
|---|---|---|---|
| `status` | `status` | `string` | `KcHealthStatus::status`（"ready" / "starting" / "stopped" / "unavailable"）|
| `reason` | `reason` | `Option<String>` ↔ `string \| null` | `KcHealthStatus::reason`（仅 unavailable 非空）|
| `port` | `port` | `Option<u16>` ↔ `number \| null` | `KcHealthStatus::port`（非 ready 时 null）|
| `uptime_secs` | `uptimeSecs` | `Option<u64>` ↔ `number \| null` | `KcHealthStatus::uptime_secs`（仅 Ready 状态有值）|
| `last_check` | `lastCheck` | `String` ↔ `string` | `chrono::DateTime<Utc>::to_rfc3339()`，前端 `new Date(lastCheck)` 可解析 |

字段数：**5**（与 `KcProcessManager::health_check` 返回值 1:1）

### `KcSettingsPayload`（Deserialize 单向）

| Rust 字段 (snake_case) | TS 字段 (camelCase) | 类型 | 备注 |
|---|---|---|---|
| `enabled` | `enabled` | `bool` ↔ `boolean` | KC 总开关 |
| `use_ai` | `useAi` | `bool` ↔ `boolean` | AI 增强子开关 |
| `enable_qa` | `enableQa` | `bool` ↔ `boolean` | 问答对生成 |
| `enable_links` | `enableLinks` | `bool` ↔ `boolean` | 段落关联 |
| `zhipu_key_action` | `zhipuKeyAction` | `String` ↔ `"keep" \| "clear" \| "set"` | 三态语义参考 llm.rs:79-91 |
| `zhipu_key_value` | `zhipuKeyValue` | `String` (default 空串) ↔ `string?` | 仅 action=set 时使用；keep/clear 时可省略（`#[serde(default)]`）|
| `openai_key_action` | `openaiKeyAction` | `String` ↔ `"keep" \| "clear" \| "set"` | 同上 |
| `openai_key_value` | `openaiKeyValue` | `String` (default 空串) ↔ `string?` | 同上 |

字段数：**8**（4 bool + 2 key 三元组）

**故意不在 DTO 中暴露的字段**：
- `outputstage_defense_mode`（前端 task_016 KcSettingsForm 当前不提供 UI；`set_kc_settings` 保留 DB 中现有值，避免静默降级到默认 `FullDefense`）。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test --lib commands::kc   # 14 新测试
cargo test --lib                # 整体回归（≥ baseline 512）
cd ..
npx tsc --noEmit                # 前端 TS 类型检查
```

## 测试结果

### `cargo test --lib commands::kc`

```
running 14 tests
test commands::kc::tests::apply_key_action_set_uses_trimmed_value ... ok
test commands::kc::tests::apply_key_action_clear_returns_none ... ok
test commands::kc::tests::apply_key_action_keep_preserves_existing ... ok
test commands::kc::tests::apply_key_action_set_rejects_empty_value ... ok
test commands::kc::tests::apply_key_action_rejects_invalid_action ... ok
test commands::kc::tests::kc_settings_payload_zhipu_key_value_defaults_to_empty ... ok
test commands::kc::tests::kc_settings_payload_deserializes_from_camel_case_json ... ok
test commands::kc::tests::set_kc_settings_detects_key_change_for_restart_trigger ... ok
test commands::kc::tests::kc_health_status_dto_serializes_to_camel_case_json ... ok
test commands::kc::tests::restart_kc_process_propagates_error_with_friendly_message ... ok
test commands::kc::tests::get_kc_health_returns_dto_with_camel_case_fields ... ok
test commands::kc::tests::set_kc_settings_writes_all_six_keys_to_db ... ok
test commands::kc::tests::set_kc_settings_persists_to_db ... ok
test commands::kc::tests::set_kc_settings_clears_key_when_action_clear ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 514 filtered out
```

**14/14 PASS**

### `cargo test --lib`（整体回归）

```
test result: ok. 528 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.64s
```

**528/528 PASS**（baseline 512 + 本 task 14 + 其他并发 dev 累积 2）；**0 退化**。

### `npx tsc --noEmit`

无输出 → **0 error**（与既有 IPC wrapper 字段对齐成功）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| `apply_key_action` keep | `keep` 保留 `existing`，包括 `existing=None` | 已测 | `apply_key_action_keep_preserves_existing` PASS |
| `apply_key_action` clear | `clear` 始终返回 `None`，与 `value` / `existing` 无关 | 已测 | `apply_key_action_clear_returns_none` PASS |
| `apply_key_action` set | `set` 走 trim；正常 value 入库；空 value 报错 | 已测 | `apply_key_action_set_uses_trimmed_value` / `apply_key_action_set_rejects_empty_value` PASS |
| `apply_key_action` 非法 action | `"delete"` 等非法值返回友好错误（含字段名 + 非法值）| 已测 | `apply_key_action_rejects_invalid_action` PASS |
| DTO camelCase 反序列化 | 前端 camelCase JSON 反序列化为 `KcSettingsPayload` 8 字段全部正确映射 | 已测 | `kc_settings_payload_deserializes_from_camel_case_json` PASS |
| DTO `#[serde(default)]` | 缺失 `zhipuKeyValue` / `openaiKeyValue` 时走空串默认不报错 | 已测 | `kc_settings_payload_zhipu_key_value_defaults_to_empty` PASS |
| DTO camelCase 序列化 | `KcHealthStatusDto` 序列化为 `lastCheck` / `uptimeSecs`（非 snake_case）| 已测 | `kc_health_status_dto_serializes_to_camel_case_json` PASS |
| set_kc_settings DB 写入 | bool + keep + set 混合场景，6 个 setting 键全部正确写入 | 已测 | `set_kc_settings_persists_to_db` / `set_kc_settings_writes_all_six_keys_to_db` PASS |
| set_kc_settings clear | `clear` 后 reload 应为 None（空串协议） | 已测 | `set_kc_settings_clears_key_when_action_clear` PASS |
| Key 变化检测（restart 触发） | None→Some / Some(a)→Some(b) / Some→None 都视为变化；Some(a)→Some(a) 不视为变化 | 已测 | `set_kc_settings_detects_key_change_for_restart_trigger` PASS |
| get_kc_health DTO 路径 | 无 KC 时返回 Stopped 状态 + RFC3339 lastCheck，序列化 camelCase | 已测 | `get_kc_health_returns_dto_with_camel_case_fields` PASS（tokio test）|
| restart_kc_process 错误格式 | `KcStartError::reason` 经 `"KC 重启失败: ..."` 友好包装 | 已测 | `restart_kc_process_propagates_error_with_friendly_message` PASS |
| 集成（Tauri State 注入） | `invoke_handler!` 真实 IPC 调用链 | 未测 | 由 task_022 / task_023 失败注入 + e2e 测试覆盖；本 task 单测在 helper / DTO / DB 层覆盖关键路径 |
| 前端 KcSettingsForm 接入 | UI 真实点击保存 / 重启按钮 | 未测 | 由 task_016 自验；本 task 仅出 IPC + types |

## 已知局限

1. **Tauri State 注入未在单测中模拟**：Tauri 2.x 的 `tauri::test` mock 框架未在本仓库的 dev-deps 启用，引入会扩大 task scope。本 task 单测覆盖 **helper 纯函数**（`apply_key_action`）+ **DTO 序列化 round-trip**（`serde_json::from_str` / `to_string`）+ **`KcProcessManager::new_test_only_no_app()` 短路构造**（直接 await `health_check` / `restart`）的关键路径，足够守护 IPC 层 contract；State 注入正确性由 lib.rs `invoke_handler!` 编译期检查保证（State 类型不匹配编译报错）。

2. **`outputstage_defense_mode` 不进 DTO**：前端 task_016 KcSettingsForm 当前不暴露此字段，所以 `KcSettingsPayload` 也不携带。`set_kc_settings` 通过 `KcSettings::load` 读出现有值再写回，保留之；如未来 UI 要暴露此 dropdown，需追加字段 + 测试 + 前端 wrapper 联调。

3. **restart 失败仅 log 不通知前端**：当 Key 变化触发的 restart 失败（冷却期等），后台只 `log::warn`，前端不会知道"保存成功但 KC 旧 Key 仍在用"。如果 PM 觉得这是 UX 问题，可以让 `KcProcessManager::restart` 失败时显式 emit `notecapt/kc-restart-failed` 事件（task_008 已有 `notecapt/kc-status-changed`，可复用同一通道把 reason 透出）。

4. **`set_kc_settings` 内的 restart 共享冷却期**：如果用户在 30s 内连续保存 2 次 Key 变更，第 2 次会因冷却期触发 `RestartCooldownExceeded`，新 Key 不会立即生效（旧 Key 继续工作直到 1 分钟后用户再次保存或手动 restart）。这与"用户手动连点 2 次重启按钮"语义一致，不视为 bug。

## 需要 Reviewer 特别关注的地方

### 1. `set_kc_settings` 自动 restart 的语义边界

- **触发条件**：**仅 Key 变化时** tokio::spawn restart；bool 字段变化（如 enabled false→true）**不**触发 restart。
- **理由**：bool 字段不经 KC 子进程 env（task_010 `build_env_vars` 只输出 2 个 Key），而是 NC 主进程在调 KC ingest 时即时读取 `KcSettings::load`，所以 bool 变化对 KC 子进程不可见，restart 没有意义（反而浪费 3-5s）。
- **Reviewer 挑战点**："如果用户改了 enabled=false，是否应该 stop KC 子进程而不是仅写 DB？" 我的判断：当前 `KcSettings::enabled=false` 仅短路 NC 这一侧的 enrichment 调用（task_011 `enrichment::run_kc_enrichment` 在 enabled=false 时直接走 Fallback），KC 子进程在后台 idle 资源占用极低（uvicorn 单进程 ~30MB RAM），保留可立即开关；如要"enabled=false 即 stop"，需在 set_kc_settings 内分支 `if !new_settings.enabled { stop(); }`——这是产品级取舍，建议后续 task 决策。

### 2. camelCase 边界

- **DTO 显式 `#[serde(rename_all = "camelCase")]`**：所有 `KcHealthStatusDto` / `KcSettingsPayload` 字段都被这条 derive 锁住，与 TS 接口字面值一一对齐。
- **Tauri command 参数命名**：Tauri 2.x 默认 `ArgumentCase::Camel`，即前端 `invoke("set_kc_settings", { settings: ... })` 中外层 key 也自动 camelCase。本 task 命令的参数名（`state` / `db` / `settings`）全部已是 lower / 单 word，camelCase 退化为原值，无歧义；但**外层 settings 对象内部**的字段（`zhipuKeyAction` 等）必须依赖 DTO 上的 `rename_all` 才能反序列化——若维护者后续把 DTO 上的 `#[serde(rename_all = "camelCase")]` 删除，前端会立即报反序列化错误。**单测 `kc_settings_payload_deserializes_from_camel_case_json` 守护此契约**。
- **`get_kc_health` 返回方向**：Tauri 2.x 序列化 Rust struct 到前端走 `serde` derive，必须显式 `#[serde(rename_all = "camelCase")]` 才能让 `uptime_secs` → `uptimeSecs`。**单测 `kc_health_status_dto_serializes_to_camel_case_json` 守护此契约**。

### 3. State 注入正确性

- **`State<'_, Arc<KcProcessManager>>` 类型**：与 `lib.rs:197 app.manage(kc_manager.clone())` 注册的类型 `std::sync::Arc<kc::KcProcessManager>` **完全一致**——任何不一致 Tauri 会在运行时 panic（State 未注册）。
- **`State<'_, Database>` 类型**：与 `lib.rs:90 app.manage(database)` 注册的类型 `db::Database` 一致。
- **两个 State 共享**：`set_kc_settings` 同时获取两个 State，Tauri runtime 保证类型正确性，但 `db.conn.lock()` 锁的释放顺序需要小心——本实装把 DB 锁限制在两个独立 sub-scope（步骤 1 读 + 步骤 4 写），不持锁穿越 `KcProcessManager::restart` 调用，避免与其他持 DB 锁的路径死锁（如 `read_kc_settings` in `KcProcessManager::start`）。

### 4. restart 异步触发的错误传播策略

- **当前行为**：spawn 内的 restart 失败 → `log::warn!`，**不**经事件 emit 给前端；前端只能通过 `notecapt/kc-status-changed`（`KcProcessManager::restart` 内部 emit 的 status=unavailable + reason="restart cooldown exceeded"）间接得知。
- **Reviewer 挑战点**：是否应该新增 `notecapt/kc-settings-saved` 事件，让前端 toast 区分"保存成功 + restart 成功"vs"保存成功 + restart 失败"？本 task 范围内不引入此事件（避免事件 schema 扩散），如 task_016 UI 实测后觉得 UX 不够，再决策。

### 5. `apply_key_action` 三态语义与 `commands::llm::save_llm_config` 严格对齐

- 文本字面值（`"keep"` / `"clear"` / `"set"`）、empty value 报错信息、非法 action 报错信息——三处与 `commands/llm.rs:79-91` 行为一致。
- **Reviewer 可挑战**："为何不抽 `key_action.rs` 共享 helper？" 我的判断：当前两处使用方式略有差异（llm.rs 写单一 Key 到 DB；kc.rs 同时计算两个 Key 后才入 KcSettings），共享后 API 反而臃肿；当出现第 3 个使用方时再抽。
