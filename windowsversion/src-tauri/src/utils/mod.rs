// 工具函数模块
// task_008（M-1 关闭）：scheduler::write_derivative_md 依赖 safe_name::sanitize_stem。
// safe_name.rs 在仓库中早已存在但 mod.rs 未声明，与 db::extraction 同属"注册缺口"。
pub mod safe_name;
// custom_prompt_v1 / task_002：`startup::workspace_startup_hooks` 依赖
// `utils::nfc::nfc_heal_workspace` 与 `utils::safe_rename::cleanup_pending_scan`；
// `safe_rename` 自身又依赖 `utils::ipc_error`。仅挂接既有孤儿模块，**不调用**
// （bootstrap 流程在 task_002 范围内不接入；本 task 只解决 R5 `AppMode` 注册路径）。
pub mod nfc;
pub mod safe_rename;
pub mod ipc_error;
