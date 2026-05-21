/**
 * task_004_T2_frontend_ipc — folder-name 同步校验单测（AC-4）
 *
 * 五种 reason + ok 路径全部覆盖；与后端 `validate_folder_name` 闭集对齐。
 */
import { describe, expect, it } from "vitest";
import { validateFolderNameSync } from "../folder-name-validate";

describe("validateFolderNameSync", () => {
  it("合法名称 → ok:true", () => {
    expect(validateFolderNameSync("参考资料")).toEqual({ ok: true });
    expect(validateFolderNameSync("draft_v2")).toEqual({ ok: true });
    expect(validateFolderNameSync("A B C")).toEqual({ ok: true });
  });

  it("blank：空串 / 纯空白 → reason=blank", () => {
    expect(validateFolderNameSync("")).toEqual({ ok: false, reason: "blank" });
    expect(validateFolderNameSync("   ")).toEqual({ ok: false, reason: "blank" });
    expect(validateFolderNameSync("\t\n")).toEqual({ ok: false, reason: "blank" });
  });

  it("has_slash：含 / \\ : → reason=has_slash", () => {
    expect(validateFolderNameSync("a/b")).toEqual({ ok: false, reason: "has_slash" });
    expect(validateFolderNameSync("a\\b")).toEqual({ ok: false, reason: "has_slash" });
    expect(validateFolderNameSync("a:b")).toEqual({ ok: false, reason: "has_slash" });
  });

  it("leading_dot：以 . 开头 → reason=leading_dot", () => {
    expect(validateFolderNameSync(".hidden")).toEqual({
      ok: false,
      reason: "leading_dot",
    });
    expect(validateFolderNameSync("..")).toEqual({ ok: false, reason: "leading_dot" });
  });

  it("too_long：UTF-8 字节 > 255 → reason=too_long", () => {
    const longAscii = "a".repeat(256);
    expect(validateFolderNameSync(longAscii)).toEqual({
      ok: false,
      reason: "too_long",
    });
    // 中文 3 字节 × 86 = 258 字节
    const longCjk = "中".repeat(86);
    expect(validateFolderNameSync(longCjk)).toEqual({ ok: false, reason: "too_long" });
    // 边界：255 字节正常
    expect(validateFolderNameSync("a".repeat(255))).toEqual({ ok: true });
  });

  it("reserved：命中保留字 → reason=reserved", () => {
    expect(validateFolderNameSync("organized")).toEqual({
      ok: false,
      reason: "reserved",
    });
  });

  it("规则优先级：blank > has_slash > leading_dot > too_long > reserved", () => {
    // 同时触发 has_slash 与 leading_dot，应先报 has_slash
    expect(validateFolderNameSync("./bad")).toEqual({
      ok: false,
      reason: "has_slash",
    });
  });
});
