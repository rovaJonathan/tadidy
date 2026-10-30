import { emit } from "@tauri-apps/api/event";
import { fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { describeHost, find, hostCore, settle } from "../test/host";
import Popover from "./Popover.svelte";

describeHost("Popover", Popover, "popover", "Tadidy clipboard history");

/** The popover with its fake core, and the tray menu's Clear history… chosen. */
async function askToClearHistory() {
  const core = hostCore();
  const view = render(Popover);
  await settle();
  const field = find<HTMLInputElement>(view.container, "#search-input");
  await emit("menu://clear-history");
  await settle();
  return { ...view, core, field };
}

describe("Popover, Clear history… from the tray menu", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows no dialog until the menu asks for it", async () => {
    hostCore();
    const view = render(Popover);
    await settle();

    expect(view.container.querySelector("#dialog-clear-history")).toBeNull();
  });

  it("opens the confirmation with the size of the history and the box checked", async () => {
    const { container, core } = await askToClearHistory();

    expect(find(container, "#dialog-clear-history")).toBeVisible();
    expect(find(container, "#dialog-clear-count")).toHaveTextContent("4 clips");
    expect(find<HTMLInputElement>(container, "#chk-keep-pinned").checked).toBe(true);
    expect(find(container, "#btn-clear-cancel")).toHaveFocus();
    expect(core.argsOf("list_clips")).toContainEqual({ q: { limit: 1 } });
    expect(core.commands()).not.toContain("clear_history");
  });

  it("clears the history, pinned clips kept, when Clear history is confirmed as it is", async () => {
    const { container, core, field } = await askToClearHistory();

    await fireEvent.click(find(container, "#btn-clear-confirm"));
    await settle();

    expect(core.argsOf("clear_history")).toEqual([{ keepPinned: true }]);
    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(field).toHaveFocus();
  });

  it("clears everything when the box was unchecked", async () => {
    const { container, core } = await askToClearHistory();

    await fireEvent.click(find(container, "#chk-keep-pinned"));
    await fireEvent.click(find(container, "#btn-clear-confirm"));
    await settle();

    expect(core.argsOf("clear_history")).toEqual([{ keepPinned: false }]);
  });

  it("clears nothing on Cancel, and the popover stays open", async () => {
    const { container, core, field } = await askToClearHistory();

    await fireEvent.click(find(container, "#btn-clear-cancel"));
    await settle();

    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(core.commands()).not.toContain("clear_history");
    expect(core.commands()).not.toContain("hide_window");
    expect(field).toHaveFocus();
  });

  it("clears nothing on Esc, wherever the focus is, and the popover stays open", async () => {
    const { container, core, field } = await askToClearHistory();
    field.focus();

    await fireEvent.keyDown(field, { key: "Escape" });
    await settle();

    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(core.commands()).not.toContain("clear_history");
    expect(core.commands()).not.toContain("hide_window");
  });

  it("keeps the clip browser's keys out of the dialog", async () => {
    const { container, core, field } = await askToClearHistory();
    field.focus();

    await fireEvent.keyDown(field, { key: "Enter" });
    await settle();

    // Return outside a button cancels; it neither pastes the selected clip nor clears.
    expect(core.commands()).not.toContain("paste_clip");
    expect(core.commands()).not.toContain("clear_history");
    expect(container.querySelector("#dialog-clear-history")).toBeNull();
  });

  it("closes with the window when the popover loses focus: nothing is cleared", async () => {
    const { container, core } = await askToClearHistory();

    await fireEvent.blur(window);
    await settle();

    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(core.commands()).not.toContain("clear_history");
  });

  it("is closed when the popover is shown again", async () => {
    const { container, core } = await askToClearHistory();

    await emit("panel://shown", { window: "popover" });
    await settle();

    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(core.commands()).not.toContain("clear_history");
  });

  it("shows no dialog, and logs only the code, when the history size cannot be read", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const core = hostCore();
    const { container } = render(Popover);
    await settle();
    core.listFails = { code: "storage", message: "The database is busy." };

    await emit("menu://clear-history");
    await settle();

    expect(container.querySelector("#dialog-clear-history")).toBeNull();
    expect(error).toHaveBeenCalledWith(expect.stringContaining("storage"));
    expect(JSON.stringify(error.mock.calls)).not.toContain("busy");
  });
});
