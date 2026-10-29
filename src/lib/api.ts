// Typed wrappers over the core's IPC commands and events. Components never call invoke
// directly (AGENTS.md §14); argument and result types come from ./bindings (ADR 0006).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PanelShown } from "./bindings/PanelShown";
import type { Settings } from "./bindings/Settings";
import type { SettingsPatch } from "./bindings/SettingsPatch";

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
export function onPanelShown(callback: (shown: PanelShown) => void): Promise<UnlistenFn> {
  return listen<PanelShown>("panel://shown", (event) => callback(event.payload));
}

/** The user's preferences. */
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/**
 * Changes the fields set in `patch` and returns the full settings. Rejects with
 * `invalid_settings` (naming the field) when a value breaks a rule; nothing is stored then.
 */
export function setSettings(patch: SettingsPatch): Promise<Settings> {
  return invoke<Settings>("set_settings", { patch });
}

/** Runs `callback` with the full settings each time they change (`settings://changed`). */
export function onSettingsChanged(callback: (settings: Settings) => void): Promise<UnlistenFn> {
  return listen<Settings>("settings://changed", (event) => callback(event.payload));
}
