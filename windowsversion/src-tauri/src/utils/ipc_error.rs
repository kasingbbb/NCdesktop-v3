//! IpcError — Tauri invoke 边界结构化错误协议（ADR-001 / T0 §A）
//!
//! 后端命令统一签名 `Result<T, IpcError>`；通过 `From<IpcError> for String`
//! 把 `IpcError` 序列化为单行 JSON 抛给 Tauri error 通道。
//! 前端 `JSON.parse(str)` 还原；解析失败降级为 `E_INTERNAL`。
//!
//! **闭集 11 项**（contracts.md §A.4）：严禁新增/删除变体。

use serde::Serialize;
use serde_json::Value;

/// 11 项错误码闭集；序列化为大写 snake case 字面量（如 `"E_NAME_INVALID"`）。
///
/// 采用 `rename_all = "SCREAMING_SNAKE_CASE"` 统一推导（T0 §A.3 MINOR 已注：
/// 与逐项 `rename` 同时出现属冗余，本实现二选一保留 `rename_all`）。
/// PascalCase `ENameInvalid` → `E_NAME_INVALID`，由测试 `all_eleven_codes_serialize_to_screaming_snake` 字符级断言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpcErrorCode {
    ENameInvalid,
    ENameDup,
    ENameReserved,
    EPathEscape,
    EProtectedKind,
    ENotFound,
    ECrossDevice,
    EPlatformUnsupported,
    ETrashFailed,
    EFolderDirty,
    EInternal,
}

#[derive(Debug, Clone, Serialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl IpcError {
    pub fn new(code: IpcErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), details: None }
    }

    pub fn with_details(code: IpcErrorCode, message: impl Into<String>, details: Value) -> Self {
        Self { code, message: message.into(), details: Some(details) }
    }

    // ── 工厂便捷构造（每个 code 一个，方便 T3 命令调用）────────

    /// T0 §A.4 #1：`reason` 闭集 = "slash" | "dot_prefix" | "whitespace" | "too_long" | "empty"
    pub fn name_invalid(name: &str, reason: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ENameInvalid,
            format!("名称非法: {name} ({reason})"),
            serde_json::json!({ "name": name, "reason": reason }),
        )
    }
    /// T0 §A.4 #2：`parentRelativePath = ""` 表示根级；**不得**用 `"__ROOT__"`
    pub fn name_dup(name: &str, parent_relative_path: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ENameDup,
            format!("同级同名: {name} @ {parent_relative_path}"),
            serde_json::json!({ "name": name, "parentRelativePath": parent_relative_path }),
        )
    }
    /// T0 §A.4 #3：`reserved` 闭集仅 `"organized"`
    pub fn name_reserved(name: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ENameReserved,
            format!("保留名: {name}"),
            serde_json::json!({ "name": name, "reserved": "organized" }),
        )
    }
    /// T0 §A.4 #4：`requestedPath`（仅上报，前端不展示）
    pub fn path_escape(requested_path: &str) -> Self {
        Self::with_details(
            IpcErrorCode::EPathEscape,
            format!("路径越界: {requested_path}"),
            serde_json::json!({ "requestedPath": requested_path }),
        )
    }
    /// T0 §A.4 #5：`kind` ∈ {"ai_organized","root_import"}；`action` ∈ {"create","rename","delete","move_in","move_out"}
    pub fn protected_kind(kind: &str, action: &str) -> Self {
        Self::with_details(
            IpcErrorCode::EProtectedKind,
            format!("受保护 kind={kind} action={action}"),
            serde_json::json!({ "kind": kind, "action": action }),
        )
    }
    /// T0 §A.4 #6：`target` ∈ {"folder","asset"}；`identifier` = relativePath / assetId
    pub fn not_found(target: &str, identifier: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ENotFound,
            format!("{target} 不存在: {identifier}"),
            serde_json::json!({ "target": target, "identifier": identifier }),
        )
    }
    /// T0 §A.4 #7：全部可选；正常 EXDEV happy path 不抛此码（仅失败时）
    pub fn cross_device(src: &str, dst: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ECrossDevice,
            format!("跨卷迁移失败: {src} -> {dst}"),
            serde_json::json!({ "src": src, "dst": dst }),
        )
    }
    /// T0 §A.4 #8：`feature` 当前闭集仅 `"trash"`；`platform` ∈ {"windows","linux","unknown"}
    pub fn platform_unsupported(feature: &str, platform: &str) -> Self {
        Self::with_details(
            IpcErrorCode::EPlatformUnsupported,
            format!("平台不支持 {feature} on {platform}"),
            serde_json::json!({ "feature": feature, "platform": platform }),
        )
    }
    /// T0 §A.4 #9：`reason` ∈ {"still_exists","crate_error"}
    pub fn trash_failed(path: &str, reason: &str) -> Self {
        Self::with_details(
            IpcErrorCode::ETrashFailed,
            format!("移到废纸篓失败 path={path} reason={reason}"),
            serde_json::json!({ "path": path, "reason": reason }),
        )
    }
    /// T0 §A.4 #10：前端文案模板使用 `now` 渲染（必填）；`old = expected_count` 入参（仅日志）
    pub fn folder_dirty(old: u32, now: u32) -> Self {
        Self::with_details(
            IpcErrorCode::EFolderDirty,
            format!("文件夹内容已变化 old={old} now={now}"),
            serde_json::json!({ "old": old, "now": now }),
        )
    }
    /// T0 §A.4 #11：`where` 可选（仅上报）
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::EInternal, message)
    }
}

/// Tauri v2 invoke 边界只允许 `Err(String)`；统一序列化为 JSON 单行字符串。
/// 兜底：极小概率 serde 失败时退化为静态 E_INTERNAL JSON 字面量。
impl From<IpcError> for String {
    fn from(e: IpcError) -> String {
        match serde_json::to_string(&e) {
            Ok(s) => s,
            Err(se) => format!(
                r#"{{"code":"E_INTERNAL","message":"serde error: {}"}}"#,
                se.to_string().replace('"', "'")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 11 项 code 序列化后必须**字符级一致**于 contracts.md §A.2 的 TS 字面量集合。
    #[test]
    fn all_eleven_codes_serialize_to_screaming_snake() {
        let expected: &[(IpcErrorCode, &str)] = &[
            (IpcErrorCode::ENameInvalid, "E_NAME_INVALID"),
            (IpcErrorCode::ENameDup, "E_NAME_DUP"),
            (IpcErrorCode::ENameReserved, "E_NAME_RESERVED"),
            (IpcErrorCode::EPathEscape, "E_PATH_ESCAPE"),
            (IpcErrorCode::EProtectedKind, "E_PROTECTED_KIND"),
            (IpcErrorCode::ENotFound, "E_NOT_FOUND"),
            (IpcErrorCode::ECrossDevice, "E_CROSS_DEVICE"),
            (IpcErrorCode::EPlatformUnsupported, "E_PLATFORM_UNSUPPORTED"),
            (IpcErrorCode::ETrashFailed, "E_TRASH_FAILED"),
            (IpcErrorCode::EFolderDirty, "E_FOLDER_DIRTY"),
            (IpcErrorCode::EInternal, "E_INTERNAL"),
        ];
        for (code, lit) in expected {
            let s = serde_json::to_string(code).unwrap();
            assert_eq!(s, format!("\"{}\"", lit), "code {:?} 序列化字面量必须为 {}", code, lit);
        }
    }

    #[test]
    fn into_string_serializes_to_json() {
        let e = IpcError::name_invalid("a/b", "has_slash");
        let s: String = e.into();
        assert!(s.starts_with("{"));
        let v: Value = serde_json::from_str(&s).expect("must be valid JSON");
        assert_eq!(v["code"], "E_NAME_INVALID");
        assert_eq!(v["details"]["name"], "a/b");
        assert_eq!(v["details"]["reason"], "has_slash");
    }

    #[test]
    fn folder_dirty_carries_old_and_now() {
        let e = IpcError::folder_dirty(3, 5);
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], "E_FOLDER_DIRTY");
        assert_eq!(v["details"]["old"], 3);
        assert_eq!(v["details"]["now"], 5);
    }

    #[test]
    fn details_omitted_when_none() {
        let e = IpcError::new(IpcErrorCode::EInternal, "x");
        let v = serde_json::to_value(&e).unwrap();
        assert!(v.get("details").is_none(), "details:None 应当 skip 序列化");
    }
}
