/**
 * WorkspaceFolderListView 单测（task_007 T5a）
 *
 * 覆盖 input.md AC-7：
 *   - 列表渲染 3 类 kind 行（__ROOT__ / root / ai_organized）
 *   - 工具栏「重命名」「移到废纸篓」按钮在选中 root 行激活、ai_organized / __ROOT__ 行 disabled
 *   - 右键 root 行：菜单显示 重命名 / 移到废纸篓 / 在文件资源管理器中显示
 *   - 右键 ai_organized 行：仅「在文件资源管理器中显示」可点，其余灰显（点击灰显项不触发 handler）
 *   - 右键 __ROOT__ 行：仅「在文件资源管理器中显示」（不含重命名 / 删除条目）
 *   - direct invoke 防御：⌘⌫ 在选中 ai_organized 时不触发删除 handler（ADR-007）
 *   - 双击行触发 setWorkspaceFolderRelativePath
 */
import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, within, act, waitFor } from "@testing-library/react";

vi.mock("../../../lib/tauri-commands", () => ({
  createWorkspaceFolder: vi.fn(),
  renameWorkspaceFolder: vi.fn(),
  deleteWorkspaceFolder: vi.fn(),
  moveAssetToWorkspaceFolder: vi.fn(),
  countFolderAssets: vi.fn(),
  revealProjectWorkspaceFolder: vi.fn(),
  listProjectWorkspaceFolders: vi.fn(),
}));

import { WorkspaceFolderListView } from "../WorkspaceFolderListView";
import { useUIStore } from "../../../stores/uiStore";
import * as tauriCommands from "../../../lib/tauri-commands";
import type { Asset, WorkspaceFolderEntry } from "../../../types";

const INITIAL_UI = useUIStore.getState();

const WS_ROOT = "/tmp/ws";

const FOLDERS: WorkspaceFolderEntry[] = [
  { relativePath: "__ROOT__", displayLabel: "（根目录）", kind: "root_import" },
  { relativePath: "参考资料", displayLabel: "参考资料", kind: "root" },
  { relativePath: "organized/2026-05", displayLabel: "2026-05", kind: "ai_organized" },
];

const ASSETS: Asset[] = [];

beforeEach(() => {
  useUIStore.setState({
    ...INITIAL_UI,
    workspaceFolderRelativePath: null,
    editingFolderPath: null,
    pendingNewFolder: false,
    pendingRenameIds: new Set<string>(),
    notifications: [],
  });
  vi.mocked(tauriCommands.createWorkspaceFolder).mockReset();
  vi.mocked(tauriCommands.renameWorkspaceFolder).mockReset();
  vi.mocked(tauriCommands.deleteWorkspaceFolder).mockReset();
});

function renderView(overrides?: {
  onReveal?: (rel: string) => void;
  onRefresh?: () => void;
}) {
  const onReveal = overrides?.onReveal ?? vi.fn();
  const onRefresh = overrides?.onRefresh ?? vi.fn();
  const utils = render(
    <WorkspaceFolderListView
      projectId="p1"
      folders={FOLDERS}
      assets={ASSETS}
      workspaceRoot={WS_ROOT}
      onReveal={onReveal}
      onRefresh={onRefresh}
    />,
  );
  return { ...utils, onReveal, onRefresh };
}

describe("WorkspaceFolderListView — 渲染骨架", () => {
  it("AC-1/AC-2 渲染 3 类 kind 行 + 列头", () => {
    renderView();
    expect(screen.getByTestId("folder-list-header")).toBeInTheDocument();
    expect(screen.getByTestId("folder-row-__ROOT__")).toBeInTheDocument();
    expect(screen.getByTestId("folder-row-参考资料")).toBeInTheDocument();
    expect(
      screen.getByTestId("folder-row-organized/2026-05"),
    ).toBeInTheDocument();
    // ai_organized 行带 Sparkles 角标
    expect(
      screen.getByTestId("folder-row-organized/2026-05-sparkle"),
    ).toBeInTheDocument();
    // root 行无角标
    expect(
      screen.queryByTestId("folder-row-参考资料-sparkle"),
    ).not.toBeInTheDocument();
  });
});

describe("WorkspaceFolderListView — 工具栏激活规则（AC-5 写动作按钮）", () => {
  it("未选中：重命名/删除 disabled", () => {
    renderView();
    expect(screen.getByTestId("toolbar-create")).not.toBeDisabled();
    expect(screen.getByTestId("toolbar-rename")).toBeDisabled();
    expect(screen.getByTestId("toolbar-delete")).toBeDisabled();
  });

  it("选中 root 行：重命名/删除 激活", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    expect(screen.getByTestId("toolbar-rename")).not.toBeDisabled();
    expect(screen.getByTestId("toolbar-delete")).not.toBeDisabled();
  });

  it("选中 ai_organized 行：重命名/删除 disabled", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-organized/2026-05"));
    expect(screen.getByTestId("toolbar-rename")).toBeDisabled();
    expect(screen.getByTestId("toolbar-delete")).toBeDisabled();
  });

  it("选中 __ROOT__ 行：重命名/删除 disabled", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-__ROOT__"));
    expect(screen.getByTestId("toolbar-rename")).toBeDisabled();
    expect(screen.getByTestId("toolbar-delete")).toBeDisabled();
  });
});

describe("WorkspaceFolderListView — 右键菜单形态", () => {
  it("root 行：显示 重命名 / 移到废纸篓 / 在文件资源管理器中显示", () => {
    renderView();
    fireEvent.contextMenu(screen.getByTestId("folder-row-参考资料"));
    const menu = screen.getByTestId("folder-ctx-root");
    expect(within(menu).getByTestId("ctx-rename")).toBeInTheDocument();
    expect(within(menu).getByTestId("ctx-delete")).toBeInTheDocument();
    expect(within(menu).getByTestId("ctx-reveal")).toBeInTheDocument();
    expect(within(menu).getByTestId("ctx-rename")).toHaveAttribute(
      "aria-disabled",
      "false",
    );
  });

  it("ai_organized 行：重命名/删除 灰显（不触发 handler），仅 reveal 可点", () => {
    const onReveal = vi.fn();
    renderView({ onReveal });
    fireEvent.contextMenu(screen.getByTestId("folder-row-organized/2026-05"));
    const menu = screen.getByTestId("folder-ctx-ai_organized");
    const rename = within(menu).getByTestId("ctx-rename");
    const del = within(menu).getByTestId("ctx-delete");
    const reveal = within(menu).getByTestId("ctx-reveal");
    expect(rename).toHaveAttribute("aria-disabled", "true");
    expect(del).toHaveAttribute("aria-disabled", "true");
    expect(rename).toHaveAttribute("title", "AI 归类目录受保护");

    // 尝试点击灰显项 — 不应触发 reveal/任何后端 wrapper（且菜单不应关闭后弹错）
    fireEvent.click(rename);
    fireEvent.click(del);
    expect(onReveal).not.toHaveBeenCalled();

    // 点击 reveal 触发 onReveal 并关闭菜单
    fireEvent.click(reveal);
    expect(onReveal).toHaveBeenCalledWith("organized/2026-05");
  });

  it("__ROOT__ 行：仅显示「在文件资源管理器中显示」，不含重命名/删除", () => {
    const onReveal = vi.fn();
    renderView({ onReveal });
    fireEvent.contextMenu(screen.getByTestId("folder-row-__ROOT__"));
    const menu = screen.getByTestId("folder-ctx-root-sentinel");
    expect(within(menu).getByTestId("ctx-reveal")).toBeInTheDocument();
    expect(within(menu).queryByTestId("ctx-rename")).not.toBeInTheDocument();
    expect(within(menu).queryByTestId("ctx-delete")).not.toBeInTheDocument();
    fireEvent.click(within(menu).getByTestId("ctx-reveal"));
    expect(onReveal).toHaveBeenCalledWith("__ROOT__");
  });
});

describe("WorkspaceFolderListView — handler 入口判定（ADR-007）", () => {
  it("⌘⌫ 在选中 ai_organized 时：不触发删除（handler 首行 return）", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    renderView();
    // 选中 ai_organized
    fireEvent.click(screen.getByTestId("folder-row-organized/2026-05"));
    const container = screen.getByTestId("workspace-folder-list-view");
    fireEvent.keyDown(container, { key: "Backspace", metaKey: true });
    // handler 占位日志「delete pending」不应出现
    const hasDeleteLog = warn.mock.calls.some((args) =>
      args.some((a) => typeof a === "string" && a.includes("delete pending")),
    );
    expect(hasDeleteLog).toBe(false);
    warn.mockRestore();
  });

  it("⌘⌫ 在选中 root 时：弹出删除二次确认 modal（T5b 接入真正 handler）", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    const container = screen.getByTestId("workspace-folder-list-view");
    fireEvent.keyDown(container, { key: "Backspace", metaKey: true });
    expect(screen.getByTestId("folder-delete-modal")).toBeInTheDocument();
  });

  it("Enter 在选中 ai_organized 时：不触发重命名入口", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-organized/2026-05"));
    const container = screen.getByTestId("workspace-folder-list-view");
    fireEvent.keyDown(container, { key: "Enter" });
    const hasRenameLog = warn.mock.calls.some((args) =>
      args.some((a) => typeof a === "string" && a.includes("rename pending")),
    );
    expect(hasRenameLog).toBe(false);
    warn.mockRestore();
  });
});

describe("WorkspaceFolderListView — 选中态与双击", () => {
  it("单击行：触发 setWorkspaceFolderRelativePath", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
  });

  it("双击行：切换选中（已选 → null）", () => {
    renderView();
    const row = screen.getByTestId("folder-row-参考资料");
    fireEvent.click(row);
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
    fireEvent.doubleClick(row);
    expect(useUIStore.getState().workspaceFolderRelativePath).toBeNull();
  });

  it("Up/Down 键盘导航：在 items 间切换选中", () => {
    renderView();
    const container = screen.getByTestId("workspace-folder-list-view");
    // 初始未选中 → ArrowDown 应聚到 idx=0 之后即 idx=1（curIdx 兜底为 0，next=1）
    fireEvent.keyDown(container, { key: "ArrowDown" });
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
    fireEvent.keyDown(container, { key: "ArrowDown" });
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe(
      "organized/2026-05",
    );
    fireEvent.keyDown(container, { key: "ArrowUp" });
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
  });
});

describe("WorkspaceFolderListView — 项目数前端聚合（AC-3）", () => {
  it("__ROOT__ = 不含 / 的文件；root 行 = firstSegment === relativePath", () => {
    const assets: Asset[] = [
      // 根目录直接文件 2 个
      makeAsset("a.png", "p1"),
      makeAsset("b.pdf", "p1"),
      // 参考资料/ 下 1 个
      makeAsset("参考资料/x.md", "p1"),
      // organized/2026-05/ 下 1 个
      makeAsset("organized/2026-05/y.png", "p1"),
      // organized/2026-04/ 下 1 个（不属于 2026-05 行）
      makeAsset("organized/2026-04/z.png", "p1"),
    ];
    render(
      <WorkspaceFolderListView
        projectId="p1"
        folders={FOLDERS}
        assets={assets}
        workspaceRoot={WS_ROOT}
        onReveal={() => {}}
        onRefresh={() => {}}
      />,
    );
    const root = screen.getByTestId("folder-row-__ROOT__");
    const ref = screen.getByTestId("folder-row-参考资料");
    const ai = screen.getByTestId("folder-row-organized/2026-05");
    // 项目数列固定宽 56；用文本断言
    expect(root.textContent).toContain("2");
    expect(ref.textContent).toContain("1");
    // T5b 修复 MAJOR-1：ai_organized 行 count 改为前缀匹配，与后端 LIKE 等价
    // folder="organized/2026-05" + asset="organized/2026-05/y.png" → count=1
    expect(ai.textContent).toContain("1");
  });

  it("前缀冲突边界（100 vs 100%）：folder=100 行不被 100%/x.png 误命中", async () => {
    const assets: Asset[] = [
      makeAsset("100/a.png", "p1"),
      makeAsset("100%/x.png", "p1"),
    ];
    const folders: WorkspaceFolderEntry[] = [
      { relativePath: "__ROOT__", displayLabel: "（根目录）", kind: "root_import" },
      { relativePath: "100", displayLabel: "100", kind: "root" },
      { relativePath: "100%", displayLabel: "100%", kind: "root" },
    ];
    render(
      <WorkspaceFolderListView
        projectId="p1"
        folders={folders}
        assets={assets}
        workspaceRoot={WS_ROOT}
        onReveal={() => {}}
        onRefresh={() => {}}
      />,
    );
    const r100 = screen.getByTestId("folder-row-100");
    const r100p = screen.getByTestId("folder-row-100%");
    expect(r100.textContent).toContain("1");
    expect(r100p.textContent).toContain("1");
  });
});

// ──────────────────────────────────────────────────────────────────────
// T5b inline 编辑用例（task_008 AC-2 / AC-3 / AC-4 / AC-5）
// ──────────────────────────────────────────────────────────────────────

describe("T5b · F1 幽灵新建（AC-2）", () => {
  it("点击工具栏「+ 新建文件夹」→ 列表末尾出现幽灵行 + 输入框默认值「未命名文件夹」", () => {
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create") as HTMLInputElement;
    expect(editor).toBeInTheDocument();
    expect(editor.value).toBe("未命名文件夹");
    expect(screen.getByTestId("folder-row-__GHOST_NEW__")).toHaveAttribute(
      "data-ghost",
      "true",
    );
  });

  it("Enter 提交 → 调用 createWorkspaceFolder + 列表刷新 + 选中新行", async () => {
    vi.mocked(tauriCommands.createWorkspaceFolder).mockResolvedValueOnce({
      relativePath: "新文件夹",
      displayLabel: "新文件夹",
      kind: "root",
    });
    const onRefresh = vi.fn();
    renderView({ onRefresh });
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create");
    fireEvent.change(editor, { target: { value: "新文件夹" } });
    await act(async () => {
      fireEvent.keyDown(editor, { key: "Enter" });
    });
    await waitFor(() => {
      expect(tauriCommands.createWorkspaceFolder).toHaveBeenCalledWith("p1", "新文件夹");
    });
    expect(onRefresh).toHaveBeenCalled();
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("新文件夹");
    expect(useUIStore.getState().pendingNewFolder).toBe(false);
  });

  it("Esc 取消 → 不发 IPC + 退出编辑态", () => {
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create");
    fireEvent.keyDown(editor, { key: "Escape" });
    expect(tauriCommands.createWorkspaceFolder).not.toHaveBeenCalled();
    expect(useUIStore.getState().pendingNewFolder).toBe(false);
    expect(screen.queryByTestId("inline-editor-create")).not.toBeInTheDocument();
  });

  it("blur 提交（与 Enter 同语义）→ 调用 createWorkspaceFolder", async () => {
    vi.mocked(tauriCommands.createWorkspaceFolder).mockResolvedValueOnce({
      relativePath: "blurred",
      displayLabel: "blurred",
      kind: "root",
    });
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create");
    fireEvent.change(editor, { target: { value: "blurred" } });
    await act(async () => {
      fireEvent.blur(editor);
    });
    await waitFor(() => {
      expect(tauriCommands.createWorkspaceFolder).toHaveBeenCalledWith("p1", "blurred");
    });
  });

  it("失败保留编辑态 + 红框（mock IpcError E_NAME_DUP）", async () => {
    vi.mocked(tauriCommands.createWorkspaceFolder).mockRejectedValueOnce({
      code: "E_NAME_DUP",
      message: "duplicate",
      details: { name: "重复" },
    });
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create") as HTMLInputElement;
    fireEvent.change(editor, { target: { value: "重复" } });
    await act(async () => {
      fireEvent.keyDown(editor, { key: "Enter" });
    });
    await waitFor(() => {
      expect(tauriCommands.createWorkspaceFolder).toHaveBeenCalled();
    });
    // 编辑态保留
    const ed = screen.getByTestId("inline-editor-create") as HTMLInputElement;
    expect(ed).toBeInTheDocument();
    expect(ed.getAttribute("data-error")).toBe("true");
    expect(useUIStore.getState().pendingNewFolder).toBe(true);
  });

  it("校验失败保留编辑态 + 红框 + 不发 IPC（输入 / 触发 has_slash）", async () => {
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    const editor = screen.getByTestId("inline-editor-create") as HTMLInputElement;
    fireEvent.change(editor, { target: { value: "a/b" } });
    expect(editor.getAttribute("data-error")).toBe("true");
    await act(async () => {
      fireEvent.keyDown(editor, { key: "Enter" });
    });
    expect(tauriCommands.createWorkspaceFolder).not.toHaveBeenCalled();
    expect(useUIStore.getState().pendingNewFolder).toBe(true);
  });

  it("编辑期点击其他行 → 弹切走二次确认 modal", () => {
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    fireEvent.change(screen.getByTestId("inline-editor-create"), {
      target: { value: "drafting" },
    });
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    expect(screen.getByTestId("folder-discard-modal")).toBeInTheDocument();
    expect(screen.getByTestId("folder-discard-modal").textContent).toContain("drafting");
  });

  it("切走 modal 确认放弃 → cancelCreating + 执行原 action（切换 selection）", () => {
    renderView();
    fireEvent.click(screen.getByTestId("toolbar-create"));
    fireEvent.change(screen.getByTestId("inline-editor-create"), {
      target: { value: "drafting" },
    });
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("folder-discard-confirm"));
    expect(useUIStore.getState().pendingNewFolder).toBe(false);
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
  });
});

describe("T5b · F2 重命名（AC-3）", () => {
  it("工具栏「重命名」→ 行内 InlineNameEditor 渲染（全选当前名）", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-rename"));
    const editor = screen.getByTestId("inline-editor-rename") as HTMLInputElement;
    expect(editor).toBeInTheDocument();
    expect(editor.value).toBe("参考资料");
    expect(useUIStore.getState().editingFolderPath).toBe("参考资料");
    expect(useUIStore.getState().pendingRenameIds.has("参考资料")).toBe(true);
  });

  it("右键菜单「重命名」→ 进入 inline 编辑", () => {
    renderView();
    fireEvent.contextMenu(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("ctx-rename"));
    expect(screen.getByTestId("inline-editor-rename")).toBeInTheDocument();
  });

  it("Enter 提交：同步乐观（UI 不抛错），调用 renameWorkspaceFolder", async () => {
    vi.mocked(tauriCommands.renameWorkspaceFolder).mockResolvedValueOnce({
      relativePath: "参考-NEW",
      displayLabel: "参考-NEW",
      kind: "root",
    });
    const onRefresh = vi.fn();
    renderView({ onRefresh });
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-rename"));
    const editor = screen.getByTestId("inline-editor-rename");
    fireEvent.change(editor, { target: { value: "参考-NEW" } });
    await act(async () => {
      fireEvent.keyDown(editor, { key: "Enter" });
    });
    await waitFor(() => {
      expect(tauriCommands.renameWorkspaceFolder).toHaveBeenCalledWith(
        "p1",
        "参考资料",
        "参考-NEW",
      );
    });
    expect(onRefresh).toHaveBeenCalled();
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考-NEW");
    expect(useUIStore.getState().editingFolderPath).toBeNull();
  });

  it("失败回滚：mock reject → 名称回到旧值 + selection 回到 oldRel + 编辑态退出", async () => {
    vi.mocked(tauriCommands.renameWorkspaceFolder).mockRejectedValueOnce({
      code: "E_NAME_DUP",
      message: "dup",
      details: { name: "X" },
    });
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-rename"));
    const editor = screen.getByTestId("inline-editor-rename");
    fireEvent.change(editor, { target: { value: "X" } });
    await act(async () => {
      fireEvent.keyDown(editor, { key: "Enter" });
    });
    await waitFor(() => {
      expect(tauriCommands.renameWorkspaceFolder).toHaveBeenCalled();
    });
    // 名称回到旧值（行内文本仍是「参考资料」，编辑器消失）
    expect(screen.queryByTestId("inline-editor-rename")).not.toBeInTheDocument();
    expect(screen.getByTestId("folder-row-参考资料")).toBeInTheDocument();
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
    expect(useUIStore.getState().editingFolderPath).toBeNull();
  });

  it("Esc 取消：finishRename + 不发 IPC + UI 名称不变", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-rename"));
    const editor = screen.getByTestId("inline-editor-rename");
    fireEvent.change(editor, { target: { value: "X" } });
    fireEvent.keyDown(editor, { key: "Escape" });
    expect(tauriCommands.renameWorkspaceFolder).not.toHaveBeenCalled();
    expect(useUIStore.getState().editingFolderPath).toBeNull();
    expect(screen.getByTestId("folder-row-参考资料")).toBeInTheDocument();
  });

  it("selection 冻结：pendingRenameIds 非空时点击其他行无响应", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-rename"));
    expect(useUIStore.getState().pendingRenameIds.has("参考资料")).toBe(true);
    // 点击 ai_organized 行
    fireEvent.click(screen.getByTestId("folder-row-organized/2026-05"));
    // selection 保持
    expect(useUIStore.getState().workspaceFolderRelativePath).toBe("参考资料");
  });
});

describe("T5b · F3 删除二次确认（AC-4）", () => {
  it("N === 0 → 文案「删除文件夹『xxx』？」", () => {
    renderView();
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-delete"));
    const modal = screen.getByTestId("folder-delete-modal");
    expect(modal.textContent).toContain("删除文件夹");
    expect(modal.textContent).toContain("参考资料");
  });

  it("N > 0 → 文案含「包含 N 个素材」", () => {
    const assets: Asset[] = [
      makeAsset("参考资料/a.md", "p1"),
      makeAsset("参考资料/b.md", "p1"),
    ];
    render(
      <WorkspaceFolderListView
        projectId="p1"
        folders={FOLDERS}
        assets={assets}
        workspaceRoot={WS_ROOT}
        onReveal={() => {}}
        onRefresh={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-delete"));
    const modal = screen.getByTestId("folder-delete-modal");
    expect(modal.textContent).toContain("包含 2 个素材");
  });

  it("确认 → 调用 deleteWorkspaceFolder(confirmNonEmpty, expectedCount)", async () => {
    vi.mocked(tauriCommands.deleteWorkspaceFolder).mockResolvedValueOnce({ trashed: 3 });
    const assets: Asset[] = [
      makeAsset("参考资料/a.md", "p1"),
      makeAsset("参考资料/b.md", "p1"),
      makeAsset("参考资料/c.md", "p1"),
    ];
    const onRefresh = vi.fn();
    render(
      <WorkspaceFolderListView
        projectId="p1"
        folders={FOLDERS}
        assets={assets}
        workspaceRoot={WS_ROOT}
        onReveal={() => {}}
        onRefresh={onRefresh}
      />,
    );
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-delete"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("folder-delete-confirm"));
    });
    await waitFor(() => {
      expect(tauriCommands.deleteWorkspaceFolder).toHaveBeenCalledWith(
        "p1",
        "参考资料",
        true,
        3,
      );
    });
    expect(onRefresh).toHaveBeenCalled();
  });

  it("E_FOLDER_DIRTY 重弹 → 文案含 details.now 数值 5", async () => {
    vi.mocked(tauriCommands.deleteWorkspaceFolder).mockRejectedValueOnce({
      code: "E_FOLDER_DIRTY",
      message: "dirty",
      details: { old: 2, now: 5 },
    });
    const assets: Asset[] = [
      makeAsset("参考资料/a.md", "p1"),
      makeAsset("参考资料/b.md", "p1"),
    ];
    render(
      <WorkspaceFolderListView
        projectId="p1"
        folders={FOLDERS}
        assets={assets}
        workspaceRoot={WS_ROOT}
        onReveal={() => {}}
        onRefresh={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("folder-row-参考资料"));
    fireEvent.click(screen.getByTestId("toolbar-delete"));
    await act(async () => {
      fireEvent.click(screen.getByTestId("folder-delete-confirm"));
    });
    // 重弹后 modal 仍在
    const modal = await screen.findByTestId("folder-delete-modal");
    expect(modal.textContent).toContain("内容已变化");
    expect(modal.textContent).toContain("5");
  });
});

// ── helpers ────────────────────────────────────────────────────────────

function makeAsset(filePath: string, projectId: string): Asset {
  return {
    id: filePath,
    projectId,
    type: "other",
    name: filePath,
    filePath,
    fileSize: 0,
    mimeType: "application/octet-stream",
    tags: [],
    capturedAt: "2026-05-12T00:00:00.000Z",
    importedAt: "2026-05-12T00:00:00.000Z",
    source: { type: "manual_import" },
    aiAnalysis: null,
    isStarred: false,
  };
}
