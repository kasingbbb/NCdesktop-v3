/**
 * 文件夹名同步校验（task_004_T2_frontend_ipc / AC-4）
 *
 * ⚠️ 红线：后端 `workspace::validate_folder_name` 是 **最终权威**（contracts.md ADR-008）。
 * 本函数仅用于 UI 输入框的即时反馈（红框 / 禁用确认按钮），**不能**替代后端校验：
 * - 后端会做 NFC 同级查重（需 fs read）+ 保留字判定，此处只能覆盖纯字符串规则；
 * - 任何写命令最终以后端 `IpcError` 为准。
 *
 * 校验规则（与后端 `validate_folder_name` 同步保持闭集；reason 字面量与 PRD §4.3 / contracts.md §A.4 #1 对齐）：
 * - `blank`     — 名称去除首尾空白后为空，或全是空白字符
 * - `has_slash` — 含 `/` `\` `:` 三个 macOS/Windows 保留路径分隔符
 * - `leading_dot` — 以 `.` 开头（含隐藏文件 / 系统目录冲突）
 * - `too_long`  — UTF-8 字节长度 > 255
 * - `reserved`  — 命中保留字闭集（当前仅 `organized`）
 */

/** 单次校验结果。 */
export type FolderNameValidation =
  | { ok: true }
  | { ok: false; reason: "has_slash" | "leading_dot" | "blank" | "too_long" | "reserved" };

/** 保留字闭集（与后端一致；新增需先回 T0 修订 contracts.md）。 */
const RESERVED_NAMES: ReadonlySet<string> = new Set(["organized"]);

/** 路径分隔/保留字符（macOS / Windows 公约） */
const FORBIDDEN_CHARS = ["/", "\\", ":"] as const;

/** UTF-8 字节长度上限（与后端一致） */
const MAX_BYTE_LEN = 255;

/**
 * 同步校验文件夹名。命中第一条规则即返；不抛错。
 *
 * 注意：调用方应在 onChange 触发，作为 UI 反馈；提交时仍以后端返回的
 * `IpcError`（E_NAME_INVALID / E_NAME_DUP / E_NAME_RESERVED）为最终判定。
 */
export function validateFolderNameSync(name: string): FolderNameValidation {
  // blank：空字符串或全空白
  if (name.trim().length === 0) {
    return { ok: false, reason: "blank" };
  }
  // has_slash：含 / \ :
  for (const ch of FORBIDDEN_CHARS) {
    if (name.includes(ch)) {
      return { ok: false, reason: "has_slash" };
    }
  }
  // leading_dot：以 . 开头
  if (name.startsWith(".")) {
    return { ok: false, reason: "leading_dot" };
  }
  // too_long：UTF-8 字节超过 255
  // 通过 TextEncoder 拿真实字节数（浏览器 / Node / vitest jsdom 均支持）
  const byteLen = new TextEncoder().encode(name).length;
  if (byteLen > MAX_BYTE_LEN) {
    return { ok: false, reason: "too_long" };
  }
  // reserved：命中保留字（区分大小写匹配；后端同此口径）
  if (RESERVED_NAMES.has(name)) {
    return { ok: false, reason: "reserved" };
  }
  return { ok: true };
}
