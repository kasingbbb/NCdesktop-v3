//! 可移动卷监听：轮询 `/Volumes/Notecapt` 的挂载状态，在「未挂载 → 已挂载」
//! 边沿触发一次扫描，把新图片清单 emit 给前端。
//!
//! 为什么轮询而不用 `notify`：macOS 下 `/Volumes` 的挂载点是动态目录，`notify`
//! 对挂载/卸载事件的覆盖在不同系统版本上不稳定；这里只关心「目标卷在不在」这一个
//! 布尔的边沿，定时轮询最简单、最可预测（demo 友好）。
//!
//! 触发模型：只在 `false → true` 边沿 emit（即「插入/重连一次 → 检测一次」），
//! 避免卡常驻时反复打扰。`USBReenum` 在 USB 硬件层重枚举会让卷真实地卸载再挂载，
//! 因此与物理插拔产生同样的边沿。

use crate::sync::usb_import;
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// 前端监听的事件名。payload 为 [`usb_import::CardScan`]。
pub const CARD_DETECTED_EVENT: &str = "usb-card-detected";

/// 轮询间隔（毫秒）。1.5s 对插拔检测足够灵敏，CPU 占用可忽略。
const POLL_INTERVAL_MS: u64 = 1500;

/// 首轮延迟（毫秒）：给前端 webview 挂载并注册事件监听留出时间，
/// 避免「启动时卡已在」的 emit 早于监听注册而丢失（仿 KC 的 500ms 延迟，放大到 2s）。
const STARTUP_DELAY_MS: u64 = 2000;

/// 启动监听循环（非阻塞，进受管 async runtime）。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mount = Path::new("/Volumes").join(usb_import::TARGET_VOLUME_NAME);

        tokio::time::sleep(std::time::Duration::from_millis(STARTUP_DELAY_MS)).await;

        // 初值 false：若启动时卡已挂载，首轮即视为一次边沿 → 主动扫描一次。
        let mut was_present = false;

        loop {
            let present = mount.is_dir();

            if present && !was_present {
                log::info!(
                    "[usb_watch] 检测到 {} 挂载，扫描新图片",
                    usb_import::TARGET_VOLUME_NAME
                );
                match usb_import::scan_target_card() {
                    Some(scan) if !scan.new_files.is_empty() => {
                        log::info!(
                            "[usb_watch] {} 发现 {} 个新图片，emit {}",
                            scan.device_name,
                            scan.new_files.len(),
                            CARD_DETECTED_EVENT
                        );
                        if let Err(e) = app.emit(CARD_DETECTED_EVENT, &scan) {
                            log::warn!("[usb_watch] emit {CARD_DETECTED_EVENT} 失败: {e}");
                        }
                    }
                    Some(_) => log::info!(
                        "[usb_watch] {} 已挂载但无新图片，跳过",
                        usb_import::TARGET_VOLUME_NAME
                    ),
                    None => {
                        // is_dir() 与 scan_target_card 之间的 TOCTOU：卷可能刚好被卸载，忽略。
                    }
                }
            }

            was_present = present;
            tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    });
}
