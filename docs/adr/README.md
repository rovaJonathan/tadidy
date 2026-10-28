# Architecture decisions (ADR)

A durable decision that commits the architecture, the tooling or the way the repository works = one file `NNNN-<slug>.md`, numbered in sequence. An ADR is not rewritten: when it no longer holds, a new ADR replaces it and the old one becomes "Superseded by NNNN".

Format: Status · Date · Context · Decision · Consequences · Rejected alternatives.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-tauri-svelte-rust-core.md) | Tauri 2 shell, Rust core, Svelte 5 + TypeScript front | Accepted |
| [0002](0002-main-and-a-pull-request-per-ticket.md) | One long-lived branch `main`, one pull request per ticket, reviewed and merged by the human | Accepted |
| [0003](0003-tickets-as-markdown.md) | Tickets are markdown files in `tickets/`, the status travels with the ticket's branch | Accepted |
| [0004](0004-local-hooks-no-ci.md) | No CI: lefthook hooks and `scripts/check.sh` are the only gate | Accepted |
| [0005](0005-english-everywhere.md) | Everything in the repository is written in English | Accepted |
| [0006](0006-ts-bindings-with-ts-rs.md) | TypeScript types of the IPC contract are generated from Rust with `ts-rs` | Accepted |
