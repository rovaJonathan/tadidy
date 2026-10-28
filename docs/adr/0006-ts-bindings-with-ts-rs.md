# 0006 — IPC types generated from Rust with `ts-rs`

- **Status**: accepted
- **Date**: 2026-10-28

## Context

The front and the core talk through Tauri IPC commands and events. Hand-written TypeScript copies of Rust types drift silently.

## Decision

- Every Rust type that crosses the IPC boundary derives `serde::Serialize`/`Deserialize` and `ts_rs::TS` with `#[ts(export)]`; `cargo test` exports them to `src/lib/bindings/`, which is committed.
- `scripts/check.sh` fails when the Rust tests change `src/lib/bindings/` (generated files out of date).
- `src/lib/api.ts` wraps `invoke` / `listen` with these types; components never call `invoke` directly.

## Consequences

- A change to an IPC type shows in the same pull request on both sides.
- Command names and argument names are still strings in `api.ts`; a test per wrapper (IPC mocked) catches a mismatch.

## Rejected alternatives

- **`tauri-specta`**: generates the commands too, but its Tauri 2 line is still a release candidate; AGENTS.md §2 forbids pre-releases. To reconsider once it is stable (new ADR).
- **Hand-written types**: drift.
