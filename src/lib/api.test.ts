import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";
import { appVersion, checkAccessibility, hideWindow, openAccessibilitySettings } from "./api";

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
