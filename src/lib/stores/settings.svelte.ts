// The preferences as the windows see them. The core owns them; this class mirrors `get_settings`, follows
// `settings://changed` and sends the changes the user makes.
import { getSettings, onSettingsChanged, setSettings } from "../api";
import type { AppError } from "../bindings/AppError";
import type { Settings } from "../bindings/Settings";
import type { SettingsPatch } from "../bindings/SettingsPatch";

export class SettingsStore {
  /** `null` until the first `get_settings` answers. */
  settings = $state.raw<Settings | null>(null);
  /** Why the last change was refused; the next change that succeeds clears it. */
  error = $state.raw<AppError | null>(null);

  #destroyed = false;
  #unlisten: (() => void) | undefined;

  /** Loads the settings and follows `settings://changed` until `destroy`. */
  async start(): Promise<void> {
    const listening = onSettingsChanged((settings) => {
      this.settings = settings;
    }).then((unlisten) => {
      if (this.#destroyed) unlisten();
      else this.#unlisten = unlisten;
    });
    const [settings] = await Promise.all([getSettings(), listening]);
    if (!this.#destroyed) this.settings = settings;
  }

  /** Releases the event listener; whatever the core announces later is ignored. */
  destroy(): void {
    this.#destroyed = true;
    this.#unlisten?.();
    this.#unlisten = undefined;
  }

  /** Sends a change. Resolves `false`, with `error` set, when the core refuses it: nothing changed then. */
  async patch(patch: SettingsPatch): Promise<boolean> {
    try {
      this.settings = await setSettings(patch);
      this.error = null;
      return true;
    } catch (error) {
      this.error = isAppError(error) ? error : { code: "unknown", message: "The setting could not be saved." };
      return false;
    }
  }
}

function isAppError(error: unknown): error is AppError {
  return typeof error === "object" && error !== null && "code" in error && "message" in error;
}
