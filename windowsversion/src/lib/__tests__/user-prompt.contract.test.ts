/**
 * task_005_dev_frontend_contract — 前端契约层测试
 *
 * 覆盖：
 * - AC-1：`PROMPT_MODULES` 顺序/长度；`PROMPT_MODULE_TITLES` 4 个 module 均有非空中文标题
 * - AC-2：4 个 tauri-commands 封装函数存在 + 入参签名与后端 commands 名称对齐
 * - AC-3：`types/index.ts` re-export 路径可达
 *
 * 测试策略：使用 `vi.mock("@tauri-apps/api/core", { invoke })` 拦截 invoke，
 * 断言每个封装函数透传正确的 command 名 + 参数包；不发起真实 IPC（AC-5）。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  getUserPrompt,
  listUserPrompts,
  resetUserPrompt,
  saveUserPrompt,
} from "../tauri-commands";
import {
  PROMPT_MODULES,
  PROMPT_MODULE_TITLES,
  type PromptInfo,
  type PromptModule,
} from "../../types/user-prompt";
// 同时从 types 桶导入一次，验证 AC-3 re-export 链路通畅
import {
  PROMPT_MODULES as PROMPT_MODULES_FROM_INDEX,
  PROMPT_MODULE_TITLES as PROMPT_MODULE_TITLES_FROM_INDEX,
} from "../../types";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("PromptModule 字面量与常量（AC-1）", () => {
  it("PROMPT_MODULES 顺序固定 tagging → para → concept → aggregation，恒 4 条", () => {
    expect(PROMPT_MODULES).toEqual(["tagging", "para", "concept", "aggregation"]);
    expect(PROMPT_MODULES).toHaveLength(4);
  });

  it("PROMPT_MODULE_TITLES 4 个 module 均有非空中文标题，文案严格按 PRD § 3.2", () => {
    expect(PROMPT_MODULE_TITLES.tagging).toBe("文件打标签");
    expect(PROMPT_MODULE_TITLES.para).toBe("PARA 分组");
    expect(PROMPT_MODULE_TITLES.concept).toBe("知识概念提取");
    expect(PROMPT_MODULE_TITLES.aggregation).toBe("知识聚合");
    // 反向：每个 PROMPT_MODULES 元素在 title map 中均有非空字符串
    for (const m of PROMPT_MODULES) {
      expect(PROMPT_MODULE_TITLES[m]).toBeTypeOf("string");
      expect(PROMPT_MODULE_TITLES[m].length).toBeGreaterThan(0);
    }
  });
});

describe("types/index.ts re-export（AC-3）", () => {
  it("PROMPT_MODULES 通过 types 桶导入与直接导入引用一致", () => {
    expect(PROMPT_MODULES_FROM_INDEX).toBe(PROMPT_MODULES);
    expect(PROMPT_MODULE_TITLES_FROM_INDEX).toBe(PROMPT_MODULE_TITLES);
  });
});

describe("tauri-commands 封装函数（AC-2）", () => {
  it("listUserPrompts 调用 'list_user_prompts'，不带参数", async () => {
    const fixture: PromptInfo[] = [];
    mockInvoke.mockResolvedValueOnce(fixture);

    const result = await listUserPrompts();

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("list_user_prompts");
    expect(result).toBe(fixture);
  });

  it.each<PromptModule>(["tagging", "para", "concept", "aggregation"])(
    "getUserPrompt(%s) 调用 'get_user_prompt' 并传 { module }",
    async (module) => {
      const fixture: PromptInfo = {
        module,
        displayTitle: PROMPT_MODULE_TITLES[module],
        defaultText: "[default]",
        userText: null,
        isCustom: false,
        builtinVersion: "1.0",
        updatedAt: null,
        requiredPlaceholders: [],
        maxBytes: 16384,
      };
      mockInvoke.mockResolvedValueOnce(fixture);

      const result = await getUserPrompt(module);

      expect(mockInvoke).toHaveBeenCalledWith("get_user_prompt", { module });
      expect(result).toBe(fixture);
    },
  );

  it("saveUserPrompt 调用 'save_user_prompt' 并传 { module, text }", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await saveUserPrompt("concept", "你是知识抽取专家，请基于 {content} 输出 JSON");

    expect(mockInvoke).toHaveBeenCalledWith("save_user_prompt", {
      module: "concept",
      text: "你是知识抽取专家，请基于 {content} 输出 JSON",
    });
  });

  it("resetUserPrompt(null) 调用 'reset_user_prompt' 并传 { module: null }（全部恢复默认）", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await resetUserPrompt(null);

    expect(mockInvoke).toHaveBeenCalledWith("reset_user_prompt", { module: null });
  });

  it("resetUserPrompt('tagging') 调用 'reset_user_prompt' 并传 { module: 'tagging' }（单条恢复）", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await resetUserPrompt("tagging");

    expect(mockInvoke).toHaveBeenCalledWith("reset_user_prompt", { module: "tagging" });
  });

  it("后端以 string 形式 reject 时，封装函数透传 string 异常（与 Result<T, String> 对齐）", async () => {
    mockInvoke.mockRejectedValueOnce("自定义 Prompt 过长（20480 字节，上限 16384 字节），请精简");

    await expect(saveUserPrompt("para", "x".repeat(20480))).rejects.toBe(
      "自定义 Prompt 过长（20480 字节，上限 16384 字节），请精简",
    );
  });
});

describe("PromptInfo 类型契约（AC-1 字段命名 camelCase）", () => {
  it("9 个字段均为预期类型（编译时检查的运行时镜像）", () => {
    const info: PromptInfo = {
      module: "tagging",
      displayTitle: "文件打标签",
      defaultText: "默认文本",
      userText: null,
      isCustom: false,
      builtinVersion: "1.0",
      updatedAt: null,
      requiredPlaceholders: [],
      maxBytes: 16384,
    };
    // 字段全 camelCase（与后端 serde rename_all = "camelCase" 严格对齐）
    expect(Object.keys(info).sort()).toEqual(
      [
        "builtinVersion",
        "defaultText",
        "displayTitle",
        "isCustom",
        "maxBytes",
        "module",
        "requiredPlaceholders",
        "updatedAt",
        "userText",
      ].sort(),
    );
  });

  it("userText / updatedAt 允许 null（未自定义场景）", () => {
    const info: PromptInfo = {
      module: "para",
      displayTitle: "PARA 分组",
      defaultText: "default",
      userText: null,
      isCustom: false,
      builtinVersion: "1.0",
      updatedAt: null,
      requiredPlaceholders: [],
      maxBytes: 16384,
    };
    expect(info.userText).toBeNull();
    expect(info.updatedAt).toBeNull();
  });
});
