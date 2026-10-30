import { describe, expect, it } from "vitest";
import { formatAccelerator, isReserved, toAccelerator, type RecordedKey } from "./shortcut";

/** A key press: `code` is the physical key, the flags are the modifiers held. */
function press(code: string, modifiers: Partial<Omit<RecordedKey, "code">> = {}): RecordedKey {
  return { code, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...modifiers };
}

describe("toAccelerator", () => {
  it("records ⌃⌥V as Ctrl+Alt+V", () => {
    expect(toAccelerator(press("KeyV", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+V");
  });

  it("writes the modifiers in a fixed order, whatever order they were pressed in", () => {
    expect(toAccelerator(press("KeyV", { shiftKey: true, metaKey: true }))).toBe("Cmd+Shift+V");
    expect(toAccelerator(press("KeyV", { shiftKey: true, metaKey: true, altKey: true, ctrlKey: true }))).toBe(
      "Ctrl+Alt+Cmd+Shift+V",
    );
  });

  it("reads the physical key, not the character ⌥ turns it into", () => {
    expect(toAccelerator(press("KeyV", { altKey: true }))).toBe("Alt+V");
  });

  it("refuses a key without ⌘, ⌃ or ⌥", () => {
    expect(toAccelerator(press("KeyV"))).toBeNull();
    expect(toAccelerator(press("KeyV", { shiftKey: true }))).toBeNull();
  });

  it("refuses a modifier on its own", () => {
    expect(toAccelerator(press("ShiftLeft", { shiftKey: true }))).toBeNull();
    expect(toAccelerator(press("MetaLeft", { metaKey: true }))).toBeNull();
    expect(toAccelerator(press("AltRight", { altKey: true }))).toBeNull();
    expect(toAccelerator(press("ControlLeft", { ctrlKey: true }))).toBeNull();
  });

  it("records digits, function keys, space, tab, escape and the arrows", () => {
    expect(toAccelerator(press("Digit3", { metaKey: true, shiftKey: true }))).toBe("Cmd+Shift+3");
    expect(toAccelerator(press("F5", { altKey: true }))).toBe("Alt+F5");
    expect(toAccelerator(press("Space", { ctrlKey: true }))).toBe("Ctrl+Space");
    expect(toAccelerator(press("Tab", { metaKey: true }))).toBe("Cmd+Tab");
    expect(toAccelerator(press("Escape", { altKey: true, metaKey: true }))).toBe("Alt+Cmd+Esc");
    expect(toAccelerator(press("ArrowUp", { metaKey: true }))).toBe("Cmd+Up");
    expect(toAccelerator(press("ArrowRight", { ctrlKey: true }))).toBe("Ctrl+Right");
  });

  it("records the punctuation keys by name", () => {
    expect(toAccelerator(press("Comma", { metaKey: true }))).toBe("Cmd+Comma");
    expect(toAccelerator(press("Backquote", { ctrlKey: true }))).toBe("Ctrl+Backquote");
  });

  it("refuses a key the core cannot register", () => {
    expect(toAccelerator(press("IntlBackslash", { metaKey: true }))).toBeNull();
    expect(toAccelerator(press("", { metaKey: true }))).toBeNull();
  });
});

describe("formatAccelerator", () => {
  it("shows the modifiers as symbols: Cmd+Shift+V is ⌘⇧V", () => {
    expect(formatAccelerator("Cmd+Shift+V")).toBe("⌘⇧V");
  });

  it("shows Ctrl+Alt+V as ⌃⌥V", () => {
    expect(formatAccelerator("Ctrl+Alt+V")).toBe("⌃⌥V");
  });

  it("puts the modifiers in the macOS order", () => {
    expect(formatAccelerator("Shift+Cmd+Alt+Ctrl+V")).toBe("⌃⌥⌘⇧V");
  });

  it("knows the other spellings of a modifier", () => {
    expect(formatAccelerator("Command+Option+Control+Shift+K")).toBe("⌃⌥⌘⇧K");
    expect(formatAccelerator("CmdOrCtrl+K")).toBe("⌘K");
  });

  it("shows the named keys the way the prototype does", () => {
    expect(formatAccelerator("Cmd+Up")).toBe("⌘↑");
    expect(formatAccelerator("Ctrl+Right")).toBe("⌃→");
    expect(formatAccelerator("Cmd+Tab")).toBe("⌘⇥");
    expect(formatAccelerator("Ctrl+Space")).toBe("⌃Space");
    expect(formatAccelerator("Alt+Cmd+Esc")).toBe("⌥⌘Esc");
    expect(formatAccelerator("Cmd+Comma")).toBe("⌘,");
  });

  it("leaves a key it does not know as it is", () => {
    expect(formatAccelerator("Cmd+F5")).toBe("⌘F5");
    expect(formatAccelerator("Cmd+Whatever")).toBe("⌘Whatever");
  });
});

describe("isReserved", () => {
  it("is true for what macOS keeps for itself", () => {
    for (const accelerator of [
      "Cmd+Space",
      "Ctrl+Space",
      "Cmd+Tab",
      "Cmd+Q",
      "Cmd+W",
      "Cmd+H",
      "Cmd+M",
      "Cmd+C",
      "Cmd+V",
      "Cmd+X",
      "Cmd+Z",
      "Cmd+A",
      "Cmd+Shift+3",
      "Cmd+Shift+4",
      "Cmd+Shift+5",
      "Alt+Cmd+Esc",
    ]) {
      expect(isReserved(accelerator), accelerator).toBe(true);
    }
  });

  it("is false for the default shortcut and for a free one", () => {
    expect(isReserved("Cmd+Shift+V")).toBe(false);
    expect(isReserved("Ctrl+Alt+V")).toBe(false);
  });

  it("does not depend on how the modifiers are written or ordered", () => {
    expect(isReserved("Shift+Command+4")).toBe(true);
    expect(isReserved("Command+Alt+Esc")).toBe(true);
  });
});
