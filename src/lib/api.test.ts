import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";
import { appVersion, checkAccessibility, hideWindow, onPanelShown, openAccessibilitySettings } from "./api";
import type { PanelShown } from "./bindings/PanelShown";

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
