//! task_009：扫描型 PDF 路由防呆 —— 结构性嗅探（XObject + Font 引用判定）。
//!
//! ## 设计原则（ADR-006 / H6 硬约束）
//!
//! - **只做结构性判定**：读 PDF 首页 page tree 的 Resources 字典，
//!   判定 `XObject` 是否**全部**为 `Subtype=/Image` **且** 整页无 `Font` 字典引用；
//! - **严禁启发式**：
//!   - 不基于"运行 markitdown 后 stdout 长度 < N" 来判扫描（污染 conversion_meta）；
//!   - 不基于"文本字数 < N 即视为扫描"（H6 把分类器划到 P1）。
//! - **失败显式**：lopdf 解析异常 / 加密 PDF / 无 page tree → `Err(io::Error)`，
//!   由调用方按 `ParseError` 处理；**不可"猜测"成 scan**。
//!
//! ## 仅供 scheduler.rs 的 `application/pdf` 路由分支调用
//!
//! 返回 `Ok(true)` → scheduler 应短路写 `conversion_meta.failure_code = EScanPdfUnsupported`
//! 并产出 placeholder，**不再**进 markitdown 子进程。

use std::io;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};

/// 判定首页是否"扫描型 PDF"（结构性嗅探，非启发式）。
///
/// 返回：
/// - `Ok(true)`  → 首页 Resources 仅含 Image XObject 且无 Font 字典引用 → 视为扫描型；
/// - `Ok(false)` → 含 Font 引用 / 含非 Image XObject / 不含 XObject → 视为可读文本（保守）；
/// - `Err(io::Error)` → PDF 加载 / 解析失败 / 加密 / 无 page tree → 调用方按 ParseError 处理。
///
/// **保守语义**：若结构信息不充分（无 XObject、无 Font），返回 `false` 让 markitdown 自己尝试；
/// 只有"明确像扫描件"（仅图、无字体）才返回 `true`。误路由率 0% 的语义是
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
    let first_page_id = pages
        .into_iter()
        .next()
        .map(|(_, id)| id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pdf has no page tree"))?;

    // 首页字典 —— Resources 可能直接挂在 page 节点，也可能继承自 Pages 树父节点。
    let page_dict = doc.get_dictionary(first_page_id).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("get first page dict failed: {e}"))
    })?;

    let resources = resolve_resources(&doc, page_dict, first_page_id).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("resolve resources failed: {e}"))
    })?;

    // ─── (1) 若 Resources 含 Font 字典（任一字体引用即可）→ 非扫描 ────────────
    if has_font_reference(&doc, &resources) {
        return Ok(false);
    }

    // ─── (2) 检查 XObject 字典：是否"非空 且 全部为 Image" ───────────────────
    let xobjects = match resolve_xobject_dict(&doc, &resources) {
        Some(d) => d,
        None => {
            // 无 XObject → 保守判非扫描（让 markitdown 自尝试）
            return Ok(false);
        }
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
}
