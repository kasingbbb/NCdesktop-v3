# Task 输入 — task_020b_ai_enabled_dto_field

## 目标
补 task_020 漏字段：`KcHealthStatusDto` 添加 `ai_enabled: Option<bool>` 字段透传 KC `/health` 返回的同名字段，让 task_016 KcSettingsForm 的 [测试连通性] 按钮能拿到真实 ai_enabled 状态。

## 背景
- task_020（commit `100e66f6`）实装了 `get_kc_health` Tauri command，但 DTO 只暴露了 status / reason / port / uptimeSecs / lastCheck，**漏了 ai_enabled**
- task_016 dev 发现这个缺口，采用 optional field forward-compat 处理（前端 `KcHealthStatus.aiEnabled?: boolean | null`）
- 本 task 关闭这个 gap：后端 DTO 加 ai_enabled 字段 + KcClient 调用 /health 时把 KC 返回的 ai_enabled 填进来

## 前置条件
- task_020（KC commands，commit `100e66f6`）已完成
- task_007 KcClient::health (or similar) 调用 KC `/health` endpoint
- task_008 KcProcessManager::health_check 返回当前状态

## 验收标准

1. **AC-1**：`src-tauri/src/commands/kc.rs::KcHealthStatusDto` 追加字段：
   ```rust
   pub struct KcHealthStatusDto {
       pub status: String,
       pub reason: Option<String>,
       pub port: Option<u16>,
       pub uptime_secs: Option<u64>,
       pub last_check: Option<String>,
       pub ai_enabled: Option<bool>,   // 新增
   }
   ```
   - `#[serde(rename_all = "camelCase")]` 保留，前端读 `aiEnabled`

2. **AC-2**：`get_kc_health` 命令体读 KcProcessManager 的 ai_enabled 状态：
   - 若 KcProcessManager 已持有上次 /health 响应的 ai_enabled（task_007/008 是否有缓存），直接用
   - 若没有，调 KcClient::health()（或 process.rs 内部 health_check）拉一次 /health；解析响应 JSON 取 `ai_enabled` 字段
   - 错误兜底：KC 不可达或 /health 解析失败时 → `ai_enabled: None`（前端会显示"未知"）

3. **AC-3**：单测追加 2 个：
   - `kc_health_dto_includes_ai_enabled_field`（DTO 序列化含 aiEnabled key）
   - `kc_health_returns_ai_enabled_when_available`（mock KC /health 返回 ai_enabled=true → DTO 字段填上）
   - 若 ai_enabled 来源是 KcProcessManager 缓存，则改测：`kc_health_returns_cached_ai_enabled`

4. **AC-4**：前端 `KcHealthStatus` interface 把 `aiEnabled?: boolean | null` 升级为更精确语义（去掉 `?` 还是保留？建议保留 `?` 兼容历史调用方）

## 技术约束
- 不破坏 task_020 既有 14 测试
- 不改 KcProcessManager 内部状态机
- 若需让 KcProcessManager 缓存最近一次 /health 的 ai_enabled，仅追加字段，不动启停/崩溃/RAII
- ≤ 50 行代码改动（不含测试）

## 参考文件
- `src-tauri/src/commands/kc.rs`（task_020 commit `100e66f6`）
- `src-tauri/src/kc/client.rs::KcClient::health`（task_007）
- `src-tauri/src/kc/process.rs::KcProcessManager`（task_008）
- task_016 output.md "关键决策" 小节（task_020b 触发原因）

## Reviewer 重点关注项
- ai_enabled 数据来源是否真实（不能是 hardcode None；要么从 KC /health 实时取，要么从缓存读，要么显式标记数据源不可用）
- DTO camelCase round-trip 不破坏（前端读 aiEnabled）
- 错误兜底：KC 不可达 → None 不阻塞 get_kc_health 整体返回

## 复杂度
XS（≤ 50 行 + 3 单测，半小时内完工）
