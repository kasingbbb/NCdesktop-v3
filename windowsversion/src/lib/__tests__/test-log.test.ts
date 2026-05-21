import { describe, expect, it, vi } from "vitest";
import { testLog } from "../test-log";

describe("testLog", () => {
  it("输出带 TEST 前缀且不抛错", () => {
    const spy = vi.spyOn(console, "info").mockImplementation(() => {});
    testLog("info", "unit", "hello", { n: 1 });
    expect(spy).toHaveBeenCalledTimes(1);
    expect(String(spy.mock.calls[0]?.[0])).toContain("[TEST][unit]");
    spy.mockRestore();
  });
});
