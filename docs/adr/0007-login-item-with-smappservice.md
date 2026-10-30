# 0007 — Launch at login through `SMAppService` (`smappservice-rs`)

- **Status**: accepted
- **Date**: 2026-10-30

## Context

SYS-01 asks for "launch at login via `SMAppService` (macOS 13+)". The spec's architecture table lists two candidates: `tauri-plugin-autostart` and `smappservice-rs`. TAD-017 had to choose (its *Risks*).

`tauri-plugin-autostart` 2.7.0 (the newest 2.x; 3.0.0-alpha is a pre-release) drives the `auto-launch` crate and offers `MacosLauncher::LaunchAgent` or `MacosLauncher::AppleScript` only. Neither goes through `SMAppService`. With a LaunchAgent, "is it enabled?" means "does the plist exist?": a user who switches Tadidy off in System Settings › General › Login Items & Extensions is not seen, and TAD-017's criterion ("removed from Login Items → `launchAtLogin` reads `false` at the next start") cannot hold.

## Decision

- The login item is `SMAppService.mainApp`, driven by `smappservice-rs` (a thin wrapper over `objc2`, already in the tree) in `src-tauri/src/macos/login_item.rs`.
- The core talks to it through `trait LoginItem { is_enabled, set_enabled }` (`src-tauri/src/login_item.rs`), with a `FakeLoginItem` for tests. `is_enabled` is `status() == Enabled`; `RequiresApproval` counts as not enabled.
- At startup the stored `launchAtLogin` is reconciled with the system state: the system wins.
- `tauri-plugin-autostart` is not added.

## Consequences

- `SMAppService.mainApp` only works from a signed `.app` bundle: `register` fails under `tauri dev` and from a bare binary. The failure is the `login_item_failed` error of `set_settings`; the real behaviour is manual QA on the signed build (`### TAD-017`).
- One more crate in `Cargo.toml` (macOS target), pre-1.0 (0.1.x): its surface is three calls, wrapped in a trait, so replacing it later costs one file.

## Rejected alternatives

- **`tauri-plugin-autostart`**: no `SMAppService`, and a stale view of the user's choice in System Settings (see Context).
- **Calling `SMAppService` through `objc2-service-management` by hand**: more `unsafe` for the same three calls.
