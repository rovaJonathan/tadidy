import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it, vi } from "vitest";
import ClearHistoryDialog from "./ClearHistoryDialog.svelte";

function mount(count = 487) {
  const onconfirm = vi.fn();
  const oncancel = vi.fn();
  const view = render(ClearHistoryDialog, { count, onconfirm, oncancel });
  return { ...view, onconfirm, oncancel };
}

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

describe("ClearHistoryDialog", () => {
  it("is the prototype's alert dialog, text for text", () => {
    const { container } = mount();

    const dialog = find(container, "#dialog-clear-history");
    expect(dialog.getAttribute("role")).toBe("alertdialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBe("dialog-clear-title");
    expect(dialog.getAttribute("aria-describedby")).toBe("dialog-clear-desc");
    expect(find(container, "#dialog-clear-title").textContent).toBe("Clear clipboard history?");
    expect(find(container, "#dialog-clear-desc").textContent).toBe(
      "487 clips will be permanently deleted from this Mac. This can’t be undone.",
    );
    expect(find(container, "#dialog-clear-count").textContent).toBe("487 clips");
    expect(find(container, "label.checkbox").textContent?.trim()).toBe("Keep pinned clips and snippets");
    expect(find(container, "#btn-clear-confirm").textContent?.trim()).toBe("Clear history");
    expect(find(container, "#btn-clear-confirm").classList).toContain("btn-destructive-text");
    expect(find(container, "#btn-clear-cancel").textContent?.trim()).toBe("Cancel");
    expect(find(container, "#btn-clear-cancel").classList).toContain("btn-primary");
  });

  it("keeps the pinned clips by default", () => {
    const { container } = mount();

    expect(find<HTMLInputElement>(container, "#chk-keep-pinned").checked).toBe(true);
  });

  it.each([
    [0, "0 clips"],
    [1, "1 clip"],
    [1234, "1,234 clips"],
  ])("counts %i clips as “%s”", (count, label) => {
    const { container } = mount(count);

    expect(find(container, "#dialog-clear-count").textContent).toBe(label);
  });

  it("puts the focus on the safe button, Cancel", async () => {
    const { container } = mount();
    await tick();

    expect(document.activeElement).toBe(find(container, "#btn-clear-cancel"));
  });

  it("confirms with the pinned clips kept when the box is left checked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-clear-confirm"));

    expect(onconfirm).toHaveBeenCalledExactlyOnceWith(true);
    expect(oncancel).not.toHaveBeenCalled();
  });

  it("confirms with nothing kept when the box was unchecked", async () => {
    const { container, onconfirm } = mount();

    await fireEvent.click(find(container, "#chk-keep-pinned"));
    await fireEvent.click(find(container, "#btn-clear-confirm"));

    expect(onconfirm).toHaveBeenCalledExactlyOnceWith(false);
  });

  it("cancels when Cancel is clicked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-clear-cancel"));

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("Esc cancels and stops there, so the window stays open", async () => {
    const { container, onconfirm, oncancel } = mount();
    const onWindowKey = vi.fn();
    window.addEventListener("keydown", onWindowKey);

    await fireEvent.keyDown(find(container, "#btn-clear-cancel"), { key: "Escape" });
    window.removeEventListener("keydown", onWindowKey);

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
    expect(onWindowKey).not.toHaveBeenCalled();
  });

  it("Return outside a button cancels: it never clears the history", async () => {
    const { container, onconfirm, oncancel } = mount();
    const box = find(container, "#chk-keep-pinned");
    box.focus();

    await fireEvent.keyDown(box, { key: "Enter" });

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("leaves Return to the focused button", async () => {
    const { container, onconfirm, oncancel } = mount();
    const confirm = find(container, "#btn-clear-confirm");
    confirm.focus();

    const notPrevented = await fireEvent.keyDown(confirm, { key: "Enter" });

    expect(notPrevented).toBe(true);
    expect(onconfirm).not.toHaveBeenCalled();
    expect(oncancel).not.toHaveBeenCalled();
  });

  it("keeps Tab inside the dialog, wrapping from the last control to the first", async () => {
    const { container } = mount();
    const cancel = find(container, "#btn-clear-cancel");
    cancel.focus();

    await fireEvent.keyDown(cancel, { key: "Tab" });
    expect(document.activeElement).toBe(find(container, "#chk-keep-pinned"));

    await fireEvent.keyDown(document.activeElement ?? cancel, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);
  });
});
