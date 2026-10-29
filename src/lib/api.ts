// Typed wrappers over the core's IPC commands and events. Components never call invoke
// directly (AGENTS.md §14); argument and result types come from ./bindings (ADR 0006).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError } from "./bindings/AppError";
import type { ClipQuery } from "./bindings/ClipQuery";
import type { ClipsChanged } from "./bindings/ClipsChanged";
import type { ClipSummary } from "./bindings/ClipSummary";
import type { Page } from "./bindings/Page";
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

/**
 * One page of the history, newest first, filtered by `q.search` when it has one. Hand
 * `nextCursor` back as `q.cursor` for the following page. Rejects with `invalid_cursor` when the
 * cursor is not one the core handed out.
 */
export function listClips(q: ClipQuery): Promise<Page<ClipSummary>> {
  return invoke<Page<ClipSummary>>("list_clips", { q });
}

/** Removes clips; the core announces it with `clips://changed`. */
export async function deleteClips(ids: string[]): Promise<void> {
  await invoke("delete_clips", { ids });
}

/** Empties the history, pinned clips spared when `keepPinned`, and returns how many clips went. */
export function clearHistory(keepPinned: boolean): Promise<number> {
  return invoke<number>("clear_history", { keepPinned });
}

/** The PNG bytes of a source app's icon, or `null` when none is cached for it. */
export async function getAppIcon(bundleId: string): Promise<ArrayBuffer | null> {
  try {
    return await invoke<ArrayBuffer>("get_app_icon", { bundleId });
  } catch (error) {
    if (isAppError(error) && error.code === "not_found") {
      return null;
    }
    throw error;
  }
}

/**
 * Runs `callback` each time the history changes (`clips://changed`). A payload with nothing set
 * means "reload the list".
 */
export function onClipsChanged(callback: (changed: ClipsChanged) => void): Promise<UnlistenFn> {
  return listen<ClipsChanged>("clips://changed", (event) => callback(event.payload));
}

function isAppError(error: unknown): error is AppError {
  return typeof error === "object" && error !== null && "code" in error;
}
