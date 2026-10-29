import { describe, expect, it } from "vitest";
import { resolveKey, type KeyContext, type KeyInput } from "./keyboard";

function press(key: string, modifiers: Partial<KeyInput> = {}): KeyInput {
  return { key, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, isComposing: false, ...modifiers };
}

const IDLE: KeyContext = { searchFocused: false, searchEmpty: true, inputHasSelection: false, dialogOpen: false };
const TYPING: KeyContext = { ...IDLE, searchFocused: true, searchEmpty: false };

describe("resolveKey", () => {
  it.each([
    ["ArrowUp", "select-previous"],
    ["ArrowDown", "select-next"],
  ])("%s moves the selection, in the search or out of it", (key, action) => {
    expect(resolveKey(press(key), IDLE)).toBe(action);
    expect(resolveKey(press(key), TYPING)).toBe(action);
  });

  it("Return pastes", () => {
    expect(resolveKey(press("Enter"), TYPING)).toBe("paste");
    expect(resolveKey(press("Enter"), IDLE)).toBe("paste");
  });

  it("⌥Return copies", () => {
    expect(resolveKey(press("Enter", { altKey: true }), TYPING)).toBe("copy");
  });

  it("⇧Return does nothing yet (plain-text paste is a later ticket)", () => {
    expect(resolveKey(press("Enter", { shiftKey: true }), TYPING)).toBeNull();
  });

  it("⌘C copies the clip", () => {
    expect(resolveKey(press("c", { metaKey: true }), IDLE)).toBe("copy");
    expect(resolveKey(press("c", { metaKey: true }), TYPING)).toBe("copy");
  });

  it("⌘C stays a normal text copy while the search has a text selection", () => {
    const selecting = { ...TYPING, inputHasSelection: true };

    expect(resolveKey(press("c", { metaKey: true }), selecting)).toBeNull();
  });

  it("⌘⌫ deletes", () => {
    expect(resolveKey(press("Backspace", { metaKey: true }), TYPING)).toBe("delete");
  });

  it("a plain ⌫ is left to the search field", () => {
    expect(resolveKey(press("Backspace"), TYPING)).toBeNull();
  });

  it("Esc clears a non-empty search", () => {
    expect(resolveKey(press("Escape"), TYPING)).toBe("clear-search");
  });

  it("Esc closes when the search is empty", () => {
    expect(resolveKey(press("Escape"), { ...IDLE, searchFocused: true })).toBe("close");
    expect(resolveKey(press("Escape"), IDLE)).toBe("close");
  });

  it("⌘F focuses the search", () => {
    expect(resolveKey(press("f", { metaKey: true }), IDLE)).toBe("focus-search");
    expect(resolveKey(press("f", { metaKey: true }), TYPING)).toBe("focus-search");
  });

  it("a printable character focuses the search when it does not have the focus", () => {
    expect(resolveKey(press("a"), IDLE)).toBe("focus-search");
    expect(resolveKey(press("A", { shiftKey: true }), IDLE)).toBe("focus-search");
    expect(resolveKey(press("7"), IDLE)).toBe("focus-search");
  });

  it("a printable character is typed as is when the search has the focus", () => {
    expect(resolveKey(press("a"), TYPING)).toBeNull();
  });

  it.each([
    ["⌘", { metaKey: true }],
    ["⌃", { ctrlKey: true }],
  ])("%s with a character is not typing", (_name, modifiers) => {
    expect(resolveKey(press("a", modifiers), IDLE)).toBeNull();
  });

  it("a key with no character (Shift, Tab, F5…) does not focus the search", () => {
    for (const key of ["Shift", "Tab", "F5", "CapsLock"]) expect(resolveKey(press(key), IDLE)).toBeNull();
  });

  it("⌘, opens the preferences", () => {
    expect(resolveKey(press(",", { metaKey: true }), TYPING)).toBe("open-preferences");
  });

  it("leaves every key to the dialog while it is open", () => {
    const dialog = { ...TYPING, dialogOpen: true };
    const keys = [
      press("ArrowDown"),
      press("Enter"),
      press("Escape"),
      press("a"),
      press("c", { metaKey: true }),
      press("Backspace", { metaKey: true }),
      press(",", { metaKey: true }),
    ];

    for (const key of keys) expect(resolveKey(key, dialog)).toBeNull();
  });

  it("ignores a key pressed while an input method is composing", () => {
    expect(resolveKey(press("Enter", { isComposing: true }), TYPING)).toBeNull();
    expect(resolveKey(press("ArrowDown", { isComposing: true }), TYPING)).toBeNull();
    expect(resolveKey(press("Process"), TYPING)).toBeNull();
  });
});
