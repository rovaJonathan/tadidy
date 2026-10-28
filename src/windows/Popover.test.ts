import { mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Popover from "./Popover.svelte";

function recordCommands(): string[] {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
  });
  return calls;
}

describe("Popover", () => {
  it("hides its window when Esc is pressed", async () => {
    const calls = recordCommands();
    render(Popover);

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(calls).toEqual(["hide_window"]);
  });

  it("does nothing on other keys", async () => {
    const calls = recordCommands();
    render(Popover);

    await fireEvent.keyDown(window, { key: "a" });

    expect(calls).toEqual([]);
  });
});
