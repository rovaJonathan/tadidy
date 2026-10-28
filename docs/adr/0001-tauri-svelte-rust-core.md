# 0001 — Tauri 2 shell, Rust core, Svelte 5 front

- **Status**: accepted
- **Date**: 2026-10-28

## Context

Tadidy is a macOS menu-bar app: a tray icon, a popover, a floating Quick Panel, a preferences window. It needs low-level macOS access (pasteboard, frontmost app, synthetic ⌘V, Keychain, Accessibility) and a polished, searchable list UI that will keep evolving. The owner wants Rust.

## Decision

- **Tauri 2** is the shell: tray, windows, global shortcut, autostart, IPC.
- **The core is Rust**: watcher, paster, storage (SQLCipher), purge, and a thin `macos/` layer on `objc2` / `core-graphics` / `security-framework`.
- **The UI is Svelte 5 + TypeScript** in the system WebKit webview: a small bundle, so the pre-created windows open in < 100 ms.

## Consequences

- One webview process (≈ 40–80 MB) on top of the Rust process; windows are hidden, not destroyed.
- The webview is untrusted (AGENTS.md §2): everything sensitive stays in Rust behind typed IPC commands.
- macOS specifics that Tauri does not cover (non-activating `NSPanel`, vibrancy) come from `tauri-nspanel` and `window-vibrancy`.

## Rejected alternatives

- **Rust + AppKit only** (`objc2`, `NSStatusItem`, `NSMenu`): the lightest and most native, but every UI change is low-level code; search, previews and preferences would cost much more.
- **Rust + egui / iced + `tray-icon`**: no web stack, but a non-native look and a hand-made popover/panel integration.
- **Leptos (Rust/WASM) front**: 100 % Rust, but a younger UI ecosystem and slower builds.
