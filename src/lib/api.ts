// Typed wrappers over the core's IPC commands and events. Components never call invoke
// directly (AGENTS.md §14); argument and result types come from ./bindings (ADR 0006).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccessibilityChanged } from "./bindings/AccessibilityChanged";
import type { AppError } from "./bindings/AppError";
import type { ClipQuery } from "./bindings/ClipQuery";
import type { ClipsChanged } from "./bindings/ClipsChanged";
import type { ClipSummary } from "./bindings/ClipSummary";
import type { Page } from "./bindings/Page";
import type { PanelShown } from "./bindings/PanelShown";
import type { PasteMode } from "./bindings/PasteMode";
import type { PasteResult } from "./bindings/PasteResult";
import type { Settings } from "./bindings/Settings";
import type { SettingsPatch } from "./bindings/SettingsPatch";
import type { ShortcutStatus } from "./bindings/ShortcutStatus";

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

/**
 * Runs `callback` each time the Accessibility permission is found granted or revoked
 * (`accessibility://changed`). The core checks it at every window show and every paste.
 */
export function onAccessibilityChanged(callback: (change: AccessibilityChanged) => void): Promise<UnlistenFn> {
  return listen<AccessibilityChanged>("accessibility://changed", (event) => callback(event.payload));
}

/** Runs `callback` each time the popover or the Quick Panel is shown (`panel://shown`). */
export function onPanelShown(callback: (shown: PanelShown) => void): Promise<UnlistenFn> {
  return listen<PanelShown>("panel://shown", (event) => callback(event.payload));
}

/**
 * Runs `callback` each time the tray menu's Clear history… is chosen (`menu://clear-history`, no
 * payload). The core has already shown the popover.
 */
export function onClearHistoryRequested(callback: () => void): Promise<UnlistenFn> {
  return listen("menu://clear-history", () => callback());
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

/** Shows the Preferences window, creating it when it does not exist yet. */
export async function openPreferences(): Promise<void> {
  await invoke("open_preferences");
}

/** How the Quick Panel shortcut was registered: the accelerator, and the error code if it failed. */
export function getShortcutStatus(): Promise<ShortcutStatus> {
  return invoke<ShortcutStatus>("get_shortcut_status");
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

/**
 * Puts the clip back on the pasteboard and into the app that was in front, and hides the calling
 * window. Resolves `copied_only` when the Accessibility permission is missing: the clip is on
 * the pasteboard, the window stays open. Rejects with `not_found` for an unknown id.
 */
export function pasteClip(id: string, mode: PasteMode): Promise<PasteResult> {
  return invoke<PasteResult>("paste_clip", { id, mode });
}

/** Puts the clip back on the pasteboard and hides the calling window; nothing is pasted. */
export async function copyClip(id: string): Promise<void> {
  await invoke("copy_clip", { id });
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
