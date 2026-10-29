// Typed wrappers over the core's IPC commands and events. Components never call invoke
// directly (AGENTS.md §14); argument and result types come from ./bindings (ADR 0006).
import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { PanelShown } from "./bindings/PanelShown";

export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}

/** Hides the window that calls it (Esc in the popover). The window is hidden, not destroyed. */
export async function hideWindow(): Promise<void> {
  await invoke("hide_window");
}

/** Whether Tadidy may post the paste keystroke. Never shows the system prompt. */
export function checkAccessibility(): Promise<boolean> {
  return invoke<boolean>("check_accessibility");
}

/**
 * Lists Tadidy under Privacy & Security → Accessibility and opens that pane. Call it from a
 * user click only: the permission is never requested silently.
 */
export async function openAccessibilitySettings(): Promise<void> {
  await invoke("open_accessibility_settings");
}

/** Runs `callback` each time the popover or the Quick Panel is shown (`panel://shown`). */
export async function onPanelShown(_callback: (shown: PanelShown) => void): Promise<UnlistenFn> {
  return () => undefined;
}
