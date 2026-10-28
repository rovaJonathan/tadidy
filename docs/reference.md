# Reference

A **snapshot** of what the project uses. It is not a list of versions to copy: always install the latest stable version (AGENTS.md §2) and update this file afterwards. Versions marked *planned* are filled by TAD-001 with what is actually installed.

## Toolchain

| Tool | Version | Source |
|---|---|---|
| Rust | stable channel (1.93 on 2026-10-28) | `rust-toolchain.toml` |
| Node.js | 24 LTS | `.nvmrc` |
| pnpm | 10.32.1 | `packageManager` in `package.json` (TAD-001) |
| lefthook | 2.2.2 | `brew install lefthook` |
| gitleaks | 8.30.1 | `brew install gitleaks` |
| cargo-llvm-cov | 0.9.1 | `brew install cargo-llvm-cov` + `rustup component add llvm-tools-preview` |
| pandoc | 3.x (spec HTML only) | `mise exec pandoc@3.1.11.1 --` |

## Main dependencies

Latest stable on 2026-10-28, for orientation only (*planned*: installed by TAD-001 and later tickets).

| Package | Latest stable seen | Note |
|---|---|---|
| `tauri` (crate) | 2.x | crates.io's newest is `3.0.0-alpha`: install `tauri@2` |
| `@tauri-apps/cli`, `@tauri-apps/api` | 2.12.4 | same minor as the `tauri` crate |
| `svelte` | 5.57.3 | |
| `vite` | 8.3.5 | |
| `typescript` | 7.0.2 | keep the newest major that `svelte-check` and `typescript-eslint` support |
| `vitest` | 5.0.5 | |
| `@playwright/test` | 1.64.1 | |
| `svelte-check` | 4.7.7 | |
| `eslint` / `prettier` | 10.13.0 / 3.9.10 | |
| `rusqlite` / `rusqlite_migration` | 0.41.0 / 2.6.0 | feature `bundled-sqlcipher-vendored-openssl` |
| `objc2` / `objc2-app-kit` | 0.6.5 / 0.3.2 | |
| `tauri-nspanel` | 2.1.1 | |
| `window-vibrancy` | 0.8.1 | |
| `security-framework` | 3.7.0 | |
| `core-graphics` | 0.25.0 | |
| `ts-rs` | 12.0.1 | ADR 0006 |

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
| `RUST_LOG` | the app | `tracing` filter, e.g. `tadidy=debug` |
