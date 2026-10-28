# Tickets

Tadidy's tracking lives in the repository: one markdown file per ticket in [`tickets/`](../../tickets/README.md) ([ADR 0003](../adr/0003-tickets-as-markdown.md)). The human talks to the agent; the agent reads and writes the tickets through the `ticket-*` skills and `scripts/tickets.mjs`. This guide is the full set of rules those skills follow.

## A ticket file

`tickets/TAD-042.md` = a YAML header + a markdown body. Copy [`tickets/_template.md`](../../tickets/_template.md) (skill `ticket-create`).

| Header field | Values | Meaning |
|---|---|---|
| `id` | `TAD-042` | Same as the file name. Three digits, never reused. |
| `title` | text | Verb + precise object. |
| `status` | `todo` · `in-progress` · `review` · `done` · `blocked` · `canceled` | See *Statuses*. |
| `phase` | `0` spikes · `1` MVP · `2` v1.0 · `3` v1.x | [Roadmap](../spec/06-delivery.md#roadmap). |
| `priority` | `P0` · `P1` · `P2` | As in the spec feature tables. |
| `estimate` | `S` (≤ 2 h) · `M` (½ day) · `L` (1 day) · `XL` (split it) | |
| `area` | `core` (Rust) · `ui` (Svelte) · `full` (both) · `repo` (tooling, docs) | |
| `blockedBy` | `[TAD-001, TAD-007]` | Tickets that must be `done` first. |
| `spec` | `[docs/spec/02-features.md#cap-01]` | Spec anchors; checked by `tickets.mjs check`. |
| `prototype` | `["#popover — #search-input"]` | Prototype ids; checked by `tickets.mjs check`. |
| `branch` | `tad-042-short-slug` | Set by `ticket-claim`. |
| `pr` | URL, or `none` | Set by `ticket-update` (`none` only for the bootstrap ticket). |
| `created` · `started` · `completed` | `YYYY-MM-DD` | |

Body sections, in this order (omit an optional section that does not apply): *Context* · *Scope* · *Out of scope* · *Steps* (checkboxes) · *Files* · *Acceptance criteria* (checkboxes, "Given… when… then…") · *Tests* (Unit / Integration / UI / Manual) · *Verification* (exact commands → expected result) · optional *IPC*, *Database*, *Interface*, *Dependencies*, *Security*, *Risks* · *Definition of done* · *References* · *Notes*.

*Notes* is an append-only log written by the skills: claim, pull request opened, review rounds, blocker, decision, closing note. Newest entry last, each starting with a date.

## Statuses

| Status | Meaning | Set by |
|---|---|---|
| `todo` | To do. **Ready** when every `blockedBy` ticket is `done`. | `ticket-create` |
| `in-progress` | Claimed, branch created. | `ticket-claim` |
| `review` | Pull request open, waiting for the human's review. | `ticket-update` (opening) |
| `done` | The human approved the pull request; closing note written. The merge follows, by the human. | `ticket-update` (closing) |
| `blocked` | A human decision is needed; cause, attempts and expected decision in *Notes*. | `ticket-problem` |
| `canceled` | Abandoned or duplicate, reason in *Notes*. | the human, through the agent |

```
todo ──claim──▶ in-progress ──PR──▶ review ──approved──▶ done ──(human merges)
  ▲                 │                  │
  └──── unblock ── blocked ◀───────────┘ (problem)
```

## Where the status lives

A status change is a commit **on the ticket's branch** (`chore(TAD-042): claim ticket`, `chore(TAD-042): open pull request`, `chore(TAD-042): close ticket`), so it reaches `main` with the merge. Until then, `main` still says `todo`.

`node scripts/tickets.mjs` therefore reads each ticket twice: the file in the working tree, and the same file on the local branch `tad-042-*` if one exists. **The branch wins.** That is how `next` knows that TAD-042 is in review while you are back on `main`.

## Commands (for the agent)

```bash
node scripts/tickets.mjs list [--status todo] [--phase 1]   # table of tickets, effective status
node scripts/tickets.mjs next                              # what to work on: in-progress / review first, then the next ready ticket
node scripts/tickets.mjs ready                             # every ready ticket, in order
node scripts/tickets.mjs show TAD-042                       # header + effective status + branch
node scripts/tickets.mjs set TAD-042 status=review pr=<url> # edit header fields in place (validated)
node scripts/tickets.mjs check                             # validate every ticket (run by the hooks)
```

## Rules

- **Order of proposal** (`next`): first a ticket `in-progress` or `review`; otherwise the ready `todo` ticket with the lowest phase, then the highest priority (P0 first), then the lowest number.
- **Taken tickets**: an `in-progress` ticket whose branch is checked out in the main folder or a worktree belongs to another session. `next` lists it under `taken` and a new session never proposes it: it proposes the next ticket instead. It is resumed only when the human names it.
- **Closed on `main` wins**: a ticket `done` or `canceled` on `main` (`origin/main`, or `main` without a remote) keeps that status even if an old local `tad-xxx` branch says `review`.
- **One ticket per session.** Several tickets may be `review` at the same time: a ticket waiting for review never blocks taking the next ready one.
- **Steps**: the agent checks a step (`- [ ]` → `- [x]`) as soon as it is done, without changing anything else in the body.
- **Blocked**: `status: blocked` + a *Notes* entry *Blocked — decision expected* (cause, attempts, decision expected with options and a recommendation). Unblocking = the decision in *Notes*, a line in `PROGRESS.md`, `status: todo` or `in-progress`.
- **Never delete** a ticket file: cancel it. Ids are never reused.
- **Branch**: `tad-042-<short-slug>` from an up-to-date `main`. Pull request title `TAD-042 — <title>`.
- `tickets.mjs check` (run by the `pre-commit` hook when a ticket changes, and by `scripts/check.sh`) refuses: a malformed header, an unknown status, a `blockedBy` that does not exist or a dependency cycle, a spec anchor or prototype id that does not exist, a missing required section, `in-progress`/`review` without `branch`, `review`/`done` without `pr`, `done` without `completed`.
