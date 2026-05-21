import { describe, it, expect } from "vitest";
import { formatDropzoneImportDetail } from "./dropzone-import-detail";

describe("formatDropzoneImportDetail", () => {
  it("已完成", () => {
    expect(formatDropzoneImportDetail(true, null)).toBe("已入库 · AI 已完成");
  });

  it("后台分析中", () => {
    expect(formatDropzoneImportDetail(false, null, true)).toBe(
      "已入库 · AI 后台分析中…"
    );
  });

  it("未配置 Key（旧版错误文案）", () => {
    expect(
      formatDropzoneImportDetail(
        false,
        "ARK_API_KEY / OPENAI_API_KEY 环境变量未设置"
      )
    ).toBe("已入库 · AI 未配置（请设置 ARK_API_KEY 或 OPENAI_API_KEY）");
  });

  it("其它错误截断", () => {
    const long = "x".repeat(60);
    const out = formatDropzoneImportDetail(false, long);
    expect(out.startsWith("已入库 · AI：")).toBe(true);
    expect(out.length).toBeLessThan(55);
  });
});
