import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it, vi } from "vitest";
import DeleteClipDialog from "./DeleteClipDialog.svelte";

function mount() {
  const onconfirm = vi.fn();
  const oncancel = vi.fn();
  const view = render(DeleteClipDialog, { onconfirm, oncancel });
  return { ...view, onconfirm, oncancel };
}

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

describe("DeleteClipDialog", () => {
  it("is the prototype's alert dialog, text for text", () => {
    const { container } = mount();

    const dialog = find(container, "#dialog-delete-clip");
    expect(dialog.getAttribute("role")).toBe("alertdialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(find(container, "#dialog-delete-title").textContent).toBe("Delete this clip?");
    expect(find(container, "#dialog-delete-desc").textContent).toBe("This can’t be undone.");
    expect(dialog.getAttribute("aria-labelledby")).toBe("dialog-delete-title");
    expect(dialog.getAttribute("aria-describedby")).toBe("dialog-delete-desc");
    expect(find(container, "label.checkbox").textContent?.trim()).toBe("Don’t ask again");
    expect(find<HTMLInputElement>(container, "#chk-dont-ask-delete").checked).toBe(false);
    expect(find(container, "#btn-delete-confirm").textContent?.trim()).toBe("Delete");
    expect(find(container, "#btn-delete-confirm").classList).toContain("btn-destructive");
    expect(find(container, "#btn-delete-cancel").textContent?.trim()).toBe("Cancel");
  });

  it("puts the focus on the default button, Delete", async () => {
    const { container } = mount();
    await tick();

    expect(document.activeElement).toBe(find(container, "#btn-delete-confirm"));
  });

  it("confirms with the box unchecked when Delete is clicked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-delete-confirm"));

    expect(onconfirm).toHaveBeenCalledExactlyOnceWith(false);
    expect(oncancel).not.toHaveBeenCalled();
  });

  it("confirms with the box checked when Don’t ask again was ticked", async () => {
    const { container, onconfirm } = mount();

    await fireEvent.click(find(container, "#chk-dont-ask-delete"));
    await fireEvent.click(find(container, "#btn-delete-confirm"));

    expect(onconfirm).toHaveBeenCalledExactlyOnceWith(true);
  });

  it("cancels when Cancel is clicked", async () => {
    const { container, onconfirm, oncancel } = mount();

    await fireEvent.click(find(container, "#btn-delete-cancel"));

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onconfirm).not.toHaveBeenCalled();
  });

  it("Return confirms, wherever the focus is inside the dialog but on Cancel", async () => {
    const { container, onconfirm } = mount();
    const box = find(container, "#chk-dont-ask-delete");
    box.focus();

    await fireEvent.keyDown(box, { key: "Enter" });

    expect(onconfirm).toHaveBeenCalledExactlyOnceWith(false);
  });

  it("leaves Return to the focused button (Cancel is not confirmed by Return)", async () => {
    const { container, onconfirm } = mount();
    const cancel = find(container, "#btn-delete-cancel");
    cancel.focus();

    const notPrevented = await fireEvent.keyDown(cancel, { key: "Enter" });

    expect(onconfirm).not.toHaveBeenCalled();
    expect(notPrevented).toBe(true);
  });

  it("Esc cancels and stops there, so the window stays open", async () => {
    const { container, oncancel } = mount();
    const onWindowKey = vi.fn();
    window.addEventListener("keydown", onWindowKey);

    await fireEvent.keyDown(find(container, "#btn-delete-confirm"), { key: "Escape" });
    window.removeEventListener("keydown", onWindowKey);

    expect(oncancel).toHaveBeenCalledOnce();
    expect(onWindowKey).not.toHaveBeenCalled();
  });

  it("keeps Tab inside the dialog, wrapping from the last control to the first", async () => {
    const { container } = mount();
    const cancel = find(container, "#btn-delete-cancel");
    cancel.focus();

    await fireEvent.keyDown(cancel, { key: "Tab" });
    expect(document.activeElement).toBe(find(container, "#chk-dont-ask-delete"));

    await fireEvent.keyDown(document.activeElement ?? cancel, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);
  });
});
