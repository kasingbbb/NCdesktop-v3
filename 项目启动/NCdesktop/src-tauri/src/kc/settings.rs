//! task_004：`KcSettings` 结构 + Setting key 常量 + DB 读取 + Key 屏蔽。
//! task_010：在 task_004 基础上**追加** Setting 写回 / env 注入 / 日志 mask helper。
//!
//! **设计依据**：
//! - Architect output.md **ADR-007**（LLM Key 注入机制）—— Key 独立于 `llm.api_key`，
//!   走 NC Settings 通道但**不复用同一个 Key 字符串**；env 变量名 `ZHIPUAI_API_KEY` /
//!   `OPENAI_API_KEY`（Python langchain 约定，task_010 `build_env_vars` 严格大写）；
//! - Architect output.md **ADR-006**（OutputStage 三层防御）—— 默认 `FullDefense`
//!   （层 1 + 层 2 + 层 3 全开，KC-MOD-2 到位后可降到 `TrustPersistFalse`）；
//! - Architect output.md **ADR-008**（Settings UI 形态）—— 本文件提供后端数据模型，
//!   前端 `KcSettingsForm.tsx` 复用 `LLMSettingsForm.tsx` 视觉模式；
//! - PRD §"不可妥协的技术底线 #5" / session_context §3 安全约束 —— Key **不明文落盘到日志**：
//!   `Debug` impl 手写屏蔽（不能 derive），`masked_*_key()` 给 UI 展示用，
//!   `mask_secrets()` / `log_with_mask()` 给日志兜底（任意 message 中含 Key 子串都被替换）。
//!
//! **task_005 占位替换说明**：`kc/mod.rs:37` 已 `pub mod settings;`；task_004 把原 2 行占位
//! 替换为实装，不动 `mod.rs` / 其他兄弟模块（`client` / `process` / `enrichment` / `defense`
//! 仍为后续 task 的占位入口）。
//!
//! **task_010 在 task_004 基础上追加的内容**（不动 task_004 已落地的结构 / Default / Debug）：
//! - [`build_env_vars`] — 输出 `Command::env(...)` 入参，**永远不输出空字符串**（避免
//!   KC 误判 "Key 已设置但空"，env 名严格大写）；
//! - [`mask_secrets`]   — 任意 message 中匹配到 Key 子串则替换为占位，**长度 < 8 不替换**
//!   （防止误命中常见短词，如 "abc123" 这种弱 Key 在防御性 mask 不应触发）；
//! - [`log_with_mask`]  — 对外日志入口，强制走 `mask_secrets` 再 `log::log!` 输出，
//!   kc::* 模块内部统一用此函数替代直接 `log::info!`；
//! - [`save_settings`]  — 7 个字段一次性写回 DB，bool 落 "true"/"false" 文本，
//!   防御档位走 `as_str()` snake_case round-trip，Key 为 `None` 时写空串
//!   （与 `load_opt_string` 把空串视为 None 对齐，UI "清除 Key" 语义一致）。
//!
//! **不在本 task scope**：
//! - Tauri command 桥接（task_016 / task_020 前端集成）；
//! - 把 env vars 真正注入到 KC 子进程（task_008 KcProcessManager::start 调用 `build_env_vars`）；
//! - 整体 NC 日志全链路 mask（log_with_mask 仅在 kc::* 模块使用，其他模块由各自团队决定）。
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
// 4. task_010：env 注入 + 日志 mask + DB 写回（公开 helper）
// =====================================================================

/// **task_010 / ADR-007** 子进程 env 注入 helper。
///
/// 输出 `Command::env(...)` 调用入参（KEY=VALUE）。被 `task_008 KcProcessManager::start`
/// 在拉起 KC 子进程前调用：`for (k, v) in build_env_vars(&settings) { cmd.env(k, v); }`。
///
/// **不变量（防 KC 误判）**：
/// - 只在 `Option::is_some() && !is_empty()` 时输出对应 env；
/// - **永远不**输出空字符串作为值（KC / langchain 看到 `ZHIPUAI_API_KEY=""` 会认为
///   "Key 已设置但空"，比"未设置"更糟，直接拒绝服务）；
/// - env 名严格大写（langchain 约定）。
///
/// **安全**：返回值进入 `std::process::Command::env(...)`，仅对子进程可见，**不污染**
/// NC 主进程 environment（session_context §3 #5 "Key 不明文落盘"在此前提下保持）。
pub fn build_env_vars(settings: &KcSettings) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(2);

    // 显式不 trim：用户在 UI 填入末尾空格属于配错，但 load 时已经 filter 空串了；
    // 这里只防"Some 但内容为空"这种极端情况（理论上 load 已挡，双保险）。
    if let Some(k) = settings.zhipu_api_key.as_deref() {
        if !k.is_empty() {
            out.push(("ZHIPUAI_API_KEY".to_string(), k.to_string()));
        }
    }
    if let Some(k) = settings.openai_api_key.as_deref() {
        if !k.is_empty() {
            out.push(("OPENAI_API_KEY".to_string(), k.to_string()));
        }
    }

    out
}

/// **task_010 / 安全护栏** 日志/异常 message 中的 Key 子串屏蔽。
///
/// 任意 `message` 中含 `settings.zhipu_api_key` 字面子串 → 替换为 `<ZHIPU_KEY_MASKED>`；
/// 含 `openai_api_key` → `<OPENAI_KEY_MASKED>`。被 `log_with_mask` 调用。
///
/// **防御性策略**（task 技术约束 "Key 短长度防御"）：
/// - 仅在 Key 长度 ≥ 8 字符时才尝试替换；
/// - < 8 字符的"伪 Key"（如 `abc123` / `test1234`）若也参与替换，会误命中文档/路径/
///   错误码中的常见词，反而污染日志可读性；
/// - 8 字符上限是经验值：智谱 / OpenAI 真实 Key 都远超 8 字符（>= 32），所以这条
///   阈值不会漏掉真实场景。
///
/// **不变量**：任意输入都不 panic（即使两个 Key 都 None / 空字符串 / 只有一个有值）。
///
/// **使用规范**：在 kc::* 模块内部，**禁止**直接调用 `log::info!("settings = {:?}", x)`
/// 形式打印任何可能含 Key 的对象（即便 Debug 已 mask，也以此函数做"运行时字符串兜底"）。
pub fn mask_secrets(message: &str, settings: &KcSettings) -> String {
    let mut result = message.to_string();

    if let Some(k) = settings.zhipu_api_key.as_deref() {
        if k.len() >= MASK_SECRETS_MIN_KEY_LEN && !k.is_empty() {
            result = result.replace(k, "<ZHIPU_KEY_MASKED>");
        }
    }
    if let Some(k) = settings.openai_api_key.as_deref() {
        if k.len() >= MASK_SECRETS_MIN_KEY_LEN && !k.is_empty() {
            result = result.replace(k, "<OPENAI_KEY_MASKED>");
        }
    }

    result
}

/// 短 Key 防御阈值：低于此长度不参与 `mask_secrets` 替换。
///
/// 选 `8` 的根据：智谱 AI Key 实际长度 32+ 字符，OpenAI `sk-...` 也是 51 字符级别；
/// 8 是足够保守的下限——真实 Key 必然超过 8 字符，而 8 字符以下的"伪 Key"
/// 几乎只可能是测试桩 / placeholder / 误填，做替换反而带来误命中风险。
const MASK_SECRETS_MIN_KEY_LEN: usize = 8;

/// **task_010 / AC-3** 日志统一入口：先 `mask_secrets` 再 `log::log!` 输出。
///
/// kc::* 模块内部**所有**含运行时变量的日志都应走此函数，**禁止**直接 `log::info!(...)`。
/// 即使 message 不含 Key 子串，走 mask 也是零成本（`str::replace` 只在子串命中时实际拷贝）。
///
/// 用法示例：
/// ```ignore
/// kc::settings::log_with_mask(
///     log::Level::Info,
///     &format!("KC 拉起命令 args={:?}", cmd_args),
///     &kc_settings,
/// );
/// ```
///
/// 选 `log::Level` 入参（而非分别提供 info/warn/error wrapper）：避免重复函数，
/// 调用方在选 level 时已经做了语义判断。
pub fn log_with_mask(level: log::Level, message: &str, settings: &KcSettings) {
    let masked = mask_secrets(message, settings);
    log::log!(level, "{}", masked);
}

/// **task_010 / AC-4** 把 `KcSettings` 7 个字段一次性写回 `settings` 表。
///
/// **bool → 文本协议**：`"true"` / `"false"`（小写，与 `load_bool` 解析对齐，
/// 与 NC 既有 setting 约定一致）。
///
/// **Option Key → 空串协议**：`None` 写入空串 `""`，与 `load_opt_string`
/// "空串 == None" 的解析对齐。这样 UI 选"清除 Key"也能正确把已存的 Key 抹掉
/// （而不是只写不存在的字段让旧值残留）。
///
/// **失败原子性**：本函数**不**包事务——`db::settings::set` 是 UPSERT，最多 1 行写入；
/// 中途失败时已写入的字段会保留新值。这与"UI 设置保存"语义一致：用户不期望
/// "保存 7 项时第 4 项失败导致前 3 项也回滚"。如未来需事务保护，再考虑加。
pub fn save_settings(conn: &Connection, settings: &KcSettings) -> Result<(), String> {
    db::settings::set(conn, SETTING_KC_ENABLED, bool_str(settings.enabled))?;
    db::settings::set(conn, SETTING_KC_USE_AI, bool_str(settings.use_ai))?;
    db::settings::set(conn, SETTING_KC_ENABLE_QA, bool_str(settings.enable_qa))?;
    db::settings::set(conn, SETTING_KC_ENABLE_LINKS, bool_str(settings.enable_links))?;
    db::settings::set(
        conn,
        SETTING_KC_ZHIPU_API_KEY,
        settings.zhipu_api_key.as_deref().unwrap_or(""),
    )?;
    db::settings::set(
        conn,
        SETTING_KC_OPENAI_API_KEY,
        settings.openai_api_key.as_deref().unwrap_or(""),
    )?;
    db::settings::set(
        conn,
        SETTING_KC_OUTPUTSTAGE_DEFENSE_MODE,
        settings.outputstage_defense_mode.as_str(),
    )?;
    Ok(())
}

/// 内部 helper：bool → 文本（与 `load_bool` 的 "true"/"false" 对齐）。
fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

// =====================================================================
// 5. 内部 DB 读取 helper
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
// 6. 单测（task_004 AC-6 + task_010 AC-5）
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

    // =================================================================
    // task_010 AC-5 测试组（7 个）
    // =================================================================

    // 高熵 fixture：避免 chars 在 message 中天然出现导致误命中（task_010 防御性 mask）。
    const FIXTURE_ZHIPU_KEY: &str = "zhipu-9KsxQ7HvR2nT4mPaY6jLcDeFgWuZ1xVbE3oMiNcA";
    const FIXTURE_OPENAI_KEY: &str = "sk-7HxVbWuZ9QnT4mPaY6jLcDeFgKsxR2oMiNcAE3";

    // ---------- AC-5.1: build_env_vars 对 None Key 不输出 ----------
    #[test]
    fn build_env_vars_omits_none_keys() {
        let s = KcSettings {
            zhipu_api_key: None,
            openai_api_key: None,
            ..Default::default()
        };
        let env = build_env_vars(&s);
        assert!(
            env.is_empty(),
            "两个 Key 都 None 时不应输出任何 env，实际: {env:?}"
        );
    }

    // ---------- AC-5.2: build_env_vars 对空字符串 Key 不输出 ----------
    //
    // 注意：`KcSettings::load` 已把空串过滤为 None，但用户直接构造
    // `KcSettings { zhipu_api_key: Some(String::new()), .. }` 也是合法路径
    // （例如未来 Tauri command 入参可能塞空串）。这条断言守护"双保险"。
    #[test]
    fn build_env_vars_omits_empty_keys() {
        let s = KcSettings {
            zhipu_api_key: Some(String::new()),
            openai_api_key: Some(String::new()),
            ..Default::default()
        };
        let env = build_env_vars(&s);
        assert!(
            env.is_empty(),
            "Some(\"\") 也不应输出 env（避免 KC 看到 'KEY=' 误判），实际: {env:?}"
        );

        // 单边 Some 有值 + 另一边 Some 空串：只输出有值的那个
        let s_half = KcSettings {
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: Some(String::new()),
            ..Default::default()
        };
        let env_half = build_env_vars(&s_half);
        assert_eq!(env_half.len(), 1, "实际: {env_half:?}");
        assert_eq!(env_half[0].0, "ZHIPUAI_API_KEY");
        assert_eq!(env_half[0].1, FIXTURE_ZHIPU_KEY);
    }

    // ---------- 附加（task_010 技术约束）：env 名严格大写 ----------
    #[test]
    fn build_env_vars_uses_strict_uppercase_names() {
        let s = KcSettings {
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: Some(FIXTURE_OPENAI_KEY.to_string()),
            ..Default::default()
        };
        let env = build_env_vars(&s);
        let names: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();

        // langchain 约定（input.md 技术约束）：必须是 ZHIPUAI_API_KEY / OPENAI_API_KEY 大写
        assert!(
            names.contains(&"ZHIPUAI_API_KEY"),
            "env 名必须严格大写，实际: {names:?}"
        );
        assert!(
            names.contains(&"OPENAI_API_KEY"),
            "env 名必须严格大写，实际: {names:?}"
        );
        // 反例：不应出现小写或混合
        for (k, _) in &env {
            assert_eq!(
                k.as_str(),
                k.to_ascii_uppercase().as_str(),
                "env 名包含非大写字符: {k}"
            );
        }
    }

    // ---------- AC-5.3: mask_secrets 替换 zhipu key ----------
    #[test]
    fn mask_secrets_replaces_zhipu_key() {
        let s = KcSettings {
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: None,
            ..Default::default()
        };
        let msg = format!("KC 启动失败 reason=auth, key={FIXTURE_ZHIPU_KEY}, retry=true");
        let masked = mask_secrets(&msg, &s);

        // 关键断言：高熵 fixture 一定不会在 mask 后残留
        assert!(
            !masked.contains(FIXTURE_ZHIPU_KEY),
            "mask 后不得含 zhipu key 原文，实际: {masked}"
        );
        // 占位符必须出现
        assert!(
            masked.contains("<ZHIPU_KEY_MASKED>"),
            "mask 后应含 <ZHIPU_KEY_MASKED> 占位，实际: {masked}"
        );
        // 其他内容应保留
        assert!(masked.contains("KC 启动失败"));
        assert!(masked.contains("retry=true"));
    }

    // ---------- AC-5.4: mask_secrets 替换 openai key ----------
    #[test]
    fn mask_secrets_replaces_openai_key() {
        let s = KcSettings {
            zhipu_api_key: None,
            openai_api_key: Some(FIXTURE_OPENAI_KEY.to_string()),
            ..Default::default()
        };
        let msg = format!("[langchain] using model=gpt-4, OPENAI_API_KEY={FIXTURE_OPENAI_KEY}");
        let masked = mask_secrets(&msg, &s);

        assert!(
            !masked.contains(FIXTURE_OPENAI_KEY),
            "mask 后不得含 openai key 原文，实际: {masked}"
        );
        assert!(
            masked.contains("<OPENAI_KEY_MASKED>"),
            "mask 后应含 <OPENAI_KEY_MASKED> 占位，实际: {masked}"
        );
        assert!(masked.contains("model=gpt-4"));
    }

    // ---------- AC-5.5: mask_secrets 保留其他文本 ----------
    #[test]
    fn mask_secrets_preserves_other_text() {
        let s = KcSettings {
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: Some(FIXTURE_OPENAI_KEY.to_string()),
            ..Default::default()
        };

        // 任何不含 Key 子串的 message 必须原样返回
        let msg = "KC 健康检查 OK，端口=12345，uptime=123s，model=glm-4";
        let masked = mask_secrets(msg, &s);
        assert_eq!(masked, msg, "不含 Key 子串时应原样返回");

        // 边界：空 message
        assert_eq!(mask_secrets("", &s), "");

        // 边界：两个 Key 都 None，但 message 长 → 不可能 panic / 误替换
        let s_none = KcSettings::default();
        let long_msg = "x".repeat(1000);
        assert_eq!(
            mask_secrets(&long_msg, &s_none),
            long_msg,
            "两个 Key 都 None 时任意 message 都原样返回"
        );
    }

    // ---------- AC-5.6: mask_secrets 不替换短 Key（< 8 字符） ----------
    #[test]
    fn mask_secrets_does_not_replace_short_key() {
        // 7 字符以下：不替换（防御性，input.md 要求"长度 < 8 不做替换"）
        let s_short = KcSettings {
            zhipu_api_key: Some("abc1234".to_string()), // 7 chars
            openai_api_key: Some("test123".to_string()), // 7 chars
            ..Default::default()
        };
        let msg = "log entry: abc1234 / test123 / other";
        let masked = mask_secrets(msg, &s_short);
        assert_eq!(
            masked, msg,
            "Key < 8 字符时不应替换（防止误命中常见短词），实际: {masked}"
        );

        // 边界：恰好 8 字符 → 应该替换（"< 8 不替换" → "≥ 8 替换"）
        let s_8 = KcSettings {
            zhipu_api_key: Some("abc12345".to_string()), // 8 chars
            openai_api_key: None,
            ..Default::default()
        };
        let msg_8 = "log: abc12345 found";
        let masked_8 = mask_secrets(msg_8, &s_8);
        assert!(
            masked_8.contains("<ZHIPU_KEY_MASKED>"),
            "Key == 8 字符时应替换（边界）"
        );
        assert!(!masked_8.contains("abc12345"));
    }

    // ---------- AC-5.7: save_and_load roundtrip ----------
    #[test]
    fn save_and_load_roundtrip() {
        let conn = fresh_conn();

        // 用非默认值，验证每个字段都真正经过 DB
        let original = KcSettings {
            enabled: false,
            use_ai: false,
            enable_qa: true,
            enable_links: false,
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: Some(FIXTURE_OPENAI_KEY.to_string()),
            outputstage_defense_mode: KcOutputStageDefenseMode::TempDirIsolation,
        };

        save_settings(&conn, &original).expect("save ok");
        let loaded = KcSettings::load(&conn).expect("load ok");

        // 7 个字段逐一比对
        assert_eq!(loaded.enabled, original.enabled);
        assert_eq!(loaded.use_ai, original.use_ai);
        assert_eq!(loaded.enable_qa, original.enable_qa);
        assert_eq!(loaded.enable_links, original.enable_links);
        assert_eq!(loaded.zhipu_api_key, original.zhipu_api_key);
        assert_eq!(loaded.openai_api_key, original.openai_api_key);
        assert_eq!(
            loaded.outputstage_defense_mode,
            original.outputstage_defense_mode
        );

        // 二次 save 应该覆盖（UPSERT 语义）
        let updated = KcSettings {
            zhipu_api_key: None, // 用户"清除 Key"路径
            openai_api_key: None,
            ..original.clone()
        };
        save_settings(&conn, &updated).expect("re-save ok");
        let reloaded = KcSettings::load(&conn).expect("reload ok");
        assert!(
            reloaded.zhipu_api_key.is_none(),
            "save None Key 后再 load 必须是 None（覆盖语义）"
        );
        assert!(reloaded.openai_api_key.is_none());

        // 但其他字段保持 updated 的值
        assert_eq!(reloaded.enabled, updated.enabled);
    }

    // ---------- 附加（task_010 安全护栏）：log_with_mask 不泄漏 Key ----------
    //
    // 注：log::Level 接口的副作用（实际是否写日志文件）由 logger backend 决定，
    // 单测环境无 logger 时也不会失败。本测试验证 `log_with_mask` 内部走 mask_secrets
    // 路径（即间接验证：调用前的 message 经过 mask 后必然不含原 Key）。
    #[test]
    fn log_with_mask_routes_through_mask_secrets() {
        let s = KcSettings {
            zhipu_api_key: Some(FIXTURE_ZHIPU_KEY.to_string()),
            openai_api_key: Some(FIXTURE_OPENAI_KEY.to_string()),
            ..Default::default()
        };

        // 含两种 Key 的 message
        let dangerous = format!(
            "ingest failed: zhipu={FIXTURE_ZHIPU_KEY}, fallback openai={FIXTURE_OPENAI_KEY}"
        );

        // 通过 mask_secrets（与 log_with_mask 内部同一路径）验证 mask 真的发生
        let masked = mask_secrets(&dangerous, &s);
        assert!(
            !masked.contains(FIXTURE_ZHIPU_KEY),
            "log_with_mask 路径必须屏蔽 zhipu key，实际: {masked}"
        );
        assert!(
            !masked.contains(FIXTURE_OPENAI_KEY),
            "log_with_mask 路径必须屏蔽 openai key，实际: {masked}"
        );
        assert!(masked.contains("<ZHIPU_KEY_MASKED>"));
        assert!(masked.contains("<OPENAI_KEY_MASKED>"));

        // log_with_mask 调用本身不能 panic（即便 logger 未初始化）
        log_with_mask(log::Level::Info, &dangerous, &s);
        log_with_mask(log::Level::Warn, &dangerous, &s);
        log_with_mask(log::Level::Error, &dangerous, &s);
        log_with_mask(log::Level::Debug, "", &s); // 空 message 也不应 panic
        log_with_mask(log::Level::Trace, "no key here", &KcSettings::default());
    }
}
