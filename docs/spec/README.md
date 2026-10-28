<a id="tadidy-specification"></a>

# Tadidy — Specification

> **Tadidy** ("to remember" in Malagasy) is a clipboard history manager for macOS that lives in the menu bar, inspired by [CopyClip](https://apps.apple.com/us/app/copyclip-clipboard-history/id595191960?mt=12) and CopyClip 2.

| | |
|---|---|
| **Document version** | 0.2 — 2026-10-28 |
| **Platform** | macOS 13 Ventura and later, Apple Silicon + Intel (universal binary) |
| **Stack** | Rust (core) · Tauri 2 (shell) · Svelte 5 + TypeScript (UI) · encrypted SQLite (SQLCipher) |
| **UI language** | English only |
| **Distribution** | Personal use, direct DMG (outside the Mac App Store, so no sandbox) |
| **Sync** | None, 100% local storage |

The spec is the source of truth. Tickets cite it with `docs/spec/<file>.md#<anchor>`: never rename or delete an anchor a ticket cites.

Domain vocabulary lives in [GLOSSARY.md](../../GLOSSARY.md).

<a id="contents"></a>

## Contents

| Part | Covers |
|---|---|
| [01 — Overview](01-overview.md) | [Problem](01-overview.md#problem) · [Goals](01-overview.md#goals) · [Out of scope](01-overview.md#out-of-scope) |
| [02 — Features](02-features.md) | [Capture](02-features.md#capture) · [History](02-features.md#history) · [Search](02-features.md#search) · [Paste](02-features.md#paste) · [Pinned & snippets](02-features.md#pinned-snippets) · [Menu bar](02-features.md#menu-bar) · [Quick Panel](02-features.md#quick-panel) · [Preferences](02-features.md#preferences) · [System](02-features.md#system) |
| [03 — Interface](03-interface.md) | [Layout](03-interface.md#layout) · [Visual design](03-interface.md#visual-design) · [Keyboard shortcuts](03-interface.md#keyboard-shortcuts) |
| [04 — Architecture](04-architecture.md) | [Overview](04-architecture.md#overview) · [Dependencies](04-architecture.md#dependencies) · [Watcher](04-architecture.md#watcher) · [Paster](04-architecture.md#paster) · [Windows](04-architecture.md#windows) · [IPC](04-architecture.md#ipc) · [Data model](04-architecture.md#data-model) · [Encryption](04-architecture.md#encryption) · [Project layout](04-architecture.md#project-layout) |
| [05 — Quality](05-quality.md) | [Non-functional requirements](05-quality.md#non-functional) · [MVP acceptance criteria](05-quality.md#mvp-acceptance) |
| [06 — Delivery](06-delivery.md) | [Roadmap](06-delivery.md#roadmap) · [Permissions & signing](06-delivery.md#permissions-signing) · [Risks](06-delivery.md#risks) |

<a id="priorities"></a>

## Priorities

**P0** = MVP · **P1** = v1.0 · **P2** = after v1.0

<a id="decisions"></a>

## Decisions made

| Question | Decision |
|---|---|
| Stack | Rust + **Tauri 2**, front end **Svelte 5 + TypeScript** |
| Scope | **CopyClip 2 + extras** level, delivered in phases |
| Content | Plain text, rich text (RTF/HTML), images, files/URLs |
| Display | **Popover** on tray click + **Quick Panel** on global shortcut |
| Distribution | Personal use, direct DMG, no sandbox |
| Language | English only |
| Privacy | **Encrypted database** (SQLCipher + Keychain) + **auto-expiry** |
| Name | **Tadidy** |
| History | **500 clips** by default |
| Sync | None, local only |

<a id="open-questions"></a>

## Open questions

1. **Ignore concealed content (passwords)**: not checked during scoping, but included here ([CAP-11](02-features.md#cap-11)), on by default and can be turned off. To be confirmed.
2. **Pause capture** ("Pause for 15 min / 1 h / until resumed"): not kept for now. Add it as P2?
3. **Quick Panel position**: under the mouse (proposed default) or centered?
4. **Sound or visual feedback on copy** (the tray icon "pulses"): wanted?
5. **Global shortcut**: is `⌘⇧V` fine, or do you prefer `⌥⌘V` / `⌃⌥V`?
6. **Signing**: do you have an Apple Developer account, or do we go with a self-signed certificate?
