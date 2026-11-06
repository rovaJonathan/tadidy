# Manual QA

The macOS layer (`src-tauri/src/macos/`: pasteboard, frontmost app, synthetic ⌘V, Accessibility, Keychain, panels) cannot run in a unit test (AGENTS.md §9). Each behaviour it adds is checked by hand with the steps below, **on a signed build** (`pnpm tauri build`, [signing runbook](../runbooks/signing-identity.md)) unless a step says `pnpm tauri dev` is enough. The pull request of the ticket reports the result of its steps (✅ / ❌ + note).

A ticket that adds behaviour to the macOS layer adds its steps here, under its own heading, in the same pull request.

## How to run: a guided session with the owner

The manual QA is run **together, one step at a time**: the agent drives, the owner clicks and looks.

1. **The agent prepares**: builds and installs the signed build, installs the tools the steps need (after saying which), starts Tadidy from a terminal with the log on, and runs every command of the set-up and of the steps itself (`security`, `sqlcipher`, `ls`, `shasum`, `chmod`, reading the log).
2. **Before a step that can lose data** (a reset, a deleted or replaced Keychain item, files moved in the data folder), the agent saves the owner's real history first: a copy of `~/Library/Application Support/app.tadidy/` and a copy of the key **in the Keychain** under another service name (never in a file, never printed), and says how it will be restored.
3. **One step per message**: the agent says what to click or look at, and what to expect. The owner answers with what they see, then says **ready** for the next one. The agent never chains two steps without that answer.
4. The agent notes each result (✅ / ❌ + what was seen). A ❌ is investigated before going on (`systematic-debugging`), or recorded and skipped if the owner prefers.
5. **At the end**, the agent restores the saved history and key, checks that Tadidy opens it again, removes the temporary copies, and writes the results table in the pull request (below).

Practical points met on the way: Tadidy writes its log to **stdout**, so redirect it with `> /tmp/tadidy.log 2>&1` (a `2>` alone captures nothing); `security find-generic-password … -w` prints the database key as 64 hex digits, and `security add-generic-password -X <hex>` stores it back; a locked login keychain is unlocked by the owner in Keychain Access (*File › Unlock Keychain "login"*), since `security unlock-keychain` gets no terminal to read the password from in the agent's shell or through `!`.

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
| 7 | **Signed build**, *Launch at login* on. Log out and back in (or restart). | Tadidy starts by itself: its tray icon is there and no window opens (was MVP criterion 7 until 2026-11-05). |
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
| 1 | Copy text in any app: it is at the top of the popover in < 500 ms | ✅ | Reported by the owner; the watcher log shows `elapsed_ms` of 3 to 6 per recorded clip. | |
| 2 | Copy the same text twice: one clip, moved to the top, `copy_count = 2` | ✅ | Reported by the owner. | |
| 3 | `⌘⇧V` in TextEdit, `↓`, `Return`: the 2nd clip is pasted, TextEdit keeps focus | ✅ | Paste and focus OK (owner). On the first launch of the fresh install the Quick Panel did not open on `⌘⇧V`; after quitting and starting Tadidy again it opened. Re-run in [TAD-026](../../tickets/TAD-026.md) on 2026-11-05 from a clean state: it opens on the first launch and `Tadidy flow A` is pasted; the first failure did not reproduce in five clean launches. | [TAD-026](../../tickets/TAD-026.md) |
| 4 | Accessibility revoked: `Return` only copies and the banner shows | ✅ | Reported by the owner. | |
| 5 | Limit 10: an 11th copy deletes the oldest unpinned clip | ✅ | Reported by the owner. | |
| 6 | Typing "sel" filters in < 50 ms and shows only matching clips | ✅ | Reported by the owner; `search_perf` median 8.8 ms over 10 000 clips. | |
| 7 | With *Launch at login* checked, Tadidy is listed and enabled in *Login Items & Extensions* (restart dropped from the MVP, owner 2026-11-05) | ✅ | Re-run in [TAD-025](../../tickets/TAD-025.md) on the 2026-11-05 build: the switch turns on, the owner sees Tadidy under *Open at Login*, `sfltool dumpbtm` shows `app.tadidy`, `[enabled, allowed, notified]`, `/Applications/Tadidy.app`. The restart itself was not run: not an MVP criterion any more. | |
| 8 | `sqlite3` on `tadidy.db` without the key: "file is not a database" | ✅ | `sqlite3 -readonly …/tadidy.db .tables` prints `Error: file is not a database`; header bytes `62da 6a71 …`, one Keychain item, no `SecurityAgent` dialog on launch. | |
| 9 | No icon in the Dock or in `⌘Tab` | ✅ | Checked by the owner; `LSUIElement = true` in the bundle `Info.plist`. | |
| 10 | Idle CPU < 0.5 % over 10 min in Activity Monitor | ✅ | `top -l` on the `tadidy` process, 121 samples at 5 s (10 min), nothing touched: mean 0.03 %, median 0.0 %, max 0.9 % (one 5 s sample). Activity Monitor itself was not opened. | |

| Measurement | Target | Result | Values | Note |
|---|---|---|---|---|
| Idle CPU, 10 min (`top -l` samples) | < 0.5 % | ✅ | mean 0.028 %, median 0.0 %, max 0.90 % (120 samples after the first) | 10-minute run: `tadidy` main process only. Added after the review of PR 24, 2 minutes each at 5 s (`top -l`), session in use: WebKit helpers (2 WebContent, GPU, Networking, identified as the only WebKit processes running, started with Tadidy) mean 0.11 % at most per process, max 0.5 %. The main process read mean 1.06 % (max 5.8 %) in a first 2-minute window that overlapped other work on the machine (a spec rebuild; the owner's activity is unknown), then mean 0.008 % (max 0.2 %) in a second one. Not reproduced, so it is recorded and not explained. |
| Popover opening, 10 openings | < 100 ms | dropped | none | Owner: opens instantly. No timing log exists on the show path. The owner dropped the "10 values and a median" criterion of TAD-019 (review of PR 24); the spec's < 100 ms target stays unmeasured. |
| Quick Panel opening, 10 openings | < 100 ms | dropped | none | Owner: opens instantly (after the first-launch problem of [TAD-026](../../tickets/TAD-026.md)). Same dropped criterion as the popover row; the < 100 ms target stays unmeasured. |
| Search over 10 000 clips (`search_perf` and one seeded database) | < 50 ms | ✅ (automated part) | `search_perf`, debug profile: median 8.8 ms, worst 10.5 ms | Not run in the real app: seeding 10 000 clips means writing into the encrypted database with its key. The automated test and the owner's check of criterion 6 (few clips) stand in for it. |
| RAM with both windows used | < 120 MB | ✅ | `tadidy` footprint 40 MB (peak 41 MB); one WebContent process 24 MB in the system log | about 65 MB in total, recorded only. The WebKit network process was not attributed. |
| `.app` binary size | none (target removed from the spec) | accepted | universal `.app` 36.6 MiB, executable 38.4 MB; arm64-only `.app` 17.8 MiB, executable 18.7 MB; `.dmg` 12.5 MiB | recorded only. The owner said the size does not matter, and the review of PR 24 pointed out that the spec still said 15 MB: the target was removed from `docs/spec/05-quality.md`. No ticket. |

Signed-build manual QA of the earlier tickets, re-run on this build:

| Section | Result | Note |
|---|---|---|
| TAD-003 — tray and popover | covered | Covered by MVP criteria 1 to 6, 9 and 10 on this build; the steps were not replayed one by one (owner's choice: key steps only). |
| TAD-004 — pasteboard and workspace | covered | Covered by MVP criteria 1 to 6, 9 and 10 on this build; the steps were not replayed one by one (owner's choice: key steps only). |
| TAD-005 — key sender and Accessibility | covered | Paste (criterion 3) and the revoked-permission path (criterion 4) pass; the rest is not replayed. |
| TAD-006 — Quick Panel and global shortcut | partial | The shortcut opens the panel on the first launch and after a restart ([TAD-026](../../tickets/TAD-026.md), 2026-11-05). The rest is not replayed. |
| TAD-007 — encrypted database | covered | Criterion 8 and TAD-008 step 8: encrypted header, one Keychain item, no dialog. |
| TAD-008 — signing and universal build (incl. steps 7 and 8) | ✅ | Steps 1 to 4: `Authority=Tadidy Dev`, verify exits 0, `x86_64 arm64`. Step 5: the designated requirement is identical across two builds (binaries differ). Step 6: with the identity unset the bundle is `Signature=adhoc`. Step 7 (owner): Accessibility still granted after installing build N+1 over N. Step 8: both builds use the same key (same header bytes), one Keychain item, no `SecurityAgent`, no `storage_key_missing`. The ad-hoc build's `.dmg` step failed on a leftover mounted image (`hdiutil` busy), unrelated to signing. |
| TAD-010 — clipboard watcher | covered | Covered by MVP criteria 1 to 6, 9 and 10 on this build; the steps were not replayed one by one (owner's choice: key steps only). |
| TAD-015 — paste and copy commands | covered | Criteria 3 and 4. |
| TAD-016 — clip browser in both windows | covered | Criteria 1 to 3 and 6; the Quick Panel first-launch problem did not reproduce in [TAD-026](../../tickets/TAD-026.md). |
| TAD-017 — Preferences | partial (step 6 ✅, steps 7 and 8 not run) | *Launch at login* could not be switched on; fixed by [TAD-025](../../tickets/TAD-025.md), whose QA covers step 6 on the 2026-11-05 build. Step 7 (restart) is no longer required (not an MVP criterion since 2026-11-05); step 8 is TAD-025 step 5. History size (criterion 5) works. The rest is not replayed. |
| TAD-018 — tray menu actions | ✅ (step 7) | Step 7 on the bundled build (owner): About opens in front of TextEdit with the copyright line. The other steps are not replayed. |

### TAD-022 — Tell the user when the history database is unavailable and offer to reset it

Signed build (`pnpm build:universal` with `APPLE_SIGNING_IDENTITY="Tadidy Dev"`), installed, Accessibility granted. Synthetic clips only (`Tadidy before reset`, `Tadidy after reset`). Data folder: `~/Library/Application Support/app.tadidy/` (`D` below). Quit Tadidy before each file operation.

Set-up for steps 1 to 10: launch once, copy `Tadidy before reset` so the history has a clip, quit, then `shasum D/tadidy.db` (note it: **S**) and `ls D`, then delete the key: `security delete-generic-password -s app.tadidy -a database-key`. Launch Tadidy.

| # | Action | Expected |
|---|---|---|
| 1 | Click the tray icon. | The popover shows, in place of the list, an icon, **Your history can’t be opened.**, **Tadidy can’t unlock its encrypted database, so nothing is being recorded.** and a **Reset history…** button. The footer shows no count. |
| 2 | Press `⌘⇧V` (close the popover first). | The Quick Panel shows the same error state. |
| 3 | In another app copy `Tadidy after reset`, look at the popover, quit Tadidy, `shasum D/tadidy.db`, launch it again. | Nothing was recorded: the error state stays and the hash is **S**. |
| 4 | Click **Reset history…**. | A dialog: **Reset clipboard history?**, **The current history can’t be recovered. Tadidy will start a new, empty one. The old encrypted file is kept in the app’s data folder as tadidy.db.bak.**, buttons **Reset history** and **Cancel**; **Cancel** has the focus. |
| 5 | Press `Return`, then reopen the dialog and click **Cancel**, then reopen it and press `Esc`. | Each time the dialog closes and the error state stays. Quit and `ls D`: no `.bak` file, `shasum D/tadidy.db` is **S**. Launch again. |
| 6 | With the popover open, click **Reset history…**, then **Reset history**. | The popover shows **Nothing copied yet.** Open the Quick Panel: it shows the empty state too (not the error). |
| 7 | `ls D`, `shasum D/tadidy.db.bak`, Keychain Access (`app.tadidy`). | `tadidy.db.bak` exists (with `tadidy.db-wal.bak` / `tadidy.db-shm.bak` when `ls D` showed `-wal` / `-shm` files before) and its hash is **S**; a new `tadidy.db` exists; one Keychain item `app.tadidy` / `database-key`. |
| 8 | Copy `Tadidy after reset` in another app, without restarting Tadidy. | It appears at the top of the popover and of the Quick Panel. |
| 9 | Quit and launch Tadidy again. | The history holds `Tadidy after reset`; no error state. |
| 10 | Quit. Run `chmod a-w D`, delete the key again (`security delete-generic-password …`), launch, click **Reset history…** then **Reset history**. Quit, `chmod u+w D`. | The error state stays and the button still works; `ls D` shows the same files as before, no `.bak.new` file. Launch and reset again: it succeeds. |
| 11 | Wrong key (the bug this ticket comes from): quit, delete `D` and the Keychain item, run `pnpm tauri dev`, copy a clip, quit; then launch the signed build. | The error state shows (the signed build cannot use the key the dev build stored). **Reset history…** then **Reset history** works with no password dialog, or the result is reported here: ❌ + what macOS showed (the Keychain adapter updates an item that another signature created). |
| 12 | `RUST_LOG=tadidy_lib=info` on the binary for steps 1 and 7 (`/Applications/Tadidy.app/Contents/MacOS/tadidy`). | The log has `storage_key_missing` (step 1) and no path, key or clip text anywhere. |
| 13 | Temporary failure: quit, `security lock-keychain ~/Library/Keychains/login.keychain-db`, launch Tadidy, **Cancel** the unlock prompt. Open the popover. Then unlock (`security unlock-keychain …`) and relaunch. | The error state shows **without** a **Reset history…** button (the failure may be temporary). After the unlock and a relaunch the history is intact: no `.bak` file in `D`. |
| 14 | In the error state of step 1, press `Tab` until **Reset history…** has the focus, then `Return` (later: `Space`). | The confirmation opens each time; nothing is pasted and the focus does not jump to the search field. |
| 15 | Make the reset fail (step 10's read-only folder). | Under the text: **The reset didn’t work. Nothing was changed: try again.** and the button is still there. |
| 16 | The Keychain item is replaced in place (step 11's set-up: the item was created by the dev build). Click **Reset history…** then **Reset history**; if macOS asks to let Tadidy use the item, click **Deny** once, then try again and allow. | While the macOS prompt is shown, **Reset history…** is greyed out. After **Deny**: the message of step 15, `ls D` unchanged, and `security find-generic-password -s app.tadidy -a database-key` still finds the item (the old key was not deleted). After allowing: the reset works, one item `app.tadidy` / `database-key`. |
| 17 | The key comes back (a reset must not throw away a readable history): with Tadidy in the error state of step 1, restore the key without quitting (note it before step 1 with `security find-generic-password -s app.tadidy -a database-key -w`, then `security add-generic-password -s app.tadidy -a database-key -w <that value>`), then click **Reset history…** and confirm. | The history opens with `Tadidy before reset` in it; `ls D` shows no `.bak` file; nothing was moved. |
| 18 | Same set-up as step 17 but with a custom Quick Panel shortcut chosen in Preferences beforehand (e.g. `⌃⌥V`), and the key deleted before launching. After the reset-first reopen of step 17, press the default `⌘⇧V`, then `⌃⌥V`. | `⌃⌥V` opens the Quick Panel and `⌘⇧V` does nothing; Preferences shows `⌃⌥V` without an error. |
| 19 | Same as step 18, but with the custom shortcut held by another app (e.g. set the same shortcut in a launcher) before the reset. | After the reset, `⌘⇧V` still opens the Quick Panel and the custom shortcut does not; Preferences shows `⌘⇧V` with no error; choosing another shortcut there works. |
| 20 | Keyboard only, in the error state of step 1: `Tab` to **Reset history…**, `Return`, `Tab` to **Reset history** in the dialog, `Return`. Then repeat with the folder read-only (step 10). | After a success the cursor is in the search field. After a failure the focus is back on **Reset history…** and `Return` opens the dialog again. |
| 21 | A reset that was stopped: with the set-up of step 1 (key deleted, app quit), run `cp D/tadidy.db D/tadidy.db.bak.new` and note `shasum D/tadidy.db.bak.new` (**L**), launch Tadidy, click **Reset history…** then **Reset history**. | The reset works. `ls D` shows `tadidy.db.bak.1` (hash **L**) and `tadidy.db.bak` (hash **S**): the file that waited as `.bak.new` was not replaced. |

### TAD-027 — Offer the history reset for a malformed key and a database that cannot be migrated

Signed build (`pnpm build:universal` with `APPLE_SIGNING_IDENTITY="Tadidy Dev"`), installed, started from a terminal so the log is visible: `RUST_LOG=tadidy_lib=info /Applications/Tadidy.app/Contents/MacOS/tadidy > /tmp/tadidy.log 2>&1`. Synthetic clips only. Data folder `D` as in TAD-022. Quit Tadidy before each file or Keychain operation. Steps 3 to 5 need the SQLCipher shell (`brew install sqlcipher`) and the key as hex: `security find-generic-password -s app.tadidy -a database-key -w` prints 64 hex digits (**K**); `SQL` below stands for `sqlcipher D/tadidy.db -cmd "PRAGMA key = \"x'K'\";"`.

| # | Action | Expected |
|---|---|---|
| 1 | Malformed key: copy `Tadidy before reset`, quit, `security delete-generic-password -s app.tadidy -a database-key`, then `security add-generic-password -s app.tadidy -a database-key -w not-a-key`. Launch; if macOS asks to let Tadidy use the item, click **Always Allow**. Open the popover. | The error state of TAD-022 step 1 **with** the **Reset history…** button. The log has `storage_key_malformed`, no path and no key. |
| 2 | Click **Reset history…**, then **Reset history** (allow the Keychain prompt if one shows). Copy `Tadidy after reset`. | **Nothing copied yet.**, then `Tadidy after reset` at the top. `ls D` shows `tadidy.db.bak`; one Keychain item `app.tadidy` / `database-key`, and `security find-generic-password … -w` prints 64 hex digits again. |
| 3 | Database that cannot be migrated: quit, note **K**, run `SQL "PRAGMA user_version = 1;"` (the second migration then runs again on tables that exist). Launch, open the popover. | The error state **with** **Reset history…**. The log has `storage_migration_failed`. |
| 4 | Click **Reset history…**, then **Reset history**. Quit and run `sqlcipher D/tadidy.db.bak -cmd "PRAGMA key = \"x'K'\";" "PRAGMA user_version;"` with the **K** of step 3. | The reset works (**Nothing copied yet.**). The `.bak` opens with the old key and answers `1`: the old history was moved aside, not changed. |
| 5 | Database from a newer version of the app: launch, copy `Tadidy newer schema`, quit, note the new **K**, run `SQL "PRAGMA user_version = 99;"`. Launch, open the popover. Then quit, run `SQL "PRAGMA user_version = 2;"` and launch again. | First launch: the error state **without** a **Reset history…** button; the log has `storage_schema_too_new`; `ls D` has no new `.bak`. After the version is put back (what an update of the app would do): the history opens with `Tadidy newer schema` in it. |
| 6 | Temporary failure: TAD-022 step 13 (locked Keychain). When macOS asks for the password to unlock the login keychain, click **Cancel** without typing it (typing it unlocks the Keychain and the history simply opens). | Still **no** **Reset history…** button; the log has `storage_error`. Afterwards unlock the Keychain in Keychain Access (*File › Unlock Keychain "login"*). |

### TAD-025 — Make Launch at login work on the signed build

Signed universal build (`pnpm build:universal` with `APPLE_SIGNING_IDENTITY="Tadidy Dev"`) installed in `/Applications`, started from a terminal so the log is visible: `RUST_LOG=tadidy_lib=debug /Applications/Tadidy.app/Contents/MacOS/tadidy 2> /tmp/tadidy.log`. Start with *Launch at login* off and Tadidy absent from *System Settings → General → Login Items & Extensions*. Open Preferences from the tray menu. The cause was the switch, not the system: in WebKit only its top-left 12 px reacted to a click (`docs/solutions/front/2026-11-05-webkit-checkbox-ignores-inset.md`), so click the switches **on their right half**, never on the label.

| # | Action | Expected |
|---|---|---|
| 1 | Click the right half of the *Launch at login* switch. | The switch turns on and no text shows under the label. `/tmp/tadidy.log` has `login item changed` with `after=Enabled`. Tadidy is listed and enabled under *Open at Login* in *Login Items & Extensions*; `sfltool dumpbtm \| grep -B6 -A4 'app.tadidy$'` shows a record with `Disposition: [enabled, …]`. |
| 2 | Click the right half of the *Confirm before delete* switch, twice. | It turns off, then on again. |
| 3 | Optional (not an MVP criterion since 2026-11-05): restart the Mac (Tadidy quit or not). | After logging in, Tadidy runs by itself: its tray icon is there, no window opens, the history is intact. |
| 4 | Click the right half of the *Launch at login* switch to turn it off. | The switch turns off; Tadidy leaves *Open at Login*. |
| 5 | Turn it on again, then in *Login Items & Extensions* switch Tadidy off (its toggle under *Allow in the Background*, or remove it from *Open at Login*). Quit and start Tadidy, open Preferences. | *Launch at login* is **off** (the system wins). Click the switch: either it turns on and Tadidy is enabled again in System Settings, or it stays off and "Allow Tadidy in System Settings › General › Login Items & Extensions, then switch it on again." shows under the label, with `after=RequiresApproval` in the log. Note which. In the second case, allow Tadidy in System Settings, then click the switch: it turns on. |
| 5b | With the history database unavailable (TAD-022 step 1 set-up: Keychain item deleted), click the right half of *Launch at login*. | The switch goes back to off and "The history database is unavailable." shows under the label: a refusal is never silent. |
| 6 | Clean up the test records: turn *Launch at login* off if the owner does not want it. | No stale *Tadidy* or *TadidyProbe* entry is left in *Login Items & Extensions* (`sfltool resetbtm` resets **every** app's record: only with the owner's consent, then restart). |

### TAD-032 — Show the popover under the tray icon when the Mac has several displays

Signed universal build installed in `/Applications`, started from a terminal so the log is visible: `RUST_LOG=tadidy_lib=debug /Applications/Tadidy.app/Contents/MacOS/tadidy 2> /tmp/tadidy.log`. A built-in Retina display (scale 2) and an external display at scale 1. The bug needs the two scales: a window is created on the display that has the keyboard focus, so **start Tadidy while the focused window is on the external display** (the terminal, for instance). With the old placement the popover then went off every display and each click logged `No monitor found for the window`.

| # | Action | Expected |
|---|---|---|
| 1 | External display on the left of the built-in one (System Settings › Displays › Arrange). Start Tadidy with the focus on the external display. Click the Tadidy icon in the built-in display's menu bar, `Esc`, click it again. | Both times the popover opens right under the icon, focused. The log has no `could not place the popover` nor `the popover is centered` line. |
| 2 | Click the Tadidy icon in the external display's menu bar, `Esc`, then on the built-in one. | Under the icon on the external display, then under the icon on the built-in one. |
| 3 | Move the external display to the right of the built-in one (its pixel range then overlaps the built-in one's). Click the icon on each menu bar. | Under the icon on both. |
| 4 | Unplug the external display. Click the icon, `Esc`, click again. | Under the icon both times (no regression with one display). |

The fallback (no tray rectangle, or an icon on no display: the popover is centered on the display under the pointer and `the tray icon was not found on a display: the popover is centered` is logged) cannot be caused by hand; the unit tests of `windows::popover_placement` cover it.

Run 2026-11-05 by the owner on a MacBook Pro (1512 × 982 points, scale 2) with a 2560 × 1440 display (scale 1): steps 1 to 4 OK, empty log.

### TAD-026 — Open the Quick Panel on the first launch with a fresh database

Signed universal build (`pnpm build:universal` with `APPLE_SIGNING_IDENTITY="Tadidy Dev"`) installed in `/Applications`, Accessibility already granted to the `Tadidy Dev` identity. Each run starts from a clean state: quit Tadidy (`pgrep -x tadidy` prints nothing), delete `~/Library/Application Support/app.tadidy/` and the Keychain item (`security delete-generic-password -s app.tadidy -a database-key`). Start it through LaunchServices with the log in a file: `open -a /Applications/Tadidy.app --stdout /tmp/tadidy.log --env RUST_LOG=tadidy_lib=info --env NO_COLOR=1` (the log goes to standard output). Synthetic clips only.

| # | Action | Expected |
|---|---|---|
| 1 | Clean state, focus on a window of the built-in display. Start Tadidy and wait for its tray icon. | The log has `the Quick Panel shortcut is registered` and no `is not registered` line. |
| 2 | In TextEdit press `⌘⇧V`. | The Quick Panel opens on the display under the pointer. |
| 3 | Clean state again, focus on a window of the external display (when there is one). Start Tadidy, copy `Tadidy flow A` then `Tadidy flow B` in TextEdit, click the tray icon, click back in TextEdit, press `⌘⇧V`, `↓`, `Return`. | The popover opens under the icon; the Quick Panel opens; `Tadidy flow A` is pasted into TextEdit, which keeps the focus (MVP criterion 3). |
| 4 | Quit Tadidy and start it again (no clean state), press `⌘⇧V` in TextEdit. | The Quick Panel opens. |

Run 2026-11-05 by the owner on a MacBook Pro (scale 2) with a 2560 × 1440 display (scale 1), four clean first launches with debug diagnostics: the current code with the focus on the external display, on the built-in one, then the step 3 sequence on the external one; and the pre-TAD-032 code with the step 3 sequence. All four registered `Cmd+Shift+V` without error, received the press and showed the panel. The TAD-019 failure did not reproduce. Then steps 1 to 4 on the final build (no diagnostics), focus on the built-in display: all OK; the log has `the Quick Panel shortcut is registered` on both launches and `clip pasted … result="pasted"` for the first clip copied.
