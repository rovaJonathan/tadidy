import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it, vi } from "vitest";
import ResetHistoryDialog from "./ResetHistoryDialog.svelte";

function mount() {
  const onconfirm = vi.fn();
  const oncancel = vi.fn();
  const view = render(ResetHistoryDialog, { onconfirm, oncancel });
  return { ...view, onconfirm, oncancel };
}

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

describe("ResetHistoryDialog", () => {
  it("states that the history can’t be recovered, in the agreed words", () => {
    const { container } = mount();

    const dialog = find(container, "#dialog-reset-history");
    expect(dialog.getAttribute("role")).toBe("alertdialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBe("dialog-reset-title");
    expect(dialog.getAttribute("aria-describedby")).toBe("dialog-reset-desc");
    expect(find(container, "#dialog-reset-title").textContent).toBe("Reset clipboard history?");
    expect(find(container, "#dialog-reset-desc").textContent).toBe(
      "The current history can’t be recovered. Tadidy will start a new, empty one. The old encrypted file is kept in the app’s data folder as tadidy.db.bak.",
    );
    expect(find(container, "#btn-reset-confirm").textContent?.trim()).toBe("Reset history");
    expect(find(container, "#btn-reset-confirm").classList).toContain("btn-destructive-text");
    expect(find(container, "#btn-reset-cancel").textContent?.trim()).toBe("Cancel");
    expect(find(container, "#btn-reset-cancel").classList).toContain("btn-primary");
  });

  it("puts the focus on the safe button, Cancel", async () => {
    const { container } = mount();
    await tick();

    expect(document.activeElement).toBe(find(container, "#btn-reset-cancel"));
  });

  it("confirms when Reset history is clicked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-reset-confirm"));

    expect(onconfirm).toHaveBeenCalledOnce();
    expect(oncancel).not.toHaveBeenCalled();
  });

  it("cancels when Cancel is clicked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-reset-cancel"));

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("cancels on Escape", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.keyDown(find(container, "#dialog-reset-history"), { key: "Escape" });

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("treats Return outside a button as Cancel, never as a reset", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.keyDown(find(container, "#dialog-reset-history"), { key: "Enter" });

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("keeps Tab inside the dialog, between its two buttons", async () => {
    const { container } = mount();
    await tick();

    await fireEvent.keyDown(find(container, "#btn-reset-cancel"), { key: "Tab" });
    expect(document.activeElement).toBe(find(container, "#btn-reset-confirm"));

    await fireEvent.keyDown(find(container, "#btn-reset-confirm"), { key: "Tab" });
    expect(document.activeElement).toBe(find(container, "#btn-reset-cancel"));
  });

  it("keeps the keys it handles from reaching the window", async () => {
    const { container } = mount();
    const reachedWindow = vi.fn();
    window.addEventListener("keydown", reachedWindow);

    await fireEvent.keyDown(find(container, "#dialog-reset-history"), { key: "ArrowDown" });

    window.removeEventListener("keydown", reachedWindow);
    expect(reachedWindow).not.toHaveBeenCalled();
  });
});
