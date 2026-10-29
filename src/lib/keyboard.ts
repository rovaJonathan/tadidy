// Every key of the clip browser goes through `resolveKey` (AGENTS.md §14): a pure map from a key press
// to what the browser should do, so the shortcut table (spec: keyboard shortcuts) is tested without a DOM.

/** What a key press asks of the clip browser. */
export type BrowserAction =
  | "select-previous"
  | "select-next"
  | "paste"
  | "copy"
  | "delete"
  | "clear-search"
  | "close"
  | "focus-search"
  | "open-preferences";

/** The part of a `KeyboardEvent` the map reads. */
export type KeyInput = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey" | "isComposing">;

export interface KeyContext {
  searchFocused: boolean;
  searchEmpty: boolean;
  /** Part of the search text is selected: ⌘C is then the usual text copy. */
  inputHasSelection: boolean;
  /** A dialog is open: it handles its own keys. */
  dialogOpen: boolean;
}

/** The action a key press means in the clip browser, or `null` to leave it to the page. */
export function resolveKey(event: KeyInput, context: KeyContext): BrowserAction | null {
  // WebKit reports the keys of an input method as `Process`, or with `isComposing` set.
  if (context.dialogOpen || event.isComposing || event.key === "Process") return null;

  if (event.metaKey) return resolveCommandKey(event.key.toLowerCase(), context);
  if (event.ctrlKey) return null;

  switch (event.key) {
    case "ArrowUp":
      return "select-previous";
    case "ArrowDown":
      return "select-next";
    case "Enter":
      if (event.altKey) return "copy";
      return event.shiftKey ? null : "paste";
    case "Escape":
      return context.searchEmpty ? "close" : "clear-search";
  }

  // A character typed away from the search lands in it: the field gets the focus, the key press follows.
  const printable = event.key.length === 1;
  return printable && !context.searchFocused ? "focus-search" : null;
}

function resolveCommandKey(key: string, context: KeyContext): BrowserAction | null {
  switch (key) {
    case "c":
      return context.searchFocused && context.inputHasSelection ? null : "copy";
    case "backspace":
      return "delete";
    case "f":
      return "focus-search";
    case ",":
      return "open-preferences";
    default:
      return null;
  }
}
