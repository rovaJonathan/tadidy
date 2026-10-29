# Reference

A **snapshot** of what the project uses. It is not a list of versions to copy: always install the latest stable version (AGENTS.md §2) and update this file afterwards. Versions marked *planned* are filled by TAD-001 with what is actually installed.

## Toolchain

| Tool | Version | Source |
|---|---|---|
| Rust | stable channel (1.99.1 on 2026-10-29) | `rust-toolchain.toml` |
| Node.js | 24 LTS (24.13.0 on 2026-10-28) | `.nvmrc` |
| pnpm | 10.34.9 | `packageManager` in `package.json`; stays on the 10 line (AGENTS.md §6) |
| lefthook | 2.2.2 | `brew install lefthook` |
| gitleaks | 8.30.1 | `brew install gitleaks` |
| cargo-llvm-cov | 0.9.1 | `brew install cargo-llvm-cov` + `rustup component add llvm-tools-preview` |
| pandoc | 3.x (spec HTML only) | `mise exec pandoc@3.1.11.1 --` |

### Coverage gate

`scripts/check.sh --cov` (run by the `pre-push` hook) ends with `node scripts/coverage-changed.mjs`: every source file changed since `git merge-base HEAD main`, working tree and untracked files included, needs ≥ 80 % line coverage (AGENTS.md §9). It reads `coverage/rust.lcov` (`cargo llvm-cov`) and `coverage/front/lcov.info` (`pnpm test:cov`); a changed file in neither report counts as 0 %.

- Gated: `src-tauri/src/**/*.rs`, `src/**/*.ts`, `src/**/*.svelte`.
- Excluded (`EXCLUDED` in the script; growing it needs a line in `PROGRESS.md`): `src-tauri/src/macos/**` (manual QA), `src-tauri/src/main.rs` and `src-tauri/src/lib.rs` (Tauri wiring, no logic: AGENTS.md §13), `src/lib/bindings/**` (generated), `*.test.ts`, `src/test/**`, `e2e/**`, `src-tauri/tests/**`, `*.d.ts`, `src/main.ts` and `src/App.svelte` (dev harness, covered by `e2e/`).
- Options: `--lcov <path>` (repeatable, replaces the defaults), `--base <ref>` (default `main`), `--min <pct>` (default 80).

## Main dependencies

Installed by TAD-001 on 2026-10-28 (exact versions in `pnpm-lock.yaml` and `src-tauri/Cargo.lock`). Later tickets add their own and update this table.

### Front (npm, pinned exactly in `package.json`)

| Package | Installed | Note |
|---|---|---|
| `@tauri-apps/api`, `@tauri-apps/cli` | 2.12.4 | same major and minor as the `tauri` crate |
| `svelte` | 5.57.3 | |
| `@sveltejs/vite-plugin-svelte` | 7.3.1 | |
| `vite` | 8.3.5 | |
| `typescript` | 6.0.3 | 7.x exists but `typescript-eslint` (`<6.1.0`) and `svelte-check` (`^5 \|\| ^6`) do not accept it yet |
| `svelte-check` | 4.7.7 | |
| `eslint` / `@eslint/js` | 10.13.0 / 10.0.1 | flat config |
| `typescript-eslint` | 8.72.1 | |
| `eslint-plugin-svelte` | 3.23.1 | |
| `globals` | 17.13.0 | |
| `prettier` / `prettier-plugin-svelte` | 3.9.10 / 4.1.1 | |
| `vitest` / `@vitest/coverage-v8` | 5.0.5 | |
| `jsdom` | 30.1.3 | |
| `@testing-library/svelte` / `@testing-library/jest-dom` | 5.4.2 / 7.0.1 | |
| `@playwright/test` | 1.64.1 | Chromium only |
| `@tsconfig/svelte` | 5.0.8 | |
| `@types/node` | 24.19.2 | the 24 line, matching Node 24 LTS (`.nvmrc`) |

### Core (crates, `src-tauri/Cargo.toml`)

| Crate | Installed | Note |
|---|---|---|
| `tauri` | 2.12.5 | `cargo add tauri@2`: crates.io's newest is `3.0.0-alpha`; features `tray-icon`, `macos-private-api`, `image-png` (TAD-003, tray icon from PNG bytes); dev feature `test` |
| `tauri-build` | 2.7.4 | |
| `serde` / `serde_json` | 1.0.229 / 1.0.154 | |
| `thiserror` / `anyhow` | 2.0.21 / 1.0.104 | |
| `tracing` / `tracing-subscriber` | 0.1.44 / 0.3.23 | `env-filter` |
| `ts-rs` | 12.0.1 | ADR 0006 |
| `tauri-plugin-positioner` | 2.4.0 | added by TAD-003, feature `tray-icon`; crates.io's latest is `3.0.0-alpha.3` (Tauri 3 pre-release), so the newest 2.x is used |
| `window-vibrancy` | 0.8.1 | added by TAD-003, popover material (`src-tauri/src/macos/vibrancy.rs`) |
| `tauri-plugin-global-shortcut` | 2.4.1 | added by TAD-006, Rust side only; crates.io's latest is `3.0.0-alpha.3` (Tauri 3 pre-release), so the newest 2.x is used |
| `tauri-nspanel` | 2.1.1 | added by TAD-006, macOS target only, published on crates.io (no git dependency needed); the Quick Panel in `src-tauri/src/macos/panel.rs` |
| `objc2` / `objc2-foundation` / `objc2-app-kit` | 0.6.5 / 0.3.2 / 0.3.2 | added by TAD-004, macOS target only, `default-features = false` and only the AppKit and Foundation classes used by `src-tauri/src/macos/` (+ `libc` for `pid_t`) |
| `core-graphics` | 0.25.0 | added by TAD-005, macOS target only, the synthetic `⌘V` (`src-tauri/src/macos/keys.rs`) |
| `objc2-application-services` / `objc2-core-foundation` | 0.3.2 / 0.3.2 | added by TAD-005 (not listed in the ticket), macOS target only, `default-features = false`: `AXIsProcessTrustedWithOptions` without hand-written FFI (`src-tauri/src/macos/accessibility.rs`); `objc2-core-foundation` builds its options dictionary |
| `rusqlite` / `rusqlite_migration` | 0.40.2 / 2.6.0 | added by TAD-007, feature `bundled-sqlcipher-vendored-openssl`; rusqlite stays on 0.40 because `rusqlite_migration` 2.6.0 requires `^0.40` (crates.io's newest rusqlite is 0.41.1); `rusqlite_migration` needs rustc 1.95 |
| `security-framework` | 3.7.0 | added by TAD-007, macOS target only (Keychain, `src-tauri/src/macos/keychain.rs`) |
| `core-foundation` / `security-framework-sys` | 0.10.1 / 2.17.0 | added by TAD-007, macOS target only: `kSecAttrAccessible` is not in the high-level `security-framework` API (`macos/keychain.rs`) |
| `getrandom` | 0.4.3 | added by TAD-007, the OS RNG for the database key |
| `tempfile` | 3.27.0 | added by TAD-007, dev-dependency (temporary databases in tests) |

### Planned (later tickets, latest stable seen on 2026-10-28)

| Crate | Latest stable seen | Note |
|---|---|---|

## Identifiers and paths

| What | Value |
|---|---|
| Bundle identifier | `app.tadidy` |
| Data folder | `~/Library/Application Support/app.tadidy/` |
| Database | `tadidy.db` (+ `-wal`, `-shm`), SQLCipher |
| Keychain item | generic password, service `app.tadidy`, account `database-key` |
| Logs | `~/Library/Logs/Tadidy/` (never clip content) |
| Pasteboard marker type | `app.tadidy.internal` |
| Default Quick Panel shortcut | ⌘⇧V (`CmdOrCtrl+Shift+V`) |
| Minimum macOS | 13 Ventura |
| Build targets | `aarch64-apple-darwin`, `x86_64-apple-darwin`, `universal-apple-darwin` |

## Environment variables

| Variable | Used by | Meaning |
|---|---|---|
| `APPLE_SIGNING_IDENTITY` | `pnpm tauri build` (TAD-008) | Name of the code-signing identity in the login keychain ([runbook](runbooks/signing-identity.md)) |
| `TADIDY_DATA_DIR` | the app, tests (*planned*, TAD-007) | Overrides the data folder; tests point it to a temporary folder |
| `RUST_LOG` | the app | `tracing` filter, e.g. `tadidy_lib=debug` (the library crate's name) |
| `TS_RS_EXPORT_DIR` | `cargo test` | Where `ts-rs` writes the bindings; set to `src/lib/bindings` by `.cargo/config.toml` at the repo root |
