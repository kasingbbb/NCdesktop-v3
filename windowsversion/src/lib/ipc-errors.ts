/**
 * IPC 错误解包 + 中文文案表（task_004_T2_frontend_ipc）
 *
 * ⚠️ 红线（contracts.md §D / §A）：
 * 1. 本文件的 `errorMessages` 是用户可见文案的 **唯一来源**；后端 `IpcError.message`
 *    字段仅用于日志/上报，**禁止**直接展示给用户。
 * 2. 11 项 code 闭集与 `IpcErrorCode` 联合类型字面量字符级一致；新增 code 必须
 *    先回 T0 修订 contracts.md。
 * 3. `details` 字段一律走 **camelCase**（contracts.md §A.4）；前端文案渲染依赖
 *    必填字段，缺失视为契约破坏，降级返回通用文案并 `console.warn`。
 * 4. 渲染规则：中文、动名词短句、不加感叹号、不加 emoji；建议长度 ≤ 32 字。
 */
import { invoke } from "@tauri-apps/api/core";
import type { IpcError, IpcErrorCode } from "../types/workspace";

/** 11 项 code 闭集（运行时校验依据；与 contracts.md §A.2 字符级一致） */
const IPC_ERROR_CODES: readonly IpcErrorCode[] = [
  "E_NAME_INVALID",
  "E_NAME_DUP",
  "E_NAME_RESERVED",
  "E_PATH_ESCAPE",
  "E_PROTECTED_KIND",
  "E_NOT_FOUND",
  "E_CROSS_DEVICE",
  "E_PLATFORM_UNSUPPORTED",
  "E_TRASH_FAILED",
  "E_FOLDER_DIRTY",
  "E_INTERNAL",
] as const;

/** 11 项 code 运行时集合，用于 `isIpcError` 守卫；与联合类型双向一致。 */
export const IPC_ERROR_CODE_SET: ReadonlySet<string> = new Set(IPC_ERROR_CODES);

/** 类型守卫：判断 unknown 是否为合法 IpcError 对象。 */
export function isIpcError(e: unknown): e is IpcError {
  if (!e || typeof e !== "object") return false;
  const obj = e as Record<string, unknown>;
  if (typeof obj.code !== "string" || !IPC_ERROR_CODE_SET.has(obj.code)) return false;
  if (typeof obj.message !== "string") return false;
  if (obj.details !== undefined && (typeof obj.details !== "object" || obj.details === null)) {
    return false;
  }
  return true;
}

/**
 * 解析 Tauri invoke 抛出的原始错误，还原为 `IpcError`。
 * 优先级（contracts.md §A.1）：
 *   1. 已是合法 `IpcError` 对象 → 原样返回；
 *   2. `string` → `JSON.parse` 后再过 `isIpcError`；
 *   3. 解析失败 / 非字符串 / 校验失败 → 兜底 `E_INTERNAL`，`message` 为原始字符串化结果。
 */
export function parseIpcError(raw: unknown): IpcError {
  if (isIpcError(raw)) return raw;

  if (typeof raw === "string") {
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (isIpcError(parsed)) return parsed;
    } catch {
      /* fall through to fallback */
    }
    return { code: "E_INTERNAL", message: raw, details: undefined };
  }

  let message: string;
  try {
    message = String(raw);
  } catch {
    message = "unknown error";
  }
  return { code: "E_INTERNAL", message, details: undefined };
}

/**
 * 统一 invoke 包装：成功透传 `T`；失败 **始终** throw `IpcError`。
 * 调用方用 `isIpcError(e)` 判别，并用 `errorMessages[e.code](e.details)` 渲染中文。
 */
export async function invokeWithIpcError<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw parseIpcError(e);
  }
}

// ─── 文案渲染：reason / action / feature 中文映射 ──────────────────────────

/** E_NAME_INVALID reason 映射（contracts.md §A.4 #1 / §D #1） */
const NAME_INVALID_REASON_TEXT: Record<string, string> = {
  slash: "不能包含 / \\ :",
  dot_prefix: "不能以 . 开头",
  whitespace: "不能含空白",
  too_long: "超过 255 字节",
  empty: "不能为空",
};

/** E_PROTECTED_KIND action 映射（contracts.md §A.4 #5 / §D #5） */
const PROTECTED_ACTION_TEXT: Record<string, string> = {
  create: "新建子文件夹",
  rename: "重命名",
  delete: "删除",
  move_in: "移入",
  move_out: "移出",
};

/** E_PROTECTED_KIND kind 映射（contracts.md §A.4 #5） */
const PROTECTED_KIND_TEXT: Record<string, string> = {
  ai_organized: "AI 归类目录",
  root_import: "导入副本",
};

/** E_PLATFORM_UNSUPPORTED feature 映射（contracts.md §A.4 #8 / §D #8） */
const PLATFORM_FEATURE_TEXT: Record<string, string> = {
  trash: "移到回收站",
};

/** details 缺字段降级时统一上报；前端绝不二次抛错。 */
function warnDetailsMissing(code: IpcErrorCode, field: string): void {
  // eslint-disable-next-line no-console
  console.warn(`ipc_error_details_missing: code=${code}, field=${field}`);
}

/**
 * 错误码 → 中文文案表（contracts.md §D 逐字搬运）。
 *
 * 渲染合约：
 * - `E_FOLDER_DIRTY` 必须用 `details.now` 渲染（用户重弹 modal 时也用 `now` 作为新 expectedCount）；
 * - 缺 `必填` 字段 → 降级返回该 code 的通用文案 + `console.warn`，**不**二次抛错；
 * - `E_NOT_FOUND` 在根目录场景（`identifier === ""`）显示「根目录」；
 * - 不展示后端 `message` / `details.requestedPath` / `details.path` 等敏感字段。
 */
export const errorMessages: Record<
  IpcErrorCode,
  (details?: Record<string, unknown>) => string
> = {
  // #1 E_NAME_INVALID — 依赖 name + reason
  E_NAME_INVALID: (d) => {
    const name = typeof d?.name === "string" ? (d.name as string) : "";
    const reason = typeof d?.reason === "string" ? (d.reason as string) : "";
    const reasonText = NAME_INVALID_REASON_TEXT[reason];
    if (!name || !reasonText) {
      warnDetailsMissing("E_NAME_INVALID", !name ? "name" : "reason");
      return "名称不合法";
    }
    return `名称「${name}」不合法（${reasonText}）`;
  },

  // #2 E_NAME_DUP — 依赖 name
  E_NAME_DUP: (d) => {
    const name = typeof d?.name === "string" ? (d.name as string) : "";
    if (!name) {
      warnDetailsMissing("E_NAME_DUP", "name");
      return "同级已存在同名文件夹";
    }
    return `同级已存在同名文件夹「${name}」`;
  },

  // #3 E_NAME_RESERVED — 依赖 name
  E_NAME_RESERVED: (d) => {
    const name = typeof d?.name === "string" ? (d.name as string) : "";
    if (!name) {
      warnDetailsMissing("E_NAME_RESERVED", "name");
      return "该名称是保留名称，请换一个";
    }
    return `「${name}」是保留名称，请换一个`;
  },

  // #4 E_PATH_ESCAPE — requestedPath 仅上报，不展示
  E_PATH_ESCAPE: () => "路径越界，已拒绝",

  // #5 E_PROTECTED_KIND — 依赖 kind + action
  E_PROTECTED_KIND: (d) => {
    const kind = typeof d?.kind === "string" ? (d.kind as string) : "";
    const action = typeof d?.action === "string" ? (d.action as string) : "";
    const kindText = PROTECTED_KIND_TEXT[kind];
    const actionText = PROTECTED_ACTION_TEXT[action];
    if (!kindText || !actionText) {
      warnDetailsMissing("E_PROTECTED_KIND", !kindText ? "kind" : "action");
      return "该目录受保护，不支持此操作";
    }
    return `${kindText}不支持${actionText}`;
  },

  // #6 E_NOT_FOUND — 依赖 target；identifier 空串代表根目录
  E_NOT_FOUND: (d) => {
    const target = typeof d?.target === "string" ? (d.target as string) : "";
    if (target === "asset") return "素材不存在或已被删除";
    if (target === "folder") {
      // 根目录场景特殊处理（§A.4 #2 parentRelativePath 空串即根口径）
      const identifier =
        typeof d?.identifier === "string" ? (d.identifier as string) : undefined;
      if (identifier === "") return "根目录不存在或已被删除";
      return "文件夹不存在或已被删除";
    }
    warnDetailsMissing("E_NOT_FOUND", "target");
    return "目标不存在或已被删除";
  },

  // #7 E_CROSS_DEVICE — details 全部可选
  E_CROSS_DEVICE: () => "跨卷迁移失败，请稍后重试",

  // #8 E_PLATFORM_UNSUPPORTED — 依赖 feature
  E_PLATFORM_UNSUPPORTED: (d) => {
    const feature = typeof d?.feature === "string" ? (d.feature as string) : "";
    const featureText = PLATFORM_FEATURE_TEXT[feature];
    if (!featureText) {
      warnDetailsMissing("E_PLATFORM_UNSUPPORTED", "feature");
      return "当前系统暂不支持该操作";
    }
    return `当前系统暂不支持${featureText}`;
  },

  // #9 E_TRASH_FAILED — path/reason 仅上报
  E_TRASH_FAILED: () => "移到回收站失败，请稍后重试",

  // #10 E_FOLDER_DIRTY — 必用 details.now 渲染
  E_FOLDER_DIRTY: (d) => {
    if (typeof d?.now !== "number") {
      warnDetailsMissing("E_FOLDER_DIRTY", "now");
      return "文件夹内容已变化，请重新确认";
    }
    return `内容已变化：当前包含 ${d.now} 个素材，请重新确认`;
  },

  // #11 E_INTERNAL — where 仅上报
  E_INTERNAL: () => "操作失败，请稍后重试或重启应用",
};

/** 便捷渲染：从任意 IpcError 取中文文案。 */
export function renderIpcError(err: IpcError): string {
  const render = errorMessages[err.code] ?? errorMessages.E_INTERNAL;
  return render(err.details);
}
