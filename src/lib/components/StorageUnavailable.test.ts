import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import StorageUnavailable from "./StorageUnavailable.svelte";

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

describe("StorageUnavailable", () => {
  it("says the history can’t be opened, in the agreed words", () => {
    const { container } = render(StorageUnavailable, { onreset: vi.fn(), canReset: true });

    const state = find(container, "#storage-unavailable");
    expect(state.getAttribute("role")).toBe("status");
    expect(find(container, ".empty-title").textContent).toBe("Your history can’t be opened.");
    expect(find(container, ".empty-body").textContent).toBe(
      "Tadidy can’t unlock its encrypted database, so nothing is being recorded.",
    );
    expect(find(container, "#btn-reset-history").textContent?.trim()).toBe("Reset history…");
  });

  it("asks to reset when the button is clicked, and does nothing else", async () => {
    const onreset = vi.fn();
    const { container } = render(StorageUnavailable, { onreset, canReset: true });

    await fireEvent.click(find(container, "#btn-reset-history"));

    expect(onreset).toHaveBeenCalledOnce();
  });

  it("offers no reset when the failure may be temporary", () => {
    const { container } = render(StorageUnavailable, { onreset: vi.fn(), canReset: false });

    expect(container.querySelector("#btn-reset-history")).toBeNull();
    expect(find(container, ".empty-title").textContent).toBe("Your history can’t be opened.");
  });

  it("says nothing is wrong with the reset until it failed", () => {
    const { container } = render(StorageUnavailable, { onreset: vi.fn(), canReset: true });

    expect(container.querySelector("#storage-reset-error")).toBeNull();
  });

  it("says the reset did not work, and that nothing changed, after a failed reset", () => {
    const { container } = render(StorageUnavailable, { onreset: vi.fn(), canReset: true, failed: true });

    expect(find(container, "#storage-reset-error").textContent).toBe(
      "The reset didn’t work. Nothing was changed: try again.",
    );
    expect(find(container, "#btn-reset-history")).not.toBeNull();
  });

  it("does not say “try again” when there is no button to try with", () => {
    const { container } = render(StorageUnavailable, { onreset: vi.fn(), canReset: false, failed: true });

    expect(container.querySelector("#storage-reset-error")).toBeNull();
    expect(container.querySelector("#btn-reset-history")).toBeNull();
  });
});
