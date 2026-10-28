---
name: scaffold-tauri-command
description: Creates a new Tauri IPC command end to end in Tadidy, tests first — Rust types with ts-rs, the command in commands.rs delegating to a service, AppError mapping, registration and capability, the generated TypeScript bindings, the typed wrapper in src/lib/api.ts, Rust tests and a Vitest test of the wrapper with mocked IPC. Use for every ticket that adds or changes an IPC command or event.
---

# scaffold-tauri-command — one command, both sides, tests first

Contract: [docs/spec/04-architecture.md#ipc](../../../docs/spec/04-architecture.md#ipc). Types are generated from Rust ([ADR 0006](../../../docs/adr/0006-ts-bindings-with-ts-rs.md)). The webview is untrusted (AGENTS.md §2).

## 1. Types (Rust)

In the service's module (not in `commands.rs`):
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListClipsQuery { pub search: Option<String>, pub cursor: Option<String>, pub limit: Option<u32> }
```
- The export folder is set once by TAD-001 (`TS_RS_EXPORT_DIR` → `src/lib/bindings/` in `src-tauri/.cargo/config.toml`): plain `#[ts(export)]`.
- `camelCase` on the wire; timestamps `i64` ms; ids `String` (UUID v7).
- Never expose the database key, a file path in the user's home, or more clip data than the view needs (a list returns summaries; the detail command returns representations).

## 2. Red tests (Rust)

- Service test (`#[cfg(test)]`) with fakes / a temporary database: the behaviour of the acceptance criterion.
- Command test in `src-tauri/tests/commands_<area>.rs`: input validation (limit bounds, unknown id → `AppError { code: "not_found" }`), the happy path.
Commit `test(TAD-xxx): …`.

## 3. The command (thin)

```rust
#[tauri::command]
pub async fn list_clips(state: tauri::State<'_, AppState>, query: ListClipsQuery) -> Result<Page<ClipSummary>, AppError> {
    state.clips.list(query).map_err(AppError::from)
}
```
- Validation and logic live in the service; the command only adapts.
- `AppError` serializes to `{ code, message }`; messages never contain clip content.
- Blocking SQLite work: `tauri::async_runtime::spawn_blocking` or a dedicated thread, never on the main thread.
- Register it in `tauri::generate_handler![…]` and allow it in `src-tauri/capabilities/*.json` for the windows that need it only.
- Events: emit after the database write (`app.emit("clips://changed", payload)`), payload type with `ts_rs::TS` too.

## 4. Bindings and front wrapper

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # exports src/lib/bindings/*.ts
```
In `src/lib/api.ts`:
```ts
import type { ListClipsQuery } from "./bindings/ListClipsQuery";
export const listClips = (query: ListClipsQuery) => invoke<Page<ClipSummary>>("list_clips", { query });
```
Vitest test of the wrapper with `mockIPC` (`@tauri-apps/api/mocks`): the command name and argument names are the ones Rust expects. Commit `feat(TAD-xxx): …`.

## 5. Check

- `scripts/check.sh --cov`: bindings committed (the drift check fails otherwise), clippy clean, coverage.
- `self-review` privacy lines: nothing sensitive returned or logged.
