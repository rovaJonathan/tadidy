import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";
import { appVersion, hideWindow } from "./api";

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
