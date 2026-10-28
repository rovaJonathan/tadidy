// Typed wrappers over the core's IPC commands and events. Components never call invoke
// directly (AGENTS.md §14); argument and result types come from ./bindings (ADR 0006).
import { invoke } from "@tauri-apps/api/core";

export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}

/** Hides the window that calls it (Esc in the popover). The window is hidden, not destroyed. */
export async function hideWindow(): Promise<void> {
  await invoke("hide_window");
}
