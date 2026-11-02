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
    const { container } = render(StorageUnavailable, { onreset: vi.fn() });

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
    const { container } = render(StorageUnavailable, { onreset });

    await fireEvent.click(find(container, "#btn-reset-history"));

    expect(onreset).toHaveBeenCalledOnce();
  });
});
