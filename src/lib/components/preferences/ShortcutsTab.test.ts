import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { fakeSettingsCore } from "../../../test/settings";
import { SettingsStore } from "../../stores/settings.svelte";
import ShortcutsTab from "./ShortcutsTab.svelte";

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
  const view = render(ShortcutsTab, { store });
  await settle();
  const recorder = find(view.container, "#shortcut-recorder");
  const keys = () => find(view.container, "#shortcut-keys").textContent;
  const error = () => find(view.container, "#shortcut-error");
  /** Presses a key on the recorder: `code` is the physical key, `modifiers` the flags held. */
  const press = async (code: string, modifiers: Partial<KeyboardEventInit> = {}) => {
    await fireEvent.keyDown(recorder, { code, key: code, ...modifiers });
    await settle();
  };
  const record = async () => {
    await fireEvent.click(recorder);
    await settle();
  };
  return { ...view, core, store, recorder, keys, error, press, record };
}

afterEach(() => {
  for (const store of stores.splice(0)) store.destroy();
});

const CONFLICT = "This shortcut is already used by macOS or another app. Choose another one.";

describe("ShortcutsTab", () => {
  it("is the prototype's Shortcuts panel, text for text", async () => {
    const { container, recorder, keys, error } = await open();

    const panel = find(container, "#prefs-shortcuts");
    expect(panel.getAttribute("role")).toBe("tabpanel");
    expect(panel.getAttribute("aria-labelledby")).toBe("prefs-tab-shortcuts");
    expect(find(container, "#shortcut-label").textContent).toBe("Open Quick Panel");
    expect(panel.textContent).toContain("Works in every app, even in full screen.");
    expect(keys()).toBe("⌘⇧V");
    expect(recorder.getAttribute("role")).toBe("button");
    expect(recorder.getAttribute("tabindex")).toBe("0");
    expect(recorder.getAttribute("aria-labelledby")).toBe("shortcut-label shortcut-keys");
    expect(recorder.getAttribute("aria-describedby")).toBe("shortcut-help");
    expect(find(container, "#shortcut-help").textContent).toBe(
      "Click, then press the new key combination. Press Escape to cancel.",
    );
    const reset = find(container, "#btn-shortcut-reset");
    expect(reset.getAttribute("aria-label")).toBe("Restore default shortcut");
    expect(reset.getAttribute("title")).toBe("Restore default (⌘⇧V)");
    expect(error().hasAttribute("hidden")).toBe(true);
    expect(error().textContent).toBe(CONFLICT);
    expect(error().getAttribute("role")).toBe("alert");
  });

  it("renders none of the phase 2 rows", async () => {
    const { container } = await open();

    for (const id of ["pref-quick-paste", "pref-plain-text-default", "pref-snippet-shortcuts"]) {
      expect(container.querySelector(`#${id}`), id).toBeNull();
    }
  });

  it("shows the stored shortcut as symbols", async () => {
    const { keys } = await open(fakeSettingsCore({ settings: { quickPanelShortcut: "Ctrl+Alt+V" } }));

    expect(keys()).toBe("⌃⌥V");
  });

  it("asks for a key combination when the recorder is clicked", async () => {
    const { recorder, keys, record, error } = await open();

    await record();

    expect(keys()).toBe("Type shortcut…");
    expect(recorder.classList.contains("is-recording")).toBe(true);
    expect(error().hasAttribute("hidden")).toBe(true);
  });

  it("starts recording from the keyboard with Enter or Space", async () => {
    for (const key of ["Enter", " "]) {
      const { recorder, keys, press } = await open();

      await press(key === " " ? "Space" : "Enter", { key });

      expect(keys(), JSON.stringify(key)).toBe("Type shortcut…");
      expect(recorder.classList.contains("is-recording")).toBe(true);
      stores.pop()?.destroy();
      document.body.innerHTML = "";
    }
  });

  it("records ⌃⌥V and saves Ctrl+Alt+V", async () => {
    const { core, keys, recorder, record, press } = await open();
    await record();

    await press("KeyV", { ctrlKey: true, altKey: true, key: "√" });

    expect(core.patches).toEqual([{ quickPanelShortcut: "Ctrl+Alt+V" }]);
    expect(keys()).toBe("⌃⌥V");
    expect(recorder.classList.contains("is-recording")).toBe(false);
  });

  it("refuses V alone and ⇧ alone: nothing is saved and it keeps listening", async () => {
    const { core, keys, recorder, record, press } = await open();
    await record();

    await press("KeyV");
    await press("ShiftLeft", { shiftKey: true, key: "Shift" });

    expect(core.patches).toEqual([]);
    expect(keys()).toBe("Type shortcut…");
    expect(recorder.classList.contains("is-recording")).toBe(true);
  });

  it("cancels on Escape and shows the stored shortcut again", async () => {
    const { core, keys, recorder, record, press } = await open();
    await record();

    await press("Escape", { key: "Escape" });

    expect(core.patches).toEqual([]);
    expect(keys()).toBe("⌘⇧V");
    expect(recorder.classList.contains("is-recording")).toBe(false);
  });

  it("cancels when the recorder loses the focus", async () => {
    const { keys, recorder, record } = await open();
    await record();

    await fireEvent.blur(recorder);
    await settle();

    expect(keys()).toBe("⌘⇧V");
    expect(recorder.classList.contains("is-recording")).toBe(false);
  });

  it("shows the conflict error for a combination macOS keeps, without asking the core", async () => {
    const { core, keys, recorder, error, record, press } = await open();
    await record();

    await press("Digit4", { metaKey: true, shiftKey: true });

    expect(core.patches).toEqual([]);
    expect(error().hasAttribute("hidden")).toBe(false);
    expect(error().textContent).toBe(CONFLICT);
    expect(recorder.classList.contains("has-error")).toBe(true);
    expect(keys()).toBe("⌘⇧4");
  });

  it("shows the conflict error when the core refuses the shortcut, and keeps the previous one stored", async () => {
    const core = fakeSettingsCore();
    core.refuse = { code: "shortcut_unavailable", message: "This shortcut is already used by macOS or another app." };
    const { store, error, recorder, record, press } = await open(core);
    await record();

    await press("KeyV", { ctrlKey: true, altKey: true });

    expect(core.patches).toEqual([{ quickPanelShortcut: "Ctrl+Alt+V" }]);
    expect(error().hasAttribute("hidden")).toBe(false);
    expect(error().textContent).toBe(CONFLICT);
    expect(recorder.classList.contains("has-error")).toBe(true);
    expect(store.settings?.quickPanelShortcut).toBe("Cmd+Shift+V");
  });

  it("shows the conflict error at open when the shortcut failed to register at startup", async () => {
    const { core, error, recorder, keys } = await open(
      fakeSettingsCore({ shortcut: { accelerator: "Cmd+Shift+V", error: "unavailable" } }),
    );

    expect(core.commands).toContain("get_shortcut_status");
    expect(error().hasAttribute("hidden")).toBe(false);
    expect(recorder.classList.contains("has-error")).toBe(true);
    expect(keys()).toBe("⌘⇧V");
  });

  it("shows no error at open when the shortcut is registered", async () => {
    const { error, recorder } = await open();

    expect(error().hasAttribute("hidden")).toBe(true);
    expect(recorder.classList.contains("has-error")).toBe(false);
  });

  it("clears the error when a new shortcut is saved", async () => {
    const { error, recorder, record, press } = await open(
      fakeSettingsCore({ shortcut: { accelerator: "Cmd+Shift+V", error: "unavailable" } }),
    );
    await record();

    await press("KeyV", { ctrlKey: true, altKey: true });

    expect(error().hasAttribute("hidden")).toBe(true);
    expect(recorder.classList.contains("has-error")).toBe(false);
  });

  it("restores the default shortcut with the reset button, without starting a recording", async () => {
    const { container, core, keys, recorder } = await open(
      fakeSettingsCore({ settings: { quickPanelShortcut: "Ctrl+Alt+V" } }),
    );

    await fireEvent.click(find(container, "#btn-shortcut-reset"));
    await settle();

    expect(core.patches).toEqual([{ quickPanelShortcut: "Cmd+Shift+V" }]);
    expect(keys()).toBe("⌘⇧V");
    expect(recorder.classList.contains("is-recording")).toBe(false);
  });

  it("does not ask the core to restore a shortcut that already is the default", async () => {
    const { container, core } = await open();

    await fireEvent.click(find(container, "#btn-shortcut-reset"));
    await settle();

    expect(core.patches).toEqual([]);
  });

  it("keeps the startup error when restoring the default has nothing to change", async () => {
    const { container, error, recorder } = await open(
      fakeSettingsCore({ shortcut: { accelerator: "Cmd+Shift+V", error: "unavailable" } }),
    );

    await fireEvent.click(find(container, "#btn-shortcut-reset"));
    await settle();

    expect(error().hasAttribute("hidden")).toBe(false);
    expect(recorder.classList.contains("has-error")).toBe(true);
  });
});
