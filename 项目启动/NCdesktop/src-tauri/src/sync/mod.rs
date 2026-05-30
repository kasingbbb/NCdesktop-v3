pub mod detector;
pub mod manifest;
pub mod session_parser;
pub mod file_copier;
pub mod meta_parser;
pub mod timeline_builder;
pub mod state;
pub mod progress;
// 裸图片 U 盘自动导入（Notecapt 实际固件写裸 Picture_*.jpg，非 .arca 结构）。
pub mod usb_import;
pub mod volume_watch;
