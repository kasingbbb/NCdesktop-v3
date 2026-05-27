//! task_004：`KcSettings` 结构 + Setting key 常量 + DB 读取 + Key 屏蔽。
//!
//! **设计依据**：
//! - Architect output.md **ADR-007**（LLM Key 注入机制）—— Key 独立于 `llm.api_key`，
//!   走 NC Settings 通道但**不复用同一个 Key 字符串**；
//! - Architect output.md **ADR-006**（OutputStage 三层防御）—— 默认 `FullDefense`
//!   （层 1 + 层 2 + 层 3 全开，KC-MOD-2 到位后可降到 `TrustPersistFalse`）；
//! - Architect output.md **ADR-008**（Settings UI 形态）—— 本文件提供后端数据模型，
//!   前端 `KcSettingsForm.tsx` 复用 `LLMSettingsForm.tsx` 视觉模式；
//! - PRD §"不可妥协的技术底线 #5" / session_context §3 安全约束 —— Key **不明文落盘到日志**：
//!   `Debug` impl 手写屏蔽（不能 derive），`masked_*_key()` 公开方法给上层 UI / 日志使用。
//!
//! **task_005 占位替换说明**：`kc/mod.rs:37` 已 `pub mod settings;`；本 task 把原 2 行占位
//! 替换为实装，不动 `mod.rs` / 其他兄弟模块（`client` / `process` / `enrichment` / `defense`
//! 仍为后续 task 的占位入口）。
//!
//! **不在本 task scope**：
//! - Setting **写**入接口（task_010 Setting Loader）；
//! - Tauri command 桥接（task_016 前端集成）；
//! - 把 key 注入到 KC 子进程的 env（task_008 KcProcessManager::start）。
//!
//! **DB 默认值机制**：NC 既有 `settings` 表是简单 KV（[db/migration.rs:1058]），
//! **不预填默认值**；缺失时由应用层（本文件 `KcSettings::load`）走 `Default::default()`。
//! 这与 `llm/client.rs` 的 `from_db_or_env` 模式一致。

use rusqlite::Connection;

use crate::db;

// =====================================================================
// 1. Setting key 常量（AC-1）
// =====================================================================
//
// 命名规范：`kc.<feature>`（与 NC 现有 `llm.api_key` 同模式，见 [llm/client.rs:7]）。
// 不变量：本表 7 个常量与 `KcSettings` 的 7 个字段严格一一对应。

pub const SETTING_KC_ENABLED: &str = "kc.enabled";
pub const SETTING_KC_USE_AI: &str = "kc.use_ai";
pub const SETTING_KC_ENABLE_QA: &str = "kc.enable_qa";
pub const SETTING_KC_ENABLE_LINKS: &str = "kc.enable_links";
pub const SETTING_KC_ZHIPU_API_KEY: &str = "kc.zhipu_api_key";
pub const SETTING_KC_OPENAI_API_KEY: &str = "kc.openai_api_key";
pub const SETTING_KC_OUTPUTSTAGE_DEFENSE_MODE: &str = "kc.outputstage_defense_mode";

// =====================================================================
// 2. OutputStage 防御模式枚举（AC-3，ADR-006）
// =====================================================================

/// KC 落盘越界防御档位（ADR-006 三层防御对应三个枚举值）。
///
/// | 变体 | 含义 | 何时使用 |
/// |--|--|--|
/// | `TrustPersistFalse`  | 仅层 1：信任 KC `persist: false`，不做兜底 | KC-MOD-2 已到位、有信心 |
/// | `TempDirIsolation`   | 层 1 + 层 2：临时目录 cwd 隔离        | KC-MOD-2 部分到位的过渡期 |
/// | `FullDefense`        | 层 1 + 层 2 + 层 3：临时目录 + 兜底扫描清理（**默认**） | MVP 默认档（最严防御） |
///
/// 不变量：`from_str` 与 `as_str` 必须严格 round-trip（写回 DB 时不丢精度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcOutputStageDefenseMode {
    TrustPersistFalse,
    TempDirIsolation,
    FullDefense,
}

impl KcOutputStageDefenseMode {
    /// 解析 DB 字面值（容错：未知值 / 空字符串 → 默认 `FullDefense`，保留最严防御）。
    ///
    /// 注意：返回类型选 `Self` 而非 `Result<Self, _>`——容错是 setting load 的常规策略
    /// （即使用户改坏 DB，也不让进程崩溃；与 ADR-006 "防御兜底"一致）。
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "trust_persist_false" => Self::TrustPersistFalse,
            "temp_dir_isolation" => Self::TempDirIsolation,
            "full_defense" => Self::FullDefense,
            // 未知值兜底：返回最严档（不放宽防御）。
            _ => Self::FullDefense,
        }
    }

    /// DB 字面值（snake_case，与 NC 既有 setting 风格一致）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrustPersistFalse => "trust_persist_false",
            Self::TempDirIsolation => "temp_dir_isolation",
            Self::FullDefense => "full_defense",
        }
    }
}

impl Default for KcOutputStageDefenseMode {
    fn default() -> Self {
        // ADR-006 §"决策" 明确默认 `FullDefense`。
        Self::FullDefense
    }
}

// =====================================================================
// 3. KcSettings 主结构（AC-2）
// =====================================================================

/// KC 集成层全部用户可配设置（7 字段，与 7 个 SETTING_KC_* 常量一一对应）。
///
/// **安全要求**：
/// - 两个 `Option<String>` key 字段（`zhipu_api_key` / `openai_api_key`）**不进**日志：
///   `Debug` impl 手写屏蔽（见下方），上层显示时必须走 `masked_*_key()`。
/// - `Display` 未实装——禁止在 `println!("{}", settings)` 场景误展示 key。
///
/// **加载策略**：通过 [`KcSettings::load`] 从 DB 读取；任一字段在 settings 表缺失时走
/// `Default::default()` 单字段兜底（不是整体兜底——存在 `kc.enabled = true` 但
/// `kc.zhipu_api_key` 未配的合法状态）。
#[derive(Clone)]
pub struct KcSettings {
    /// KC 总开关。`false` 时整条增强路径短路，scheduler 走 markitdown 原 MD 落地。
    pub enabled: bool,

    /// AI 增强子开关。`false` 时强制 KC 走规则增强（不调智谱/OpenAI），即使 Key 已配。
    pub use_ai: bool,

    /// 是否启用问答对生成（KC 内部 `enable_qa` 入参）。
    pub enable_qa: bool,

    /// 是否启用段落关联（KC 内部 `enable_links` 入参）。
    pub enable_links: bool,

    /// 智谱 AI API Key（注入 KC 子进程的 `ZHIPUAI_API_KEY` 环境变量，task_008 负责）。
    /// `None` 表示未配置；在 UI 上展示为"未配置（AI 功能受限）"。
    pub zhipu_api_key: Option<String>,

    /// OpenAI API Key（注入 KC 子进程的 `OPENAI_API_KEY`，作为智谱不可达时的兜底）。
    pub openai_api_key: Option<String>,

    /// OutputStage 越界防御档位（ADR-006）。
    pub outputstage_defense_mode: KcOutputStageDefenseMode,
}

impl Default for KcSettings {
    /// 默认值（AC-2）：四个 bool 开关全开、两个 Key 未配置、防御档位最严。
    ///
    /// 设计意图：用户首次启动时 KC 立即 ready（enabled=true），但因为 Key 未配置，
    /// `use_ai` 实际效果会被 KC 内部 fallback 到规则增强；用户配置 Key 后即生效。
    fn default() -> Self {
        Self {
            enabled: true,
            use_ai: true,
            enable_qa: true,
            enable_links: true,
            zhipu_api_key: None,
            openai_api_key: None,
            outputstage_defense_mode: KcOutputStageDefenseMode::default(),
        }
    }
}

/// 手写 `Debug`（**不 derive**）——屏蔽两个 API Key 字段。
///
/// 不变量：`format!("{:?}", settings)` 输出**不得**包含 key 真实内容；
/// 即使日后字段顺序调整，也必须保留此屏蔽（单测 `kc_settings_debug_masks_keys` 守护）。
///
/// 为什么必要：NC 现有 `log` 框架（[llm/client.rs] 同款）无敏感字段 wrapper，
/// 任何 `log::info!("settings = {:?}", settings)` 都会把 key 写入 RotatingFileAppender；
/// session_context §3 不可妥协底线 "Key 不明文落盘到日志" 通过此屏蔽兜底。
impl std::fmt::Debug for KcSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KcSettings")
            .field("enabled", &self.enabled)
            .field("use_ai", &self.use_ai)
            .field("enable_qa", &self.enable_qa)
            .field("enable_links", &self.enable_links)
            .field("zhipu_api_key", &mask_for_debug(self.zhipu_api_key.as_deref()))
            .field("openai_api_key", &mask_for_debug(self.openai_api_key.as_deref()))
            .field("outputstage_defense_mode", &self.outputstage_defense_mode)
            .finish()
    }
}

/// 内部辅助：把 `Option<&str>` 转成 Debug 安全的占位字符串。
/// `None` → `"<unset>"`；`Some` → `"<redacted>"`（不暴露任何字节，连前缀都不给）。
///
/// 与 `masked_zhipu_key()` / `masked_openai_key()` 区别：
/// - `masked_*_key()` 是给 **UI 显示**（前 6 字符 + `***`）用，让用户能识别"是不是我自己配的那个 Key"；
/// - `mask_for_debug` 是给 **日志/Debug** 用，零信息量，避免日志泄露任何 Key 字节。
fn mask_for_debug(opt: Option<&str>) -> &'static str {
    match opt {
        None => "<unset>",
        Some(_) => "<redacted>",
    }
}

impl KcSettings {
    /// 从 DB 读取全部 7 个字段（AC-4）。
    ///
    /// 字段级容错：任一 key 在 `settings` 表中缺失或值非法（如 bool 不是 "true"/"false"），
    /// 都走该字段的 `Default`，**不向上抛错**。
    /// 仅当 DB 读取本身失败（连接断、SQL 语法错）才返回 `Err`——此时上层应日志 + 走整体 default。
    pub fn load(conn: &Connection) -> Result<Self, String> {
        Ok(Self {
            enabled: load_bool(conn, SETTING_KC_ENABLED, true)?,
            use_ai: load_bool(conn, SETTING_KC_USE_AI, true)?,
            enable_qa: load_bool(conn, SETTING_KC_ENABLE_QA, true)?,
            enable_links: load_bool(conn, SETTING_KC_ENABLE_LINKS, true)?,
            zhipu_api_key: load_opt_string(conn, SETTING_KC_ZHIPU_API_KEY)?,
            openai_api_key: load_opt_string(conn, SETTING_KC_OPENAI_API_KEY)?,
            outputstage_defense_mode: load_defense_mode(conn)?,
        })
    }

    /// 智谱 Key 的 UI 展示版本（AC-5）：前 6 字符 + `***`；`None` → `"未配置"`。
    ///
    /// 边界（task 文档要求"不能 panic"）：
    /// - `None` → `"未配置"`；
    /// - 空字符串 → `"未配置"`（DB 中明文存空串等价未配置，避免显示 "***" 误导用户）；
    /// - 长度 ≤ 6 → `"***"`（前 6 字符不存在，只能给固定 mask）；
    /// - 长度 > 6 → 前 6 字符 + `"***"`（用 `chars().take(6)` 而非字节切片，UTF-8 安全）。
    pub fn masked_zhipu_key(&self) -> String {
        mask_key_for_display(self.zhipu_api_key.as_deref())
    }

    /// OpenAI Key 的 UI 展示版本（AC-5），规则同 `masked_zhipu_key`。
    pub fn masked_openai_key(&self) -> String {
        mask_key_for_display(self.openai_api_key.as_deref())
    }
}

/// 内部 helper：UI 展示版 mask。
///
/// 选 `chars().take(6)` 是为了应对 KC 用户在中国大陆环境，Key 中可能混入 UTF-8 字符
/// （虽然智谱 API Key 实际上是 ASCII，但显式 UTF-8 安全永远不亏）。
fn mask_key_for_display(opt: Option<&str>) -> String {
    match opt {
        None => "未配置".to_string(),
        Some(s) if s.is_empty() => "未配置".to_string(),
        Some(s) if s.chars().count() <= 6 => "***".to_string(),
        Some(s) => {
            let prefix: String = s.chars().take(6).collect();
            format!("{prefix}***")
        }
    }
}

// =====================================================================
// 4. 内部 DB 读取 helper
// =====================================================================

/// 读 bool（DB 文本 "true"/"false"，沿用 NC 既有约定）。
///
/// 容错（task 技术约束 "Key 屏蔽不能 panic" 思路推广）：
/// - 字段缺失 → 默认值；
/// - 字面值无法解析（`"yes"` / `""` / `"1"`）→ 默认值。
fn load_bool(conn: &Connection, key: &str, default: bool) -> Result<bool, String> {
    match db::settings::get(conn, key)? {
        None => Ok(default),
        Some(v) => match v.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            // 未知字面值：宁可走默认也不抛错（user-configurable setting 的鲁棒性约定）。
            _ => Ok(default),
        },
    }
}

/// 读可选字符串。空字符串视为 `None`（与 `masked_*_key` 的空字符串视为未配置一致）。
fn load_opt_string(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    Ok(db::settings::get(conn, key)?.filter(|s| !s.is_empty()))
}

/// 读防御模式（缺失 / 非法 → `FullDefense`，保留最严防御）。
fn load_defense_mode(conn: &Connection) -> Result<KcOutputStageDefenseMode, String> {
    Ok(match db::settings::get(conn, SETTING_KC_OUTPUTSTAGE_DEFENSE_MODE)? {
        None => KcOutputStageDefenseMode::default(),
        Some(s) => KcOutputStageDefenseMode::from_str(&s),
    })
}

// =====================================================================
// 5. 单测（AC-6）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use crate::db::settings as db_settings;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    // ---------- AC-6.1: 空 DB 读出全默认 ----------
    #[test]
    fn kc_settings_default_when_db_empty() {
        let conn = fresh_conn();
        let s = KcSettings::load(&conn).expect("load ok");

        // 与 AC-2 默认值表精确一致。
        assert!(s.enabled);
        assert!(s.use_ai);
        assert!(s.enable_qa);
        assert!(s.enable_links);
        assert!(s.zhipu_api_key.is_none());
        assert!(s.openai_api_key.is_none());
        assert_eq!(
            s.outputstage_defense_mode,
            KcOutputStageDefenseMode::FullDefense
        );
    }

    // ---------- AC-6.2: 写入 DB 后能正确加载 ----------
    #[test]
    fn kc_settings_load_from_db() {
        let conn = fresh_conn();
        db_settings::set(&conn, SETTING_KC_ENABLED, "false").unwrap();
        db_settings::set(&conn, SETTING_KC_USE_AI, "false").unwrap();
        db_settings::set(&conn, SETTING_KC_ENABLE_QA, "true").unwrap();
        db_settings::set(&conn, SETTING_KC_ENABLE_LINKS, "false").unwrap();
        db_settings::set(&conn, SETTING_KC_ZHIPU_API_KEY, "zhipu-abc1234567890").unwrap();
        db_settings::set(&conn, SETTING_KC_OPENAI_API_KEY, "sk-openai9876543").unwrap();
        db_settings::set(
            &conn,
            SETTING_KC_OUTPUTSTAGE_DEFENSE_MODE,
            "temp_dir_isolation",
        )
        .unwrap();

        let s = KcSettings::load(&conn).expect("load ok");
        assert!(!s.enabled);
        assert!(!s.use_ai);
        assert!(s.enable_qa);
        assert!(!s.enable_links);
        assert_eq!(s.zhipu_api_key.as_deref(), Some("zhipu-abc1234567890"));
        assert_eq!(s.openai_api_key.as_deref(), Some("sk-openai9876543"));
        assert_eq!(
            s.outputstage_defense_mode,
            KcOutputStageDefenseMode::TempDirIsolation
        );
    }

    // ---------- AC-6.3: masked 显示 ----------
    #[test]
    fn kc_settings_masks_api_key() {
        // 长 Key：前 6 字符 + ***
        let s = KcSettings {
            zhipu_api_key: Some("zhipu-abc1234567890".to_string()),
            openai_api_key: Some("sk-openai9876543".to_string()),
            ..Default::default()
        };
        assert_eq!(s.masked_zhipu_key(), "zhipu-***");
        assert_eq!(s.masked_openai_key(), "sk-ope***");

        // None → "未配置"
        let s_none = KcSettings::default();
        assert_eq!(s_none.masked_zhipu_key(), "未配置");
        assert_eq!(s_none.masked_openai_key(), "未配置");

        // 空字符串 → "未配置"（不能 panic）
        let s_empty = KcSettings {
            zhipu_api_key: Some(String::new()),
            openai_api_key: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(s_empty.masked_zhipu_key(), "未配置");
        assert_eq!(s_empty.masked_openai_key(), "未配置");

        // 短 Key（<= 6 字符）→ "***"
        let s_short = KcSettings {
            zhipu_api_key: Some("abc".to_string()),
            openai_api_key: Some("123456".to_string()),
            ..Default::default()
        };
        assert_eq!(s_short.masked_zhipu_key(), "***");
        assert_eq!(s_short.masked_openai_key(), "***");
    }

    // ---------- 附加：Debug 不暴露真实 Key（安全护栏，session_context §3） ----------
    #[test]
    fn kc_settings_debug_masks_keys() {
        let s = KcSettings {
            zhipu_api_key: Some("zhipu-SUPERSECRET-DO-NOT-LOG".to_string()),
            openai_api_key: Some("sk-OPENAI-ULTRA-SENSITIVE-KEY".to_string()),
            ..Default::default()
        };
        let dbg = format!("{:?}", s);
        // 让 `cargo test ... -- --nocapture` 时能眼见为实。
        eprintln!("debug-with-keys = {dbg}");

        // 必须不含 key 的任何真实片段（即使 6 字符前缀也不行——Debug 比 UI mask 更严）。
        assert!(
            !dbg.contains("SUPERSECRET"),
            "Debug 输出不得含 zhipu key 真实片段，实际: {dbg}"
        );
        assert!(
            !dbg.contains("ULTRA-SENSITIVE"),
            "Debug 输出不得含 openai key 真实片段，实际: {dbg}"
        );
        assert!(
            !dbg.contains("zhipu-SUPERSECRET"),
            "Debug 输出不得含完整 zhipu key，实际: {dbg}"
        );
        assert!(
            !dbg.contains("sk-OPENAI"),
            "Debug 输出不得含 openai key 前缀，实际: {dbg}"
        );

        // 必须含 redacted 占位（让人能看出"这里有 Key 但被 mask 了"）。
        assert!(
            dbg.contains("<redacted>"),
            "Debug 输出应含 <redacted> 占位，实际: {dbg}"
        );

        // None 字段应展示 <unset>
        let s_none = KcSettings::default();
        let dbg_none = format!("{:?}", s_none);
        assert!(
            dbg_none.contains("<unset>"),
            "Debug 输出 None 字段应展示 <unset>，实际: {dbg_none}"
        );
    }

    // ---------- 附加：KcOutputStageDefenseMode round-trip ----------
    #[test]
    fn kc_defense_mode_roundtrip() {
        for m in [
            KcOutputStageDefenseMode::TrustPersistFalse,
            KcOutputStageDefenseMode::TempDirIsolation,
            KcOutputStageDefenseMode::FullDefense,
        ] {
            let s = m.as_str();
            assert_eq!(KcOutputStageDefenseMode::from_str(s), m);
        }

        // 未知值兜底 FullDefense（保留最严防御，ADR-006）
        assert_eq!(
            KcOutputStageDefenseMode::from_str("unknown"),
            KcOutputStageDefenseMode::FullDefense
        );
        assert_eq!(
            KcOutputStageDefenseMode::from_str(""),
            KcOutputStageDefenseMode::FullDefense
        );
    }

    // ---------- 附加：DB 字面值非法时单字段走默认（容错语义） ----------
    #[test]
    fn kc_settings_load_with_invalid_bool_falls_back_to_default() {
        let conn = fresh_conn();
        // 写入非法 bool 值
        db_settings::set(&conn, SETTING_KC_ENABLED, "yes").unwrap();
        db_settings::set(&conn, SETTING_KC_USE_AI, "1").unwrap();

        let s = KcSettings::load(&conn).expect("load ok");
        // 容错：走默认 true（不向上抛错）
        assert!(s.enabled, "非法 bool 'yes' 应走默认 true");
        assert!(s.use_ai, "非法 bool '1' 应走默认 true");
    }

    // ---------- 附加：DB 空字符串 Key 视为 None ----------
    #[test]
    fn kc_settings_empty_key_string_loads_as_none() {
        let conn = fresh_conn();
        db_settings::set(&conn, SETTING_KC_ZHIPU_API_KEY, "").unwrap();
        db_settings::set(&conn, SETTING_KC_OPENAI_API_KEY, "").unwrap();

        let s = KcSettings::load(&conn).expect("load ok");
        assert!(s.zhipu_api_key.is_none(), "空字符串 Key 应等价于 None");
        assert!(s.openai_api_key.is_none(), "空字符串 Key 应等价于 None");
    }
}
