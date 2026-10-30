// The Quick Panel shortcut as the recorder sees it: a key press becomes the accelerator string the core
// registers (`Ctrl+Alt+V`), and an accelerator becomes the symbols the Shortcuts tab shows (`⌃⌥V`).

/** The part of a `KeyboardEvent` the recorder reads. */
export type RecordedKey = Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/**
 * The accelerator a key press records, or `null` when it cannot be a global shortcut: a modifier on
 * its own, a key without ⌘, ⌃ or ⌥, or a key the core cannot register.
 */
export function toAccelerator(event: RecordedKey): string | null {
  void event;
  throw new Error("not implemented");
}

/** An accelerator as symbols in the macOS order (⌃⌥⌘⇧, then the key), e.g. `Cmd+Shift+V` → `⌘⇧V`. */
export function formatAccelerator(accelerator: string): string {
  void accelerator;
  throw new Error("not implemented");
}

/** Whether macOS keeps the accelerator for itself (Spotlight, the app switcher, screenshots…). */
export function isReserved(accelerator: string): boolean {
  void accelerator;
  throw new Error("not implemented");
}
