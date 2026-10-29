import { mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Panel from "./Panel.svelte";

function recordCommands(): string[] {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
  });
  return calls;
}

describe("Panel", () => {
  it("hides its window when Esc is pressed", async () => {
    const calls = recordCommands();
    render(Panel);

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(calls).toEqual(["hide_window"]);
  });

  it("does nothing on other keys", async () => {
    const calls = recordCommands();
    render(Panel);

    await fireEvent.keyDown(window, { key: "a" });

    expect(calls).toEqual([]);
  });

  it("is the Quick Panel view of the prototype", () => {
    recordCommands();
    const { container } = render(Panel);

    expect(container.querySelector("#quick-panel")).toHaveAttribute("aria-label", "Tadidy Quick Panel");
  });
});
