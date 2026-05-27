//! task_006_mock_kc_server：跨集成测试 binary 复用的测试基础设施。
//!
//! ## 设计依据
//!
//! Rust 的"集成测试"机制是：`tests/` 下每个 `.rs` 是独立的 crate（独立编译为单 binary）。
//! 为了跨 binary 共享 helper，惯用 `tests/common/mod.rs` 模式：每个集成测试 `.rs` 用
//! `mod common;` 引入（编译期被 inline 到该 binary，没有"公共 crate"概念）。
//!
//! ## 当前导出
//!
//! - [`mock_kc`] — `MockKcServer`（wiremock-based KC HTTP mock，task_007/008/011/022/023 复用）。
//!
//! ## dead_code 抑制
//!
//! 不同集成测试 binary 仅使用本模块的子集（例如 `mock_kc_server_basic.rs` 只用 4 个 scenario，
//! task_022 失败注入会用其余 3 个）。Rust 编译每个 binary 时会把未使用的 `pub fn` 标为 dead_code 警告。
//! 用 `#![allow(dead_code)]` 抑制（unique 到 common，不影响 src/ 主代码的 dead_code 检测）。

#![allow(dead_code)]

pub mod mock_kc;
