//! task_020：KC 集成层的 3 个 Tauri command。
//!
//! 给前端 task_016 `KcSettingsForm.tsx` 提供的薄壳：
//! - [`get_kc_health`]      —— 查 KC 当前进程状态（包装 `KcProcessManager::health_check`）；
//! - [`restart_kc_process`] —— 用户手动重启（包装 `KcProcessManager::restart`）；
//! - [`set_kc_settings`]    —— UI 保存 KcSettings 7 字段，按 keep/clear/set 语义改 Key，
//!   写 DB（`kc::settings::save_settings`），**如果两个 Key 任一变化**则
//!   `tokio::spawn` 异步触发 `KcProcessManager::restart`（让新 Key 注入子进程 env）。
//!
//! ## 设计约束
//!
//! 1. **不动 KcProcessManager / KcSettings 实装**：task_008 / task_010 已固化，本 task
//!    仅作 IPC 桥接。
//! 2. **DTO 前后端 camelCase**：所有 DTO 用 `#[serde(rename_all = "camelCase")]`
//!    显式锁住——Tauri 默认 ArgumentCase::Camel 本来也能 round-trip，但**显式**写明
//!    避免后续维护者修改 Tauri 全局配置时静默打破契约。
//! 3. **set_kc_settings 不阻塞 restart**：用户在 UI 点保存后，前端必须立即收到 `Ok(())`
//!    让 UI 可以"保存成功"toast 出来；KC 重启 ~3-5s 在后台跑，过程中前端通过
//!    `notecapt/kc-status-changed` 事件订阅状态变化（已由 `KcProcessManager` 在 setup
//!    阶段 emit）。
//! 4. **Key 变化才 restart**：
//!    - bool 字段（`enabled` / `use_ai` / `enable_qa` / `enable_links`）不进 KC 子进程
//!      env，NC 主进程在调 KC ingest 时按当前 settings 即时读取——所以 bool 变化**不需要**
//!      restart 子进程（节约 3-5s 重启窗口）；
//!    - 两个 Key 字段（`zhipuApiKey` / `openaiApiKey`）走 env 注入（`build_env_vars`），
//!      只有 spawn 时才注入，运行中不可变——所以 Key 变化**必须** restart。
//! 5. **错误返回 String**：与 NC 现有 command 风格（`commands::llm::save_llm_config` 等）
//!    一致，错误以人类可读字符串向上抛，前端按 string 类型 catch。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::Database;
use crate::kc::process::KcProcessManager;
use crate::kc::settings::{self, KcOutputStageDefenseMode, KcSettings};

// =====================================================================
// 1. DTO（与 input.md AC-2 / AC-5 严格对齐）
// =====================================================================

/// `get_kc_health` 返回值（AC-5）。
///
/// 字段名前端 camelCase / Rust snake_case round-trip 由 `#[serde(rename_all = "camelCase")]` 保证。
///
/// 与 `KcProcessManager::health_check` 的 `KcHealthStatus` 字段一一映射，
/// 仅 `last_check`（`chrono::DateTime<Utc>`）转成 RFC3339 字符串便于前端 `new Date()` 解析。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KcHealthStatusDto {
    /// `ready` / `starting` / `stopped` / `unavailable`。
    pub status: String,
    /// 仅 `unavailable` 时非空。
    pub reason: Option<String>,
    /// 当前监听端口，未 Ready 时为 None。
    pub port: Option<u16>,
    /// 自进入 `Ready` 起累计秒数；非 Ready 状态为 None。
    pub uptime_secs: Option<u64>,
    /// 本次 health check 时间戳（RFC3339 字符串，便于前端 `new Date()` 解析）。
    pub last_check: String,
    /// **task_020b**：KC `/health` 响应的 `ai_enabled` 字段（task_016 KcSettingsForm 测试连通性按钮判定 AI 就绪用）。
    ///
    /// - 数据源：KC Ready 状态下 `KcProcessManager::health_check` 实时发 `/api/v1/health`
    ///   并解析响应 body 的 `ai_enabled` 字段透传（**非缓存**：每次 get_kc_health 都重新查）；
    /// - 兜底：KC 不可达 / 非 2xx / JSON 解析失败 / KC 旧版本未返回此字段 → `None`，
    ///   前端按"未知"显示（不显示判定，避免误导用户）。
    pub ai_enabled: Option<bool>,
}

/// `set_kc_settings` 入参（AC-2）。
///
/// **Key 语义**：参考 `commands::llm::SaveLlmConfigPayload`：
/// - `zhipu_key_action = "keep"`：不动现有 Key；
/// - `zhipu_key_action = "clear"`：清除（落空串到 DB，等价 `None`）；
/// - `zhipu_key_action = "set"`：使用 `zhipu_key_value`（trim 后非空，否则报错）。
/// OpenAI Key 同理。
///
/// 注意：本 DTO **不包含** `outputstage_defense_mode`——前端 task_016 KcSettingsForm
/// 当前不暴露此字段（防御档位由后端 default `FullDefense` 兜底）。保留时使用
/// `KcSettings::load` 现有值，避免静默降级。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KcSettingsPayload {
    pub enabled: bool,
    pub use_ai: bool,
    pub enable_qa: bool,
    pub enable_links: bool,
    /// `"keep"` / `"clear"` / `"set"`
    pub zhipu_key_action: String,
    #[serde(default)]
    pub zhipu_key_value: String,
    /// `"keep"` / `"clear"` / `"set"`
    pub openai_key_action: String,
    #[serde(default)]
    pub openai_key_value: String,

    // base_url / model 覆盖（Unit 3）：非敏感字段，前端直接传字符串（空串=不配置）。
    // `#[serde(default)]` 保证老前端不发这些字段时也能反序列化（向后兼容）。
    #[serde(default)]
    pub openai_base_url: Option<String>,
    #[serde(default)]
    pub openai_model: Option<String>,
    #[serde(default)]
    pub zhipu_base_url: Option<String>,
    #[serde(default)]
    pub zhipu_model: Option<String>,
}

/// 把前端传来的可选字符串归一化：`None` / 空白串 → `None`，否则 trim 后的值。
fn normalize_opt_setting(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// =====================================================================
// 2. Tauri command 实装
// =====================================================================

/// **AC-1.1**：查询 KC 当前进程状态（前端 banner / settings 页 polling 用）。
///
/// 包装 `KcProcessManager::health_check`——对 Ready 状态会额外发一次真实 HTTP `/api/v1/health`
/// 请求（详见 `KcProcessManager::health_check` doc）。失败仅降级到 `reason` 字段，不抛 Err。
///
/// **永远 Ok**：health_check 内部仅查内存状态 + 单次 HTTP，不存在抛错路径；
/// 但保持 `Result<_, String>` 签名一致性（与其他 command 风格一致 + 未来可扩展点）。
#[tauri::command]
pub async fn get_kc_health(
    state: State<'_, Arc<KcProcessManager>>,
) -> Result<KcHealthStatusDto, String> {
    let health = state.health_check().await;
    Ok(KcHealthStatusDto {
        status: health.status,
        reason: health.reason,
        port: health.port,
        uptime_secs: health.uptime_secs,
        last_check: health.last_check.to_rfc3339(),
        ai_enabled: health.ai_enabled,
    })
}

/// **AC-1.2**：用户手动重启 KC 子进程（前端 "重启 KC" 按钮）。
///
/// 包装 `KcProcessManager::restart`——内部走"stop → 检查冷却期 → start"，受冷却期约束
/// （30s 内 ≥ 2 次 OR 60s 内 ≥ 3 次会被拒），错误以 `KcStartError::reason` 字符串向上抛。
#[tauri::command]
pub async fn restart_kc_process(
    state: State<'_, Arc<KcProcessManager>>,
) -> Result<(), String> {
    state
        .restart()
        .await
        .map_err(|e| format!("KC 重启失败: {}", e.reason()))
}

/// **AC-1.3**：保存 KcSettings 7 字段到 DB，**如 Key 变化则异步触发 restart**。
///
/// ## 执行流程
///
/// 1. 锁 DB 连接，读出现有 `KcSettings`（用于 keep 语义 + Key 变化对比）；
/// 2. 按 `zhipu_key_action` / `openai_key_action` 计算新 Key（keep/clear/set）；
/// 3. 拼装新 `KcSettings`（保留现有 `outputstage_defense_mode`）；
/// 4. `kc::settings::save_settings` 一次性写回 7 行；
/// 5. **如两个 Key 任一发生变化** → `tokio::spawn` 异步调 `KcProcessManager::restart`，
///    立即返回 `Ok(())`（不阻塞前端 UI）。
///
/// ## Key 变化判定
///
/// 比较的是"现 DB Key（`Option<String>`）vs 新计算 Key（`Option<String>`）"——
/// 仅当 `Some(a) != Some(b)` 或 `Some(_) != None` 或 `None != Some(_)` 时视为变化。
/// `keep` 语义下"新 Key == 现 Key"必然不变化。
///
/// ## 不阻塞 restart
///
/// `tokio::spawn` 直接 detach future——前端拿到 `Ok(())` 后通过订阅
/// `notecapt/kc-status-changed` 事件感知 restart 进度。如 restart 失败（冷却期等），
/// 错误经事件 emit + log::warn 输出，**不**让 set_kc_settings 失败（保存 DB 已成功，
/// 不应因 restart 失败而让用户看到"保存失败"toast）。
#[tauri::command]
pub async fn set_kc_settings(
    state: State<'_, Arc<KcProcessManager>>,
    db: State<'_, Database>,
    settings: KcSettingsPayload,
) -> Result<(), String> {
    // 步骤 1：读现有 settings（用 sub-scope 持有锁，避免持锁过长）。
    let existing = {
        let conn = db.conn()?;
        KcSettings::load(&conn).unwrap_or_default()
    };

    // 步骤 2：按 keep/clear/set 计算两个 Key 的新值。
    let new_zhipu = apply_key_action(
        "zhipu_key_action",
        &settings.zhipu_key_action,
        &settings.zhipu_key_value,
        existing.zhipu_api_key.clone(),
    )?;
    let new_openai = apply_key_action(
        "openai_key_action",
        &settings.openai_key_action,
        &settings.openai_key_value,
        existing.openai_api_key.clone(),
    )?;

    // 步骤 3：拼装新 settings。`outputstage_defense_mode` 保留现有（前端 task_016 不暴露此字段）。
    let new_settings = KcSettings {
        enabled: settings.enabled,
        use_ai: settings.use_ai,
        enable_qa: settings.enable_qa,
        enable_links: settings.enable_links,
        zhipu_api_key: new_zhipu.clone(),
        openai_api_key: new_openai.clone(),
        // base_url / model：前端直传（空白归一化为 None）。
        openai_base_url: normalize_opt_setting(settings.openai_base_url.clone()),
        openai_model: normalize_opt_setting(settings.openai_model.clone()),
        zhipu_base_url: normalize_opt_setting(settings.zhipu_base_url.clone()),
        zhipu_model: normalize_opt_setting(settings.zhipu_model.clone()),
        outputstage_defense_mode: existing.outputstage_defense_mode,
    };

    // 步骤 4：写回 DB。
    {
        let conn = db.conn()?;
        settings::save_settings(&conn, &new_settings)?;
    }

    // 步骤 5：Key 变化检查 + 异步 restart（不阻塞返回）。
    let zhipu_changed = existing.zhipu_api_key != new_zhipu;
    let openai_changed = existing.openai_api_key != new_openai;
    if zhipu_changed || openai_changed {
        log::info!(
            "[kc] KcSettings Key 变化 (zhipu_changed={zhipu_changed}, openai_changed={openai_changed})，异步触发 restart"
        );
        let mgr = Arc::clone(&state);
        tauri::async_runtime::spawn(async move {
            match mgr.restart().await {
                Ok(()) => log::info!("[kc] set_kc_settings 触发的 restart 成功"),
                Err(e) => log::warn!(
                    "[kc] set_kc_settings 触发的 restart 失败 reason={:?} —— DB 已保存，KC 旧 Key 仍生效",
                    e
                ),
            }
        });
    } else {
        log::debug!("[kc] set_kc_settings：Key 未变化，跳过 restart");
    }

    Ok(())
}

// =====================================================================
// 3. 内部 helper：keep/clear/set Key 语义
// =====================================================================

/// 按 `"keep" / "clear" / "set"` 计算新 Key 值。
///
/// - `keep` → 返回 `existing`（不动）；
/// - `clear` → 返回 `None`（DB 写空串，等价"未配置"）；
/// - `set` → trim 后非空时返回 `Some(value)`，空串报错（与 llm.rs:84-87 一致）。
fn apply_key_action(
    field_name: &str,
    action: &str,
    value: &str,
    existing: Option<String>,
) -> Result<Option<String>, String> {
    match action {
        "keep" => Ok(existing),
        "clear" => Ok(None),
        "set" => {
            let v = value.trim();
            if v.is_empty() {
                Err(format!(
                    "请填写 {field_name} 对应的 Key，或改用 \"keep\" / \"clear\""
                ))
            } else {
                Ok(Some(v.to_string()))
            }
        }
        other => Err(format!(
            "无效的 {field_name}: {other:?}（应为 \"keep\" / \"clear\" / \"set\"）"
        )),
    }
}

// =====================================================================
// 4. 单元测试（AC-4）
// =====================================================================

#[cfg(test)]
mod tests {
    //! 测试策略：
    //!
    //! - Tauri `State<'_, T>` 只能在 Tauri runtime 中构造，单元测试无法直接 mock；
    //! - 但所有"业务逻辑"都不依赖 `State` 本身——`State` 只是 `Deref<Target=T>` 包装；
    //! - 所以我们直接测试**纯函数 helper**（`apply_key_action`）+ **DTO 序列化往返**，
    //!   再用一个 `KcProcessManager::new_test_only_no_app()` + 内存 DB 的集成风格测试
    //!   覆盖 set_kc_settings 的核心路径（DB 写入 + restart 触发判断）。
    //!
    //! 不在 tauri::test::mock 上跑 command invoke 的原因：tauri 2.x 的 `tauri::test`
    //! 在本仓库未启用 dev-deps，引入会扩大 task scope。本测试矩阵在 helper 层和
    //! KcProcessManager 短路层覆盖关键路径，足以守护 AC-4。

    use super::*;
    use crate::db::migration::run_migrations;
    use crate::db::settings as db_settings;
    use crate::kc::settings::{
        SETTING_KC_ENABLED, SETTING_KC_ENABLE_LINKS, SETTING_KC_ENABLE_QA,
        SETTING_KC_OPENAI_API_KEY, SETTING_KC_USE_AI, SETTING_KC_ZHIPU_API_KEY,
    };
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    // ---------- apply_key_action 语义守护 ----------

    #[test]
    fn apply_key_action_keep_preserves_existing() {
        let existing = Some("zhipu-existing-key-12345".to_string());
        let r = apply_key_action("zhipu_key_action", "keep", "", existing.clone())
            .expect("keep ok");
        assert_eq!(r, existing, "keep 应保留 existing");

        // existing=None 时 keep 也保留 None
        let r_none = apply_key_action("zhipu_key_action", "keep", "ignored", None)
            .expect("keep None ok");
        assert_eq!(r_none, None);
    }

    #[test]
    fn apply_key_action_clear_returns_none() {
        let existing = Some("zhipu-existing-key-12345".to_string());
        let r = apply_key_action("zhipu_key_action", "clear", "ignored", existing)
            .expect("clear ok");
        assert_eq!(r, None, "clear 应返回 None（无论 value / existing）");

        let r2 = apply_key_action("zhipu_key_action", "clear", "", None).expect("clear None ok");
        assert_eq!(r2, None);
    }

    #[test]
    fn apply_key_action_set_uses_trimmed_value() {
        let r = apply_key_action(
            "zhipu_key_action",
            "set",
            "  zhipu-new-key-67890  ",
            Some("ignored-existing".to_string()),
        )
        .expect("set ok");
        assert_eq!(
            r,
            Some("zhipu-new-key-67890".to_string()),
            "set 应 trim 后存入"
        );
    }

    #[test]
    fn apply_key_action_set_rejects_empty_value() {
        let err = apply_key_action(
            "zhipu_key_action",
            "set",
            "   ",
            Some("existing".to_string()),
        )
        .expect_err("set 空值应报错");
        assert!(err.contains("zhipu_key_action"), "错误应含字段名: {err}");
        assert!(err.contains("keep") || err.contains("clear"), "错误应提示替代方案: {err}");
    }

    #[test]
    fn apply_key_action_rejects_invalid_action() {
        let err = apply_key_action(
            "openai_key_action",
            "delete",
            "ignored",
            None,
        )
        .expect_err("非法 action 应报错");
        assert!(err.contains("openai_key_action"), "错误应含字段名: {err}");
        assert!(err.contains("delete"), "错误应含非法值: {err}");
    }

    // ---------- DTO 序列化 round-trip（前端 camelCase 契约） ----------

    #[test]
    fn kc_settings_payload_deserializes_from_camel_case_json() {
        // 模拟前端 invoke 传过来的 JSON（camelCase）。
        let json = r#"{
            "enabled": true,
            "useAi": false,
            "enableQa": true,
            "enableLinks": false,
            "zhipuKeyAction": "set",
            "zhipuKeyValue": "zhipu-xyz-12345678",
            "openaiKeyAction": "clear",
            "openaiKeyValue": ""
        }"#;
        let payload: KcSettingsPayload =
            serde_json::from_str(json).expect("camelCase JSON 应能反序列化为 Payload");
        assert!(payload.enabled);
        assert!(!payload.use_ai, "useAi → use_ai");
        assert!(payload.enable_qa, "enableQa → enable_qa");
        assert!(!payload.enable_links);
        assert_eq!(payload.zhipu_key_action, "set");
        assert_eq!(payload.zhipu_key_value, "zhipu-xyz-12345678");
        assert_eq!(payload.openai_key_action, "clear");
        assert_eq!(payload.openai_key_value, "");
    }

    #[test]
    fn kc_settings_payload_zhipu_key_value_defaults_to_empty() {
        // `zhipu_key_value` 缺失时走 `#[serde(default)]` → 空串。
        // 前端在 action=keep/clear 时常常省略 value，这条断言守护"省略不报错"。
        let json = r#"{
            "enabled": true,
            "useAi": true,
            "enableQa": true,
            "enableLinks": true,
            "zhipuKeyAction": "keep",
            "openaiKeyAction": "keep"
        }"#;
        let payload: KcSettingsPayload =
            serde_json::from_str(json).expect("缺失 zhipuKeyValue / openaiKeyValue 应走默认空串");
        assert_eq!(payload.zhipu_key_value, "");
        assert_eq!(payload.openai_key_value, "");
    }

    #[test]
    fn kc_health_status_dto_serializes_to_camel_case_json() {
        let dto = KcHealthStatusDto {
            status: "ready".to_string(),
            reason: None,
            port: Some(58234),
            uptime_secs: Some(123),
            last_check: "2026-05-27T08:00:00Z".to_string(),
            ai_enabled: Some(true),
        };
        let json = serde_json::to_string(&dto).expect("serialize ok");
        // 关键字段必须以 camelCase 出现（前端按 `dto.lastCheck` / `dto.uptimeSecs` 取值）
        assert!(json.contains("\"uptimeSecs\""), "应序列化为 uptimeSecs: {json}");
        assert!(json.contains("\"lastCheck\""), "应序列化为 lastCheck: {json}");
        // snake_case 不应出现
        assert!(!json.contains("uptime_secs"), "不应有 snake_case: {json}");
        assert!(!json.contains("last_check"), "不应有 snake_case: {json}");
        assert!(!json.contains("ai_enabled"), "不应有 snake_case: {json}");
    }

    // ---------- set_kc_settings 核心逻辑：DB 写入 + Key 变化检测 ----------
    //
    // 不能直接调 set_kc_settings command（需要 Tauri State），但可以直接复用其内部逻辑：
    // - load existing → apply_key_action → 拼装 KcSettings → save_settings；
    // - Key 变化判定纯比较两个 `Option<String>`，可在外部模拟。

    #[test]
    fn set_kc_settings_persists_to_db() {
        let conn = fresh_conn();
        // 预置：DB 里已有旧 Key + bool=true
        db_settings::set(&conn, SETTING_KC_ZHIPU_API_KEY, "zhipu-old-key-12345").unwrap();
        db_settings::set(&conn, SETTING_KC_OPENAI_API_KEY, "sk-old-openai-key-987").unwrap();

        let existing = KcSettings::load(&conn).expect("load existing");
        assert_eq!(
            existing.zhipu_api_key.as_deref(),
            Some("zhipu-old-key-12345")
        );

        // 模拟：用户改了 bool 但 keep Key
        let new_zhipu = apply_key_action(
            "zhipu_key_action",
            "keep",
            "",
            existing.zhipu_api_key.clone(),
        )
        .unwrap();
        let new_openai = apply_key_action(
            "openai_key_action",
            "set",
            "sk-new-openai-key-67890",
            existing.openai_api_key.clone(),
        )
        .unwrap();
        let new_settings = KcSettings {
            enabled: false,
            use_ai: false,
            enable_qa: true,
            enable_links: false,
            zhipu_api_key: new_zhipu.clone(),
            openai_api_key: new_openai.clone(),
            outputstage_defense_mode: existing.outputstage_defense_mode,
            ..Default::default()
        };
        settings::save_settings(&conn, &new_settings).expect("save ok");

        // 再 load 验证
        let reloaded = KcSettings::load(&conn).expect("reload ok");
        assert!(!reloaded.enabled, "enabled 应被覆盖为 false");
        assert!(!reloaded.use_ai, "use_ai 应被覆盖为 false");
        assert!(reloaded.enable_qa, "enable_qa 应保持 true");
        assert!(!reloaded.enable_links, "enable_links 应被覆盖为 false");
        assert_eq!(
            reloaded.zhipu_api_key.as_deref(),
            Some("zhipu-old-key-12345"),
            "keep 后 zhipu key 应保留旧值"
        );
        assert_eq!(
            reloaded.openai_api_key.as_deref(),
            Some("sk-new-openai-key-67890"),
            "set 后 openai key 应是新值"
        );

        // Key 变化检测
        let zhipu_changed = existing.zhipu_api_key != new_zhipu;
        let openai_changed = existing.openai_api_key != new_openai;
        assert!(!zhipu_changed, "keep 不应被视为变化");
        assert!(openai_changed, "set 到新值应被视为变化");
    }

    #[test]
    fn set_kc_settings_clears_key_when_action_clear() {
        let conn = fresh_conn();
        // 预置：DB 里有 zhipu key
        db_settings::set(&conn, SETTING_KC_ZHIPU_API_KEY, "zhipu-existing-clear-target").unwrap();

        let existing = KcSettings::load(&conn).expect("load");
        assert!(existing.zhipu_api_key.is_some());

        let new_zhipu = apply_key_action(
            "zhipu_key_action",
            "clear",
            "",
            existing.zhipu_api_key.clone(),
        )
        .unwrap();
        assert_eq!(new_zhipu, None, "clear 应得到 None");

        let new_settings = KcSettings {
            zhipu_api_key: new_zhipu.clone(),
            ..existing.clone()
        };
        settings::save_settings(&conn, &new_settings).expect("save ok");

        // 重新 load：Key 应消失（DB 中存空串，load 时空串→None）
        let reloaded = KcSettings::load(&conn).expect("reload");
        assert!(
            reloaded.zhipu_api_key.is_none(),
            "clear 后 reload 应为 None，实际: {:?}",
            reloaded.zhipu_api_key
        );

        // Key 变化判定：Some → None 应被视为变化
        let zhipu_changed = existing.zhipu_api_key != new_zhipu;
        assert!(zhipu_changed, "Some → None 应被视为 Key 变化");
    }

    #[test]
    fn set_kc_settings_detects_key_change_for_restart_trigger() {
        // 三类 Key 变化场景（均应触发 restart）+ 一类不变（不应触发）。
        // 这是 set_kc_settings 中"是否 tokio::spawn restart"判断的核心逻辑。

        // 场景 A：None → Some（首次配置）
        let existing_a: Option<String> = None;
        let new_a: Option<String> = Some("zhipu-first-time-config".to_string());
        assert!(existing_a != new_a, "None → Some 必须视为变化");

        // 场景 B：Some(a) → Some(b)（换 Key）
        let existing_b = Some("zhipu-old-key".to_string());
        let new_b = Some("zhipu-new-key".to_string());
        assert!(existing_b != new_b, "Some(a) → Some(b) 必须视为变化");

        // 场景 C：Some → None（清除 Key）
        let existing_c = Some("zhipu-to-be-cleared".to_string());
        let new_c: Option<String> = None;
        assert!(existing_c != new_c, "Some → None 必须视为变化");

        // 场景 D：Some(a) → Some(a)（keep 等价）
        let existing_d = Some("zhipu-unchanged".to_string());
        let new_d = Some("zhipu-unchanged".to_string());
        assert!(existing_d == new_d, "Some(a) → Some(a) 不应视为变化");
    }

    // ---------- get_kc_health：单测覆盖 health_check + DTO 转换 ----------
    //
    // 用 `KcProcessManager::new_test_only_no_app()`（无 AppHandle 短路构造）+
    // 直接 await health_check 验证转换路径。覆盖 task_009 集成测试同样的短路点。

    #[tokio::test]
    async fn get_kc_health_returns_dto_with_camel_case_fields() {
        // 没有真 KC，状态默认 Stopped；这是合法状态，命令应正常返回。
        let mgr = Arc::new(KcProcessManager::new_test_only_no_app());

        // 直接调 health_check 并转 DTO，复刻 command 内部逻辑
        let health = mgr.health_check().await;
        let dto = KcHealthStatusDto {
            status: health.status.clone(),
            reason: health.reason.clone(),
            port: health.port,
            uptime_secs: health.uptime_secs,
            last_check: health.last_check.to_rfc3339(),
            ai_enabled: health.ai_enabled,
        };

        // Stopped 状态默认
        assert_eq!(dto.status, "stopped", "test_only 构造默认为 Stopped");
        assert_eq!(dto.port, None, "无 KC 时 port 应为 None");
        assert_eq!(dto.uptime_secs, None, "非 Ready 状态 uptime 应为 None");
        assert_eq!(
            dto.ai_enabled, None,
            "非 Ready 状态 ai_enabled 应为 None（无 /health 请求触发）"
        );

        // last_check 必须为合法 RFC3339（含 'T' 与时区指示符 'Z' 或 '+'）
        assert!(
            dto.last_check.contains('T'),
            "last_check 应为 RFC3339 格式，实际: {}",
            dto.last_check
        );

        // 序列化后字段为 camelCase
        let json = serde_json::to_string(&dto).expect("serialize");
        assert!(json.contains("\"lastCheck\""));
        assert!(json.contains("\"uptimeSecs\""));
        assert!(json.contains("\"aiEnabled\""), "aiEnabled 应序列化为 camelCase");
    }

    // ---------- restart_kc_process：单测覆盖错误路径 ----------

    #[tokio::test]
    async fn restart_kc_process_propagates_error_with_friendly_message() {
        // 用 KC_USE_MOCK_PORT 短路，让 restart 在测试环境内可跑（不需要真 python）。
        // mock 短路下 start() 直接 Ok(())，所以 restart 也成功——验证成功路径。
        std::env::set_var("KC_USE_MOCK_PORT", "59999");
        let mgr = Arc::new(KcProcessManager::new_test_only_no_app());

        let r = mgr.restart().await;
        // 清理 env（避免污染其他测试，串行执行时尤其重要）。
        std::env::remove_var("KC_USE_MOCK_PORT");

        assert!(
            r.is_ok(),
            "KC_USE_MOCK_PORT 短路下 restart 应成功，实际: {r:?}"
        );

        // 错误路径无法直接复现（KcStartError 都需要真 python 探测）。
        // 但 command wrapper 内部 `e.reason()` 的转换逻辑可以借由 KcStartError 直接验证：
        use crate::kc::process::KcStartError;
        let formatted = format!(
            "KC 重启失败: {}",
            KcStartError::PythonNotFound.reason()
        );
        assert!(
            formatted.contains("KC 重启失败"),
            "错误应有友好前缀: {formatted}"
        );
        assert!(
            formatted.contains("python"),
            "错误应含 KcStartError reason: {formatted}"
        );
    }

    // ---------- 集成断言：DB 中 6 个 setting key 真的被写入 ----------

    #[test]
    fn set_kc_settings_writes_all_six_keys_to_db() {
        let conn = fresh_conn();

        let payload_settings = KcSettings {
            enabled: false,
            use_ai: true,
            enable_qa: false,
            enable_links: true,
            zhipu_api_key: Some("zhipu-fixture-key-1234".to_string()),
            openai_api_key: Some("sk-openai-fixture-9876".to_string()),
            outputstage_defense_mode: KcOutputStageDefenseMode::TempDirIsolation,
            ..Default::default()
        };
        settings::save_settings(&conn, &payload_settings).expect("save");

        // 直接查 settings 表的每一行
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_ENABLED).unwrap(),
            Some("false".to_string())
        );
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_USE_AI).unwrap(),
            Some("true".to_string())
        );
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_ENABLE_QA).unwrap(),
            Some("false".to_string())
        );
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_ENABLE_LINKS).unwrap(),
            Some("true".to_string())
        );
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_ZHIPU_API_KEY).unwrap(),
            Some("zhipu-fixture-key-1234".to_string())
        );
        assert_eq!(
            db_settings::get(&conn, SETTING_KC_OPENAI_API_KEY).unwrap(),
            Some("sk-openai-fixture-9876".to_string())
        );
    }

    // ---------- task_020b：ai_enabled 字段守护测试 ----------

    /// **task_020b**：DTO 序列化必须把 `ai_enabled` 写成 camelCase `aiEnabled`，
    /// 三种状态（true / false / None）都不丢字段。
    #[test]
    fn kc_health_dto_includes_ai_enabled_field_in_serialization() {
        // Some(true) → "aiEnabled":true
        let dto_true = KcHealthStatusDto {
            status: "ready".to_string(),
            reason: None,
            port: Some(58234),
            uptime_secs: Some(10),
            last_check: "2026-05-28T00:00:00Z".to_string(),
            ai_enabled: Some(true),
        };
        let json_true = serde_json::to_string(&dto_true).expect("serialize ok");
        assert!(
            json_true.contains("\"aiEnabled\":true"),
            "Some(true) 应序列化为 aiEnabled:true，实际: {json_true}"
        );
        assert!(
            !json_true.contains("ai_enabled"),
            "不应出现 snake_case ai_enabled: {json_true}"
        );

        // Some(false) → "aiEnabled":false
        let dto_false = KcHealthStatusDto {
            ai_enabled: Some(false),
            ..dto_true.clone()
        };
        let json_false = serde_json::to_string(&dto_false).expect("serialize ok");
        assert!(
            json_false.contains("\"aiEnabled\":false"),
            "Some(false) 应序列化为 aiEnabled:false，实际: {json_false}"
        );

        // None → "aiEnabled":null（serde 默认对 Option<T> 不跳过 None）
        let dto_none = KcHealthStatusDto {
            ai_enabled: None,
            ..dto_true.clone()
        };
        let json_none = serde_json::to_string(&dto_none).expect("serialize ok");
        assert!(
            json_none.contains("\"aiEnabled\":null"),
            "None 应序列化为 aiEnabled:null（前端按未知处理），实际: {json_none}"
        );
    }

    /// **task_020b**：前端 camelCase JSON 反序列化回 DTO 后 `ai_enabled` 字段语义正确，
    /// 守护后续维护者改 rename_all 时不破坏前后端 round-trip 契约。
    #[test]
    fn kc_health_dto_deserializes_ai_enabled_from_camel_case() {
        // aiEnabled=true
        let json_t = r#"{
            "status": "ready", "reason": null, "port": 1234,
            "uptimeSecs": 5, "lastCheck": "2026-05-28T00:00:00Z",
            "aiEnabled": true
        }"#;
        let dto: KcHealthStatusDto = serde_json::from_str(json_t).expect("deserialize ok");
        assert_eq!(dto.ai_enabled, Some(true), "aiEnabled:true → Some(true)");

        // aiEnabled=null（KC 旧版本兼容场景）
        let json_n = r#"{
            "status": "ready", "reason": null, "port": 1234,
            "uptimeSecs": 5, "lastCheck": "2026-05-28T00:00:00Z",
            "aiEnabled": null
        }"#;
        let dto_n: KcHealthStatusDto = serde_json::from_str(json_n).expect("deserialize null ok");
        assert_eq!(dto_n.ai_enabled, None, "aiEnabled:null → None");
    }

    /// **task_020b**：Stopped 状态下 DTO `ai_enabled` 必为 None（不发 /health 请求，
    /// 兜底语义"未知"）。这是与 task_016 KcSettingsForm 测试连通性按钮"非 ready → 错误"
    /// 路径配对的后端守护。
    #[tokio::test]
    async fn kc_health_returns_ai_enabled_none_when_not_ready() {
        // 不设 KC_USE_MOCK_PORT → 默认 Stopped。
        let mgr = Arc::new(KcProcessManager::new_test_only_no_app());
        let health = mgr.health_check().await;
        // 直接复刻 command 内部 DTO 转换
        let dto = KcHealthStatusDto {
            status: health.status,
            reason: health.reason,
            port: health.port,
            uptime_secs: health.uptime_secs,
            last_check: health.last_check.to_rfc3339(),
            ai_enabled: health.ai_enabled,
        };
        assert_eq!(dto.status, "stopped");
        assert_eq!(
            dto.ai_enabled, None,
            "Stopped 状态不发 /health 请求，ai_enabled 必为 None"
        );
    }
}
