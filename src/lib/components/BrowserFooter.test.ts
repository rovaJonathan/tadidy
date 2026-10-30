import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import BrowserFooter from "./BrowserFooter.svelte";

describe("BrowserFooter", () => {
  it("shows the count it is given", () => {
    const { container } = render(BrowserFooter, { count: "487 clips" });

    expect(container.querySelector("#clip-count")).toHaveTextContent("487 clips");
  });

  it("hints only at the actions of the MVP", () => {
    const { container } = render(BrowserFooter, { count: "1 clip" });

    const hints = container.querySelector(".footer-hints");
    expect(hints).toHaveAttribute("aria-hidden", "true");
    expect([...(hints?.children ?? [])].map((hint) => hint.textContent)).toEqual(["↩ Paste", "⌥↩ Copy"]);
  });

  it("has a Preferences button that tells the host", async () => {
    const onpreferences = vi.fn();
    const { container } = render(BrowserFooter, { count: "1 clip", onpreferences });

    const button = container.querySelector("#btn-preferences");
    expect(button).toHaveAttribute("aria-label", "Preferences");
    expect(button).toHaveAttribute("title", "Preferences (⌘,)");
    await fireEvent.click(button as Element);

    expect(onpreferences).toHaveBeenCalledOnce();
  });
});
