import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { fakeSettingsCore } from "../../../test/settings";
import { SettingsStore } from "../../stores/settings.svelte";
import GeneralTab from "./GeneralTab.svelte";

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

const stores: SettingsStore[] = [];

/** The tab over a store that has loaded the fake core's settings. */
async function open(core = fakeSettingsCore()) {
  const store = new SettingsStore();
  stores.push(store);
  await store.start();
  const view = render(GeneralTab, { store });
  await settle();
  return { ...view, core, store };
}

afterEach(() => {
  for (const store of stores.splice(0)) store.destroy();
});

describe("GeneralTab", () => {
  it("is the prototype's General panel, text for text", async () => {
    const { container } = await open();

    const panel = find(container, "#prefs-general");
    expect(panel.getAttribute("role")).toBe("tabpanel");
    expect(panel.getAttribute("aria-labelledby")).toBe("prefs-tab-general");
    expect(find(container, 'label[for="pref-launch-at-login"]').textContent).toBe("Launch at login");
    expect(find(container, 'label[for="pref-history-size"]').textContent).toBe("History size");
    expect(panel.textContent).toContain("Pinned clips and snippets don’t count toward the limit.");
    expect(find(container, 'label[for="pref-confirm-delete"]').textContent).toBe("Confirm before delete");
    expect(find(container, "#pref-launch-at-login").getAttribute("role")).toBe("switch");
    expect(find(container, "#pref-confirm-delete").getAttribute("role")).toBe("switch");
  });

  it("offers the prototype's history sizes, and Unlimited sends 0", async () => {
    const { container } = await open();

    const options = [...find<HTMLSelectElement>(container, "#pref-history-size").options];

    expect(options.map((option) => [option.value, option.textContent])).toEqual([
      ["10", "10 clips"],
      ["50", "50 clips"],
      ["100", "100 clips"],
      ["250", "250 clips"],
      ["500", "500 clips"],
      ["1000", "1,000 clips"],
      ["5000", "5,000 clips"],
      ["0", "Unlimited"],
    ]);
  });

  it("renders none of the phase 2 rows", async () => {
    const { container } = await open();

    for (const id of ["pref-auto-expire", "pref-pinned-on-top", "pref-sound-on-copy"]) {
      expect(container.querySelector(`#${id}`), id).toBeNull();
    }
  });

  it("shows the stored settings", async () => {
    const { container } = await open(
      fakeSettingsCore({ settings: { launchAtLogin: true, historySize: 100, confirmBeforeDelete: false } }),
    );

    expect(find<HTMLInputElement>(container, "#pref-launch-at-login").checked).toBe(true);
    expect(find<HTMLSelectElement>(container, "#pref-history-size").value).toBe("100");
    expect(find<HTMLInputElement>(container, "#pref-confirm-delete").checked).toBe(false);
  });

  it("checking Launch at login sends launchAtLogin: true", async () => {
    const { container, core } = await open();

    await fireEvent.click(find(container, "#pref-launch-at-login"));
    await settle();

    expect(core.patches).toEqual([{ launchAtLogin: true }]);
    expect(find<HTMLInputElement>(container, "#pref-launch-at-login").checked).toBe(true);
  });

  it("unchecks the box and shows the error when the login item cannot be changed", async () => {
    const core = fakeSettingsCore();
    core.refuse = { code: "login_item_failed", message: "Launch at login could not be changed." };
    const { container } = await open(core);

    await fireEvent.click(find(container, "#pref-launch-at-login"));
    await settle();

    expect(find<HTMLInputElement>(container, "#pref-launch-at-login").checked).toBe(false);
    const error = find(container, "#pref-launch-at-login-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error.textContent).toContain("Launch at login could not be changed.");
  });

  it("shows no error while nothing failed", async () => {
    const { container } = await open();

    expect(container.querySelector("#pref-launch-at-login-error")).toBeNull();
  });

  it("choosing 10 clips sends historySize: 10", async () => {
    const { container, core } = await open();

    await fireEvent.change(find(container, "#pref-history-size"), { target: { value: "10" } });
    await settle();

    expect(core.patches).toEqual([{ historySize: 10 }]);
  });

  it("choosing Unlimited sends historySize: 0", async () => {
    const { container, core } = await open();

    await fireEvent.change(find(container, "#pref-history-size"), { target: { value: "0" } });
    await settle();

    expect(core.patches).toEqual([{ historySize: 0 }]);
  });

  it("unchecking Confirm before delete sends confirmBeforeDelete: false", async () => {
    const { container, core } = await open();

    await fireEvent.click(find(container, "#pref-confirm-delete"));
    await settle();

    expect(core.patches).toEqual([{ confirmBeforeDelete: false }]);
  });

  it("follows a change announced by the core", async () => {
    const { container, store } = await open();

    const { emit } = await import("@tauri-apps/api/event");
    await emit("settings://changed", { ...store.settings, historySize: 50 });
    await settle();

    expect(find<HTMLSelectElement>(container, "#pref-history-size").value).toBe("50");
  });
});
