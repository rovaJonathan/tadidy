# Manual QA

The macOS layer (`src-tauri/src/macos/`: pasteboard, frontmost app, synthetic ⌘V, Accessibility, Keychain, panels) cannot run in a unit test (AGENTS.md §9). Each behaviour it adds is checked by hand with the steps below, **on a signed build** (`pnpm tauri build`, [signing runbook](../runbooks/signing-identity.md)) unless a step says `pnpm tauri dev` is enough. The pull request of the ticket reports the result of its steps (✅ / ❌ + note).

A ticket that adds behaviour to the macOS layer adds its steps here, under its own heading, in the same pull request.

## How to report

In the pull request, section *Verification*:

```
Manual QA (docs/guides/manual-qa.md#tad-004) — signed build, macOS 15.1, Apple Silicon
| Step | Result |
|---|---|
| 1 | ✅ |
| 2 | ❌ the source app is Finder instead of TextEdit → TAD-0xx |
```

## Steps by ticket

<!-- Each ticket adds a "### TAD-xxx — <title>" heading with numbered steps: action → expected result. -->

### TAD-003 — Tray icon, tray menu and hidden popover window

`pnpm tauri dev` is enough (no Accessibility or signing involved). Start the app from a terminal and keep it running between steps 1 and 10.

| # | Action | Expected |
|---|---|---|
| 1 | Run `pnpm tauri dev`. | The clipboard icon appears in the menu bar. No Dock icon; Tadidy is absent from the `⌘Tab` switcher. |
| 2 | Switch System Settings › Appearance between Light and Dark, and look at the icon at Retina scale (zoom in a screenshot). | The glyph is black on a light menu bar and white on a dark one (template image), sharp, 18 pt high. |
| 3 | Left-click the icon. | The popover appears right below the icon, horizontally centred on it, about 380 × 520 pt, translucent with 10 pt corners, showing the placeholder "Tadidy". It shows without a perceptible delay (< 100 ms). |
| 4 | With the popover open, press `Esc`, without clicking inside first. | The popover hides (it has the focus). |
| 5 | Open it again, then click on the desktop or in another app. | The popover hides. |
| 6 | Open it, then click the tray icon. | The popover closes and does **not** reopen. |
| 6b | Open it, then click the tray icon slowly (hold the button more than 250 ms before releasing). | The popover closes and does not reopen. If it reopens, the focus loss happens on mouse down and the debounce must also be stamped on the press: report it. |
| 7 | Hide the popover by clicking elsewhere, wait about a second, then click the icon. | The popover opens again. |
| 8 | Right-click the icon. | A native menu: Preferences… `⌘,`, Clear history…, separator, About Tadidy, Check for updates…, separator, Quit Tadidy `⌘Q`. Only Quit Tadidy was enabled at TAD-003; the other items are wired by [TAD-018](#tad-018--tray-menu-actions-preferences-clear-history-about-quit). |
| 9 | With the popover visible, hold `⌘Tab` and look at the Dock. | No Tadidy entry in the switcher, no Dock icon. |
| 10 | Choose Quit Tadidy (or `⌘Q` with the menu open). | The icon disappears and `pgrep -x tadidy` prints nothing. |

### TAD-004 — Pasteboard and workspace layer

Run a developer probe; no signing or Accessibility is involved. The probe prints types, byte sizes and the frontmost app, never the clipboard data. Start it from a terminal and keep it running for steps 1 to 6. It polls from its own thread while the main thread runs the run loop, like the watcher inside Tauri: `frontmostApplication` is only refreshed while the main run loop runs, so a probe without one keeps reporting the app it started in.

```
cargo run --manifest-path src-tauri/Cargo.toml --example pasteboard_probe
```

| # | Action | Expected |
|---|---|---|
| 1 | Start the probe. | `polling every 300 ms`, then one `change count N` block listing the current types, and `frontmost app` with the terminal's name and bundle id. |
| 2 | Select a few words in TextEdit and copy them with `⌘C`. | `change count` grows by one, `public.utf8-plain-text` is listed with the byte size of the selection, and the frontmost app is TextEdit (`com.apple.TextEdit`). |
| 3 | Copy a selection in Safari. | `public.html`, `public.rtf` or `public.utf8-plain-text` are listed; the frontmost app is Safari (`com.apple.Safari`). |
| 4 | Copy a file in the Finder (`⌘C` on a file). | `public.file-url` is listed; the frontmost app is Finder. |
| 5 | Open the PNG the probe printed after `icon:` in Preview (`open <path>`). | A 64 × 64 px image of the app icon, transparent corners, sharp (`sips -g pixelWidth -g dpiWidth <path>` → 64 and 144). |
| 6 | Leave the probe idle for 60 s without copying and watch it in Activity Monitor (Memory column). | The memory stays flat: no growth, nothing printed after the first blocks. |
| 7 | Stop the probe (`Ctrl-C`). | The process exits. |

### TAD-005 — Key sender and Accessibility permission

Run a developer probe; no signed build is needed. macOS ties the Accessibility grant to the program that launched the probe (the terminal), not to the probe binary, so the grant goes to the terminal you run it from. The probe sends `⌘V`, which pastes whatever is on the clipboard: copy a harmless sentence first. It never reads or prints the clipboard.

```
cargo run --manifest-path src-tauri/Cargo.toml --example paste_probe
cargo run --manifest-path src-tauri/Cargo.toml --example paste_probe -- --request
```

Open TextEdit with an empty document and the insertion point in it before each run, then put TextEdit in front within 3 s of starting the probe.

| # | Action | Expected |
|---|---|---|
| 1 | In *System Settings → Privacy & Security → Accessibility*, make sure the terminal is **off** (or absent). Run the probe, then bring TextEdit to the front. | `trusted: false`, then `not trusted: no key event sent`. Nothing is pasted into TextEdit. |
| 2 | Run the probe with `-- --request`. | System Settings opens on *Privacy & Security → Accessibility*, and the terminal is listed there. The probe prints `requested; System Settings opened on Accessibility` and exits. |
| 3 | Turn the terminal **on** in that pane. Run the probe and bring TextEdit to the front within 3 s. | `trusted: true`, `sent Command+V`; the copied sentence appears in TextEdit, which keeps focus (no other app comes forward). |
| 4 | Switch the keyboard layout to AZERTY (*System Settings → Keyboard → Input Sources*) and repeat step 3. | The sentence is pasted: `⌘V` is not read as another shortcut. |
| 5 | Start the probe, bring TextEdit to the front, then hold `⇧` until the sentence appears (about 3 s after the start), and release it. | The sentence is pasted. The event carries `Command` only, so the held `⇧` is not merged in. If it were, the app would receive `⌘⇧V`, which does nothing in TextEdit: an empty document means the step failed. |
| 6 | Turn the terminal off again in the Accessibility pane. | The state is back to step 1; restore your own layout and settings. |

### TAD-006 — Non-activating Quick Panel and global shortcut

`pnpm tauri dev` is enough for steps 1 to 9. Step 10 (full-screen app) is best run on a signed build. Keep the app running between steps 1 and 11. Use two screens for step 3 when you have them.

| # | Action | Expected |
|---|---|---|
| 1 | Run `pnpm tauri dev`, then press `⌘⇧V` from any app. | The Quick Panel (about 640 × 460 pt, translucent, 10 pt corners, placeholder "Tadidy") appears in the middle of the screen, without a perceptible delay. |
| 2 | Press `⌘⇧V` again. | The panel hides. |
| 3 | Move the pointer to the other screen, press `⌘⇧V`. | The panel appears centred on the screen under the pointer. |
| 4 | Open TextEdit, type a few words, leave the caret in the middle of them, press `⌘⇧V`, then type `abc`. | TextEdit stays the active app: its menu bar stays visible and its title is still highlighted. The caret does not move and no `abc` appears in the document (the keys go to the panel). |
| 5 | With the panel open over TextEdit, press `Esc`. | The panel hides. TextEdit is still the active app and its caret blinks where it was. |
| 6 | Open the panel, then click in the TextEdit window. | The panel hides and the click reaches TextEdit (the caret moves to the click). |
| 7 | Switch to another Space (`⌃→`), press `⌘⇧V`. | The panel appears on that Space, not on the previous one. |
| 8 | Hold `⌘Tab` with the panel visible, and look at the Dock. | No Tadidy entry in the switcher, no Dock icon. |
| 9 | Left-click the tray icon to show the popover, then press `⌘⇧V`. | The panel appears. The popover hides (it lost the key window), and pressing `⌘⇧V` again hides the panel. |
| 10 | Put Safari in full screen (own Space), press `⌘⇧V`. | The panel appears above Safari on that Space; Safari stays full screen and active. |
| 11 | Quit Tadidy. Make another app hold `⌘⇧V` first (for example a Hammerspoon `hs.hotkey.bind({"cmd","shift"}, "v", …)`), then start Tadidy. | Tadidy starts and keeps running with its tray icon, and no warning is logged: macOS does not refuse a combination another app already holds (known limit, TAD-006 *Notes*). Note which app receives `⌘⇧V`. |

### TAD-007 — Encrypted database foundation

`pnpm tauri dev` is enough for steps 1 to 5 (the Keychain item belongs to whichever binary created it, so macOS may ask for the login password once per rebuild). Use a throwaway data folder state: if a `tadidy.db` already exists from an earlier run, delete the folder and the Keychain item first (`security delete-generic-password -s app.tadidy -a database-key`).

```
cargo run --manifest-path src-tauri/Cargo.toml --example keychain_probe
```

| # | Action | Expected |
|---|---|---|
| 1 | Run the `keychain_probe` example. | `before: Ok(false)`, `set: Ok(())`, `get: found true, same key: true`, `set again: Ok(Err(AlreadyExists)), key unchanged: true`, `cleanup: Ok(())`. It uses the service `app.tadidy.probe` and prints no key. |
| 2 | Delete `~/Library/Application Support/app.tadidy/` and the item `app.tadidy` / `database-key` if they exist, then run `pnpm tauri dev`. | The app starts as before (tray icon). `tadidy.db` (+ `-wal`, `-shm`) exists in the folder. |
| 3 | In Keychain Access, search `app.tadidy`. | One "application password": name `app.tadidy`, account `database-key`. |
| 4 | `sqlite3 ~/Library/Application\ Support/app.tadidy/tadidy.db .tables` | `Error: file is not a database`. `head -c 16 tadidy.db` does not print `SQLite format 3`. |
| 5 | Quit the app, run `pnpm tauri dev` again. | It starts, no new Keychain item (still one), no error in the terminal. |
| 6 | Quit the app. Delete the Keychain item, keep `tadidy.db`. Run `RUST_LOG=tadidy_lib=info pnpm tauri dev`. | The terminal logs `storage_key_missing`; the tray icon still works; `shasum tadidy.db` is the same as before the launch; no new Keychain item. |

### TAD-008 — Stable signing identity and universal build

Needs the `Tadidy Dev` identity from [docs/runbooks/signing-identity.md](../runbooks/signing-identity.md) and `export APPLE_SIGNING_IDENTITY="Tadidy Dev"` in the shell. `APP` below is `src-tauri/target/universal-apple-darwin/release/bundle/macos/Tadidy.app`. Steps 1 to 5 need only this ticket. Step 7 needs a build that shows the permission state (a UI ticket): TAD-019 runs it. Step 8 needs TAD-007 and can be run today.

| # | Action | Expected |
|---|---|---|
| 1 | `security find-identity -v -p codesigning` | Lists `"Tadidy Dev"` as a valid identity. |
| 2 | `pnpm build:universal` | Succeeds and produces `APP` and a `.dmg` next to it, under `bundle/dmg/`. |
| 3 | `codesign -dv --verbose=2 "$APP" 2>&1 \| grep -E 'Authority\|Signature'`, then `codesign --verify --deep --strict "$APP"` | `Authority=Tadidy Dev`, no `Signature=adhoc`; the verify command prints nothing and exits 0. |
| 4 | `lipo -archs "$APP/Contents/MacOS/"*` | `x86_64 arm64`. |
| 5 | `codesign -d -r- "$APP" 2>&1 \| grep designated > /tmp/dr-1.txt`; change any source file (a comment is enough); `pnpm build:universal`; the same command into `/tmp/dr-2.txt`; `diff /tmp/dr-1.txt /tmp/dr-2.txt` | No difference: the designated requirement does not depend on the build. |
| 6 | Unset `APPLE_SIGNING_IDENTITY`, rebuild, and run `codesign -dv --verbose=2 "$APP" 2>&1 \| grep Signature` | `Signature=adhoc`: this is the failure the identity prevents. Export the variable again. |
| 7 | Install build N in `/Applications`, grant it Accessibility, install build N+1 over it, launch it. | *System Settings → Privacy & Security → Accessibility* still lists Tadidy enabled, and `check_accessibility` returns `true` (TAD-019). |
| 8 | Start from a clean state (no `~/Library/Application Support/app.tadidy/`, no `app.tadidy` / `database-key` item). Copy `APP` to `/tmp/N.app`, change a source file, `pnpm build:universal`, copy the new `APP` to `/tmp/N1.app`. Run `/tmp/N.app/Contents/MacOS/tadidy`, wait 5 s, quit it; run `/tmp/N1.app/Contents/MacOS/tadidy`, wait 5 s. Then `pgrep -x SecurityAgent`, `security dump-keychain ~/Library/Keychains/login.keychain-db \| grep -c '"svce"<blob>="app.tadidy"'` and `head -c 16 ~/Library/Application\ Support/app.tadidy/tadidy.db \| xxd` after each run. | Build N creates `tadidy.db` and one Keychain item; its header is not `SQLite format 3`. Build N+1 starts without any Keychain dialog (no `SecurityAgent`), still one item, the same header bytes (same key), and logs no `storage_key_missing`. Delete the item and the folder afterwards. |

### TAD-010 — Clipboard watcher for text clips

Needs the signed build of TAD-008 and the terminal, because the only trace of a clip is the log line (id, kind, byte size and duration; never the text). `APP` is the `Tadidy.app` of the build. Start it with `RUST_LOG=tadidy_lib=info "$APP/Contents/MacOS/tadidy"` and keep the terminal visible. The history database is encrypted and no list exists before TAD-013: the source app and its icon are checked there (the automated tests cover them with fakes).

| # | Action | Expected |
|---|---|---|
| 1 | Copy a sentence in TextEdit (`⌘C`), then start the app. | The tray icon appears and no `clip recorded` line is printed: what was on the clipboard at launch is not captured. |
| 2 | In TextEdit, select a few words and press `⌘C`. | At once (< 500 ms after the keystroke) one line `clip recorded id=<uuid> kind="text" bytes=<N> elapsed_ms=<M>`, `N` the byte size of the selection and `M` a few milliseconds. The line does not contain the copied words. |
| 3 | Press `⌘C` again on the same selection. | A new `clip recorded` line with the **same** `id` as step 2: the clip was bumped, not duplicated. |
| 4 | Select other words and press `⌘C`. | A `clip recorded` line with a **different** `id`. |
| 5 | Select only a space or an empty line in TextEdit and press `⌘C`; then copy a screenshot to the clipboard (`⌃⌘⇧4`). | No line is printed for either (empty text and images are not captured in this phase). |
| 6 | Quit the app and start it again with the same clipboard content. | No `clip recorded` line at launch. |
| 7 | With the app running, leave the Mac idle (no keyboard or mouse) for 6 min and watch `tadidy` in Activity Monitor (CPU column). | CPU stays under 0.5 %, also after the 5 min mark when the poll slows from 300 ms to 1 s. |
| 8 | Follow step 6 of [TAD-007](#tad-007--encrypted-database-foundation) (Keychain item deleted, `tadidy.db` kept) and start the app with `RUST_LOG=tadidy_lib=info`. | The terminal logs `storage_key_missing` and `the clipboard watcher is not started: no history database`; copying text prints no `clip recorded` line; the tray icon still works. |

Until CAP-10 and CAP-11 land (phase 2), text copied from a password manager is recorded too (encrypted in the database): step 2 with a password from the password manager prints a `clip recorded` line.

### TAD-015 — Paste and copy commands

Run a developer probe; no signed build is needed. It runs the real `Paster` (pasteboard, workspace, `⌘V`, Accessibility) on a synthetic sentence, `Tadidy paste flow probe`, kept in a throwaway database with a fixed key (never the Keychain). The Accessibility grant goes to the terminal you run it from, as for [TAD-005](#tad-005--key-sender-and-accessibility-permission). The probe prints the app it recorded, which window it would hide, and the result. It never reads or prints the clipboard.

```
cargo run --manifest-path src-tauri/Cargo.toml --example paste_flow_probe [-- --panel]
```

Open TextEdit with an empty document and the insertion point in it before each run. The probe waits 3 s so you can put the target app in front, records it, then waits 3 s more before pasting. Without `--panel` the origin is the popover (it re-activates the recorded app first); with `--panel` it is the Quick Panel (it activates nothing).

| # | Action | Expected |
|---|---|---|
| 1 | Turn the terminal **off** in *System Settings → Privacy & Security → Accessibility*. Run `-- --panel` and keep TextEdit in front. | `recorded TextEdit (com.apple.TextEdit)`, `result: CopiedOnly`, no `hide` line. Nothing is typed in TextEdit. Press `⌘V` yourself: `Tadidy paste flow probe` is pasted (the clip is on the clipboard). |
| 2 | Turn the terminal **on**. Run `-- --panel` and keep TextEdit in front. | `hide the Panel window`, `result: Pasted`; the sentence appears in TextEdit, which stays the front app (the Quick Panel never activates anything). |
| 3 | Same as step 2, with a text field of a Safari page (a search box, say) in front instead of TextEdit. | The sentence appears in the Safari text field. |
| 4 | Run without `--panel` and keep TextEdit in front. | `hide the Popover window`, `result: Pasted`, no crash. The re-activation itself is **not** checked here: since macOS 14 an app can only bring another one forward while it is the active app (cooperative activation), and a command-line probe never is. If another app is in front at paste time, the sentence goes there (Finder turns it into a text clipping in its folder: delete it). Re-activation is checked from the real popover in TAD-016, where Tadidy has the focus. |
| 5 | Run without `--panel`; during the second wait, quit TextEdit (the recorded app) and leave another app in front. | `result: Pasted` and no crash: the app to re-activate is gone, nothing is brought forward, and the keystroke goes to the app in front. |

If step 2 or 3 sometimes misses the keystroke on a loaded machine, raise `PASTE_DELAY` in `src-tauri/src/paster.rs` (50 ms) and note it in `PROGRESS.md`.

The same flows from the real popover and Quick Panel are run in [TAD-016](#tad-016--popover-and-quick-panel-host-the-clip-browser): MVP criterion 3 is run there.

### TAD-016 — Popover and Quick Panel host the clip browser

Needs a **signed build** with Accessibility granted to it ([TAD-008](#tad-008--stable-signing-identity-and-universal-build)): `pnpm tauri build`, then run the app from `/Applications`. Copy a few synthetic sentences first (`Tadidy flow A`, `Tadidy flow B`, `Tadidy flow C`, copied in that order), so the newest clip is `Tadidy flow C` and the second is `Tadidy flow B`. Open TextEdit with an empty document.

| # | Action | Expected |
|---|---|---|
| 1 | In TextEdit press `⌘⇧V`, `↓`, `Return`. | `Tadidy flow B` is pasted into TextEdit, the Quick Panel is gone and TextEdit keeps the focus (MVP criterion 3). |
| 2 | Same in a text field of a Safari page. | The sentence is pasted into the Safari field. |
| 3 | Click the tray icon, press `↓`, `Return`. | The popover hides, TextEdit comes back to the front and `Tadidy flow B` is pasted (the re-activation that [TAD-015](#tad-015--paste-and-copy-commands) step 4 could not check). |
| 4 | Open the Quick Panel, type `flow`, press `↓` twice, press `Esc` twice (the first clears the search). Open it again. | The search is empty, the first row is selected and the search field has the focus. Same for the popover. |
| 5 | Open the Quick Panel, `⌥Return` on the first clip, then paste with `⌘V` in TextEdit. | The panel hides; the clip is on the clipboard and pastes. Nothing was typed by Tadidy. |
| 6 | Open the popover and the panel one after the other, copy `Tadidy flow D` in TextEdit. | Both show it first when they are shown again (they refresh on `clips://changed`). |
| 7 | Click `⚙` in the popover, then `⌘,` in the Quick Panel. | The Preferences window opens (and is brought forward, not duplicated). |
| 8 | In the Preferences, turn *Confirm before delete* off. Then `⌘⌫` in either window. | The selected clip is deleted at once, without a dialog. Turn it back on: `⌘⌫` asks again. |
| 9 | With the confirmation on, `⌘⌫`, tick *Don’t ask again*, *Delete*. | The clip is deleted and the Preferences show *Confirm before delete* off. |
| 10 | Revoke Accessibility for Tadidy (*System Settings → Privacy & Security → Accessibility*), open the Quick Panel, `Return`. | The clip is copied only (`⌘V` pastes it), the panel **stays open** and the banner "Auto-paste disabled — clips are copied only." shows above the footer. |
| 11 | With the banner showing, click *Enable*. | System Settings opens on the Accessibility pane. |
| 12 | Look at the footer of both windows. | Count at the left, `⚙` at the right. The Quick Panel also shows `↩ Paste` and `⌥↩ Copy` in the middle; the popover shows no hints. |

### TAD-017 — Preferences window: General and Shortcuts

Steps 1 to 5 and 9 to 14 only need `pnpm tauri dev`. Steps 6 to 8 (launch at login) need a **signed build**: `SMAppService` refuses an unsigned or bare binary, which step 6 checks too. Nothing opens the window from the UI yet (the tray item is TAD-018, `⌘,` in the browser TAD-016): open it from the popover's web inspector. Left-click the tray icon, right-click in the popover → *Inspect Element* → *Console*, then run:

```
await window.__TAURI_INTERNALS__.invoke("open_preferences")
```

| # | Action | Expected |
|---|---|---|
| 1 | Put TextEdit in front, run the command above. | A 640 × 480 window opens in the middle of the screen **in front of TextEdit and focused** (typing goes to it). No title text; the traffic lights sit over the left sidebar, which lists **General** and **Shortcuts** only. Tadidy now shows in the Dock and in `⌘Tab`. |
| 2 | Run the command again while the window is open (and once more after clicking another app). | The same window comes to the front; there is never a second one. |
| 3 | Look at the green button; drag the window by the sidebar's top area and by the "General" header. | The green button is disabled and the window cannot be resized. Both areas drag the window. |
| 4 | Close with the red button, then reopen it with the command; close it with `⌘W`. | The Dock icon and the `⌘Tab` entry disappear on each close; the tray icon stays and Tadidy keeps running. Reopening shows the window again. |
| 5 | Switch the system to dark mode and back with the window open. | The window follows the theme (sidebar, rows, toggles). Light and dark both match the prototype. |
| 6 | **Signed build.** Check *Launch at login*. Then run `pnpm tauri dev` (unsigned) and check it there. | Signed: the box stays checked and Tadidy is listed and enabled in *System Settings → General → Login Items & Extensions*. Unsigned: the box unchecks itself and "Launch at login could not be changed." shows under the label. |
| 7 | **Signed build**, *Launch at login* on. Log out and back in (or restart). | Tadidy starts by itself: its tray icon is there and no window opens (MVP criterion 7). |
| 8 | **Signed build.** In *System Settings → General → Login Items & Extensions*, switch Tadidy off (or remove it), quit and start Tadidy, open Preferences. | *Launch at login* is **unchecked**: the system's state wins. |
| 9 | Pick *10 clips*, then *Unlimited*, then *500 clips* in *History size*. Quit and start Tadidy. | Each choice is kept after the restart. With more than 10 clips stored, choosing *10 clips* trims the history at once. |
| 10 | Shortcuts tab → click the recorder → press `⌃⌥V`. Switch to TextEdit and press `⌃⌥V`; then press `⌘⇧V`. | The recorder shows `⌃⌥V`. `⌃⌥V` opens the Quick Panel and `⌘⇧V` no longer does. |
| 11 | Quit and start Tadidy. | `⌃⌥V` still opens the Quick Panel (the stored shortcut is registered at startup) and the tab shows `⌃⌥V`. |
| 12 | Click the recorder and press `V` alone, `⇧` alone, then `Esc`. Click it again and press `⌘Space`, then `⌘⇧4`. | The first two are ignored; `Esc` restores `⌃⌥V`. `⌘Space` and `⌘⇧4` show the red ring and "This shortcut is already used by macOS or another app. Choose another one."; `⌃⌥V` still opens the panel. |
| 13 | Click the ⓧ in the recorder. | `⌘⇧V` is back, the error is gone and `⌘⇧V` opens the Quick Panel. |
| 14 | Quit Tadidy, make another app hold `⌘⇧V` (see [TAD-006](#tad-006--non-activating-quick-panel-and-global-shortcut) step 11), start Tadidy, open the Shortcuts tab. | Known limit: macOS does not refuse a combination another app holds, so no error shows and both apps receive the key. Note which one. The error text itself is checked by steps 12 and by the tests. |

Run the steps on a screen where the pointer is not on the primary display too (step 1): the window opens centred on the primary display, as `.center()` does.

### TAD-018 — Tray menu actions: preferences, clear history, about, quit

Steps 1 to 6 and 9 only need `pnpm tauri dev`. Step 7 (the copyright line) needs a **bundled build** (`pnpm tauri build`): the line comes from the bundle's `Info.plist`. Copy a few synthetic sentences first (`Tadidy flow A`, `Tadidy flow B`, `Tadidy flow C`) and pin none of them, so the history holds three clips. Keep TextEdit in front between steps.

| # | Action | Expected |
|---|---|---|
| 1 | Right-click the tray icon. | Preferences… `⌘,`, Clear history…, separator, About Tadidy, Check for updates… (greyed), separator, Quit Tadidy `⌘Q`. Every item but Check for updates… is enabled. |
| 2 | With TextEdit in front, choose Preferences…. | The Preferences window opens **in front of TextEdit and focused** (typing goes to it). Choosing Preferences… again brings the same window forward; there is never a second one. |
| 3 | Close the Preferences. Choose Clear history… with the popover closed. | The popover opens under the icon, dimmed, with the alert "Clear clipboard history?", "3 clips will be permanently deleted from this Mac. This can’t be undone.", *Keep pinned clips and snippets* checked, **Cancel** focused. Nothing is deleted yet. |
| 4 | Press `Return` (Cancel has the focus), then choose Clear history… again and press `Esc`. | Each time the alert closes, the search field has the focus, the three clips are still listed and the popover stays open. |
| 5 | Choose Clear history… and click *Clear history* with the box checked. | The alert closes and the three clips go (none is pinned); the list shows the empty history state. |
| 6 | Copy `Tadidy flow D`, open the popover, pin nothing. Choose Clear history… and click away on the desktop with the alert open. Open the popover again. | The popover hides with the alert; **nothing is deleted**. The popover shows `Tadidy flow D` and no alert. |
| 7 | Choose About Tadidy with TextEdit in front. | The native about panel shows "Tadidy", its version and, on a bundled build, "Copyright © 2026 Tadidy. All rights reserved." It opens **in front of TextEdit**. If it opens behind, report it: the item fires no event, so the app has to be activated another way (see the PR). |
| 8 | With the popover open, right-click the tray icon, choose Clear history…. | The popover closes on the right click (focus loss) and reopens with the alert. |
| 9 | Choose Quit Tadidy. | The icon disappears and `pgrep -x tadidy` prints nothing. |

### TAD-019 — MVP acceptance

Runs on the signed universal build (`pnpm build:universal` with `APPLE_SIGNING_IDENTITY="Tadidy Dev"`), installed once in `/Applications` and granted Accessibility once; do not reinstall between rows 3, 4 and 7. Start from a clean state (no `~/Library/Application Support/app.tadidy/`, no `app.tadidy` / `database-key` Keychain item) and use synthetic clips only (`Tadidy flow A`, `select me`…). Plan row 7 last, with *Launch at login* checked. Criteria are those of [docs/spec/05-quality.md](../spec/05-quality.md#mvp-acceptance).

| # | Criterion | Result | Measure / observation | Follow-up |
|---|---|---|---|---|
| 1 | Copy text in any app: it is at the top of the popover in < 500 ms | not run | | |
| 2 | Copy the same text twice: one clip, moved to the top, `copy_count = 2` | not run | | |
| 3 | `⌘⇧V` in TextEdit, `↓`, `Return`: the 2nd clip is pasted, TextEdit keeps focus | not run | | |
| 4 | Accessibility revoked: `Return` only copies and the banner shows | not run | | |
| 5 | Limit 10: an 11th copy deletes the oldest unpinned clip | not run | | |
| 6 | Typing "sel" filters in < 50 ms and shows only matching clips | not run | | |
| 7 | After a restart of the Mac, Tadidy runs and the history is intact | not run | | |
| 8 | `sqlite3` on `tadidy.db` without the key: "file is not a database" | not run | | |
| 9 | No icon in the Dock or in `⌘Tab` | not run | | |
| 10 | Idle CPU < 0.5 % over 10 min in Activity Monitor | not run | | |

| Measurement | Target | Result | Values | Note |
|---|---|---|---|---|
| Idle CPU, 10 min (Activity Monitor and `top -l` samples) | < 0.5 % | not run | | |
| Popover opening, 10 openings | < 100 ms | not run | | median: |
| Quick Panel opening, 10 openings | < 100 ms | not run | | median: |
| Search over 10 000 clips (`search_perf` and one seeded database) | < 50 ms | not run | | |
| RAM with both windows used | < 120 MB | not run | | recorded only |
| `.app` binary size | < 15 MB | not run | | recorded only |

Signed-build manual QA of the earlier tickets, re-run on this build:

| Section | Result | Note |
|---|---|---|
| TAD-003 — tray and popover | not run | |
| TAD-004 — pasteboard and workspace | not run | |
| TAD-005 — key sender and Accessibility | not run | |
| TAD-006 — Quick Panel and global shortcut | not run | |
| TAD-007 — encrypted database | not run | |
| TAD-008 — signing and universal build (incl. steps 7 and 8) | not run | |
| TAD-010 — clipboard watcher | not run | |
| TAD-015 — paste and copy commands | not run | |
| TAD-016 — clip browser in both windows | not run | |
| TAD-017 — Preferences | not run | |
| TAD-018 — tray menu actions | not run | |
