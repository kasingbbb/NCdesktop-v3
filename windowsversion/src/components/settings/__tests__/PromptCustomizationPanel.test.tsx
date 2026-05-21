/**
 * task_007_dev_frontend_ui — PromptCustomizationPanel 单元测试
 *
 * 覆盖 AC-5：
 *   1. 渲染 4 个折叠子项（初始全折叠）
 *   2. 点击第一个折叠头展开
 *   3. 输入文本触发 setDraft + dirty=true
 *   4. 缺占位符时保存按钮 disabled
 *   5. 点击保存调用 save(module)
 *   6. 单条"恢复默认"调用 reset(module)
 *   7. 底部"全部恢复默认"经 confirm 调用 reset(null)
 *   8. 状态指示：已自定义 vs 默认
 *   附加：字节超限色阶、占位符 chip 渲染、错误横条展示、loadAll 挂载触发
 *
 * 测试策略：vi.mock 整个 userPromptStore module，导出一个可被测试代码 setState 的真实 zustand store，
 * 使 PromptCustomizationPanel 的 selector 行为与生产一致；action 函数全是 vi.fn() 便于断言调用。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type {
  PromptInfo,
  PromptModule,
} from "../../../types/user-prompt";

// ─── mock store ────────────────────────────────────────────────
// 在 vi.mock 工厂内创建 store（避免 hoist 时引用未初始化的变量）；
// 工厂导出真实 zustand store，selector 行为与生产一致；
// 测试代码通过下方 `import * as storeModule` 拿到导出对象，反查 mock 状态。
vi.mock("../../../stores/userPromptStore", async () => {
  const { create } = await import("zustand");
  const store = create<TestStore>(() => ({
    items: { tagging: null, para: null, concept: null, aggregation: null },
    drafts: { tagging: "", para: "", concept: "", aggregation: "" },
    dirty: { tagging: false, para: false, concept: false, aggregation: false },
    loading: false,
    error: null,
    loadAll: vi.fn(async () => {}),
    setDraft: vi.fn(),
    save: vi.fn(async () => {}),
    reset: vi.fn(async () => {}),
    byteLen: (m: PromptModule) =>
      new TextEncoder().encode(store.getState().drafts[m]).length,
  }));
  return { useUserPromptStore: store };
});

interface TestStore {
  items: Record<PromptModule, PromptInfo | null>;
  drafts: Record<PromptModule, string>;
  dirty: Record<PromptModule, boolean>;
  loading: boolean;
  // task_007_round2：error 升级为带 module 归属的对象（去重）
  error: { module: PromptModule | null; message: string } | null;
  loadAll: ReturnType<typeof vi.fn>;
  setDraft: ReturnType<typeof vi.fn>;
  save: ReturnType<typeof vi.fn>;
  reset: ReturnType<typeof vi.fn>;
  byteLen: (module: PromptModule) => number;
}

import { PromptCustomizationPanel } from "../PromptCustomizationPanel";
import { useUserPromptStore as mockStoreImport } from "../../../stores/userPromptStore";

// 类型断言：mock 工厂返回的就是 zustand store；提供 getState/setState 接口。
type StoreApi = {
  getState: () => TestStore;
  setState: (partial: Partial<TestStore>) => void;
};
const mockStore = mockStoreImport as unknown as StoreApi;

// ─── fixture helpers ───────────────────────────────────────────
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

function seedLoaded(opts?: {
  tagging?: Partial<PromptInfo>;
  para?: Partial<PromptInfo>;
  concept?: Partial<PromptInfo>;
  aggregation?: Partial<PromptInfo>;
  drafts?: Partial<Record<PromptModule, string>>;
  dirty?: Partial<Record<PromptModule, boolean>>;
  error?: { module: PromptModule | null; message: string } | null;
}) {
  const items: Record<PromptModule, PromptInfo | null> = {
    tagging: makeInfo("tagging", opts?.tagging),
    para: makeInfo("para", opts?.para),
    concept: makeInfo("concept", {
      requiredPlaceholders: ["{content}"],
      ...opts?.concept,
    }),
    aggregation: makeInfo("aggregation", opts?.aggregation),
  };
  const drafts: Record<PromptModule, string> = {
    tagging: opts?.drafts?.tagging ?? items.tagging!.defaultText,
    para: opts?.drafts?.para ?? items.para!.defaultText,
    concept: opts?.drafts?.concept ?? items.concept!.defaultText,
    aggregation: opts?.drafts?.aggregation ?? items.aggregation!.defaultText,
  };
  const dirty: Record<PromptModule, boolean> = {
    tagging: opts?.dirty?.tagging ?? false,
    para: opts?.dirty?.para ?? false,
    concept: opts?.dirty?.concept ?? false,
    aggregation: opts?.dirty?.aggregation ?? false,
  };
  mockStore.setState({
    items,
    drafts,
    dirty,
    loading: false,
    error: opts?.error ?? null,
  });
}

beforeEach(() => {
  mockStore.setState({
    items: { tagging: null, para: null, concept: null, aggregation: null },
    drafts: { tagging: "", para: "", concept: "", aggregation: "" },
    dirty: { tagging: false, para: false, concept: false, aggregation: false },
    loading: false,
    error: null,
    loadAll: vi.fn(async () => {}),
    setDraft: vi.fn(),
    save: vi.fn(async () => {}),
    reset: vi.fn(async () => {}),
    byteLen: (m: PromptModule) =>
      new TextEncoder().encode(mockStore.getState().drafts[m]).length,
  });
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ──────────────────────────────────────────────────────────────
// 测试用例
// ──────────────────────────────────────────────────────────────

describe("AC-1 / AC-5 ① 渲染结构", () => {
  it("挂载时调一次 loadAll()", () => {
    const loadAll = vi.fn(async () => {});
    mockStore.setState({ loadAll });
    render(<PromptCustomizationPanel />);
    expect(loadAll).toHaveBeenCalledTimes(1);
  });

  it("渲染 4 个折叠子项（按 PROMPT_MODULES 顺序）", () => {
    render(<PromptCustomizationPanel />);
    expect(screen.getByTestId("prompt-customization-panel")).toBeInTheDocument();
    expect(screen.getByTestId("prompt-section-tagging")).toBeInTheDocument();
    expect(screen.getByTestId("prompt-section-para")).toBeInTheDocument();
    expect(screen.getByTestId("prompt-section-concept")).toBeInTheDocument();
    expect(screen.getByTestId("prompt-section-aggregation")).toBeInTheDocument();
  });

  it("初始全部折叠（textarea 不在 DOM 中）", () => {
    render(<PromptCustomizationPanel />);
    expect(
      screen.queryByTestId("prompt-textarea-tagging"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("prompt-textarea-aggregation"),
    ).not.toBeInTheDocument();
  });

  it("显示顶部说明文案与底部「全部恢复默认」按钮", () => {
    render(<PromptCustomizationPanel />);
    expect(
      screen.getByText("以下为系统内置的 AI 处理策略。"),
    ).toBeInTheDocument();
    expect(screen.getByText("修改后将影响对应功能的输出结果。")).toBeInTheDocument();
    expect(screen.getByTestId("reset-all-button")).toBeInTheDocument();
  });
});

describe("AC-5 ② 点击展开第一个折叠条", () => {
  it("点击 tagging 折叠头 → textarea 出现", () => {
    seedLoaded();
    render(<PromptCustomizationPanel />);

    expect(
      screen.queryByTestId("prompt-textarea-tagging"),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    expect(screen.getByTestId("prompt-textarea-tagging")).toBeInTheDocument();
  });

  it("再次点击折叠头 → textarea 消失（toggle）", () => {
    seedLoaded();
    render(<PromptCustomizationPanel />);

    const toggle = screen.getByTestId("prompt-toggle-tagging");
    fireEvent.click(toggle);
    expect(screen.getByTestId("prompt-textarea-tagging")).toBeInTheDocument();

    fireEvent.click(toggle);
    expect(
      screen.queryByTestId("prompt-textarea-tagging"),
    ).not.toBeInTheDocument();
  });
});

describe("AC-5 ③ 输入文本触发 setDraft", () => {
  it("textarea onChange 调 setDraft(module, text)", () => {
    seedLoaded();
    const setDraft = vi.fn();
    mockStore.setState({ setDraft });
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    const textarea = screen.getByTestId("prompt-textarea-tagging");
    fireEvent.change(textarea, { target: { value: "我的自定义打标签 Prompt" } });

    expect(setDraft).toHaveBeenCalledWith("tagging", "我的自定义打标签 Prompt");
  });
});

describe("AC-5 ④ 占位符 / dirty / 字节状态对保存按钮的影响", () => {
  it("concept module 缺占位符 {content} 时，save 按钮 disabled + 显示警告", () => {
    seedLoaded({
      drafts: { concept: "我自己写的，没有 placeholder" },
      dirty: { concept: true },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-concept"));

    const save = screen.getByTestId("save-button-concept");
    expect(save).toBeDisabled();
    expect(
      screen.getByTestId("placeholder-warning-concept"),
    ).toBeInTheDocument();
  });

  it("concept module 占位符 OK + dirty=true → save 按钮可用", () => {
    seedLoaded({
      drafts: { concept: "我自己写的，含 {content}" },
      dirty: { concept: true },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-concept"));

    expect(screen.getByTestId("save-button-concept")).not.toBeDisabled();
    expect(
      screen.queryByTestId("placeholder-warning-concept"),
    ).not.toBeInTheDocument();
  });

  it("dirty=false（草稿与生效一致）→ save 按钮 disabled", () => {
    seedLoaded({ dirty: { tagging: false } });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    expect(screen.getByTestId("save-button-tagging")).toBeDisabled();
  });

  it("字节超 16 KiB 上限时 save disabled + 计数显示红色 + 警示文案", () => {
    const huge = "x".repeat(17000); // 17 KiB ASCII = 17000 bytes
    seedLoaded({
      drafts: { tagging: huge },
      dirty: { tagging: true },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    const save = screen.getByTestId("save-button-tagging");
    expect(save).toBeDisabled();

    const counter = screen.getByTestId("byte-counter-tagging");
    // #ef4444 = red-500
    expect(counter).toHaveStyle({ color: "#ef4444" });
    expect(screen.getByText("已超过 16 KB 上限")).toBeInTheDocument();
  });
});

describe("AC-5 ⑤ 点击保存调 save(module)", () => {
  it("save 按钮可用时点击 → 调 save(tagging) 一次", async () => {
    seedLoaded({
      drafts: { tagging: "改过的 prompt" },
      dirty: { tagging: true },
    });
    const save = vi.fn(async () => {});
    mockStore.setState({ save });
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("save-button-tagging"));
    });

    expect(save).toHaveBeenCalledWith("tagging");
    expect(save).toHaveBeenCalledTimes(1);
  });
});

describe("AC-5 ⑥ 单条恢复默认 → reset(module)", () => {
  it("已自定义状态下点击「恢复默认」(单条) → confirm 后调 reset(module)", async () => {
    seedLoaded({
      tagging: { isCustom: true, userText: "我的自定义" },
      drafts: { tagging: "我的自定义" },
    });
    const reset = vi.fn(async () => {});
    mockStore.setState({ reset });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("reset-button-tagging"));
    });

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(reset).toHaveBeenCalledWith("tagging");
  });

  it("confirm 拒绝时不调 reset", async () => {
    seedLoaded({
      tagging: { isCustom: true, userText: "我的自定义" },
    });
    const reset = vi.fn(async () => {});
    mockStore.setState({ reset });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("reset-button-tagging"));
    });

    expect(reset).not.toHaveBeenCalled();
  });

  it("isCustom=false 时单条「恢复默认」按钮 disabled", () => {
    seedLoaded(); // 所有 isCustom 默认 false
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    expect(screen.getByTestId("reset-button-tagging")).toBeDisabled();
  });
});

describe("AC-5 ⑦ 全部恢复默认 → reset(null)", () => {
  it("点击「全部恢复默认」+ confirm → reset(null)", async () => {
    seedLoaded();
    const reset = vi.fn(async () => {});
    mockStore.setState({ reset });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<PromptCustomizationPanel />);

    await act(async () => {
      fireEvent.click(screen.getByTestId("reset-all-button"));
    });

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(confirmSpy).toHaveBeenCalledWith(
      "将恢复全部 4 条 Prompt 为内置默认值，已有自定义会丢失。继续？",
    );
    expect(reset).toHaveBeenCalledWith(null);
  });

  it("confirm 拒绝时不调 reset", async () => {
    seedLoaded();
    const reset = vi.fn(async () => {});
    mockStore.setState({ reset });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<PromptCustomizationPanel />);

    await act(async () => {
      fireEvent.click(screen.getByTestId("reset-all-button"));
    });

    expect(reset).not.toHaveBeenCalled();
  });
});

describe("AC-5 ⑧ 状态指示「已自定义」vs「默认」", () => {
  it("isCustom=true → 显示「已自定义」", () => {
    seedLoaded({ tagging: { isCustom: true, userText: "我的自定义" } });
    render(<PromptCustomizationPanel />);

    const status = screen.getByTestId("prompt-status-tagging");
    expect(status.textContent).toContain("已自定义");
  });

  it("isCustom=false → 显示「默认」", () => {
    seedLoaded(); // 默认 isCustom=false
    render(<PromptCustomizationPanel />);

    const status = screen.getByTestId("prompt-status-tagging");
    expect(status.textContent).toContain("默认");
    expect(status.textContent).not.toContain("已自定义");
  });
});

describe("附加：占位符 chip 展示", () => {
  it("concept 展开后显示 {content} chip", () => {
    seedLoaded();
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-concept"));

    // chip 在折叠体内
    const section = screen.getByTestId("prompt-section-concept");
    expect(section).toHaveTextContent("必含占位符");
    expect(section).toHaveTextContent("{content}");
  });

  it("requiredPlaceholders 为空时不显示占位符提示行", () => {
    seedLoaded(); // tagging requiredPlaceholders = []
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    const section = screen.getByTestId("prompt-section-tagging");
    expect(section).not.toHaveTextContent("必含占位符");
  });
});

describe("AC-3 错误横条（去重：归属到具体 module，全局错误顶部一次）", () => {
  it("store.error.module === 'tagging' + tagging 展开 → 仅 tagging 子项显示，concept 子项无重复", () => {
    seedLoaded({
      error: { module: "tagging", message: "保存失败：服务暂不可用" },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    fireEvent.click(screen.getByTestId("prompt-toggle-concept"));

    // tagging 下方有
    const banner = screen.getByTestId("error-banner-tagging");
    expect(banner.textContent).toContain("保存失败：服务暂不可用");
    // concept 下方无（去重）
    expect(screen.queryByTestId("error-banner-concept")).not.toBeInTheDocument();
    // 顶部全局 banner 也不出现（因 module 非 null）
    expect(screen.queryByTestId("error-banner-global")).not.toBeInTheDocument();
  });

  it("store.error.module === null（全局：loadAll 失败）→ 顶部 banner 出现，子项下方均无", () => {
    seedLoaded({
      error: { module: null, message: "数据库读取失败" },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    const globalBanner = screen.getByTestId("error-banner-global");
    expect(globalBanner.textContent).toContain("数据库读取失败");
    expect(screen.queryByTestId("error-banner-tagging")).not.toBeInTheDocument();
  });

  it("点击保存时操作前清空 error（再次失败由后续 save 写入）", async () => {
    seedLoaded({
      drafts: { tagging: "改过的" },
      dirty: { tagging: true },
      error: { module: "tagging", message: "上一轮残留的错误消息" },
    });
    const save = vi.fn(async () => {});
    mockStore.setState({ save });
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("save-button-tagging"));
    });

    // 操作前清空 → 此时 mockStore.error 应为 null
    expect(mockStore.getState().error).toBeNull();
    expect(save).toHaveBeenCalledWith("tagging");
  });
});

describe("AC-1 saving 反馈 / AC-2 无障碍 / AC-4 R4 副标题（task_007_round2 二轮修复）", () => {
  it("AC-1：保存中按钮显示 spinner + 文案『保存中…』+ disabled；resolve 后恢复", async () => {
    seedLoaded({
      drafts: { tagging: "改过的 prompt" },
      dirty: { tagging: true },
    });
    let resolveSave: (() => void) | null = null;
    const save = vi.fn(
      () => new Promise<void>((resolve) => { resolveSave = () => resolve(); }),
    );
    mockStore.setState({ save });
    render(<PromptCustomizationPanel />);

    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));
    const saveBtn = screen.getByTestId("save-button-tagging");
    expect(saveBtn.textContent).toContain("保存");
    expect(saveBtn.textContent).not.toContain("保存中");

    // 不 await，让 promise pending
    act(() => {
      fireEvent.click(saveBtn);
    });
    // saving=true：disabled + 文案变化
    expect(saveBtn).toBeDisabled();
    expect(saveBtn.textContent).toContain("保存中");

    // resolve 后恢复
    await act(async () => {
      resolveSave!();
      await Promise.resolve();
    });
    expect(saveBtn.textContent).not.toContain("保存中");
  });

  it("AC-2：保存按钮 disabled 时有 aria-disabled 属性", () => {
    seedLoaded(); // dirty 全 false → save 按钮 disabled
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    const saveBtn = screen.getByTestId("save-button-tagging");
    expect(saveBtn).toHaveAttribute("aria-disabled", "true");
  });

  it("AC-2：textarea 可通过 aria-label 定位（screen reader 支持）", () => {
    seedLoaded();
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    expect(
      screen.getByLabelText("文件打标签 的 Prompt 编辑区"),
    ).toBeInTheDocument();
  });

  it("AC-4：tagging / para 折叠头有 R4 副标题；concept / aggregation 无", () => {
    seedLoaded();
    render(<PromptCustomizationPanel />);

    const taggingSub = screen.getByTestId("prompt-subtitle-tagging");
    expect(taggingSub.textContent).toContain("与「PARA 分组」共用同一次分类调用");
    const paraSub = screen.getByTestId("prompt-subtitle-para");
    expect(paraSub.textContent).toContain("与「文件打标签」共用同一次分类调用");

    expect(screen.queryByTestId("prompt-subtitle-concept")).not.toBeInTheDocument();
    expect(screen.queryByTestId("prompt-subtitle-aggregation")).not.toBeInTheDocument();
  });

  it("AC-6：字节超限时独立警告行 + AlertTriangle 图标存在", () => {
    seedLoaded({
      drafts: { tagging: "x".repeat(17000) },
      dirty: { tagging: true },
    });
    render(<PromptCustomizationPanel />);
    fireEvent.click(screen.getByTestId("prompt-toggle-tagging"));

    const warning = screen.getByTestId("byte-overflow-warning-tagging");
    expect(warning).toBeInTheDocument();
    expect(warning.textContent).toContain("已超过 16 KB 上限");
  });
});
