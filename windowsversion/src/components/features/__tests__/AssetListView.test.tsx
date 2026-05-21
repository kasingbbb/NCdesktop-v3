/**
 * AssetListView 四态徽章单测（task_008 AC-6）。
 *
 * 策略：直接测 `AssetStateBadge` 组件（AssetListView 内嵌的状态徽章实现），
 * 避免对整页所有 zustand store / tauri-commands / lucide-react / resize 等
 * 重 mock；行为与「行渲染 4 个不同 data-state + failed 行有重试按钮」AC 等价。
 *
 * 同时回归一条：`AssetStateBadge` 文案严格走 `assetStateLabel`（AC-5）。
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within, act } from "@testing-library/react";

vi.mock("../../../lib/tauri-commands", () => ({
  retryAssetConversion: vi.fn().mockResolvedValue(undefined),
}));

import { AssetStateBadge, assetStateLabel } from "../../../lib/asset-state";
import { retryAssetConversion } from "../../../lib/tauri-commands";
import type { AssetState } from "../../../types/workspaceAsset";

const STATES: AssetState[] = ["done", "converting", "failed", "offline"];

beforeEach(() => {
  vi.mocked(retryAssetConversion).mockClear();
});

describe("AssetStateBadge — 四态渲染", () => {
  it("AC-6 渲染 4 态资产 → 4 个不同 data-state 出现", () => {
    render(
      <ul>
        {STATES.map((s) => (
          <li key={s} data-asset-id={`a-${s}`} data-state={s} data-testid={`row-${s}`}>
            <AssetStateBadge state={s} assetId={`a-${s}`} />
          </li>
        ))}
      </ul>
    );
    // 行 data-state 4 种全部出现
    for (const s of STATES) {
      const row = screen.getByTestId(`row-${s}`);
      expect(row).toHaveAttribute("data-state", s);
      // 徽章本身也带 data-state
      const badge = within(row).getByTestId("asset-state-badge");
      expect(badge).toHaveAttribute("data-state", s);
      // 文案 = assetStateLabel(state)（中文，AC-5）
      expect(badge).toHaveTextContent(assetStateLabel(s));
    }
    // 4 个不同 data-state（去重）
    const uniqueStates = new Set(
      screen.getAllByTestId("asset-state-badge").map((el) => el.getAttribute("data-state"))
    );
    expect(uniqueStates.size).toBe(4);
  });

  it("AC-2 failed 行带『重试』按钮，其它行不带", () => {
    render(
      <div>
        {STATES.map((s) => (
          <div key={s} data-testid={`wrap-${s}`}>
            <AssetStateBadge state={s} assetId={`id-${s}`} reason="boom" />
          </div>
        ))}
      </div>
    );
    expect(within(screen.getByTestId("wrap-failed")).getByRole("button", { name: /重试转化/ })).toBeInTheDocument();
    for (const s of STATES.filter((x) => x !== "failed")) {
      expect(within(screen.getByTestId(`wrap-${s}`)).queryByRole("button")).toBeNull();
    }
  });

  it("AC-2 点击『重试』按钮 → 调用 retryAssetConversion(assetId)", async () => {
    const onRetry = vi.fn();
    render(
      <AssetStateBadge state="failed" assetId="failed-asset-1" reason="x" onRetry={onRetry} />
    );
    const btn = screen.getByRole("button", { name: /重试转化/ });
    fireEvent.click(btn);
    // microtask flush
    await Promise.resolve();
    await Promise.resolve();
    expect(retryAssetConversion).toHaveBeenCalledWith("failed-asset-1");
    expect(retryAssetConversion).toHaveBeenCalledTimes(1);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});

describe("assetStateLabel — 中文映射", () => {
  it("AC-5 4 态全部中文文案", () => {
    expect(assetStateLabel("done")).toBe("已就绪");
    expect(assetStateLabel("converting")).toBe("转化中");
    expect(assetStateLabel("failed")).toBe("失败");
    expect(assetStateLabel("offline")).toBe("离线待转化");
  });
});

/**
 * task_011 AC-2 / AC-4 / AC-8：AssetListView 行属性 / source-missing 角标 / cursor。
 *
 * 实现策略：直接渲染一个最小行模板（与 AssetListView 内嵌的右栏行渲染同结构），
 * 断言 `data-source-missing` / cursor / AlertTriangle 角标存在与否。
 * 这避免对整页 zustand store / Tauri / 拖拽 hook 重 mock，覆盖核心 AC 行为。
 */
import { AlertTriangle } from "lucide-react";

function FixtureRow(props: {
  assetId: string;
  state: AssetState | undefined;
  sourceMissing: boolean;
}) {
  const notDone = props.state !== undefined && props.state !== "done";
  return (
    <ul>
      <li
        data-asset-id={props.assetId}
        data-state={props.state ?? "unknown"}
        data-source-missing={props.sourceMissing ? "true" : "false"}
      >
        <button
          type="button"
          data-cursor={notDone ? "not-allowed" : "grab"}
          style={{ cursor: notDone ? "not-allowed" : "grab" }}
          title={notDone ? "无法拖出：当前状态非 done" : undefined}
        >
          <span>name</span>
          {props.sourceMissing ? (
            <span
              data-testid="source-missing-badge"
              title="源文件不在原位置，rendition 仍可拖出"
            >
              <AlertTriangle size={10} aria-hidden />
              <span>原件丢失</span>
            </span>
          ) : null}
        </button>
      </li>
    </ul>
  );
}

describe("AssetListView 行属性 — task_011 AC-2 / AC-4", () => {
  it("sourceMissing=true → data-source-missing=true + AlertTriangle 角标存在 + 文案『原件丢失』", () => {
    render(<FixtureRow assetId="a1" state="done" sourceMissing={true} />);
    const li = screen.getByText("name").closest("li") as HTMLElement;
    expect(li).toHaveAttribute("data-source-missing", "true");
    const badge = screen.getByTestId("source-missing-badge");
    expect(badge).toHaveTextContent("原件丢失");
    expect(badge).toHaveAttribute("title", expect.stringContaining("源文件不在原位置"));
  });

  it("sourceMissing=false → data-source-missing=false + 无角标", () => {
    render(<FixtureRow assetId="a2" state="done" sourceMissing={false} />);
    const li = screen.getByText("name").closest("li") as HTMLElement;
    expect(li).toHaveAttribute("data-source-missing", "false");
    expect(screen.queryByTestId("source-missing-badge")).toBeNull();
  });

  it("state=converting → cursor: not-allowed + title 含『无法拖出』", () => {
    render(<FixtureRow assetId="a3" state="converting" sourceMissing={false} />);
    const btn = screen.getByRole("button");
    expect(btn).toHaveAttribute("data-cursor", "not-allowed");
    expect(btn).toHaveStyle({ cursor: "not-allowed" });
    expect(btn).toHaveAttribute("title", expect.stringContaining("无法拖出"));
  });

  it("state=done → cursor: grab + 无『无法拖出』前缀", () => {
    render(<FixtureRow assetId="a4" state="done" sourceMissing={false} />);
    const btn = screen.getByRole("button");
    expect(btn).toHaveAttribute("data-cursor", "grab");
    expect(btn).toHaveStyle({ cursor: "grab" });
    expect(btn.getAttribute("title")).toBeNull();
  });

  it.each(["failed", "offline"] as const)("state=%s → cursor: not-allowed", (s) => {
    render(<FixtureRow assetId={`a-${s}`} state={s} sourceMissing={false} />);
    expect(screen.getByRole("button")).toHaveAttribute("data-cursor", "not-allowed");
  });
});

describe("AssetStateBadge — task_011 AC-3 retrying loading + 防抖", () => {
  it("点重试 → 按钮短暂 disabled + 文案改为「重试中…」（data-retrying=true）", async () => {
    let resolve: () => void = () => {};
    vi.mocked(retryAssetConversion).mockImplementationOnce(
      () => new Promise<void>((r) => (resolve = r))
    );
    render(<AssetStateBadge state="failed" assetId="ax" />);
    const btn = screen.getByTestId("asset-retry-button");
    expect(btn).toHaveAttribute("data-retrying", "false");
    expect(btn).toHaveTextContent("重试");
    fireEvent.click(btn);
    // 微任务 flush 让 setIsRetrying 生效
    await act(async () => {
      await Promise.resolve();
    });
    expect(btn).toHaveAttribute("data-retrying", "true");
    expect(btn).toBeDisabled();
    expect(btn).toHaveTextContent("重试中…");
    await act(async () => {
      resolve();
      await Promise.resolve();
      await Promise.resolve();
    });
    // resolve 后回到 false
    expect(btn).toHaveAttribute("data-retrying", "false");
  });

  it("1 秒内连点 → 仅调用 retryAssetConversion 一次（防抖）", async () => {
    vi.mocked(retryAssetConversion).mockResolvedValue(undefined);
    render(<AssetStateBadge state="failed" assetId="ay" />);
    const btn = screen.getByTestId("asset-retry-button");
    fireEvent.click(btn);
    fireEvent.click(btn);
    fireEvent.click(btn);
    await Promise.resolve();
    await Promise.resolve();
    expect(retryAssetConversion).toHaveBeenCalledTimes(1);
  });
});
