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
