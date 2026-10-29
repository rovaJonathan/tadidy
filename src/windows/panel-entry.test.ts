import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

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

describe("panel entry", () => {
  beforeEach(() => {
    vi.resetModules();
    document.body.innerHTML = '<div id="app"></div>';
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("mounts the Quick Panel into #app", async () => {
    stubSystemTheme(false);

    await import("./panel");

    expect(document.querySelector("#app #quick-panel")).not.toBeNull();
  });

  it("follows the system theme, also when it changes", async () => {
    const system = stubSystemTheme(true);

    await import("./panel");
    expect(document.documentElement.dataset.theme).toBe("dark");

    system.setDark(false);
    system.change();
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("fails loudly when #app is missing", async () => {
    stubSystemTheme(false);
    document.body.innerHTML = "";

    await expect(import("./panel")).rejects.toThrow("#app is missing from panel.html");
  });
});
