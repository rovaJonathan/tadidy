import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { describe, expect, it } from "vitest";
import PermissionBanner from "./PermissionBanner.svelte";

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

/** A core whose Accessibility permission is `trusted`; records every command it receives. */
function core(trusted: boolean | Promise<boolean>, openSettings: () => void = () => undefined) {
  const commands: string[] = [];
  mockIPC(
    (cmd) => {
      commands.push(cmd);
      if (cmd === "check_accessibility") return trusted;
      if (cmd === "open_accessibility_settings") return openSettings();
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return commands;
}

describe("PermissionBanner", () => {
  it("is the prototype's banner, text for text, when the permission is missing", async () => {
    core(false);

    const { container } = render(PermissionBanner);
    await settle();

    const banner = find(container, "#banner-accessibility");
    expect(banner.hidden).toBe(false);
    expect(banner.getAttribute("role")).toBe("status");
    expect(find(banner, ".banner-text").textContent).toBe("Auto-paste disabled — clips are copied only.");
    expect(find(banner, ".ico").getAttribute("aria-hidden")).toBe("true");
    const button = find(banner, "#btn-enable-accessibility");
    expect(button.textContent?.trim()).toBe("Enable");
    expect(button.getAttribute("type")).toBe("button");
  });

  it("stays hidden while the permission is not known yet", async () => {
    core(new Promise<boolean>(() => undefined));

    const { container } = render(PermissionBanner);
    await settle();

    expect(find(container, "#banner-accessibility").hidden).toBe(true);
  });

  it("stays hidden when the permission is granted", async () => {
    core(true);

    const { container } = render(PermissionBanner);
    await settle();

    expect(find(container, "#banner-accessibility").hidden).toBe(true);
  });

  it("opens the Accessibility settings when Enable is clicked", async () => {
    const commands = core(false);
    const { container } = render(PermissionBanner);
    await settle();

    await fireEvent.click(find(container, "#btn-enable-accessibility"));
    await settle();

    expect(commands).toContain("open_accessibility_settings");
    expect(find(container, "#banner-accessibility").hidden).toBe(false);
  });

  it("survives the settings failing to open", async () => {
    core(false, () => {
      throw { code: "accessibility_settings_unavailable", message: "The Accessibility settings could not be opened." };
    });
    const { container } = render(PermissionBanner);
    await settle();

    await fireEvent.click(find(container, "#btn-enable-accessibility"));
    await settle();

    expect(find(container, "#banner-accessibility").hidden).toBe(false);
  });

  it("hides when the permission is granted and shows again when it is revoked", async () => {
    core(false);
    const { container } = render(PermissionBanner);
    await settle();

    await emit("accessibility://changed", { trusted: true });
    await settle();
    expect(find(container, "#banner-accessibility").hidden).toBe(true);

    await emit("accessibility://changed", { trusted: false });
    await settle();
    expect(find(container, "#banner-accessibility").hidden).toBe(false);
  });

  it("trusts an event that arrives before the first check answers", async () => {
    let answer: (trusted: boolean) => void = () => undefined;
    core(new Promise<boolean>((resolve) => (answer = resolve)));
    const { container } = render(PermissionBanner);
    await settle();

    await emit("accessibility://changed", { trusted: true });
    answer(false);
    await settle();

    expect(find(container, "#banner-accessibility").hidden).toBe(true);
  });

  it("stops listening once removed", async () => {
    core(false);
    const { container, unmount } = render(PermissionBanner);
    await settle();
    const banner = find(container, "#banner-accessibility");

    unmount();
    await emit("accessibility://changed", { trusted: true });
    await settle();

    expect(banner.hidden).toBe(false);
  });
});
