import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it } from "vitest";
import { fakeSettingsCore } from "../test/settings";
import Settings from "./Settings.svelte";

/** Lets the IPC answers, event listeners and DOM updates run. */
async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
}

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

async function open(core = fakeSettingsCore()) {
  const view = render(Settings);
  await settle();
  return { ...view, core };
}

describe("Settings window", () => {
  it("is the prototype's sidebar with the General and Shortcuts tabs only", async () => {
    const { container } = await open();

    const list = find(container, '[role="tablist"]');
    expect(list.getAttribute("aria-label")).toBe("Preferences sections");
    expect(list.getAttribute("aria-orientation")).toBe("vertical");
    const tabs = [...container.querySelectorAll('[role="tab"]')];
    expect(tabs.map((tab) => [tab.id, tab.textContent?.trim()])).toEqual([
      ["prefs-tab-general", "General"],
      ["prefs-tab-shortcuts", "Shortcuts"],
    ]);
    expect(tabs.map((tab) => tab.getAttribute("aria-controls"))).toEqual(["prefs-general", "prefs-shortcuts"]);
  });

  it("holds only #prefs-general and #prefs-shortcuts in the DOM", async () => {
    const { container } = await open();

    expect([...container.querySelectorAll(".prefs-panel")].map((panel) => panel.id)).toEqual([
      "prefs-general",
      "prefs-shortcuts",
    ]);
    for (const id of ["prefs-capture", "prefs-appearance", "prefs-storage", "prefs-about"]) {
      expect(container.querySelector(`#${id}`), id).toBeNull();
      expect(container.querySelector(`#${id.replace("prefs-", "prefs-tab-")}`), id).toBeNull();
    }
  });

  it("opens on General, with the title of the tab", async () => {
    const { container } = await open();

    expect(find(container, "#prefs-tab-general").getAttribute("aria-selected")).toBe("true");
    expect(find(container, "#prefs-tab-shortcuts").getAttribute("aria-selected")).toBe("false");
    expect(find(container, "#prefs-title").textContent).toBe("General");
    expect(find(container, "#prefs-general").hasAttribute("hidden")).toBe(false);
    expect(find(container, "#prefs-shortcuts").hasAttribute("hidden")).toBe(true);
  });

  it("shows the stored settings in the General tab", async () => {
    const { container } = await open(fakeSettingsCore({ settings: { launchAtLogin: true, historySize: 50 } }));

    expect(find<HTMLInputElement>(container, "#pref-launch-at-login").checked).toBe(true);
    expect(find<HTMLSelectElement>(container, "#pref-history-size").value).toBe("50");
  });

  it("switches to Shortcuts on click", async () => {
    const { container } = await open();

    await fireEvent.click(find(container, "#prefs-tab-shortcuts"));

    expect(find(container, "#prefs-tab-shortcuts").getAttribute("aria-selected")).toBe("true");
    expect(find(container, "#prefs-tab-general").getAttribute("aria-selected")).toBe("false");
    expect(find(container, "#prefs-title").textContent).toBe("Shortcuts");
    expect(find(container, "#prefs-shortcuts").hasAttribute("hidden")).toBe(false);
    expect(find(container, "#prefs-general").hasAttribute("hidden")).toBe(true);
  });

  it("keeps one tab in the tab order (roving tabindex)", async () => {
    const { container } = await open();
    const tabindexes = () => ["general", "shortcuts"].map((id) => find(container, `#prefs-tab-${id}`).tabIndex);

    expect(tabindexes()).toEqual([0, -1]);
    await fireEvent.click(find(container, "#prefs-tab-shortcuts"));

    expect(tabindexes()).toEqual([-1, 0]);
  });

  it("moves between the tabs with ↑ and ↓, wrapping around, and focuses the new tab", async () => {
    const { container } = await open();
    const list = find(container, '[role="tablist"]');
    const selected = () => (find(container, "#prefs-title").textContent ?? "").toString();

    await fireEvent.keyDown(list, { key: "ArrowDown" });
    expect(selected()).toBe("Shortcuts");
    expect(document.activeElement?.id).toBe("prefs-tab-shortcuts");

    await fireEvent.keyDown(list, { key: "ArrowDown" });
    expect(selected()).toBe("General");

    await fireEvent.keyDown(list, { key: "ArrowUp" });
    expect(selected()).toBe("Shortcuts");
  });

  it("ignores the other keys on the tab list", async () => {
    const { container } = await open();

    await fireEvent.keyDown(find(container, '[role="tablist"]'), { key: "a" });

    expect(find(container, "#prefs-title").textContent).toBe("General");
  });

  it("stays usable, with empty panels, when the settings cannot be read", async () => {
    const core = fakeSettingsCore();
    core.unreadable = { code: "storage_unavailable", message: "The history database is unavailable." };

    const { container } = await open(core);

    expect(find(container, "#prefs-general")).not.toBeNull();
    expect(container.querySelector("#pref-launch-at-login")).toBeNull();
    await fireEvent.click(find(container, "#prefs-tab-shortcuts"));
    expect(find(container, "#prefs-title").textContent).toBe("Shortcuts");
  });
});
