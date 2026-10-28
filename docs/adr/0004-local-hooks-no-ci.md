# 0004 — No CI: local hooks are the gate

- **Status**: accepted
- **Date**: 2026-10-28

## Context

A macOS app needs macOS runners to build and test; GitHub-hosted macOS minutes are expensive for a private repository, and a self-hosted runner only works when the Mac is on. The owner chose no CI.

## Decision

- **lefthook** (`lefthook.yml`) runs:
  - `pre-commit`: gitleaks on the staged changes, `tickets.mjs check` when tickets, spec or prototype change, `cargo fmt --check` and `prettier --check` on staged files.
  - `commit-msg`: Conventional Commits scoped to a ticket (`scripts/commit-msg.mjs`).
  - `pre-push`: no direct push to `main` (`scripts/pre-push-guard.sh`), then `scripts/check.sh --cov` — repository scripts, tickets, repository consistency, `cargo fmt`, `clippy -D warnings`, Rust tests with coverage, bindings drift, front lint, typecheck, tests with coverage, 80 % on changed files.
- The repository sets a **local** `core.hooksPath` to `.git/hooks` so lefthook can install even when a global hooks path exists; the global "no push to main" rule is reproduced by `pre-push-guard.sh`.
- **Bypassing a hook is forbidden** (`--no-verify`, `LEFTHOOK=0`): AGENTS.md §2.
- The pull request lists the commands that were run and their result (template), since nothing re-runs them on the server.

## Consequences

- Zero cost, and the feedback comes before the push.
- The gate depends on discipline: a clone without `lefthook install --force` has no gate. The README setup does it.
- `pre-push` takes as long as the full test suite; it is the price of having no CI.

## Rejected alternatives

- **GitHub Actions on `macos-latest`**: the most reliable gate, but paid minutes (×10) on a private repository.
- **Self-hosted runner on the owner's Mac**: free, but only runs when the Mac is awake, and it shares the machine with development.
