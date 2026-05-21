/**
 * task_004_T2_frontend_ipc — IPC 错误解包 + 中文文案表单测
 *
 * 覆盖 input.md AC-2 / AC-5 / AC-6：
 * - 11 项 code 通过 `isIpcError` 守卫
 * - `parseIpcError`：(a) 已是 IpcError 对象直返；(b) 合法 JSON string 还原；
 *   (c) 非法 JSON 降级 E_INTERNAL；(d) 非 string 非 IpcError 降级 E_INTERNAL
 * - `errorMessages` 11 项全产出中文 + 各 code 渲染规则（reason/action/feature/target 映射）
 * - `E_FOLDER_DIRTY({ old:3, now:5 })` 含 "5"
 * - `E_NAME_INVALID({ name:"a/b", reason:"slash" })` 含 "a/b"
 * - 缺 `details` 必填字段 → 降级通用文案 + console.warn 一次
 * - `invokeWithIpcError`：成功透传 / 抛 JSON → IpcError / 抛非 JSON → E_INTERNAL
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import type { IpcErrorCode } from "../../types/workspace";
import {
  IPC_ERROR_CODE_SET,
  errorMessages,
  invokeWithIpcError,
  isIpcError,
  parseIpcError,
  renderIpcError,
} from "../ipc-errors";

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

afterEach(() => {
  invokeMock.mockReset();
  vi.restoreAllMocks();
});

const ALL_CODES: IpcErrorCode[] = [
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
];

describe("IPC_ERROR_CODE_SET 与联合类型双向一致", () => {
  it("11 项 code 都在运行时集合中", () => {
    expect(IPC_ERROR_CODE_SET.size).toBe(11);
    for (const code of ALL_CODES) {
      expect(IPC_ERROR_CODE_SET.has(code)).toBe(true);
    }
  });
});

describe("isIpcError 类型守卫", () => {
  it("11 项 code 构造的对象都识别为 IpcError", () => {
    for (const code of ALL_CODES) {
      expect(isIpcError({ code, message: "x" })).toBe(true);
      expect(
        isIpcError({ code, message: "x", details: { name: "a" } }),
      ).toBe(true);
    }
  });
  it("非法 code / 缺字段 / 非对象 → false", () => {
    expect(isIpcError({ code: "E_UNKNOWN", message: "x" })).toBe(false);
    expect(isIpcError({ code: "E_NAME_DUP" })).toBe(false);
    expect(isIpcError({ message: "x" })).toBe(false);
    expect(isIpcError({ code: "E_NAME_DUP", message: 123 })).toBe(false);
    expect(isIpcError({ code: "E_NAME_DUP", message: "x", details: null })).toBe(false);
    expect(isIpcError({ code: "E_NAME_DUP", message: "x", details: "bad" })).toBe(false);
    expect(isIpcError("E_NAME_DUP")).toBe(false);
    expect(isIpcError(null)).toBe(false);
    expect(isIpcError(undefined)).toBe(false);
    expect(isIpcError(42)).toBe(false);
  });
});

describe("parseIpcError", () => {
  it("(a) 已是合法 IpcError 对象 → 原样返回（同一引用）", () => {
    const obj = { code: "E_INTERNAL" as IpcErrorCode, message: "x" };
    expect(parseIpcError(obj)).toBe(obj);
  });

  it("(b) 合法 JSON 字符串 → IpcError", () => {
    const raw = JSON.stringify({
      code: "E_NAME_DUP",
      message: "duplicate",
      details: { name: "参考", parentRelativePath: "" },
    });
    const err = parseIpcError(raw);
    expect(err.code).toBe("E_NAME_DUP");
    expect(err.message).toBe("duplicate");
    expect(err.details).toEqual({ name: "参考", parentRelativePath: "" });
  });

  it("(c) 非 JSON 字符串 → 兜底 E_INTERNAL（message=原始字符串）", () => {
    const err = parseIpcError("oops not json");
    expect(err.code).toBe("E_INTERNAL");
    expect(err.message).toBe("oops not json");
    expect(err.details).toBeUndefined();
  });

  it("(c') JSON 字符串但 code 不在闭集 → 兜底 E_INTERNAL", () => {
    const raw = JSON.stringify({ code: "E_FAKE", message: "x" });
    const err = parseIpcError(raw);
    expect(err.code).toBe("E_INTERNAL");
    expect(err.message).toBe(raw);
  });

  it("(d) 非 string 非 IpcError（数字 / null / 对象） → 兜底 E_INTERNAL", () => {
    expect(parseIpcError(42)).toMatchObject({ code: "E_INTERNAL", message: "42" });
    expect(parseIpcError(null)).toMatchObject({ code: "E_INTERNAL" });
    expect(parseIpcError({ random: "obj" })).toMatchObject({ code: "E_INTERNAL" });
  });
});

describe("errorMessages — 11 项中文文案", () => {
  it("每个 code 在 details 完整时都能产出非空中文字符串", () => {
    const fullDetails: Record<IpcErrorCode, Record<string, unknown>> = {
      E_NAME_INVALID: { name: "x", reason: "slash" },
      E_NAME_DUP: { name: "x", parentRelativePath: "" },
      E_NAME_RESERVED: { name: "organized", reserved: "organized" },
      E_PATH_ESCAPE: { requestedPath: "../../etc" },
      E_PROTECTED_KIND: { kind: "ai_organized", action: "delete" },
      E_NOT_FOUND: { target: "folder", identifier: "参考" },
      E_CROSS_DEVICE: {},
      E_PLATFORM_UNSUPPORTED: { feature: "trash", platform: "windows" },
      E_TRASH_FAILED: { path: "/tmp/x", reason: "still_exists" },
      E_FOLDER_DIRTY: { old: 3, now: 5 },
      E_INTERNAL: {},
    };
    for (const code of ALL_CODES) {
      const text = errorMessages[code](fullDetails[code]);
      expect(text.length).toBeGreaterThan(0);
      expect(text).toMatch(/[一-龥]/); // 至少一个 CJK 字符
      expect(text).not.toContain("undefined");
      expect(text).not.toContain("[object Object]");
    }
  });

  it("E_FOLDER_DIRTY 必用 details.now 渲染（含 '5'）", () => {
    const text = errorMessages.E_FOLDER_DIRTY({ old: 3, now: 5 });
    expect(text).toContain("5");
    expect(text).toContain("当前");
  });

  it("E_NAME_INVALID 拼入 details.name 与 reason 映射", () => {
    const text = errorMessages.E_NAME_INVALID({ name: "a/b", reason: "slash" });
    expect(text).toContain("a/b");
    expect(text).toContain("/");
  });

  it("E_NAME_INVALID 五种 reason 映射均有可读中文", () => {
    const reasons = ["slash", "dot_prefix", "whitespace", "too_long", "empty"];
    for (const r of reasons) {
      const text = errorMessages.E_NAME_INVALID({ name: "x", reason: r });
      expect(text).toContain("x");
      expect(text).toMatch(/[一-龥]/);
    }
  });

  it("E_NAME_DUP 拼入 details.name（含 '参考'）", () => {
    const text = errorMessages.E_NAME_DUP({ name: "参考" });
    expect(text).toContain("参考");
  });

  it("E_PROTECTED_KIND kind + action 映射", () => {
    const text1 = errorMessages.E_PROTECTED_KIND({
      kind: "ai_organized",
      action: "rename",
    });
    expect(text1).toContain("AI 归类目录");
    expect(text1).toContain("重命名");
    const text2 = errorMessages.E_PROTECTED_KIND({
      kind: "root_import",
      action: "move_out",
    });
    expect(text2).toContain("导入副本");
    expect(text2).toContain("移出");
  });

  it("E_NOT_FOUND target=asset / folder 双分支", () => {
    expect(errorMessages.E_NOT_FOUND({ target: "asset", identifier: "abc" })).toContain(
      "素材",
    );
    expect(
      errorMessages.E_NOT_FOUND({ target: "folder", identifier: "参考" }),
    ).toContain("文件夹");
  });

  it("E_NOT_FOUND target=folder 且 identifier='' → 显示「根目录」", () => {
    const text = errorMessages.E_NOT_FOUND({ target: "folder", identifier: "" });
    expect(text).toContain("根目录");
  });

  it("E_PLATFORM_UNSUPPORTED feature=trash → 含「移到回收站」", () => {
    const text = errorMessages.E_PLATFORM_UNSUPPORTED({
      feature: "trash",
      platform: "windows",
    });
    expect(text).toContain("移到回收站");
  });

  it("E_PATH_ESCAPE 不展示 requestedPath（防泄漏）", () => {
    const text = errorMessages.E_PATH_ESCAPE({ requestedPath: "/etc/passwd" });
    expect(text).not.toContain("/etc/passwd");
    expect(text).toContain("路径越界");
  });
});

describe("errorMessages — 缺 details 必填字段降级", () => {
  it("E_FOLDER_DIRTY 缺 now → 通用文案 + console.warn", () => {
    const spy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const text = errorMessages.E_FOLDER_DIRTY({ old: 3 });
    expect(text).toMatch(/[一-龥]/);
    expect(text).not.toContain("undefined");
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0]?.[0]).toContain("ipc_error_details_missing");
  });

  it("E_NAME_INVALID 缺 reason → 通用文案 + console.warn", () => {
    const spy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const text = errorMessages.E_NAME_INVALID({ name: "x" });
    expect(text).not.toContain("undefined");
    expect(spy).toHaveBeenCalled();
  });

  it("E_PROTECTED_KIND 缺 action → 通用文案 + console.warn", () => {
    const spy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const text = errorMessages.E_PROTECTED_KIND({ kind: "ai_organized" });
    expect(text).not.toContain("undefined");
    expect(spy).toHaveBeenCalled();
  });

  it("E_PLATFORM_UNSUPPORTED 缺 feature → 通用文案 + console.warn", () => {
    const spy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const text = errorMessages.E_PLATFORM_UNSUPPORTED({});
    expect(text).not.toContain("undefined");
    expect(spy).toHaveBeenCalled();
  });

  it("E_NOT_FOUND 缺 target → 通用文案 + console.warn", () => {
    const spy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const text = errorMessages.E_NOT_FOUND({});
    expect(text).not.toContain("undefined");
    expect(spy).toHaveBeenCalled();
  });

  it("不依赖 details 的 code 在 details 为 undefined 时仍可渲染", () => {
    expect(errorMessages.E_CROSS_DEVICE()).toMatch(/[一-龥]/);
    expect(errorMessages.E_TRASH_FAILED()).toMatch(/[一-龥]/);
    expect(errorMessages.E_INTERNAL()).toMatch(/[一-龥]/);
    expect(errorMessages.E_PATH_ESCAPE()).toMatch(/[一-龥]/);
  });
});

describe("invokeWithIpcError", () => {
  it("成功 → 数据原样透传", async () => {
    invokeMock.mockResolvedValueOnce({ trashed: 5 });
    const out = await invokeWithIpcError<{ trashed: number }>(
      "delete_workspace_folder",
      { projectId: "p1" },
    );
    expect(out).toEqual({ trashed: 5 });
    expect(invokeMock).toHaveBeenCalledWith("delete_workspace_folder", {
      projectId: "p1",
    });
  });

  it("invoke 抛合法 JSON 字符串 → 重抛 IpcError 对象", async () => {
    const raw = JSON.stringify({
      code: "E_NAME_DUP",
      message: "x",
      details: { name: "参考", parentRelativePath: "" },
    });
    invokeMock.mockRejectedValueOnce(raw);
    await expect(
      invokeWithIpcError("create_workspace_folder", {}),
    ).rejects.toMatchObject({
      code: "E_NAME_DUP",
      details: { name: "参考", parentRelativePath: "" },
    });
  });

  it("invoke 抛非 JSON 字符串 → 兜底 E_INTERNAL", async () => {
    invokeMock.mockRejectedValueOnce("boom");
    await expect(
      invokeWithIpcError("create_workspace_folder", {}),
    ).rejects.toMatchObject({ code: "E_INTERNAL", message: "boom" });
  });
});

describe("renderIpcError", () => {
  it("根据 code 取对应文案，E_FOLDER_DIRTY 用 details.now", () => {
    const text = renderIpcError({
      code: "E_FOLDER_DIRTY",
      message: "ignored",
      details: { old: 2, now: 7 },
    });
    expect(text).toContain("7");
  });
});
