# PROGRESS — Tadidy

Decisions taken along the way. Ticket progress lives in the tickets ([tickets/](tickets/README.md)); this file keeps what the tickets do not say: the choices made and why.

## Decisions and notes

Add here, with the date and the ticket (`TAD-xxx`), any decision taken along the way (technical choice, accepted deviation from the prototype or the spec, answer to a `blocked` ticket). One entry per decision, newest first, in English.

- 2026-10-28 · TAD-000 · This repository's git config sets a local `core.hooksPath=.git/hooks`, because a global `core.hooksPath` (`~/.githooks`) stopped lefthook from installing. The global hook's only rule (no direct push to `main`) is reproduced by `scripts/pre-push-guard.sh`, which also lets the very first push create `main` on the remote. A fresh clone runs `git config --local core.hooksPath .git/hooks && lefthook install --force` (README).
- 2026-10-28 · TAD-000 · TypeScript bindings come from `ts-rs`, not `tauri-specta`: the Tauri 2 line of `tauri-specta` is still `2.0.0-rc.*`, and pre-releases are forbidden (ADR 0006).
- 2026-10-28 · TAD-000 · crates.io's newest `tauri` is `3.0.0-alpha`: the project installs the Tauri 2 line explicitly (`cargo add tauri@2`); AGENTS.md §6.
- 2026-10-28 · TAD-000 · The development environment mirrors fluent-test without Linear and without the review bot: tickets are markdown files (ADR 0003), one branch `main` with a pull request per ticket reviewed and merged by the owner (ADR 0002), no CI but lefthook hooks (ADR 0004), everything in English (ADR 0005).
