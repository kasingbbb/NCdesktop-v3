/**
 * RenameAssetModal — task_011 AC-6 / AC-8 单测。
 *
 * 覆盖：
 *  - 基础渲染（输入框 + 字节计数 + sanitize 提示）。
 *  - 字节计数随输入更新；超过 200 字节红色提示并禁用确认。
 *  - 校验失败（空 / 路径分隔符）显示中文错误。
 *  - 通过校验后点确认 → onSubmit(trimmedName) 触发。
 *  - 提交失败由调用方上抛 toast（这里通过 onSubmit 抛错 → 校验 toast 不属于本组件职责，只断
 *    言 onSubmit 被调用）。
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { RenameAssetModal } from "../RenameAssetModal";

describe("RenameAssetModal — 基础渲染", () => {
  it("渲染输入框 + 字节计数 + sanitize 提示", () => {
    render(
      <RenameAssetModal initialName="hello" onCancel={vi.fn()} onSubmit={vi.fn()} />
    );
    expect(screen.getByTestId("rename-asset-modal")).toBeInTheDocument();
    expect(screen.getByTestId("rename-asset-input")).toHaveValue("hello");
    // 字节计数
    expect(screen.getByTestId("rename-asset-byte-count")).toHaveTextContent("5 / 200 字节");
    // sanitize 简介存在
    expect(screen.getByText(/不能包含/)).toBeInTheDocument();
  });
});

describe("RenameAssetModal — 字节计数 / 校验", () => {
  it("UTF-8 中文字节正确计数（一汉字 3 字节）", () => {
    render(
      <RenameAssetModal initialName="你好" onCancel={vi.fn()} onSubmit={vi.fn()} />
    );
    expect(screen.getByTestId("rename-asset-byte-count")).toHaveTextContent("6 / 200 字节");
  });

  it("超过 200 字节 → 红色提示 + 确认按钮 disabled", () => {
    const longName = "a".repeat(201);
    render(
      <RenameAssetModal initialName="seed" onCancel={vi.fn()} onSubmit={vi.fn()} />
    );
    const input = screen.getByTestId("rename-asset-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: longName } });
    expect(screen.getByTestId("rename-asset-byte-count")).toHaveTextContent("201 / 200 字节");
    expect(screen.getByTestId("rename-asset-confirm")).toBeDisabled();
    expect(screen.getByTestId("rename-asset-validation")).toHaveTextContent("超过 200 字节");
  });

  it("名称含路径分隔符 → 显示中文校验错误 + 禁用确认", () => {
    render(<RenameAssetModal initialName="ok" onCancel={vi.fn()} onSubmit={vi.fn()} />);
    fireEvent.change(screen.getByTestId("rename-asset-input"), {
      target: { value: "a/b" },
    });
    expect(screen.getByTestId("rename-asset-validation")).toHaveTextContent(
      /不能包含 \/ 或 \\ 路径分隔符/
    );
    expect(screen.getByTestId("rename-asset-confirm")).toBeDisabled();
  });

  it("名称为空 → 禁用确认", () => {
    render(<RenameAssetModal initialName="ok" onCancel={vi.fn()} onSubmit={vi.fn()} />);
    fireEvent.change(screen.getByTestId("rename-asset-input"), {
      target: { value: "   " },
    });
    expect(screen.getByTestId("rename-asset-confirm")).toBeDisabled();
  });

  it("与初始名相同 → 禁用确认（避免无效提交）", () => {
    render(<RenameAssetModal initialName="ok" onCancel={vi.fn()} onSubmit={vi.fn()} />);
    expect(screen.getByTestId("rename-asset-confirm")).toBeDisabled();
  });
});

describe("RenameAssetModal — 提交 / 取消", () => {
  it("有效新名 → 点确认调 onSubmit(trimmed)", () => {
    const onSubmit = vi.fn();
    render(<RenameAssetModal initialName="old" onCancel={vi.fn()} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByTestId("rename-asset-input"), {
      target: { value: "  new name  " },
    });
    fireEvent.click(screen.getByTestId("rename-asset-confirm"));
    expect(onSubmit).toHaveBeenCalledWith("new name");
  });

  it("点取消 → 调 onCancel", () => {
    const onCancel = vi.fn();
    render(<RenameAssetModal initialName="x" onCancel={onCancel} onSubmit={vi.fn()} />);
    fireEvent.click(screen.getByTestId("rename-asset-cancel"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("busy=true → 输入框 + 按钮全部 disabled，确认显示「提交中…」", () => {
    render(
      <RenameAssetModal initialName="x" onCancel={vi.fn()} onSubmit={vi.fn()} busy />
    );
    expect(screen.getByTestId("rename-asset-input")).toBeDisabled();
    expect(screen.getByTestId("rename-asset-cancel")).toBeDisabled();
    expect(screen.getByTestId("rename-asset-confirm")).toBeDisabled();
    expect(screen.getByTestId("rename-asset-confirm")).toHaveTextContent("提交中…");
  });
});
