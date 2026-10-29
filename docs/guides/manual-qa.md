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
| 8 | Right-click the icon. | A native menu: Preferences… `⌘,`, Clear history…, separator, About Tadidy, Check for updates…, separator, Quit Tadidy `⌘Q`. Only Quit Tadidy is enabled. |
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
