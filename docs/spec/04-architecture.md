<a id="04-architecture"></a>

# Technical architecture

<a id="overview"></a>

## Overview

```
┌──────────────────────────── Tadidy.app ─────────────────────────────┐
│                                                                     │
│  ┌──────────── WebView (Svelte 5 + TS) ────────────┐                │
│  │  Popover · Quick Panel · Preferences · Onboarding│                │
│  └───────────────▲──────────────────┬───────────────┘                │
│        events     │                  │  invoke() commands             │
│  ┌────────────────┴──────────────────▼───────────────────────────┐  │
│  │                    Tauri 2 core (Rust)                         │  │
│  │  commands.rs · tray.rs · windows.rs · shortcuts.rs · state.rs  │  │
│  └──┬─────────────┬──────────────┬───────────────┬───────────────┘  │
│     │             │              │               │                  │
│ ┌───▼──────┐ ┌────▼─────┐ ┌──────▼──────┐ ┌──────▼───────┐          │
│ │ watcher  │ │ storage  │ │   paster    │ │   purger     │          │
│ │ (thread) │ │ SQLCipher│ │ CGEvent ⌘V  │ │ (tokio task) │          │
│ └───┬──────┘ │ + FTS5   │ └──────┬──────┘ └──────────────┘          │
│     │        └────▲─────┘        │                                  │
│ ┌───▼────────────┴───────────────▼─────┐   ┌─────────────────────┐   │
│ │  macos/ : pasteboard · workspace ·   │   │ keychain (DB key)   │   │
│ │  accessibility · app_icon  (objc2)   │   │ security-framework  │   │
│ └──────────────────────────────────────┘   └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

<a id="dependencies"></a>

## Main dependencies

| Need | Crate / lib |
|---|---|
| App shell, IPC, windows, tray | `tauri` 2 (features `tray-icon`, `macos-private-api`) |
| Global shortcut | `tauri-plugin-global-shortcut` |
| Launch at login | `tauri-plugin-autostart` (or `smappservice-rs`) |
| Positioning under the tray | `tauri-plugin-positioner` (feature `tray-icon`) |
| Non-activating panel | `tauri-nspanel` (turns a `WebviewWindow` into an `NSPanel`) |
| Vibrancy | `window-vibrancy` |
| Updates (P2) | `tauri-plugin-updater` |
| AppKit / Foundation | `objc2`, `objc2-app-kit`, `objc2-foundation` |
| Keyboard simulation | `core-graphics` (`CGEvent`) |
| Accessibility permission | `AXIsProcessTrustedWithOptions` (ApplicationServices FFI) |
| Database | `rusqlite` (feature `bundled-sqlcipher-vendored-openssl`) + `rusqlite_migration` migrations |
| Keychain | `security-framework` (or `keyring`) |
| Hash | `sha2` |
| Images / thumbnails | `image` |
| IDs | `uuid` (v7, time-sortable) |
| Async / tasks | `tokio` (runtime provided by Tauri) |
| Serialization | `serde`, `serde_json` |
| Logs | `tracing`, `tracing-appender` |
| Errors | `thiserror`, `anyhow` |
| Front end | Svelte 5, TypeScript, Vite, `@tauri-apps/api`, virtual list (`@tanstack/svelte-virtual` or a home-made implementation) |

<a id="watcher"></a>

## Clipboard watcher

macOS provides **no notification** when the clipboard changes. So we have to poll `changeCount`:

1. A dedicated thread reads `NSPasteboard.generalPasteboard.changeCount` every **300 ms** (internal setting). It is a plain integer, so the cost is negligible.
2. If the value has changed:
   1. Read `NSWorkspace.sharedWorkspace.frontmostApplication` to get the bundle ID and name of the source app.
   2. If the app is excluded, ignore the change.
   3. Read the list of `types`. If it contains a concealed or transient type, ignore it.
   4. If it contains the internal marker `app.tadidy.internal`, it is a write by Tadidy: move the existing clip to the top and stop ([CAP-12](02-features.md#cap-12)).
   5. Extract the useful representations in this priority order: `file-url` > image (`png`/`tiff`) > `rtf`/`html` > `url` > `utf8-plain-text`.
   6. Compute the hash, deduplicate, insert into the database, generate the thumbnail off the polling thread.
   7. Emit the Tauri event `clips://changed` to the webview.
3. Polling is **adaptive**: 300 ms by default, 1 s when the session has been idle for more than 5 min (`CGEventSourceSecondsSinceLastEventType`), to save battery.

<a id="paster"></a>

## Paster

1. Write all the clip's representations to the clipboard, plus the `app.tadidy.internal` marker type.
2. Hide the Quick Panel or the popover. Since the Quick Panel is a non-activating NSPanel, the target app keeps focus. The popover, on the other hand, is a real window: we must remember the active app when it opens and reactivate it with `NSRunningApplication.activateWithOptions`.
3. Wait about 50 ms, then post `CGEvent` keyDown/keyUp `V` with the `Command` flag on `kCGHIDEventTap`.
4. If `AXIsProcessTrusted()` returns false, stop at step 1 (copy only) and show [SYS-03](02-features.md#sys-03).
5. "Restore previous clipboard after paste" option (P2): put the previous item back 500 ms later.

<a id="windows"></a>

## Windows

| Window | Label | Type | Behavior |
|---|---|---|---|
| Popover | `popover` | Undecorated, transparent WebviewWindow, `alwaysOnTop`, `skipTaskbar` | Created at startup **then hidden** (opens in < 100 ms). Positioned under the tray (`Position::TrayBottomCenter`). Hidden on `Focused(false)`. |
| Quick Panel | `panel` | Non-activating NSPanel (`tauri-nspanel`), `nonactivatingPanel` style, `floating` level, `canJoinAllSpaces` + `fullScreenAuxiliary` | Created at startup then hidden. Appears on the current Space, including above full-screen apps. |
| Preferences | `settings` | Standard window | Created on demand. Temporarily switches the activation policy to `Regular` so it can take focus, then back to `Accessory` when closed. |
| Onboarding | `onboarding` | Standard window | First launch only. |

<a id="ipc"></a>

## IPC commands (Rust ⇄ Svelte)

```ts
// Read
list_clips(q: { search?: string; kinds?: ClipKind[]; app?: string;
               tab: 'all'|'pinned'|'snippets'; collectionId?: string;
               cursor?: string; limit?: number }) -> Page<ClipSummary>
get_clip(id: string) -> ClipDetail            // representations + full preview
get_thumbnail(id: string) -> Uint8Array        // or asset protocol tadidy://thumb/{id}
get_stats() -> Stats

// Actions
paste_clip(id: string, mode: 'default'|'plain') -> PasteResult  // 'pasted' | 'copied_only'
copy_clip(id: string) -> void
paste_many(ids: string[], separator: string) -> PasteResult       // P2
transform_and_paste(id: string, t: Transform) -> PasteResult      // P2
delete_clips(ids: string[]) -> void
toggle_pin(id: string) -> boolean
save_snippet(input: { id?: string; title: string; content?: string; collectionId?: string }) -> Clip
update_clip_text(id: string, text: string) -> void                // P2
clear_history(keepPinned: boolean) -> number
reveal_in_finder(id: string) / open_url(id: string)

// Collections
list_collections() / create_collection(name) / rename_collection(id, name)
delete_collection(id) / reorder_collections(ids)

// Settings & system
get_settings() -> Settings ; set_settings(patch: Partial<Settings>) -> Settings
list_running_apps() -> AppInfo[] ; add_excluded_app(bundleId) ; remove_excluded_app(bundleId)
check_accessibility() -> boolean ; open_accessibility_settings()
export_history(path) -> number ; import_history(path) -> number
```

**Events emitted by Rust**: `clips://changed` (`{ added?: id, updated?: id, removed?: id[] }`), `settings://changed`, `accessibility://changed`, `panel://shown` (to reset the search and the selection).

<a id="data-model"></a>

## Data model (SQLite / SQLCipher)

```sql
CREATE TABLE clips (
  id              TEXT PRIMARY KEY,          -- UUID v7
  kind            TEXT NOT NULL,             -- 'text'|'rich'|'image'|'file'|'url'|'color'
  content_hash    TEXT NOT NULL UNIQUE,      -- SHA-256 of the primary representation
  plain_text      TEXT,                      -- searchable text (text, image alt, file names, URL)
  preview         TEXT,                      -- truncated preview (300 chars)
  title           TEXT,                      -- snippets
  source_bundle   TEXT,
  source_name     TEXT,
  byte_size       INTEGER NOT NULL,
  width           INTEGER, height INTEGER,   -- images
  pinned          INTEGER NOT NULL DEFAULT 0,
  pin_order       REAL,
  collection_id   TEXT REFERENCES collections(id) ON DELETE SET NULL,
  copy_count      INTEGER NOT NULL DEFAULT 1,
  created_at      INTEGER NOT NULL,          -- ms epoch, first copy
  last_copied_at  INTEGER NOT NULL,          -- main sort key
  last_used_at    INTEGER                    -- last paste from Tadidy
);
CREATE INDEX idx_clips_recent ON clips(pinned, last_copied_at DESC);
CREATE INDEX idx_clips_kind   ON clips(kind, last_copied_at DESC);
CREATE INDEX idx_clips_last_copied ON clips(last_copied_at DESC, id DESC);  -- unfiltered newest-first list, total order

CREATE TABLE representations (
  clip_id  TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
  uti      TEXT NOT NULL,                    -- public.rtf, public.png, public.file-url…
  data     BLOB NOT NULL,
  PRIMARY KEY (clip_id, uti)
);

CREATE TABLE thumbnails (
  clip_id  TEXT PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
  png      BLOB NOT NULL
);

CREATE TABLE collections (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, sort REAL NOT NULL
);

CREATE TABLE excluded_apps (
  bundle_id TEXT PRIMARY KEY, name TEXT NOT NULL
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY, value TEXT NOT NULL  -- JSON
);

CREATE TABLE app_icons (                      -- source app icon cache
  bundle_id TEXT PRIMARY KEY, png BLOB NOT NULL
);

CREATE VIRTUAL TABLE clips_fts USING fts5(
  plain_text, title, source_name,
  content='clips', content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);
-- + triggers on clips keep clips_fts in sync: AFTER INSERT, AFTER DELETE, and AFTER UPDATE OF
--   plain_text, title, source_name (only when one of them changed, so a bump rewrites nothing)
```

**Location**: `~/Library/Application Support/app.tadidy/tadidy.db` (+ WAL).

**"Everything in the database" choice**: images are stored as BLOBs in the encrypted database rather than as files. This gives a single encryption mechanism, atomic deletes and simple backups. The trade-off is that the database grows, hence the limits [CAP-13](02-features.md#cap-13) and [HIS-04](02-features.md#his-04), and an incremental `VACUUM` (`auto_vacuum = INCREMENTAL`).

<a id="encryption"></a>

## Encryption

- The database is encrypted with **SQLCipher** (AES-256, `PRAGMA key`).
- On first launch, a random 256-bit key is generated and stored in the **macOS Keychain** (`kSecClassGenericPassword`, service `app.tadidy`, accessibility `AfterFirstUnlockThisDeviceOnly`).
- The key never leaves the process memory and is never logged.
- If the key is missing while the database exists (Keychain reset), offer to **reset the history**. No recovery is possible.
- The JSON export is **in plain text**, and the export dialog says so.
- ⚠️ Keychain access is tied to the code signature: see [Permissions, signing, distribution](06-delivery.md#permissions-signing) (stable signature).

<a id="project-layout"></a>

## Project layout

```
tadidy/
├─ AGENTS.md · CLAUDE.md          # working contract for agents and humans
├─ DESIGN.md · GLOSSARY.md · PROGRESS.md
├─ lefthook.yml                   # local git hooks (no CI)
├─ tickets/                       # TAD-xxx.md, one file per ticket
├─ docs/ spec/ prototype/ adr/ plans/ solutions/ runbooks/ guides/ archive/
├─ scripts/                       # repo scripts (node:test), check.sh
├─ .claude/skills/                # project skills
├─ package.json · vite.config.ts · svelte.config.js · tsconfig.json
├─ src/                              # Svelte 5 front end
│  ├─ main.ts
│  ├─ routes/ (or multiple Vite entries)
│  │  ├─ popover.html / panel.html / settings.html / onboarding.html
│  ├─ lib/
│  │  ├─ api.ts                      # typed wrappers for invoke() + listen()
│  │  ├─ types.ts                    # ClipSummary, Settings… (generated via ts-rs or specta)
│  │  ├─ stores/ clips.svelte.ts · settings.svelte.ts   # $state runes
│  │  ├─ components/
│  │  │  ├─ ClipList.svelte · ClipRow.svelte · SearchBar.svelte
│  │  │  ├─ Tabs.svelte · KindFilters.svelte · Preview.svelte
│  │  │  ├─ ContextMenu.svelte · EmptyState.svelte · PermissionBanner.svelte
│  │  └─ keyboard.ts                 # centralized shortcut handling
│  └─ styles/ tokens.css
└─ src-tauri/
   ├─ Cargo.toml · tauri.conf.json · capabilities/default.json
   ├─ Info.plist                     # LSUIElement = true
   ├─ icons/ (app + tray template @1x/@2x)
   └─ src/
      ├─ main.rs · lib.rs            # setup, plugins, activation policy
      ├─ commands.rs                 # #[tauri::command]
      ├─ state.rs                    # AppState (db pool, settings, last_active_app)
      ├─ tray.rs · windows.rs · shortcuts.rs
      ├─ watcher.rs                  # polling thread
      ├─ paster.rs                   # pasteboard write + CGEvent
      ├─ purger.rs                   # expiry, quota, history limit
      ├─ transforms.rs               # P2
      ├─ storage/ mod.rs · migrations.rs · clips.rs · collections.rs · settings.rs · fts.rs
      ├─ crypto.rs                   # Keychain
      └─ macos/ pasteboard.rs · workspace.rs · accessibility.rs · app_icon.rs
```
