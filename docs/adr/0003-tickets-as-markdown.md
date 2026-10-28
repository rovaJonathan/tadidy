# 0003 — Tickets as markdown files

- **Status**: accepted
- **Date**: 2026-10-28

## Context

The work is split into tickets that an agent picks up, implements with TDD and closes. There is no Linear (or other tracker) for this project; the owner wants the tracking in the repository, readable on GitHub and editable by hand.

## Decision

- One file per ticket: `tickets/TAD-042.md`, a YAML header (`status`, `phase`, `priority`, `estimate`, `area`, `blockedBy`, `spec`, `prototype`, `branch`, `pr`, dates) and a markdown body with fixed sections (template: `tickets/_template.md`; rules: `docs/guides/tickets.md`).
- The header is changed only through `node scripts/tickets.mjs set`, called by the `ticket-*` skills; `tickets.mjs check` validates every ticket (fields, references to spec anchors and prototype ids, dependencies, cycles, required sections) in the `pre-commit` hook and in `scripts/check.sh`.
- **The status travels with the ticket's branch**: claim, pull request and close are commits on `tad-042-*`. `tickets.mjs` reads the ticket on `main` and on the local branch; the branch wins. Comments that Linear would hold go in the ticket's *Notes* section.

## Consequences

- No external tool, no API key, no quota; the history of a ticket is the git history of its file.
- No board UI: `node scripts/tickets.mjs list|next|ready` replaces it.
- A status change made on a branch is invisible on another clone until the branch is pushed; acceptable for one person.

## Rejected alternatives

- **One JSON file per ticket** (the first tracking of fluent-test): strict, but unpleasant to read and edit by hand, and it needed a generated board.
- **A single `BOARD.md`**: simplest, but it grows long and every ticket change conflicts with every other.
- **Linear / GitHub Issues**: a second source of truth outside the repository, and an MCP or API dependency for the agent.
