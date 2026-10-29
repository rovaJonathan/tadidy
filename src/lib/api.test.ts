import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";
import {
  appVersion,
  checkAccessibility,
  clearHistory,
  deleteClips,
  getAppIcon,
  getSettings,
  hideWindow,
  listClips,
  onClipsChanged,
  onPanelShown,
  onSettingsChanged,
  openAccessibilitySettings,
  setSettings,
} from "./api";
import type { ClipQuery } from "./bindings/ClipQuery";
import type { ClipsChanged } from "./bindings/ClipsChanged";
import type { ClipSummary } from "./bindings/ClipSummary";
import type { Page } from "./bindings/Page";
import type { PanelShown } from "./bindings/PanelShown";
import type { Settings } from "./bindings/Settings";
import type { SettingsPatch } from "./bindings/SettingsPatch";

const defaults: Settings = {
  historySize: 500,
  confirmBeforeDelete: true,
  launchAtLogin: false,
  quickPanelShortcut: "Cmd+Shift+V",
};

describe("appVersion", () => {
  it("invokes app_version and returns its value", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      return cmd === "app_version" ? "1.2.3" : undefined;
    });

    await expect(appVersion()).resolves.toBe("1.2.3");
    expect(calls).toEqual(["app_version"]);
  });
});

describe("hideWindow", () => {
  it("invokes hide_window", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
    });

    await hideWindow();

    expect(calls).toEqual(["hide_window"]);
  });
});

describe("checkAccessibility", () => {
  it.each([true, false])("invokes check_accessibility and returns %s", async (trusted) => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      return cmd === "check_accessibility" ? trusted : undefined;
    });

    await expect(checkAccessibility()).resolves.toBe(trusted);
    expect(calls).toEqual(["check_accessibility"]);
  });
});

describe("openAccessibilitySettings", () => {
  it("invokes open_accessibility_settings", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
    });

    await openAccessibilitySettings();

    expect(calls).toEqual(["open_accessibility_settings"]);
  });

  it("rejects with the core's error", async () => {
    mockIPC(() => {
      throw { code: "accessibility_settings_unavailable", message: "no" };
    });

    await expect(openAccessibilitySettings()).rejects.toEqual({
      code: "accessibility_settings_unavailable",
      message: "no",
    });
  });
});

describe("onPanelShown", () => {
  it("hands the payload of panel://shown to the callback", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const shown: PanelShown[] = [];
    await onPanelShown((payload) => shown.push(payload));

    await emit("panel://shown", { window: "panel" });
    await emit("panel://shown", { window: "popover" });

    expect(shown).toEqual([{ window: "panel" }, { window: "popover" }]);
  });

  it("stops calling the callback once unlistened", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const shown: PanelShown[] = [];
    const unlisten = await onPanelShown((payload) => shown.push(payload));

    unlisten();
    await emit("panel://shown", { window: "panel" });

    expect(shown).toEqual([]);
  });

  it("ignores other events", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const shown: PanelShown[] = [];
    await onPanelShown((payload) => shown.push(payload));

    await emit("clips://changed", { window: "panel" });

    expect(shown).toEqual([]);
  });
});

describe("getSettings", () => {
  it("invokes get_settings and returns the settings", async () => {
    const calls: string[] = [];
    mockIPC((cmd) => {
      calls.push(cmd);
      return cmd === "get_settings" ? defaults : undefined;
    });

    await expect(getSettings()).resolves.toEqual(defaults);
    expect(calls).toEqual(["get_settings"]);
  });
});

describe("setSettings", () => {
  it("invokes set_settings with the patch under the argument name Rust expects", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    const saved: Settings = { ...defaults, confirmBeforeDelete: false };
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return saved;
    });
    const patch: SettingsPatch = { confirmBeforeDelete: false };

    await expect(setSettings(patch)).resolves.toEqual(saved);
    expect(calls).toEqual([{ cmd: "set_settings", args: { patch } }]);
  });

  it("rejects with the core's invalid_settings error", async () => {
    mockIPC(() => {
      throw { code: "invalid_settings", message: "historySize must be 0 (unlimited) or at least 10" };
    });

    await expect(setSettings({ historySize: 5 })).rejects.toMatchObject({ code: "invalid_settings" });
  });
});

describe("onSettingsChanged", () => {
  it("hands the settings of settings://changed to the callback", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: Settings[] = [];
    await onSettingsChanged((settings) => seen.push(settings));

    await emit("settings://changed", { ...defaults, historySize: 10 });

    expect(seen).toEqual([{ ...defaults, historySize: 10 }]);
  });

  it("stops calling the callback once unlistened", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: Settings[] = [];
    const unlisten = await onSettingsChanged((settings) => seen.push(settings));

    unlisten();
    await emit("settings://changed", defaults);

    expect(seen).toEqual([]);
  });

  it("ignores other events", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: Settings[] = [];
    await onSettingsChanged((settings) => seen.push(settings));

    await emit("panel://shown", defaults);

    expect(seen).toEqual([]);
  });
});

const summary: ClipSummary = {
  id: "0192f3a4-7b1c-7d2e-8f00-123456789abc",
  kind: "text",
  preview: "Selected items",
  sourceBundle: "com.example.Editor",
  sourceName: "Editor",
  pinned: false,
  copyCount: 1,
  lastCopiedAt: 1_761_700_000_123,
};

describe("listClips", () => {
  it("invokes list_clips with the query under the argument name Rust expects", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    const page: Page<ClipSummary> = { items: [summary], nextCursor: null, total: 1 };
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return page;
    });
    const q: ClipQuery = { search: "sel", limit: 50 };

    await expect(listClips(q)).resolves.toEqual(page);
    expect(calls).toEqual([{ cmd: "list_clips", args: { q } }]);
  });

  it("rejects with the core's invalid_cursor error", async () => {
    mockIPC(() => {
      throw { code: "invalid_cursor", message: "The list position is not valid." };
    });

    await expect(listClips({ cursor: "nonsense" })).rejects.toMatchObject({ code: "invalid_cursor" });
  });
});

describe("deleteClips", () => {
  it("invokes delete_clips with the ids", async () => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
    });

    await expect(deleteClips(["a", "b"])).resolves.toBeUndefined();
    expect(calls).toEqual([{ cmd: "delete_clips", args: { ids: ["a", "b"] } }]);
  });
});

describe("clearHistory", () => {
  it.each([true, false])("invokes clear_history with keepPinned %s and returns the count", async (keepPinned) => {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return 8;
    });

    await expect(clearHistory(keepPinned)).resolves.toBe(8);
    expect(calls).toEqual([{ cmd: "clear_history", args: { keepPinned } }]);
  });
});

describe("getAppIcon", () => {
  it("resolves to the PNG bytes of the app's icon", async () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47]).buffer;
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return png;
    });

    await expect(getAppIcon("com.example.Editor")).resolves.toBe(png);
    expect(calls).toEqual([{ cmd: "get_app_icon", args: { bundleId: "com.example.Editor" } }]);
  });

  it("resolves to null when the app has no cached icon", async () => {
    mockIPC(() => {
      throw { code: "not_found", message: "No icon is cached for this app." };
    });

    await expect(getAppIcon("com.example.Unknown")).resolves.toBeNull();
  });

  it("still rejects with any other error", async () => {
    mockIPC(() => {
      throw { code: "storage_unavailable", message: "The history database is unavailable." };
    });

    await expect(getAppIcon("com.example.Editor")).rejects.toMatchObject({ code: "storage_unavailable" });
  });
});

describe("onClipsChanged", () => {
  it("hands the payload of clips://changed to the callback", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: ClipsChanged[] = [];
    await onClipsChanged((changed) => seen.push(changed));
    const removed: ClipsChanged = { added: null, updated: null, removed: ["a", "b"] };
    const reload: ClipsChanged = { added: null, updated: null, removed: null };

    await emit("clips://changed", removed);
    await emit("clips://changed", reload);

    expect(seen).toEqual([removed, reload]);
  });

  it("stops calling the callback once unlistened", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: ClipsChanged[] = [];
    const unlisten = await onClipsChanged((changed) => seen.push(changed));

    unlisten();
    await emit("clips://changed", { added: null, updated: null, removed: null });

    expect(seen).toEqual([]);
  });

  it("ignores other events", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const seen: ClipsChanged[] = [];
    await onClipsChanged((changed) => seen.push(changed));

    await emit("settings://changed", defaults);

    expect(seen).toEqual([]);
  });
});
