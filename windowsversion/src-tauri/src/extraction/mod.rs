pub mod conversion;
pub mod extractors;
pub mod failure_code;
pub mod models;
pub mod runtime_check;
// task_009：扫描型 PDF 路由防呆 —— 结构性嗅探（XObject + Font 引用判定）。
// 仅由 scheduler.rs 的 `application/pdf` 路由分支调用，禁止启发式（H6）。
pub mod scan_pdf_detect;
// task_008 已重新激活：M-1 关闭点。scheduler 依赖的底层符号
// （Asset.source_asset_id / db::extraction / sha2 / conversion_meta /
//  embedded venv 探测 / get_fallback_extractor_for_excluding）
// 由 task_002~007 全部铺好，scheduler.rs 主循环改造为 primary→fallback→placeholder
// 三级编排。
pub mod scheduler;

use std::path::Path;

use models::{ExtractionError, ExtractionResult, ExtractOptions};

/// 提取器 trait — 所有格式的文件内容提取器须实现此接口
pub trait Extractor: Send + Sync {
    /// 判断此提取器是否能处理指定 MIME 类型
    fn can_handle(&self, mime_type: &str) -> bool;

    /// 提取器名称标识
    fn name(&self) -> &'static str;

    /// 执行提取（同步，调用方负责在 spawn_blocking 中运行）
    fn extract(
        &self,
        file_path: &Path,
        options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError>;
}
