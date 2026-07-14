import React, { useState } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
import {
  CursorRoleCard,
  AppToast,
  RoleAssignmentDialog,
  UnassignedCursorCard,
  type CursorFile,
  type CursorRole,
  type SkinPackage,
} from "./main";

const ROLE_NAMES = [
  ["Normal Select", "Arrow"],
  ["Help Select", "Help"],
  ["Working In Background", "AppStarting"],
  ["Busy", "Wait"],
  ["Precision Select", "Crosshair"],
  ["Text Select", "IBeam"],
  ["Handwriting", "NWPen"],
  ["Unavailable", "No"],
  ["Vertical Resize", "SizeNS"],
  ["Horizontal Resize", "SizeWE"],
  ["Diagonal Resize 1", "SizeNWSE"],
  ["Diagonal Resize 2", "SizeNESW"],
  ["Move", "SizeAll"],
  ["Alternate Select", "UpArrow"],
  ["Link Select", "Hand"],
] as const;

describe("cursor role interactions", () => {
  it("only opens replacement from the explicit action button and blocks interaction while busy", async () => {
    const user = userEvent.setup();
    const onReplace = vi.fn();
    const { rerender } = render(
      <CursorRoleCard
        cursor={role("Normal Select", "Arrow", true)}
        disabled={false}
        processing={false}
        onReplace={onReplace}
      />
    );
    const action = screen.getByRole("button", { name: "替换 Normal Select 光标文件" });

    expect(screen.getByText("替换文件")).toBeInTheDocument();
    await user.click(screen.getByText("Normal Select"));
    expect(onReplace).not.toHaveBeenCalled();
    await user.click(action);
    expect(onReplace).toHaveBeenCalledTimes(1);
    action.focus();
    expect(action).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onReplace).toHaveBeenCalledTimes(2);

    rerender(
      <CursorRoleCard
        cursor={role("Normal Select", "Arrow", true)}
        disabled
        processing
        onReplace={onReplace}
      />
    );
    expect(action).toBeDisabled();
    expect(screen.getByText("处理中...")).toBeInTheDocument();
    await user.click(action);
    expect(onReplace).toHaveBeenCalledTimes(2);
  });

  it("only assigns from the explicit action button and disables duplicate assignment while busy", async () => {
    const user = userEvent.setup();
    const onAssign = vi.fn();
    const file = unassignedFile();
    const { rerender } = render(
      <UnassignedCursorCard file={file} disabled={false} processing={false} onAssign={onAssign} />
    );
    const action = screen.getByRole("button", { name: `将 ${file.fileName} 分配到光标角色` });

    expect(screen.getByText("分配到角色")).toBeInTheDocument();
    await user.click(screen.getByText(file.fileName));
    expect(onAssign).not.toHaveBeenCalled();
    await user.click(action);
    expect(onAssign).toHaveBeenCalledTimes(1);
    action.focus();
    expect(action).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onAssign).toHaveBeenCalledTimes(2);

    rerender(<UnassignedCursorCard file={file} disabled processing onAssign={onAssign} />);
    expect(action).toBeDisabled();
    expect(screen.getByText("处理中...")).toBeInTheDocument();
    await user.click(action);
    expect(onAssign).toHaveBeenCalledTimes(2);
  });
});

describe("operation toast", () => {
  it("auto-dismisses cursor edit success messages but keeps errors visible", () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    const { rerender } = render(
      <AppToast message="Normal Select 已替换。" onDismiss={onDismiss} />
    );

    act(() => vi.advanceTimersByTime(3199));
    expect(onDismiss).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));
    expect(onDismiss).toHaveBeenCalledTimes(1);

    rerender(<AppToast message="替换失败：文件已损坏" onDismiss={onDismiss} />);
    act(() => vi.advanceTimersByTime(5000));
    expect(onDismiss).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });
});

describe("role assignment dialog", () => {
  it("lists all roles, requires a selection, confirms with Enter, and traps Tab", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    function Harness() {
      const [selectedRole, setSelectedRole] = useState<string | null>(null);
      return (
        <RoleAssignmentDialog
          skin={skin()}
          file={unassignedFile()}
          selectedRoleKey={selectedRole}
          busy={false}
          onSelectRole={setSelectedRole}
          onCancel={onCancel}
          onConfirm={onConfirm}
        />
      );
    }

    render(<Harness />);
    expect(screen.getAllByRole("radio")).toHaveLength(15);
    const confirm = screen.getByRole("button", { name: "确认替换" });
    expect(confirm).toBeDisabled();

    await user.click(screen.getByRole("radio", { name: /Normal Select/ }));
    expect(confirm).toBeEnabled();
    confirm.focus();
    await user.keyboard("{Enter}");
    expect(onConfirm).toHaveBeenCalledTimes(1);

    confirm.focus();
    await user.tab();
    expect(screen.getByRole("button", { name: "关闭角色选择" })).toHaveFocus();
  });

  it("closes with Escape or the close button and cannot close while busy", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const props = {
      skin: skin(),
      file: unassignedFile(),
      selectedRoleKey: "Arrow",
      onSelectRole: vi.fn(),
      onCancel,
      onConfirm: vi.fn(),
    };
    const { rerender } = render(<RoleAssignmentDialog {...props} busy={false} />);

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "关闭角色选择" }));
    expect(onCancel).toHaveBeenCalledTimes(2);

    rerender(<RoleAssignmentDialog {...props} busy />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "正在分配..." })).toBeDisabled();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(screen.getByRole("dialog")).toHaveFocus();
  });

  it("keeps hover and focus styles on both card types without layout-driven selectors", () => {
    const css = readFileSync(resolve(process.cwd(), "src", "styles.css"), "utf8");
    expect(css).toContain(".cursor-card:hover:not(.disabled)");
    expect(css).toContain(".cursor-card:focus-within");
    expect(css).toContain(".unassigned-item:hover:not(.disabled)");
    expect(css).toContain(".unassigned-item:focus-within");
    expect(css).toContain("grid-template-columns: 40px minmax(0, 1fr) 108px");
  });
});

function role(name: string, windowsKey: string, exists = false): CursorRole {
  return {
    role: name,
    windowsKey,
    filePath: exists ? `C:\\skin\\${windowsKey}.cur` : null,
    fileName: exists ? `${windowsKey}.cur` : null,
    previewPath: null,
    previewDataUrl: null,
    type: exists ? "cur" : null,
    exists,
  };
}

function unassignedFile(): CursorFile {
  return {
    filePath: "C:\\skin\\未分配 #1.cur",
    fileName: "未分配 #1.cur",
    previewPath: null,
    previewDataUrl: null,
    type: "cur",
    exists: true,
  };
}

function skin(): SkinPackage {
  return {
    id: "test-skin",
    name: "Test Skin",
    sourcePath: "C:\\source",
    storagePath: "C:\\skin",
    importedAt: "0",
    hasInf: true,
    isComplete: false,
    cursorCount: 1,
    isApplied: false,
    importNote: null,
    roles: ROLE_NAMES.map(([name, key], index) => role(name, key, index === 0)),
    unassignedFiles: [unassignedFile()],
  };
}
