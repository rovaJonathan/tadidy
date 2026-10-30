// The preferences as the windows see them. The core owns them; this class mirrors `get_settings`, follows
// `settings://changed` and sends the changes the user makes.
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getSettings, onSettingsChanged, setSettings } from "../api";
import type { AppError } from "../bindings/AppError";
import type { Settings } from "../bindings/Settings";
import type { SettingsPatch } from "../bindings/SettingsPatch";

export class SettingsStore {
  /** `null` until the first `get_settings` answers. */
  settings = $state.raw<Settings | null>(null);
  /** Why the last change was refused; the next change that succeeds clears it. */
  error = $state.raw<AppError | null>(null);

  async start(): Promise<void> {
    throw new Error("not implemented");
  }

  destroy(): void {
    throw new Error("not implemented");
  }

  async patch(patch: SettingsPatch): Promise<boolean> {
    void [patch, getSettings, onSettingsChanged, setSettings, {} as UnlistenFn];
    throw new Error("not implemented");
  }
}
