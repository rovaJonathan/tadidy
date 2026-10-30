// The Quick Panel shortcut as the recorder sees it: a key press becomes the accelerator string the core
// registers (`Ctrl+Alt+V`), and an accelerator becomes the symbols the Shortcuts tab shows (`⌃⌥V`).

/** The part of a `KeyboardEvent` the recorder reads. */
export type RecordedKey = Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/** The modifiers in the order the prototype writes them (⌃⌥⌘⇧), with the spellings the core accepts. */
const MODIFIERS = [
  { name: "Ctrl", symbol: "⌃", aliases: ["ctrl", "control"] },
  { name: "Alt", symbol: "⌥", aliases: ["alt", "option"] },
  { name: "Cmd", symbol: "⌘", aliases: ["cmd", "command", "super", "cmdorctrl", "commandorcontrol"] },
  { name: "Shift", symbol: "⇧", aliases: ["shift"] },
] as const;

type ModifierName = (typeof MODIFIERS)[number]["name"];

/** Keys with a name of their own: `KeyboardEvent.code` → the accelerator's key → how it is shown. */
const NAMED_KEYS: Record<string, { key: string; symbol: string }> = {
  Space: { key: "Space", symbol: "Space" },
  Tab: { key: "Tab", symbol: "⇥" },
  Escape: { key: "Esc", symbol: "Esc" },
  Enter: { key: "Enter", symbol: "↩" },
  Backspace: { key: "Backspace", symbol: "⌫" },
  Delete: { key: "Delete", symbol: "⌦" },
  ArrowUp: { key: "Up", symbol: "↑" },
  ArrowDown: { key: "Down", symbol: "↓" },
  ArrowLeft: { key: "Left", symbol: "←" },
  ArrowRight: { key: "Right", symbol: "→" },
  Home: { key: "Home", symbol: "Home" },
  End: { key: "End", symbol: "End" },
  PageUp: { key: "PageUp", symbol: "PageUp" },
  PageDown: { key: "PageDown", symbol: "PageDown" },
  Comma: { key: "Comma", symbol: "," },
  Period: { key: "Period", symbol: "." },
  Slash: { key: "Slash", symbol: "/" },
  Semicolon: { key: "Semicolon", symbol: ";" },
  Quote: { key: "Quote", symbol: "'" },
  Backquote: { key: "Backquote", symbol: "`" },
  Backslash: { key: "Backslash", symbol: "\\" },
  BracketLeft: { key: "BracketLeft", symbol: "[" },
  BracketRight: { key: "BracketRight", symbol: "]" },
  Minus: { key: "Minus", symbol: "-" },
  Equal: { key: "Equal", symbol: "=" },
};

const SYMBOL_OF_KEY = new Map(Object.values(NAMED_KEYS).map(({ key, symbol }) => [key, symbol]));

/** What macOS keeps for itself, written as accelerators (the prototype's list). */
const RESERVED = new Set([
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
]);

/** The accelerator's key for a physical key, or `null` when the core cannot register it. */
function keyOf(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^Digit([0-9])$/.exec(code);
  if (digit) return digit[1] ?? null;
  if (/^F([1-9]|1[0-9])$/.test(code)) return code;
  return NAMED_KEYS[code]?.key ?? null;
}

/**
 * The accelerator a key press records, or `null` when it cannot be a global shortcut: a modifier on
 * its own, a key without ⌘, ⌃ or ⌥, or a key the core cannot register.
 */
export function toAccelerator(event: RecordedKey): string | null {
  if (!(event.metaKey || event.ctrlKey || event.altKey)) return null;
  const key = keyOf(event.code);
  if (key === null) return null;
  const held: Record<ModifierName, boolean> = {
    Ctrl: event.ctrlKey,
    Alt: event.altKey,
    Cmd: event.metaKey,
    Shift: event.shiftKey,
  };
  const modifiers = MODIFIERS.filter(({ name }) => held[name]).map(({ name }) => name);
  return [...modifiers, key].join("+");
}

/** Splits an accelerator into its modifiers, in the fixed order, and its key; any spelling of a modifier. */
function parts(accelerator: string): { modifiers: (typeof MODIFIERS)[number][]; key: string } {
  const tokens = accelerator.split("+");
  const key = tokens.pop() ?? "";
  const used = new Set(tokens.map((token) => token.toLowerCase()));
  return { modifiers: MODIFIERS.filter(({ aliases }) => aliases.some((alias) => used.has(alias))), key };
}

/** An accelerator as symbols in the macOS order (⌃⌥⌘⇧, then the key), e.g. `Cmd+Shift+V` → `⌘⇧V`. */
export function formatAccelerator(accelerator: string): string {
  const { modifiers, key } = parts(accelerator);
  return modifiers.map(({ symbol }) => symbol).join("") + (SYMBOL_OF_KEY.get(key) ?? key);
}

/** Whether macOS keeps the accelerator for itself (Spotlight, the app switcher, screenshots…). */
export function isReserved(accelerator: string): boolean {
  const { modifiers, key } = parts(accelerator);
  return RESERVED.has([...modifiers.map(({ name }) => name), key].join("+"));
}
