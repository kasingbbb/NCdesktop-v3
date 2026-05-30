//! NoteCapt 测试基础设施：统一日志初始化，便于 CI / 本地 `cargo test` 排障。

/// 初始化 `env_logger`（测试进程内只需调用一次；重复调用会被忽略）
pub fn init_test_logger() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .is_test(true)
    .format_timestamp_secs()
    .try_init();
}

/// 带 `[TEST]` 前缀的结构化日志，与运行时 `log` 宏一致
#[macro_export]
macro_rules! test_log {
    ($($arg:tt)*) => {
        log::info!(target: "notecapt_test", "[TEST] {}", format_args!($($arg)*));
    };
}
