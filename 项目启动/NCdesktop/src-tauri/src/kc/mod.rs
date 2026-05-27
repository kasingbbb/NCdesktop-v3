//! task_005：KC 增强模块骨架（与 `extraction/` 平级）。
//!
//! 本模块封装 **NCdesktop ↔ KnowledgeCompiler（HTTP 子进程模式）** 的所有集成代码。
//! 设计依据：Architect output.md §"系统架构" + ADR-001 / ADR-002 / ADR-003 / ADR-004 / ADR-006。
//!
//! ## 子模块职责（对照 Architect output.md §"模块职责"表）
//!
//! - [`process`]    — KC 子进程启停 / 健康检查 / 崩溃恢复 / RAII Drop（task_008 实装）；
//! - [`client`]     — HTTP 客户端 + Semaphore 串行化 + 60s 超时 + 错误分类（task_007 实装）；
//! - [`errors`]     — `KcCallError` / `KcEnrichmentOutcome` / `KcFallbackReason` / `KcMeta`（**本 task 实装**）；
//! - [`settings`]   — `KcSettings` 结构 + DB 读写 + Key 屏蔽（task_004 / task_010 实装）；
//! - [`enrichment`] — `enrich()` 入口 + `resolve_outcome()` 纯函数（task_011 实装）；
//! - [`defense`]    — OutputStage 三层防御（cwd 隔离 + 扫描清理，task_014 实装）。
//!
//! ## 本 task 实装的"完整品"
//!
//! 仅 [`errors`] 子模块：
//! - 6 个 `KcCallError` 变体（含 `Malformed`，覆盖 ADR-002 §"错误分类"全部分支）；
//! - 3 个 `KcEnrichmentOutcome` 变体（ADR-004 §"5 类失败兜底状态机"完整三态）；
//! - 6 个 `KcFallbackReason` 变体（Disabled + 5 类失败成因）；
//! - `KcMeta` 11 字段（Architect output.md §"KcMeta 结构"完整定义）；
//! - `KcCallError::to_failure_code()` + `KcFallbackReason::to_failure_code()` 映射函数，
//!   与 `extraction::failure_code::FailureCode::EKc*`（task_003 已落地）一一对齐。
//!
//! 其他 5 个子模块仅占位一行注释，保留为后续 task 的实装入口（避免 task 间编译冲突）。
//!
//! ## 重导出策略
//!
//! 公共类型（`KcCallError` / `KcEnrichmentOutcome` / `KcFallbackReason` / `KcMeta`）通过 `pub use`
//! 暴露在 `kc::` 命名空间，方便上层调用 `crate::kc::KcCallError` 而非 `crate::kc::errors::KcCallError`。

pub mod client; // task_007 实装
pub mod defense; // task_014 实装
pub mod enrichment; // task_011 实装
pub mod errors; // 本 task 实装基础类型
pub mod process; // task_008 实装
pub mod settings; // task_004 / task_010 实装

// 公共类型重导出（task_005 AC-6 + task_009 扩展）。
// 注意：核心 4 个（KcCallError / KcEnrichmentOutcome / KcFallbackReason / KcMeta）来自 errors；
// `KcQaPair` / `KcParagraphLink` / `KcTagsSource` 作为支撑类型保留在 `errors::` 命名空间，
// 调用方需要时显式写 `kc::errors::KcTagsSource`（避免根命名空间被过多边缘类型污染）。
pub use errors::{KcCallError, KcEnrichmentOutcome, KcFallbackReason, KcMeta};

// task_009：lifecycle integration 公共类型重导出，方便 lib.rs / commands / scheduler 引用。
// 把"进程管理 / HTTP 客户端 / 用户配置"三个最高频使用的类型抬升到 `crate::kc::*` 一级路径。
pub use process::{KcProcessManager, KcStatus, KcHealthStatus, KcStartError, PortProvider};
pub use client::{KcClient, KcIngestOptions, KcIngestOutcome};
pub use settings::{
    KcOutputStageDefenseMode, KcSettings, build_env_vars, log_with_mask, mask_secrets_by_keys,
    save_settings,
};
