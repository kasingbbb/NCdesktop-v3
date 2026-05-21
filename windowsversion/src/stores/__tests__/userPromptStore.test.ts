/**
 * task_006_dev_frontend_store — userPromptStore 单元测试
 *
 * 覆盖 AC：
 *   - AC-1：初始 state（items 全 null / drafts 全 "" / dirty 全 false）
 *   - AC-2：loadAll 成功 → items + drafts(userText ?? defaultText) + dirty=false；错误 → error 透传
 *   - AC-3：setDraft 切换 dirty；纯本地不发 IPC
 *   - AC-4：save 成功 → IPC 调用顺序 + items 刷新 + dirty 归零；错误 → 抛出 + error 写入 + drafts 不动
 *   - AC-5：reset(null) → resetUserPrompt(null) + loadAll；reset(module) → resetUserPrompt(module) + getUserPrompt
 *   - AC-6：byteLen 中文 / emoji / 英文混合 / 空串
 *
 * 测试策略：vi.mock 拦截 `../../lib/tauri-commands`，每用例独立 fixture，
 * 通过 useUserPromptStore.getState() 操作与断言，不渲染组件（store 是纯状态容器）。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// 把 tauri-commands 4 个 *UserPrompt* 函数全部 mock 掉 — store 不应触达真实 Tauri IPC。
vi.mock("../../lib/tauri-commands", () => ({
  listUserPrompts: vi.fn(),
  getUserPrompt: vi.fn(),
  saveUserPrompt: vi.fn(),
  resetUserPrompt: vi.fn(),
}));

import { useUserPromptStore } from "../userPromptStore";
import * as cmd from "../../lib/tauri-commands";
import type { PromptInfo, PromptModule } from "../../types/user-prompt";

const mockListUserPrompts = vi.mocked(cmd.listUserPrompts);
const mockGetUserPrompt = vi.mocked(cmd.getUserPrompt);
const mockSaveUserPrompt = vi.mocked(cmd.saveUserPrompt);
const mockResetUserPrompt = vi.mocked(cmd.resetUserPrompt);

/** 测试 fixture：构造单条 PromptInfo。 */
function makeInfo(
  module: PromptModule,
  overrides: Partial<PromptInfo> = {},
): PromptInfo {
  return {
    module,
    displayTitle: module,
    defaultText: `[default ${module}]`,
    userText: null,
    isCustom: false,
    builtinVersion: "1.0",
    updatedAt: null,
    requiredPlaceholders: [],
    maxBytes: 16384,
    ...overrides,
  };
}

/** 构造 4 module 全集（默认全未自定义）。 */
function makeFullList(
  overrides: Partial<Record<PromptModule, Partial<PromptInfo>>> = {},
): PromptInfo[] {
  return (["tagging", "para", "concept", "aggregation"] as const).map((m) =>
    makeInfo(m, overrides[m]),
  );
}

const INITIAL = useUserPromptStore.getState();

beforeEach(() => {
  // 每个用例前重置 store 与 mock。
  useUserPromptStore.setState({
    items: { tagging: null, para: null, concept: null, aggregation: null },
    drafts: { tagging: "", para: "", concept: "", aggregation: "" },
    dirty: { tagging: false, para: false, concept: false, aggregation: false },
    loading: false,
    error: null,
  });
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AC-1 初始 state", () => {
  it("items 全 null / drafts 全空串 / dirty 全 false / loading=false / error=null", () => {
    // 注意：这里读初始快照（而非 beforeEach 重置后的状态）— 验证 create() 默认值
    expect(INITIAL.items).toEqual({
      tagging: null,
      para: null,
      concept: null,
      aggregation: null,
    });
    expect(INITIAL.drafts).toEqual({
      tagging: "",
      para: "",
      concept: "",
      aggregation: "",
    });
    expect(INITIAL.dirty).toEqual({
      tagging: false,
      para: false,
      concept: false,
      aggregation: false,
    });
    expect(INITIAL.loading).toBe(false);
    expect(INITIAL.error).toBeNull();
  });
});

describe("AC-2 loadAll", () => {
  it("成功路径：4 module 全部初始化，drafts 用 userText ?? defaultText", async () => {
    mockListUserPrompts.mockResolvedValueOnce(
      makeFullList({
        tagging: { userText: "我的 tagging 覆写", isCustom: true },
        // para / concept / aggregation 全未自定义（userText = null）
      }),
    );

    await useUserPromptStore.getState().loadAll();

    const s = useUserPromptStore.getState();
    expect(mockListUserPrompts).toHaveBeenCalledTimes(1);
    expect(s.loading).toBe(false);
    expect(s.error).toBeNull();

    // items 全填充
    expect(s.items.tagging?.isCustom).toBe(true);
    expect(s.items.tagging?.userText).toBe("我的 tagging 覆写");
    expect(s.items.para?.isCustom).toBe(false);
    expect(s.items.concept?.isCustom).toBe(false);
    expect(s.items.aggregation?.isCustom).toBe(false);

    // drafts：自定义 module 用 userText；其余用 defaultText
    expect(s.drafts.tagging).toBe("我的 tagging 覆写");
    expect(s.drafts.para).toBe("[default para]");
    expect(s.drafts.concept).toBe("[default concept]");
    expect(s.drafts.aggregation).toBe("[default aggregation]");

    // dirty 全 false（加载后 drafts ≡ 当前生效文本）
    expect(s.dirty).toEqual({
      tagging: false,
      para: false,
      concept: false,
      aggregation: false,
    });
  });

  it("加载中：loading=true → 完成后 loading=false", async () => {
    let resolveList: ((v: PromptInfo[]) => void) | null = null;
    mockListUserPrompts.mockReturnValueOnce(
      new Promise<PromptInfo[]>((resolve) => {
        resolveList = resolve;
      }),
    );

    const promise = useUserPromptStore.getState().loadAll();
    expect(useUserPromptStore.getState().loading).toBe(true);

    resolveList!(makeFullList());
    await promise;

    expect(useUserPromptStore.getState().loading).toBe(false);
  });

  it("错误路径：error 字段透传字符串消息 + loading=false（task_007_round2：升级为带 module 归属的对象，loadAll 失败 module=null 全局）", async () => {
    mockListUserPrompts.mockRejectedValueOnce("数据库读取失败");

    await useUserPromptStore.getState().loadAll();

    const s = useUserPromptStore.getState();
    expect(s.error).toEqual({ module: null, message: "数据库读取失败" });
    expect(s.loading).toBe(false);
  });
});

describe("AC-3 setDraft", () => {
  it("初始装载后 setDraft 与 defaultText 不同 → dirty=true", async () => {
    mockListUserPrompts.mockResolvedValueOnce(makeFullList());
    await useUserPromptStore.getState().loadAll();

    useUserPromptStore.getState().setDraft("tagging", "我的新草稿");

    const s = useUserPromptStore.getState();
    expect(s.drafts.tagging).toBe("我的新草稿");
    expect(s.dirty.tagging).toBe(true);
    // 其余 module 不受影响
    expect(s.dirty.para).toBe(false);
    expect(s.dirty.concept).toBe(false);
    expect(s.dirty.aggregation).toBe(false);
  });

  it("setDraft 回到 effectiveText → dirty 回 false", async () => {
    mockListUserPrompts.mockResolvedValueOnce(makeFullList());
    await useUserPromptStore.getState().loadAll();

    useUserPromptStore.getState().setDraft("tagging", "我的新草稿");
    expect(useUserPromptStore.getState().dirty.tagging).toBe(true);

    // 改回 defaultText（因 userText=null，effectiveText = defaultText）
    useUserPromptStore.getState().setDraft("tagging", "[default tagging]");
    expect(useUserPromptStore.getState().dirty.tagging).toBe(false);
  });

  it("已自定义场景：effectiveText = userText，setDraft 与 userText 比较", async () => {
    mockListUserPrompts.mockResolvedValueOnce(
      makeFullList({
        tagging: { userText: "我已经自定义过", isCustom: true },
      }),
    );
    await useUserPromptStore.getState().loadAll();

    // 改回 userText → dirty=false
    useUserPromptStore.getState().setDraft("tagging", "我已经自定义过");
    expect(useUserPromptStore.getState().dirty.tagging).toBe(false);

    // 改成 defaultText（不等于 userText）→ dirty=true
    useUserPromptStore.getState().setDraft("tagging", "[default tagging]");
    expect(useUserPromptStore.getState().dirty.tagging).toBe(true);
  });

  it("不发 IPC（纯本地）", () => {
    useUserPromptStore.getState().setDraft("para", "abc");
    expect(mockListUserPrompts).not.toHaveBeenCalled();
    expect(mockGetUserPrompt).not.toHaveBeenCalled();
    expect(mockSaveUserPrompt).not.toHaveBeenCalled();
    expect(mockResetUserPrompt).not.toHaveBeenCalled();
  });
});

describe("AC-4 save", () => {
  it("成功：调用 saveUserPrompt + getUserPrompt + items 刷新 + dirty 归零", async () => {
    mockListUserPrompts.mockResolvedValueOnce(makeFullList());
    await useUserPromptStore.getState().loadAll();
    useUserPromptStore.getState().setDraft("concept", "新的 concept 草稿 {content}");
    expect(useUserPromptStore.getState().dirty.concept).toBe(true);

    mockSaveUserPrompt.mockResolvedValueOnce(undefined);
    mockGetUserPrompt.mockResolvedValueOnce(
      makeInfo("concept", {
        userText: "新的 concept 草稿 {content}",
        isCustom: true,
        updatedAt: "2026-05-15T12:00:00Z",
      }),
    );

    await useUserPromptStore.getState().save("concept");

    expect(mockSaveUserPrompt).toHaveBeenCalledWith(
      "concept",
      "新的 concept 草稿 {content}",
    );
    expect(mockGetUserPrompt).toHaveBeenCalledWith("concept");

    const s = useUserPromptStore.getState();
    expect(s.items.concept?.isCustom).toBe(true);
    expect(s.items.concept?.userText).toBe("新的 concept 草稿 {content}");
    expect(s.items.concept?.updatedAt).toBe("2026-05-15T12:00:00Z");
    expect(s.dirty.concept).toBe(false);
    // drafts 保持用户当前输入
    expect(s.drafts.concept).toBe("新的 concept 草稿 {content}");
    expect(s.error).toBeNull();
  });

  it("错误：error 字段透传中文消息 + 抛出 + drafts/dirty/items 不变", async () => {
    mockListUserPrompts.mockResolvedValueOnce(makeFullList());
    await useUserPromptStore.getState().loadAll();
    useUserPromptStore.getState().setDraft("para", "超长".repeat(20000));
    const draftBeforeSave = useUserPromptStore.getState().drafts.para;
    const itemBeforeSave = useUserPromptStore.getState().items.para;

    mockSaveUserPrompt.mockRejectedValueOnce(
      "自定义 Prompt 过长（120000 字节，上限 16384 字节），请精简",
    );

    await expect(useUserPromptStore.getState().save("para")).rejects.toBe(
      "自定义 Prompt 过长（120000 字节，上限 16384 字节），请精简",
    );

    const s = useUserPromptStore.getState();
    // task_007_round2：save 失败 → error 归属到失败的 module
    expect(s.error).toEqual({
      module: "para",
      message: "自定义 Prompt 过长（120000 字节，上限 16384 字节），请精简",
    });
    // 失败时不调 getUserPrompt（避免覆盖用户正在编辑的草稿）
    expect(mockGetUserPrompt).not.toHaveBeenCalled();
    // drafts / dirty / items 保留，便于用户修改后重试
    expect(s.drafts.para).toBe(draftBeforeSave);
    expect(s.dirty.para).toBe(true);
    expect(s.items.para).toBe(itemBeforeSave);
  });
});

describe("AC-5 reset", () => {
  it("reset(null)：调 resetUserPrompt(null) + loadAll 重载全部 4 条", async () => {
    // 先加载一次（含 tagging 自定义）
    mockListUserPrompts.mockResolvedValueOnce(
      makeFullList({
        tagging: { userText: "已自定义", isCustom: true },
      }),
    );
    await useUserPromptStore.getState().loadAll();
    expect(useUserPromptStore.getState().items.tagging?.isCustom).toBe(true);

    // reset(null) → 调 resetUserPrompt(null) + loadAll 再触发一次 listUserPrompts
    mockResetUserPrompt.mockResolvedValueOnce(undefined);
    mockListUserPrompts.mockResolvedValueOnce(
      // 重置后全部回 default（userText=null, isCustom=false）
      makeFullList(),
    );

    await useUserPromptStore.getState().reset(null);

    expect(mockResetUserPrompt).toHaveBeenCalledWith(null);
    expect(mockListUserPrompts).toHaveBeenCalledTimes(2); // 初始 + reset 后
    const s = useUserPromptStore.getState();
    expect(s.items.tagging?.isCustom).toBe(false);
    expect(s.items.tagging?.userText).toBeNull();
    expect(s.drafts.tagging).toBe("[default tagging]");
    expect(s.dirty.tagging).toBe(false);
    expect(s.error).toBeNull();
  });

  it("reset(module)：调 resetUserPrompt(module) + getUserPrompt + drafts 同步 defaultText", async () => {
    mockListUserPrompts.mockResolvedValueOnce(
      makeFullList({
        aggregation: { userText: "用户自定义聚合", isCustom: true },
      }),
    );
    await useUserPromptStore.getState().loadAll();
    expect(useUserPromptStore.getState().items.aggregation?.isCustom).toBe(true);

    mockResetUserPrompt.mockResolvedValueOnce(undefined);
    mockGetUserPrompt.mockResolvedValueOnce(
      makeInfo("aggregation"), // 重置后 userText=null, defaultText="[default aggregation]"
    );

    await useUserPromptStore.getState().reset("aggregation");

    expect(mockResetUserPrompt).toHaveBeenCalledWith("aggregation");
    expect(mockGetUserPrompt).toHaveBeenCalledWith("aggregation");
    const s = useUserPromptStore.getState();
    expect(s.items.aggregation?.userText).toBeNull();
    expect(s.items.aggregation?.isCustom).toBe(false);
    // 关键：drafts 同步为新的 defaultText（reset 后的种子）
    expect(s.drafts.aggregation).toBe("[default aggregation]");
    expect(s.dirty.aggregation).toBe(false);
    expect(s.error).toBeNull();
  });

  it("reset 错误：error 字段写入 + 抛出（task_007_round2：单条 reset 失败归属该 module）", async () => {
    mockResetUserPrompt.mockRejectedValueOnce("数据库写入失败");

    await expect(useUserPromptStore.getState().reset("tagging")).rejects.toBe(
      "数据库写入失败",
    );

    expect(useUserPromptStore.getState().error).toEqual({
      module: "tagging",
      message: "数据库写入失败",
    });
  });
});

describe("AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）", () => {
  beforeEach(() => {
    useUserPromptStore.setState({
      drafts: { tagging: "", para: "", concept: "", aggregation: "" },
    });
  });

  it("空串 = 0 字节", () => {
    useUserPromptStore.getState().setDraft("tagging", "");
    expect(useUserPromptStore.getState().byteLen("tagging")).toBe(0);
  });

  it("纯英文 ASCII：每字符 1 字节", () => {
    useUserPromptStore.setState({
      drafts: {
        ...useUserPromptStore.getState().drafts,
        para: "abcDEF 123",
      },
    });
    expect(useUserPromptStore.getState().byteLen("para")).toBe(10);
  });

  it("纯中文（CJK）：每字符 3 字节", () => {
    useUserPromptStore.setState({
      drafts: {
        ...useUserPromptStore.getState().drafts,
        concept: "你好",
      },
    });
    // "你" + "好" = 6 字节
    expect(useUserPromptStore.getState().byteLen("concept")).toBe(6);
  });

  it("emoji（U+1F31F 星）= 4 字节", () => {
    useUserPromptStore.setState({
      drafts: {
        ...useUserPromptStore.getState().drafts,
        aggregation: "🌟",
      },
    });
    expect(useUserPromptStore.getState().byteLen("aggregation")).toBe(4);
  });

  it("中文 + emoji + 英文混合：分别加和", () => {
    useUserPromptStore.setState({
      drafts: {
        ...useUserPromptStore.getState().drafts,
        tagging: "Hi 你好 🌟",
      },
    });
    // "Hi " = 3 + "你好" = 6 + " " = 1 + "🌟" = 4 → 14 字节
    expect(useUserPromptStore.getState().byteLen("tagging")).toBe(14);
  });

  it("byteLen 与 TextEncoder 等价（防止实现回归）", () => {
    const samples = [
      "",
      "abc",
      "你好世界",
      "🌟⭐",
      "Hello 世界 🌟",
      "{content}",
      "user_text_test_123",
    ];
    for (const s of samples) {
      useUserPromptStore.setState({
        drafts: {
          ...useUserPromptStore.getState().drafts,
          tagging: s,
        },
      });
      expect(useUserPromptStore.getState().byteLen("tagging")).toBe(
        new TextEncoder().encode(s).length,
      );
    }
  });
});

describe("AC-1 / AC-3 不可变性 smoke", () => {
  it("setDraft 返回新引用（不就地 mutate drafts 对象）", () => {
    const before = useUserPromptStore.getState().drafts;
    useUserPromptStore.getState().setDraft("tagging", "abc");
    const after = useUserPromptStore.getState().drafts;
    expect(after).not.toBe(before);
    expect(after.tagging).toBe("abc");
  });
});
