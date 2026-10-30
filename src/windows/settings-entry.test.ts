import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { defaultSettings } from "../test/settings";

function stubSystemTheme(dark: boolean): { change: () => void; setDark: (value: boolean) => void } {
  let listener: () => void = () => undefined;
  const media = {
    matches: dark,
    addEventListener: (_type: string, callback: () => void) => {
      listener = callback;
    },
  };
  vi.stubGlobal("matchMedia", () => media);
  return {
    change: () => listener(),
    setDark: (value) => {
      media.matches = value;
    },
  };
}

describe("settings entry", () => {
  beforeEach(() => {
    vi.resetModules();
    document.body.innerHTML = '<div id="app"></div>';
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("mounts the Preferences window into #app", async () => {
    stubSystemTheme(false);
    mockIPC((cmd) => (cmd === "get_settings" ? defaultSettings : undefined), { shouldMockEvents: true });

    await import("./settings");

    expect(document.querySelector("#app #preferences")).not.toBeNull();
  });

  it("follows the system theme, also when it changes", async () => {
    const system = stubSystemTheme(true);
    mockIPC((cmd) => (cmd === "get_settings" ? defaultSettings : undefined), { shouldMockEvents: true });

    await import("./settings");
    expect(document.documentElement.dataset.theme).toBe("dark");

    system.setDark(false);
    system.change();
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("fails loudly when #app is missing", async () => {
    stubSystemTheme(false);
    document.body.innerHTML = "";

    await expect(import("./settings")).rejects.toThrow("#app is missing from settings.html");
  });
});
