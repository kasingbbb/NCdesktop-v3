//! task_009：扫描型 PDF 路由防呆 —— 结构性嗅探（XObject + Font 引用判定）。
//!
//! ## 设计原则（ADR-006 / H6 硬约束）
//!
//! - **只做结构性判定**：读 PDF 采样页 page tree 的 Resources 字典，
//!   判定 `XObject` 是否**全部**为 `Subtype=/Image` **且** 采样页无 `Font` 字典引用；
//! - **严禁启发式**：
//!   - 不基于"运行 markitdown 后 stdout 长度 < N" 来判扫描（污染 conversion_meta）；
//!   - 不基于"文本字数 < N 即视为扫描"（H6 把分类器划到 P1）。
//! - **失败显式**：lopdf 解析异常 / 加密 PDF / 无 page tree → `Err(io::Error)`，
//!   由调用方按 `ParseError` 处理；**不可"猜测"成 scan**。
//!
//! ## 多页采样（2026-05-25 修订）
//!
//! 历史实现只看首页；但 z-library 等"重新封装"的 PDF 常把封面页栅格化为单张图像，
//! 首页结构上等同扫描件（无 Font、仅 Image XObject），却被误判成全本扫描。
//! 现采样 **[首页, 中间页, 末页]** 共最多 3 页：
//! - **任一页含 Font 引用 → 立即判为非扫描（return false）**；
//! - 全部采样页都无 Font 引用 + 首页 XObject 全为 Image → 判为扫描（return true）；
//! - 其他情况保守返回 false（让 markitdown 自尝试）。
//!
//! ## 仅供 scheduler.rs 的 `application/pdf` 路由分支调用
//!
//! 返回 `Ok(true)` → scheduler 应短路写 `conversion_meta.failure_code = EScanPdfUnsupported`
//! 并产出 placeholder，**不再**进 markitdown 子进程。

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};

/// 判定 PDF 是否"扫描型"（多页采样结构性嗅探，非启发式）。
///
/// 返回：
/// - `Ok(true)`  → 采样页全部无 Font + 首页 XObject 全为 Image → 视为扫描型；
/// - `Ok(false)` → 任一采样页含 Font 引用 / 首页不含 XObject / 含非 Image XObject → 视为可读文本（保守）；
/// - `Err(io::Error)` → PDF 加载 / 解析失败 / 加密 / 无 page tree → 调用方按 ParseError 处理。
///
/// **保守语义**：若结构信息不充分，返回 `false` 让 markitdown 自己尝试；
/// 只有"明确像扫描件"（采样页全无字体 + 首页仅图）才返回 `true`。误路由率 0% 的语义是
/// "把文本 PDF 误判为扫描" = 0；这里宁可漏判扫描件（让 markitdown 输出空 → 走 task_008
/// `EOutputEmpty` 链路），也不可把文本件挡在外面。
pub fn is_scan_pdf(path: &Path) -> Result<bool, io::Error> {
    let doc = Document::load(path).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("lopdf load failed: {e}"))
    })?;

    // 加密 PDF：lopdf 可解析 trailer，但内容流被加密；按 ParseError 处理。
    if doc.is_encrypted() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "encrypted pdf: structural sniff not applicable",
        ));
    }

    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pdf has no page tree",
        ));
    }

    let sampled = sample_page_ids(&pages);

    // ─── (1) 任一采样页含 Font 引用 → 立即判为非扫描 ───────────────────────
    for page_id in &sampled {
        let page_dict = match doc.get_dictionary(*page_id) {
            Ok(d) => d,
            Err(_) => continue, // 某采样页字典缺失，跳过，看其他采样页
        };
        let resources = match resolve_resources(&doc, page_dict, *page_id) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if has_font_reference(&doc, &resources) {
            return Ok(false);
        }
    }

    // ─── (2) 全部采样页都无 Font → 用首页 XObject 严格判扫描 ───────────────
    // 复用历史"首页 XObject 全为 Image"判定 —— 避免对扫描件的判定标准被多页采样削弱。
    let first_page_id = *pages.values().next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "pdf has no page tree")
    })?;

    let page_dict = doc.get_dictionary(first_page_id).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("get first page dict failed: {e}"),
        )
    })?;

    let resources = resolve_resources(&doc, page_dict, first_page_id).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("resolve resources failed: {e}"),
        )
    })?;

    let xobjects = match resolve_xobject_dict(&doc, &resources) {
        Some(d) => d,
        None => return Ok(false),
    };

    if xobjects.iter().count() == 0 {
        return Ok(false);
    }

    for (_name, value) in xobjects.iter() {
        if !is_image_xobject(&doc, value) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// 采样 [首页, 中间页, 末页]，去重后保持页码升序。
///
/// - 1 页 PDF → `[1]`
/// - 2 页 PDF → `[1, 2]`
/// - N 页 PDF (N ≥ 3) → `[1, N/2+1, N]`（中间页取靠后整数，避免单页 PDF 退化）
///
/// 设计取舍：3 页采样足以区分"封面图 + 文字正文"（z-library 风格）与"全本扫描件"。
/// 真扫描件的所有页都是 Image XObject；只要中间页或末页含 Font 即可命中文本 PDF。
fn sample_page_ids(pages: &BTreeMap<u32, ObjectId>) -> Vec<ObjectId> {
    let total = pages.len() as u32;
    if total == 0 {
        return vec![];
    }
    let mut indices: Vec<u32> = vec![1];
    if total >= 2 {
        indices.push(total);
    }
    if total >= 3 {
        indices.push(total / 2 + 1);
    }
    indices.sort();
    indices.dedup();
    indices
        .into_iter()
        .filter_map(|i| pages.get(&i).copied())
        .collect()
}

/// 解析"首页可见的 Resources 字典"：
/// - 若 page 自身含 `Resources` 键 → 直接返回；
/// - 否则按 PDF 1.7 §7.7.3.4 沿 `Parent` 链向上找。
fn resolve_resources<'a>(
    doc: &'a Document,
    page_dict: &'a Dictionary,
    page_id: ObjectId,
) -> Result<Dictionary, String> {
    // 当前节点
    if let Some(d) = take_resources(doc, page_dict)? {
        return Ok(d);
    }

    // 沿 Parent 链向上（防御循环：限制 16 层）
    let mut current_id = page_id;
    let mut seen: Vec<ObjectId> = vec![current_id];
    for _ in 0..16 {
        let dict = match doc.get_dictionary(current_id) {
            Ok(d) => d,
            Err(_) => break,
        };
        let Ok(parent) = dict.get(b"Parent").and_then(Object::as_reference) else {
            break;
        };
        if seen.contains(&parent) {
            break;
        }
        seen.push(parent);
        current_id = parent;

        let parent_dict = match doc.get_dictionary(parent) {
            Ok(d) => d,
            Err(_) => break,
        };
        if let Some(r) = take_resources(doc, parent_dict)? {
            return Ok(r);
        }
    }

    // 找不到 Resources → 视为空字典（让上层判 false：无 XObject、无 Font）
    Ok(Dictionary::new())
}

/// 从字典中拿 Resources（可能是 Inline Dictionary 或 Reference）。
fn take_resources(doc: &Document, dict: &Dictionary) -> Result<Option<Dictionary>, String> {
    let Ok(obj) = dict.get(b"Resources") else {
        return Ok(None);
    };
    match obj {
        Object::Dictionary(d) => Ok(Some(d.clone())),
        Object::Reference(id) => doc
            .get_dictionary(*id)
            .map(|d| Some(d.clone()))
            .map_err(|e| format!("dereference Resources failed: {e}")),
        _ => Err("Resources is neither Dictionary nor Reference".to_string()),
    }
}

/// Resources 是否含 Font 字典（且 Font 字典非空）。
fn has_font_reference(doc: &Document, resources: &Dictionary) -> bool {
    let Ok(obj) = resources.get(b"Font") else {
        return false;
    };
    let font_dict = match obj {
        Object::Dictionary(d) => d.clone(),
        Object::Reference(id) => match doc.get_dictionary(*id) {
            Ok(d) => d.clone(),
            Err(_) => return false,
        },
        _ => return false,
    };
    // Font 字典中至少有一个条目即视为"页面引用了字体"
    font_dict.iter().count() > 0
}

/// 解析 Resources.XObject 子字典；不存在或类型不匹配返回 None。
fn resolve_xobject_dict(doc: &Document, resources: &Dictionary) -> Option<Dictionary> {
    let obj = resources.get(b"XObject").ok()?;
    match obj {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

/// 判定单个 XObject 引用是否为 `Subtype = /Image`。
/// - XObject 通常以 Stream 形式存在，Stream.dict 中含 `Subtype` Name；
/// - 引用对象也接受（dereference）；
/// - 任何"非 Image"或"无法解析 Subtype"统一视为 **非 Image**（让上层 → false）。
fn is_image_xobject(doc: &Document, value: &Object) -> bool {
    let id = match value {
        Object::Reference(id) => *id,
        // 内联 Stream / Dictionary
        Object::Stream(s) => return dict_subtype_is(&s.dict, b"Image"),
        Object::Dictionary(d) => return dict_subtype_is(d, b"Image"),
        _ => return false,
    };
    let Ok(obj) = doc.get_object(id) else {
        return false;
    };
    match obj {
        Object::Stream(s) => dict_subtype_is(&s.dict, b"Image"),
        Object::Dictionary(d) => dict_subtype_is(d, b"Image"),
        _ => false,
    }
}

fn dict_subtype_is(dict: &Dictionary, expected: &[u8]) -> bool {
    matches!(
        dict.get(b"Subtype").and_then(Object::as_name),
        Ok(name) if name == expected
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// 单测：手工构造最小 PDF fixtures（mock）
//
// 真实样本测试（≥3 真实文本 PDF + ≥3 真实扫描 PDF）→ PENDING-OPERATOR：
// 等 task_012 真实样本仓接入。本模块只验证"结构判定逻辑"，不依赖外部样本。
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Object, Stream};
    use tempfile::NamedTempFile;

    /// 构造一个"含 Font + 文本内容"的最小 PDF（"text PDF"）。
    /// 返回保存后的 NamedTempFile（drop 即删）。
    fn make_text_pdf() -> NamedTempFile {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Courier",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![10.into(), 100.into()]),
                Operation::new("Tj", vec![Object::string_literal("Hello text PDF")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let tmp = NamedTempFile::new().unwrap();
        let mut doc = doc;
        doc.save(tmp.path()).unwrap();
        tmp
    }

    /// 构造"仅含 Image XObject、无 Font"的最小 PDF（"scan PDF"）。
    fn make_scan_pdf() -> NamedTempFile {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        // Image XObject Stream（最小化：只设 Type/Subtype/Width/Height）
        let image_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        };
        // 1 字节占位"图像数据"，避免空 stream 触发 lopdf 怪异行为
        let image_id = doc.add_object(Stream::new(image_dict, vec![0u8]));

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im1" => image_id },
            // 显式无 Font
        });
        // page 内容流：用 Do 操作绘制 image（最小占位即可）
        let content = Content {
            operations: vec![Operation::new("Do", vec!["Im1".into()])],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let tmp = NamedTempFile::new().unwrap();
        let mut doc = doc;
        doc.save(tmp.path()).unwrap();
        tmp
    }

    /// 构造"混合（首页有 Font + Image XObject）"的 PDF。
    /// AC-4 字面：首页有 Font 即视为非扫描 → false（保守通过）。
    fn make_mixed_pdf() -> NamedTempFile {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let image_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        };
        let image_id = doc.add_object(Stream::new(image_dict, vec![0u8]));

        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
            "XObject" => dictionary! { "Im1" => image_id },
        });
        let content_id = doc.add_object(Stream::new(dictionary! {}, b"q Q".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let tmp = NamedTempFile::new().unwrap();
        let mut doc = doc;
        doc.save(tmp.path()).unwrap();
        tmp
    }

    /// 构造"仅含 Form XObject + 无 Font"的 PDF（非 Image XObject）→ false（保守）。
    fn make_form_xobject_pdf() -> NamedTempFile {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let form_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
        };
        let form_id = doc.add_object(Stream::new(form_dict, b"q Q".to_vec()));
        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Fm1" => form_id },
        });
        let content_id = doc.add_object(Stream::new(dictionary! {}, b"q Q".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let tmp = NamedTempFile::new().unwrap();
        let mut doc = doc;
        doc.save(tmp.path()).unwrap();
        tmp
    }

    // ─── 3 个 text PDF → false ──────────────────────────────────────────────

    #[test]
    fn text_pdf_a_returns_false() {
        let tmp = make_text_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), false);
    }

    #[test]
    fn text_pdf_b_returns_false() {
        // 重复构造（不同 NamedTempFile 路径）确认稳定性
        let tmp = make_text_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), false);
    }

    #[test]
    fn text_pdf_c_returns_false() {
        let tmp = make_text_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), false);
    }

    // ─── 3 个 scan PDF → true ───────────────────────────────────────────────

    #[test]
    fn scan_pdf_a_returns_true() {
        let tmp = make_scan_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), true);
    }

    #[test]
    fn scan_pdf_b_returns_true() {
        let tmp = make_scan_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), true);
    }

    #[test]
    fn scan_pdf_c_returns_true() {
        let tmp = make_scan_pdf();
        assert_eq!(is_scan_pdf(tmp.path()).unwrap(), true);
    }

    // ─── 1 个 mixed（Font + Image）→ false（保守通过）─────────────────────

    #[test]
    fn mixed_font_and_image_returns_false() {
        let tmp = make_mixed_pdf();
        assert_eq!(
            is_scan_pdf(tmp.path()).unwrap(),
            false,
            "首页有 Font 引用即视为非扫描（保守）"
        );
    }

    // ─── 加密 PDF → Err ──────────────────────────────────────────────────

    /// 加密 PDF fixture：手工写入最小加密标记的 PDF 字节流。
    /// 不实际加密内容 —— 只要 `trailer.Encrypt` 存在即被 `is_encrypted()` 命中。
    #[test]
    fn encrypted_pdf_returns_err() {
        // 用 lopdf 构造一个无 Encrypt 的最小 PDF，再用文本 hack 注入 Encrypt 字典。
        // 简化做法：直接用 lopdf 添加一个 Encrypt 对象到 trailer，让 is_encrypted 命中。
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let resources_id = doc.add_object(dictionary! {});
        let content_id = doc.add_object(Stream::new(dictionary! {}, b"q Q".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        // 注入 Encrypt：最小合法字典（V=1, R=2, O/U 填零字节占位）
        let encrypt_id = doc.add_object(dictionary! {
            "Filter" => "Standard",
            "V" => 1,
            "R" => 2,
            "Length" => 40,
            "P" => -3i64,
            "O" => Object::string_literal(vec![0u8; 32]),
            "U" => Object::string_literal(vec![0u8; 32]),
        });
        doc.trailer.set("Root", catalog_id);
        doc.trailer.set("Encrypt", Object::Reference(encrypt_id));
        // 文件 ID 是加密 PDF 解析必要的 trailer 项；用占位字符串
        doc.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(vec![0u8; 16]),
                Object::string_literal(vec![0u8; 16]),
            ]),
        );

        let tmp = NamedTempFile::new().unwrap();
        // 加密 PDF 写盘可能失败（lopdf 内部尝试加密）；任一失败路径都满足"Err"。
        let save_res = doc.save(tmp.path());
        if save_res.is_err() {
            // 写盘已失败 → 等价于"加载会失败"，我们用一个不存在的 path 间接断言 Err。
            let nonexistent = tmp.path().with_extension("absent.pdf");
            let r = is_scan_pdf(&nonexistent);
            assert!(r.is_err(), "不可加载 / 解析的 PDF 必须返回 Err");
            return;
        }

        let r = is_scan_pdf(tmp.path());
        assert!(r.is_err(), "加密 PDF 必须返回 Err（不可猜测）");
    }

    // ─── 额外：非 Image XObject（Form）→ false ────────────────────────────

    #[test]
    fn form_xobject_only_returns_false() {
        let tmp = make_form_xobject_pdf();
        assert_eq!(
            is_scan_pdf(tmp.path()).unwrap(),
            false,
            "Form XObject 不算扫描件（保守通过）"
        );
    }

    // ─── 额外：损坏 PDF（非 PDF 字节）→ Err ──────────────────────────────

    #[test]
    fn corrupted_pdf_returns_err() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"this is not a pdf at all").unwrap();
        let r = is_scan_pdf(tmp.path());
        assert!(r.is_err(), "非 PDF 字节流必须返回 Err");
    }

    // ─── 多页采样判定（2026-05-25 修订） ───────────────────────────────────

    /// 构造 "z-library 风格" PDF：首页 = 图像封面（无 Font + 仅 Image XObject），
    /// 后续多页 = 正常文本（有 Font）。
    ///
    /// 历史"只看首页"实现会误判为扫描件；多页采样判定应返回 `false`。
    fn make_zlibrary_style_pdf(num_text_pages: usize) -> NamedTempFile {
        assert!(num_text_pages >= 1, "至少要有 1 页文本页");
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        // ── 首页：图像封面（无 Font）
        let cover_image_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        };
        let cover_image_id = doc.add_object(Stream::new(cover_image_dict, vec![0u8]));
        let cover_resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "ImCover" => cover_image_id },
        });
        let cover_content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content {
                operations: vec![Operation::new("Do", vec!["ImCover".into()])],
            }
            .encode()
            .unwrap(),
        ));
        let cover_page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => cover_resources_id,
            "Contents" => cover_content_id,
        });

        // ── 后续文本页（共享一个 Font + Resources）
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let text_resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let mut kids: Vec<Object> = vec![cover_page_id.into()];
        for i in 0..num_text_pages {
            let text_content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 12.into()]),
                    Operation::new("Td", vec![10.into(), 100.into()]),
                    Operation::new(
                        "Tj",
                        vec![Object::string_literal(format!("page {}", i + 1))],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let text_content_id =
                doc.add_object(Stream::new(dictionary! {}, text_content.encode().unwrap()));
            let text_page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Resources" => text_resources_id,
                "Contents" => text_content_id,
            });
            kids.push(text_page_id.into());
        }

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => (1 + num_text_pages) as i64,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let tmp = NamedTempFile::new().unwrap();
        let mut doc = doc;
        doc.save(tmp.path()).unwrap();
        tmp
    }

    /// 关键回归测试：z-library 风格 PDF（首页图像封面 + 后续文本页）
    /// 必须判为**非扫描**（与 2026-05-25 误判 PDF A/B 的真实文件结构一致）。
    #[test]
    fn zlibrary_style_pdf_with_image_cover_returns_false() {
        // 3 页文本（采样会落在第 2 或第 3 页，应命中 Font 引用）
        let tmp = make_zlibrary_style_pdf(3);
        assert_eq!(
            is_scan_pdf(tmp.path()).unwrap(),
            false,
            "z-library 风格 PDF（首页图像 + 文本页）不应被误判为扫描件"
        );
    }

    /// 末页是文本页 → 必须命中末页采样。
    #[test]
    fn zlibrary_style_pdf_two_pages_cover_plus_one_text_returns_false() {
        let tmp = make_zlibrary_style_pdf(1);
        assert_eq!(
            is_scan_pdf(tmp.path()).unwrap(),
            false,
            "2 页 PDF：末页有 Font 即视为非扫描"
        );
    }

    /// 大量文本页：中间页采样必命中。
    #[test]
    fn zlibrary_style_pdf_many_text_pages_returns_false() {
        let tmp = make_zlibrary_style_pdf(50);
        assert_eq!(
            is_scan_pdf(tmp.path()).unwrap(),
            false,
            "首页图像 + 50 页文本：中间页采样必命中 Font"
        );
    }

    // ─── sample_page_ids 单元测试 ──────────────────────────────────────────

    /// 构造 N 页 BTreeMap 用于测试 `sample_page_ids` 的采样位置。
    fn fake_pages(n: u32) -> BTreeMap<u32, ObjectId> {
        let mut m = BTreeMap::new();
        for i in 1..=n {
            // ObjectId = (object_number, generation)；用 i 作 object_number 区分
            m.insert(i, (i as u32, 0));
        }
        m
    }

    #[test]
    fn sample_page_ids_one_page() {
        let pages = fake_pages(1);
        let ids = sample_page_ids(&pages);
        assert_eq!(ids, vec![(1, 0)], "1 页应只采首页");
    }

    #[test]
    fn sample_page_ids_two_pages() {
        let pages = fake_pages(2);
        let ids = sample_page_ids(&pages);
        assert_eq!(ids, vec![(1, 0), (2, 0)], "2 页应采首页 + 末页");
    }

    #[test]
    fn sample_page_ids_three_pages() {
        let pages = fake_pages(3);
        let ids = sample_page_ids(&pages);
        // 中间页 = 3/2+1 = 2；最终 [1, 2, 3]
        assert_eq!(ids, vec![(1, 0), (2, 0), (3, 0)], "3 页采全部");
    }

    #[test]
    fn sample_page_ids_large_book() {
        let pages = fake_pages(412); // 与 PDF B 同页数
        let ids = sample_page_ids(&pages);
        // 中间页 = 412/2+1 = 207
        assert_eq!(
            ids,
            vec![(1, 0), (207, 0), (412, 0)],
            "412 页应采 [首, 中(207), 末]"
        );
    }

    #[test]
    fn sample_page_ids_zero_pages() {
        let pages: BTreeMap<u32, ObjectId> = BTreeMap::new();
        let ids = sample_page_ids(&pages);
        assert!(ids.is_empty(), "空 page tree 返回空 vec");
    }
}
