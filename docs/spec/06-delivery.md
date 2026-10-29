<a id="06-delivery"></a>

# Delivery

<a id="roadmap"></a>

## Roadmap by phase

| Phase | Content | Expected outcome |
|---|---|---|
| **0 — Spikes** (≈ 3 days) | Tauri tray + popover · read `changeCount` via objc2 · paste via `CGEvent` · non-activating NSPanel · SQLCipher + Keychain | Retire the 5 technical risks before writing the app |
| **1 — MVP** | [CAP-01](02-features.md#cap-01)/[02](02-features.md#cap-02)/[08](02-features.md#cap-08)/[09](02-features.md#cap-09)/[12](02-features.md#cap-12) · [HIS-01](02-features.md#his-01)/[02](02-features.md#his-02)/[05](02-features.md#his-05)/[06](02-features.md#his-06) · [SRC-01](02-features.md#src-01) · [PST-01](02-features.md#pst-01)/[02](02-features.md#pst-02) · [MNU-*](02-features.md#menu-bar) · [QPN-01](02-features.md#qpn-01)/[02](02-features.md#qpn-02)/[04](02-features.md#qpn-04)/[06](02-features.md#qpn-06) · [SYS-01](02-features.md#sys-01)/[03](02-features.md#sys-03) · [Prefs](02-features.md#preferences) General + Shortcuts · [encryption](04-architecture.md#encryption) | Usable every day for text |
| **2 — v1.0** | All P1: rich text, images, files, URLs, excluded apps, concealed types, expiry, quota, filters, plain-text paste, ⌘1–9, pinned clips, snippets, collections, preview, onboarding, full prefs, export/import | CopyClip 2 parity |
| **3 — v1.x** | All P2: transforms, paste many, per-snippet shortcuts, editing, colors, advanced filters, fuzzy search, updater, stats, position at caret | Extras |

<a id="permissions-signing"></a>

## Permissions, signing, distribution

| Topic | Decision |
|---|---|
| **Accessibility (TCC)** | Required for auto-paste (`CGEvent`). Requested during onboarding. Without it, the app works in "copy only" mode. |
| **Sandbox** | Disabled, since there is no App Store. No sandbox entitlement. |
| **`Info.plist`** | `LSUIElement = true` (no Dock icon), `NSHumanReadableCopyright`, bundle ID `app.tadidy`. |
| **Signing** | ⚠️ **Important**: macOS ties the Accessibility permission and Keychain access to the **signing identity**. An *ad hoc* signature changes on every build, so the app would have to be re-authorized every time. **Solution**: sign with a stable identity, either a "Developer ID Application" certificate ($99/year, which also allows notarization) or, for strictly personal use, a self-signed certificate created in Keychain Access and referenced through the `APPLE_SIGNING_IDENTITY` environment variable (`signingIdentity` stays `null` in `tauri.conf.json`, so no identity name is committed). |
| **Notarization** | Optional for personal use. Required if the app is shared, to avoid being blocked by Gatekeeper. |
| **Packaging** | `tauri build --target universal-apple-darwin` produces `.app` + `.dmg`. |
| **Updates (P2)** | `tauri-plugin-updater`: Tauri signing key (separate from the Apple signature), `latest.json` published on GitHub Releases. |

<a id="risks"></a>

## Risks and points of attention

| Risk | Impact | Mitigation |
|---|---|---|
| The popover steals focus, and `⌘V` goes to the wrong app | High | Quick Panel as a non-activating NSPanel. For the popover, remember the active app and reactivate it before pasting. To validate in a spike (phase 0). |
| Accessibility permission lost on every rebuild | Medium (dev) | Stable signature from day one ([Permissions, signing, distribution](#permissions-signing)). In dev, launch from the signed `.app`, not via `cargo run`. |
| Building SQLCipher (OpenSSL) as a universal binary | Medium | `bundled-sqlcipher-vendored-openssl` feature, tested in CI from phase 0. |
| Database growing with images | Medium | Size limits, quota, `incremental_vacuum` after purges. |
| Polling and battery | Low | Adaptive polling. Read only `changeCount`, never the content, as long as it does not change. |
| Exotic content (proprietary app formats such as Figma or Office) | Low | Keep only known UTIs. P2 option "keep all representations" with a size limit. |
| `⌘⇧V` shortcut already used by another app | Low | Detect the registration failure and report it in the preferences. |
| WebKit memory footprint | Low | Windows are hidden, not destroyed. A single webview shared by the popover and the panel is possible if needed. |
