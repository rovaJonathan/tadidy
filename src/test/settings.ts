// A fake core for the preferences tests: it keeps the settings the way the real one does, answers
// `get_settings`, `set_settings`, `get_shortcut_status` and `open_preferences`, and announces every change with
// `settings://changed`. Synthetic values only (AGENTS.md §2).
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppError } from "../lib/bindings/AppError";
import type { Settings } from "../lib/bindings/Settings";
import type { SettingsPatch } from "../lib/bindings/SettingsPatch";
import type { ShortcutStatus } from "../lib/bindings/ShortcutStatus";

export const defaultSettings: Settings = {
  historySize: 500,
  confirmBeforeDelete: true,
  launchAtLogin: false,
  quickPanelShortcut: "Cmd+Shift+V",
};

export const defaultShortcutStatus: ShortcutStatus = { accelerator: "Cmd+Shift+V", error: null };

export function fakeSettingsCore(initial: { settings?: Partial<Settings>; shortcut?: Partial<ShortcutStatus> } = {}) {
  const core = {
    settings: { ...defaultSettings, ...initial.settings } as Settings,
    shortcut: { ...defaultShortcutStatus, ...initial.shortcut } as ShortcutStatus,
    /** Every command invoked, in order. */
    commands: [] as string[],
    /** The patch of every `set_settings`, refused or not. */
    patches: [] as SettingsPatch[],
    /** When set, `set_settings` rejects with it and changes nothing. */
    refuse: undefined as AppError | undefined,
    /** When set, `get_settings` rejects with it. */
    unreadable: undefined as AppError | undefined,
    /** When set, `get_shortcut_status` rejects with it. */
    statusUnreadable: undefined as AppError | undefined,
  };
  mockIPC(
    (cmd, args) => {
      core.commands.push(cmd);
      switch (cmd) {
        case "get_settings":
          if (core.unreadable) throw core.unreadable;
          return core.settings;
        case "set_settings": {
          const { patch } = args as { patch: SettingsPatch };
          core.patches.push(patch);
          if (core.refuse) throw core.refuse;
          core.settings = { ...core.settings, ...definedOnly(patch) };
          if (patch.quickPanelShortcut !== undefined)
            core.shortcut = { accelerator: patch.quickPanelShortcut, error: null };
          void emit("settings://changed", core.settings);
          return core.settings;
        }
        case "get_shortcut_status":
          if (core.statusUnreadable) throw core.statusUnreadable;
          return core.shortcut;
        default:
          return undefined;
      }
    },
    { shouldMockEvents: true },
  );
  return core;
}

function definedOnly(patch: SettingsPatch): Partial<Settings> {
  return Object.fromEntries(Object.entries(patch).filter(([, value]) => value !== undefined)) as Partial<Settings>;
}
