import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { defaultSettings, fakeSettingsCore } from "../../test/settings";
import { SettingsStore } from "./settings.svelte";

let store: SettingsStore;

beforeEach(() => {
  store = new SettingsStore();
});

afterEach(() => {
  store.destroy();
});

describe("SettingsStore", () => {
  it("has no settings before the core answers", async () => {
    fakeSettingsCore();

    const started = store.start();
    expect(store.settings).toBeNull();
    await started;

    expect(store.settings).toEqual(defaultSettings);
  });

  it("loads the settings with get_settings on start", async () => {
    const core = fakeSettingsCore({ settings: { historySize: 50, launchAtLogin: true } });

    await store.start();

    expect(core.commands).toContain("get_settings");
    expect(store.settings).toEqual({ ...defaultSettings, historySize: 50, launchAtLogin: true });
  });

  it("follows settings://changed", async () => {
    fakeSettingsCore();
    await store.start();

    await emit("settings://changed", { ...defaultSettings, confirmBeforeDelete: false });

    expect(store.settings?.confirmBeforeDelete).toBe(false);
  });

  it("sends a change as a patch and shows what the core returns", async () => {
    const core = fakeSettingsCore();
    await store.start();

    const saved = await store.patch({ historySize: 10 });

    expect(saved).toBe(true);
    expect(core.patches).toEqual([{ historySize: 10 }]);
    expect(store.settings?.historySize).toBe(10);
    expect(store.error).toBeNull();
  });

  it("keeps the settings and tells why when the core refuses a change", async () => {
    const core = fakeSettingsCore();
    core.refuse = { code: "login_item_failed", message: "Launch at login could not be changed." };
    await store.start();

    const saved = await store.patch({ launchAtLogin: true });

    expect(saved).toBe(false);
    expect(store.settings).toEqual(defaultSettings);
    expect(store.error).toEqual({ code: "login_item_failed", message: "Launch at login could not be changed." });
  });

  it("forgets the refusal once a change goes through", async () => {
    const core = fakeSettingsCore();
    core.refuse = { code: "shortcut_unavailable", message: "taken" };
    await store.start();
    await store.patch({ quickPanelShortcut: "Ctrl+Alt+V" });
    core.refuse = undefined;

    await store.patch({ confirmBeforeDelete: false });

    expect(store.error).toBeNull();
  });

  it("describes a failure that is not an AppError", async () => {
    fakeSettingsCore();
    await store.start();
    // The IPC layer itself failing (window closing, say): a plain Error, no code.
    mockIPC(
      (cmd) => {
        if (cmd === "set_settings") throw new Error("channel closed");
      },
      { shouldMockEvents: true },
    );

    const saved = await store.patch({ historySize: 10 });

    expect(saved).toBe(false);
    expect(store.error?.code).toBe("unknown");
  });

  it("stops following the core once destroyed", async () => {
    fakeSettingsCore();
    await store.start();
    store.destroy();

    await emit("settings://changed", { ...defaultSettings, historySize: 10 });

    expect(store.settings?.historySize).toBe(500);
  });

  it("releases a listener that was registered after destroy", async () => {
    fakeSettingsCore();
    const started = store.start();
    store.destroy();
    await started;

    await emit("settings://changed", { ...defaultSettings, historySize: 10 });

    expect(store.settings?.historySize).not.toBe(10);
  });
});
