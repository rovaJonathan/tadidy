# PROGRESS — Tadidy

Decisions taken along the way. Ticket progress lives in the tickets ([tickets/](tickets/README.md)); this file keeps what the tickets do not say: the choices made and why.

## Decisions and notes

Add here, with the date and the ticket (`TAD-xxx`), any decision taken along the way (technical choice, accepted deviation from the prototype or the spec, answer to a `blocked` ticket). One entry per decision, newest first, in English.

- 2026-10-28 · TAD-001 · TypeScript is on 6.0.3, not 7.0.2: `typescript-eslint` 8.72.1 requires `<6.1.0` and `svelte-check` 4.7.7 `^5 || ^6`. Move to 7 once both accept it (AGENTS.md §6: newest major the toolchain accepts).
- 2026-10-28 · TAD-001 · pnpm stays on the 10 line (10.34.9) although npm's `latest` is 12.x: AGENTS.md §6 fixes pnpm 10; a jump needs an ADR. `@types/node` follows the 24 line to match Node 24 LTS.
- 2026-10-28 · TAD-001 · `.cargo/config.toml` (sets `TS_RS_EXPORT_DIR`) lives at the repo root, not in `src-tauri/.cargo/` as the ticket said: cargo reads config from the working directory and its parents, never from `--manifest-path`, so `cargo test --manifest-path src-tauri/Cargo.toml` from the root wrote the bindings to `src-tauri/bindings/`.
- 2026-10-28 · TAD-001 · `TauriEventSink<R: Runtime = Wry>` is generic so its test runs on Tauri's mock runtime (`tauri` dev-dependency feature `test`); later services keep using `RecordingEventSink`.
- 2026-10-28 · TAD-001 · The dev harness (`src/main.ts`, `src/App.svelte`) is excluded from the front coverage report: it is covered by `e2e/smoke.spec.ts`. Later UI components are not excluded.
- 2026-10-28 · TAD-020 · `scripts/check.sh` unsets the repository variables git exports to hooks (`GIT_DIR`…): from a worktree they are absolute, and the first pre-push from one made the tests' temporary repositories write to the real repository (`core.bare=true`, a test identity, test branches). See `docs/solutions/repo/2026-10-28-git-dir-from-hooks-in-worktrees.md`.
- 2026-10-28 · TAD-020 · A session that starts while the main folder is busy (off `main` or with changes) works in a git worktree in `../tadidy-worktrees/`, created by `scripts/worktree.mjs`. The folder sits next to the repository, not in `.claude/worktrees/`, so Vite, Vitest, ESLint and Cargo never scan the copies. Each worktree has its own `node_modules` and `src-tauri/target/`.
- 2026-10-28 · TAD-020 · `core.hooksPath` is now absolute (`<common git dir>/hooks`): the relative `.git/hooks` of TAD-000 points at a file inside a worktree, and a commit with an invalid message went through there without any hook running. Replaces the README command of the TAD-000 entry below.
- 2026-10-28 · TAD-020 · `review-pr` posts the agent review directly on GitHub (the human's choice), as a `COMMENT` review only: the agent uses the human's account, and only the human approves. `address-pr` processes its comments like the human's.

- 2026-10-28 · TAD-000 · This repository's git config sets a local `core.hooksPath=.git/hooks`, because a global `core.hooksPath` (`~/.githooks`) stopped lefthook from installing. The global hook's only rule (no direct push to `main`) is reproduced by `scripts/pre-push-guard.sh`, which also lets the very first push create `main` on the remote. A fresh clone runs `git config --local core.hooksPath .git/hooks && lefthook install --force` (README).
- 2026-10-28 · TAD-000 · TypeScript bindings come from `ts-rs`, not `tauri-specta`: the Tauri 2 line of `tauri-specta` is still `2.0.0-rc.*`, and pre-releases are forbidden (ADR 0006).
- 2026-10-28 · TAD-000 · crates.io's newest `tauri` is `3.0.0-alpha`: the project installs the Tauri 2 line explicitly (`cargo add tauri@2`); AGENTS.md §6.
- 2026-10-28 · TAD-000 · The development environment mirrors fluent-test without Linear and without the review bot: tickets are markdown files (ADR 0003), one branch `main` with a pull request per ticket reviewed and merged by the owner (ADR 0002), no CI but lefthook hooks (ADR 0004), everything in English (ADR 0005).
