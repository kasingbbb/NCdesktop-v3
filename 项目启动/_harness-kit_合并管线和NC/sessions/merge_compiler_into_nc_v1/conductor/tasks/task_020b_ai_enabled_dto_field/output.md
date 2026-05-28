# Task 交付 — task_020b_ai_enabled_dto_field

## 状态
**完成**（XS task，3 AC 全量落地 + 5 新测试，0 退化）

## 工作目录验证

```
$ cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop && git rev-parse --show-toplevel
/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop
$ git branch --show-current
feat/windows-unit-13-cloud-ai
$ git rev-parse HEAD (commit 前)
e3c2ae02
```

## 数据源决策（reviewer 重点关注项）

**最终选择：实时拉 /health，不引入缓存。**

### 决策矩阵

| 方案 | 改动量 | 数据新鲜度 | 状态机风险 |
|---|---|---|---|
| A. KcProcessManager 内部缓存 `last_ai_enabled` | +字段 + setter + 锁开销 | 旧（取决于何时更新） | 中（增加状态一致性维护负担） |
| **B. health_check 内实时发 /health 解析 JSON**（采纳） | +1 helper 函数 | 实时（每次调用刷） | 0（不动状态机） |
| C. 在 commands/kc.rs::get_kc_health 内单独调 KcClient | 跨模块跳一次 | 实时 | 0 但破坏单一职责 |

### B 方案落地细节

- `KcProcessManager::health_check` 在 Ready 状态下**已经**发了一次 `/api/v1/health`（原 `single_health_request`，只返 bool）。
- task_020b 新增 `single_health_request_with_ai_enabled(port) -> Option<Option<bool>>`：复用同一 endpoint，但解析响应 body 的 `ai_enabled` 字段。
- 原 `single_health_request` 保留：`poll_health_check`（startup 阶段轮询）不需要解析 body，零分配路径继续走旧函数；`health_check`（运行期单次查询）走新函数。
- **不增加额外网络往返**（同一端点同一周期），不引入缓存复杂度。

### 兜底语义（外层 health_check 据此构造 ai_enabled）

| 场景 | `single_health_request_with_ai_enabled` 返回 | `KcHealthStatus.ai_enabled` | 前端展示 |
|---|---|---|---|
| KC Ready + /health 200 + body 含 `ai_enabled: true` | `Some(Some(true))` | `Some(true)` | "AI 已就绪" |
| KC Ready + /health 200 + body 含 `ai_enabled: false` | `Some(Some(false))` | `Some(false)` | "Key 配置但 AI 未启用" |
| KC Ready + /health 200 + body 缺 `ai_enabled`（KC 旧版本） | `Some(None)` | `None` | "未知" |
| KC Ready + /health 非 2xx / 请求失败 / JSON 解析失败 | `None` | `None`（+ reason="transient health request failure"） | "未知" |
| KC Stopped / Starting / Unavailable | （不发请求） | `None` | "未知" |

**关键点**：缺字段（`Some(None)`）vs 请求失败（`None`）在内部区分（外层 reason 不同），对外都映射到 `ai_enabled: None`——前端只关心"未知"语义，不关心具体原因。

## 改动文件清单（3 个）

| 文件 | 变更 | 说明 |
|---|---|---|
| `src-tauri/src/kc/process.rs` | +30 行生产 / +52 行测试 | `KcHealthStatus` 加 `ai_enabled: Option<bool>` 字段；`health_check` Ready 分支改调 `single_health_request_with_ai_enabled`；新增该 helper（reqwest + serde_json::Value 解析）；保留 `single_health_request` 给 `poll_health_check` |
| `src-tauri/src/commands/kc.rs` | +7 行生产 / +76 行测试 | `KcHealthStatusDto` 加 `ai_enabled: Option<bool>` 字段（`#[serde(rename_all = "camelCase")]` 自动序列化为 `aiEnabled`）；`get_kc_health` 透传字段 |
| `src/lib/tauri-commands.ts` | +5 行注释更新 | `KcHealthStatus.aiEnabled` 注释从"task_020 尚未透传 / forward-compat"更新为"task_020b 已落地"，类型 `aiEnabled?: boolean \| null` 保持不变（向后兼容） |

**未触及**：
- `KcProcessManager` 启停 / 崩溃 / RAII / 状态机逻辑（未引入缓存字段）
- `KcClient`（task_007）— 不调它，直接复用 process.rs 内既有 HTTP 客户端
- `KcSettingsForm.tsx` — task_016 已用 `aiEnabled` 三态判定，无需改动
- `Cargo.toml` / 任何 dep — 全部用已有 `reqwest` + `serde_json`

## AC 落地

### AC-1：DTO 加字段 ✓
`KcHealthStatusDto` 追加 `pub ai_enabled: Option<bool>`，`#[serde(rename_all = "camelCase")]` 保留，前端读 `aiEnabled`。

### AC-2：数据源 ✓
- 实时调 KC `/health` 解析 JSON 取 `ai_enabled`（无缓存）
- 同一 endpoint 复用既有 health 请求路径（无额外 RTT）
- 4 类兜底（不可达 / 非 2xx / JSON fail / 字段缺失）→ `None`，不阻塞 get_kc_health 返回

### AC-3：单测 ✓（5 个新增）
| 测试名 | 文件 | 验证点 |
|---|---|---|
| `kc_health_dto_includes_ai_enabled_field_in_serialization` | commands/kc.rs | DTO 序列化三态（true/false/None）都生成 `aiEnabled` 字段 + camelCase 正确 |
| `kc_health_dto_deserializes_ai_enabled_from_camel_case` | commands/kc.rs | 前端 JSON `aiEnabled: true/null` 反序列化为 DTO 字段正确 |
| `kc_health_returns_ai_enabled_none_when_not_ready` | commands/kc.rs | Stopped 状态 DTO `ai_enabled` 必为 None |
| `health_check_returns_ai_enabled_none_when_field_missing` | kc/process.rs | mock /health body 缺 `ai_enabled` → KcHealthStatus.ai_enabled = None（KC 旧版本兼容守护） |
| `health_check_returns_ai_enabled_false_when_kc_reports_false` | kc/process.rs | mock /health 返 `ai_enabled: false` → KcHealthStatus.ai_enabled = Some(false)（三态判定守护 task_016 AC-7） |

另：原 `process_starts_with_mock_via_env_var` 集成测试追加 ai_enabled 断言（既有 mock body 已含 `ai_enabled: true`）。

### AC-4：前端 ✓
- `KcHealthStatus.aiEnabled?: boolean | null` 保持 optional（task_016 已加，本 task 不动类型，仅刷新注释）
- 与 `KcSettingsForm.tsx` 三态判定（`true`/`false`/`null/undefined`）无缝衔接

## 测试命令 / 结果

```
$ cd 项目启动/NCdesktop/src-tauri
$ cargo test --lib commands::kc
test result: ok. 17 passed; 0 failed; 0 ignored  # 原 14 + 新 3

$ cargo test --lib
test result: ok. 537 passed; 0 failed; 0 ignored  # 原 532 + 新 5

$ cd 项目启动/NCdesktop && pnpm tsc --noEmit
（无输出 = 0 error）
```

## 代码改动行数（约束 ≤ 50 生产行）

| 文件 | 生产 | 测试 |
|---|---|---|
| commands/kc.rs | 7 | 76 |
| kc/process.rs | 30 | 52 |
| tauri-commands.ts | 5（注释更新） | 0 |
| **总计** | **42** | **128** |

生产代码 42 行（含 `single_health_request_with_ai_enabled` helper + DTO 字段 + health_check 分支重构 + 测试断言修补），低于 50 行约束。

## 关键技术决策

### 1. 不替换 `single_health_request`，新增 `single_health_request_with_ai_enabled`
原函数被 `poll_health_check`（startup 阶段每 200ms 一次，最多 50 次）调用，零分配路径敏感。新函数仅 `health_check`（运行期单次）使用，需要解析 JSON body 走完整 reqwest::Response::json 路径。两个函数职责分离更清晰，不污染 startup 性能。

### 2. `Option<Option<bool>>` 返回类型语义
外层（health_check）需要区分"请求失败"和"成功但缺字段"，因为 reason 文案不同（前者 transient failure，后者无 reason）。但 DTO 对外只暴露 `Option<bool>`——前端只关心"未知"语义。内部双层 Option 在 health_check 函数内 collapse 为单层。

### 3. 不引入 KcProcessManager 缓存字段
input.md 提出"如果 KcProcessManager 已持有上次 /health 响应的 ai_enabled" 作为方案 A。决策：health_check 本身已在每次调用时发 HTTP，缓存收益不大（只能节约 polling 频率 5s 一次的轻量请求），而引入缓存字段会污染状态机（需要在 stop/restart/crash 时清理；与 startup_time/restart_history 之类配对维护）。**简单优于复杂**，本 task 不动状态机。

### 4. 序列化语义 `Option<bool>` → `null` 而非省略
serde 默认对 `Option::None` 不跳过（不加 `#[serde(skip_serializing_if = "Option::is_none")]`）。前端拿到 `aiEnabled: null` 与 `aiEnabled` 字段缺失等价（TS 类型 `boolean | null | undefined` 已经吸收两种），断言显式守护 `aiEnabled:null` 序列化路径，避免后续添加 `skip_serializing_if` 静默破坏。

## Reviewer 重点回应

- **ai_enabled 数据源是否真实**：✓ 实时调 KC `/health` 并解析响应 JSON（非 hardcode None），数据源透明记录在 `single_health_request_with_ai_enabled` doc 中。
- **DTO camelCase round-trip**：✓ 单元测试 `kc_health_dto_includes_ai_enabled_field_in_serialization` + `kc_health_dto_deserializes_ai_enabled_from_camel_case` 双向守护。
- **KC 不可达兜底**：✓ `single_health_request_with_ai_enabled` 任何错误路径都返 `None`，外层 health_check 把它 collapse 为 DTO `ai_enabled: None` + 已有的 reason="transient health request failure"。get_kc_health command 整体仍 Ok 返回（与 task_020 既有契约一致）。
