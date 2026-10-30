// What the Popover and the Quick Panel have in common, tested once for both: the same browser, wired to the same
// commands (QPN-06). Synthetic values only (AGENTS.md §2).
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render } from "@testing-library/svelte";
import { tick, type Component } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ClipQuery } from "../lib/bindings/ClipQuery";
import type { PasteResult } from "../lib/bindings/PasteResult";
import type { Settings } from "../lib/bindings/Settings";
import type { SettingsPatch } from "../lib/bindings/SettingsPatch";
import { clip, page } from "./clips";
import { defaultSettings } from "./settings";

interface Call {
  cmd: string;
  args: unknown;
}

/** A fake core that answers what a hosted browser asks: the clips, the settings and the paste commands. */
export function hostCore(options: { settings?: Partial<Settings>; paste?: PasteResult; trusted?: boolean } = {}) {
  const core = {
    calls: [] as Call[],
    settings: { ...defaultSettings, ...options.settings } as Settings,
    /** When set, `paste_clip` rejects with it. */
    pasteFails: undefined as { code: string; message: string } | undefined,
    /** When set, `list_clips` rejects with it. */
    listFails: undefined as { code: string; message: string } | undefined,
    commands: () => core.calls.map((call) => call.cmd),
    argsOf: (cmd: string) => core.calls.filter((call) => call.cmd === cmd).map((call) => call.args),
  };
  const everyClip = [clip(1), clip(2), clip(3), clip(4)];
  mockIPC(
    (cmd, args) => {
      core.calls.push({ cmd, args });
      switch (cmd) {
        case "list_clips": {
          if (core.listFails) throw core.listFails;
          const { search } = (args as { q: ClipQuery }).q;
          return page(search ? everyClip.filter((c) => c.preview?.includes(search)) : everyClip);
        }
        case "get_app_icon":
          throw { code: "not_found", message: "No icon is cached for this app." };
        case "get_settings":
          return core.settings;
        case "set_settings": {
          const patch = (args as { patch: SettingsPatch }).patch;
          core.settings = { ...core.settings, ...patch } as Settings;
          void emit("settings://changed", core.settings);
          return core.settings;
        }
        case "paste_clip":
          if (core.pasteFails) throw core.pasteFails;
          return options.paste ?? "pasted";
        case "check_accessibility":
          return options.trusted ?? true;
        default:
          return undefined;
      }
    },
    { shouldMockEvents: true },
  );
  return core;
}

/** Lets the IPC answers, the debounce, the event listeners and the DOM updates run. */
export async function settle(ms = 0) {
  await new Promise((resolve) => setTimeout(resolve, ms));
  await tick();
}

export function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

function selectedIds(container: HTMLElement): (string | null)[] {
  return [...container.querySelectorAll(".clip-row[aria-selected='true']")].map((row) => row.getAttribute("data-id"));
}

/**
 * Registers the tests of a window that hosts the clip browser. `label` is the window's own label, the one
 * `panel://shown` names.
 */
export function describeHost(name: string, Window: Component, label: "popover" | "panel", ariaLabel: string) {
  async function open(options: Parameters<typeof hostCore>[0] = {}) {
    const core = hostCore(options);
    const view = render(Window);
    await settle();
    return { ...view, core, field: find<HTMLInputElement>(view.container, "#search-input") };
  }

  describe(name, () => {
    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("is the window of the prototype, around the shared clip browser", async () => {
      const { container } = await open();

      expect(container.querySelector(`#${label === "panel" ? "quick-panel" : "popover"}`)).toHaveAttribute(
        "aria-label",
        ariaLabel,
      );
      for (const id of ["#search-input", "#clip-list", "#clip-count", "#btn-preferences"]) find(container, id);
      expect(find(container, "#clip-browser")).toHaveAttribute("data-layout", label);
      expect(find(container, "#clip-count")).toHaveTextContent("4 clips");
    });

    it("pastes the second clip in the default mode on ↓ and Return", async () => {
      const { core, field } = await open();

      await fireEvent.keyDown(field, { key: "ArrowDown" });
      await fireEvent.keyDown(field, { key: "Enter" });
      await settle();

      expect(core.argsOf("paste_clip")).toEqual([{ id: "clip-2", mode: "default" }]);
    });

    it("copies the selected clip on ⌥Return", async () => {
      const { core, field } = await open();

      await fireEvent.keyDown(field, { key: "Enter", altKey: true });
      await settle();

      expect(core.argsOf("copy_clip")).toEqual([{ id: "clip-1" }]);
    });

    it("hides its window on Esc", async () => {
      const { core, field } = await open();

      await fireEvent.keyDown(field, { key: "Escape" });
      await settle();

      expect(core.commands().filter((cmd) => cmd === "hide_window")).toEqual(["hide_window"]);
    });

    it("stays open and shows the banner when Accessibility is revoked and the paste only copied", async () => {
      const { core, container, field } = await open({ paste: "copied_only", trusted: true });
      expect(find(container, "#banner-accessibility")).not.toBeVisible();

      await fireEvent.keyDown(field, { key: "Enter" });
      await emit("accessibility://changed", { trusted: false });
      await settle();

      expect(core.argsOf("paste_clip")).toHaveLength(1);
      expect(core.commands()).not.toContain("hide_window");
      expect(find(container, "#banner-accessibility")).toBeVisible();
    });

    it("keeps the banner hidden while Accessibility is granted", async () => {
      const { container } = await open({ trusted: true });

      expect(find(container, "#banner-accessibility")).not.toBeVisible();
    });

    it("logs a failed command by its code only, and stays usable", async () => {
      const error = vi.spyOn(console, "error").mockImplementation(() => undefined);
      const { core, container, field } = await open();
      core.pasteFails = { code: "not_found", message: "The clip is gone." };

      await fireEvent.keyDown(field, { key: "Enter" });
      await settle();

      expect(error).toHaveBeenCalledWith(expect.stringContaining("not_found"));
      expect(JSON.stringify(error.mock.calls)).not.toContain("Synthetic");
      expect(find(container, "#clip-list")).toBeVisible();
    });

    it("starts afresh when it is shown: no search, first row, search focused", async () => {
      const { container, core, field } = await open();
      await fireEvent.input(field, { target: { value: "clip" } });
      await settle(100);
      await fireEvent.keyDown(field, { key: "ArrowDown" });
      await fireEvent.keyDown(field, { key: "ArrowDown" });
      expect(selectedIds(container)).toEqual(["clip-3"]);
      field.blur();
      core.calls.length = 0;

      await emit("panel://shown", { window: label });
      await settle();

      expect(field.value).toBe("");
      expect(selectedIds(container)).toEqual(["clip-1"]);
      expect(field).toHaveFocus();
      expect(core.commands()).toContain("list_clips");
    });

    it("closes the delete dialog when it is shown again, so no old clip can be deleted", async () => {
      const { container, core, field } = await open();
      await fireEvent.keyDown(field, { key: "ArrowDown" });
      await fireEvent.keyDown(field, { key: "Backspace", metaKey: true });
      expect(find(container, "#dialog-delete-clip")).toBeVisible();

      await emit("panel://shown", { window: label });
      await settle();
      await fireEvent.keyDown(field, { key: "Enter" });
      await settle();

      expect(container.querySelector("#dialog-delete-clip")).toBeNull();
      expect(core.argsOf("delete_clips")).toEqual([]);
      expect(field).toHaveFocus();
    });

    it("ignores the show of the other window", async () => {
      const { container, field } = await open();
      await fireEvent.input(field, { target: { value: "clip" } });
      await settle(100);
      await fireEvent.keyDown(field, { key: "ArrowDown" });
      field.blur();

      await emit("panel://shown", { window: label === "panel" ? "popover" : "panel" });
      await settle();

      expect(field.value).toBe("clip");
      expect(selectedIds(container)).toEqual(["clip-2"]);
      expect(field).not.toHaveFocus();
    });

    it("opens the preferences from the ⚙ button and from ⌘,", async () => {
      const { core, container, field } = await open();

      await fireEvent.click(find(container, "#btn-preferences"));
      await fireEvent.keyDown(field, { key: ",", metaKey: true });
      await settle();

      expect(core.commands().filter((cmd) => cmd === "open_preferences")).toEqual([
        "open_preferences",
        "open_preferences",
      ]);
    });

    it("stops asking before a delete when Don’t ask again is ticked", async () => {
      const { core, container, field } = await open();
      await fireEvent.keyDown(field, { key: "Backspace", metaKey: true });

      await fireEvent.click(find(container, "#chk-dont-ask-delete"));
      await fireEvent.click(find(container, "#btn-delete-confirm"));
      await settle();

      expect(core.argsOf("set_settings")).toEqual([{ patch: { confirmBeforeDelete: false } }]);
      expect(core.argsOf("delete_clips")).toEqual([{ ids: ["clip-1"] }]);
    });

    it("deletes without a dialog when the settings say not to ask", async () => {
      const { core, container, field } = await open({ settings: { confirmBeforeDelete: false } });

      await fireEvent.keyDown(field, { key: "Backspace", metaKey: true });
      await settle();

      expect(container.querySelector("#dialog-delete-clip")).toBeNull();
      expect(core.argsOf("delete_clips")).toEqual([{ ids: ["clip-1"] }]);
    });

    it("follows the preferences when they change while it is open", async () => {
      const { core, container, field } = await open();

      await emit("settings://changed", { ...core.settings, confirmBeforeDelete: false });
      await settle();
      await fireEvent.keyDown(field, { key: "Backspace", metaKey: true });
      await settle();

      expect(container.querySelector("#dialog-delete-clip")).toBeNull();
      expect(core.argsOf("delete_clips")).toEqual([{ ids: ["clip-1"] }]);
    });
  });
}
