---
name: ticket-create
description: Creates a complete new Tadidy ticket file (tickets/TAD-xxx.md) when missing work, an out-of-scope bug or a needed split is discovered — next free id, header (phase, priority, estimate, area, blockedBy, spec anchors, prototype ids), body in the same format as the other tickets, validated by tickets.mjs check. Also used to create the phase 2 and 3 backlog from the spec. Use when work is covered by no ticket, a ticket is too big, or the human says "add a ticket for…".
---

# ticket-create — all work has a ticket

Rules: [docs/guides/tickets.md](../../../docs/guides/tickets.md). Template: [tickets/_template.md](../../../tickets/_template.md).

## 1. Check it does not exist

`grep -ril "<keyword>" tickets/` (titles, bodies, file paths). If a ticket covers it, complete that one (only if it is still `todo`) instead.

## 2. Classify

- **Id**: the next free number: `ls tickets | grep -E '^TAD-[0-9]{3}\.md$' | sort | tail -1`, plus one. Ids are never reused.
- **phase**: 0 spikes · 1 MVP · 2 v1.0 · 3 v1.x ([roadmap](../../../docs/spec/06-delivery.md#roadmap)). **priority**: that of the spec features it covers (P0/P1/P2). **estimate**: S, M, L — XL means split it. **area**: core, ui, full, repo.
- **blockedBy**: the tickets that must be `done` before.
- **Title**: verb + precise object, like the others.

## 3. Write the file

Copy the template, keep every section in the same order (*Context* · *Scope* · *Out of scope* · *Steps* · *Files* · *Acceptance criteria* · *Tests* · *Verification* · optional *IPC*, *Database*, *Interface*, *Dependencies*, *Security*, *Risks* · *Definition of done* · *References* · *Notes*). Take a neighbouring ticket of the same area as a model.

- The quote line under the H1: `> Phase N · Px · area · Discovered in: TAD-yyy` (the last part only if found during another ticket).
- *Steps* start with the red tests and end with "Open the pull request to `main`."
- *Acceptance criteria*: "Given… when… then…", each one testable.
- *Tests*: Unit / Integration / UI / Manual (Manual only for the macOS layer).
- *Verification*: exact commands with the expected result, last line `scripts/check.sh` → exit 0.
- *Dependencies*: crate / npm **names** only, never versions (AGENTS.md §2).
- `spec:` cites existing anchors (`docs/spec/<file>.md#<anchor>`), `prototype:` existing ids (`"#view — #element"`).
- Header: `status: todo`, `created: <today>`, `started: ""`, `completed: ""`, `branch: ""`, `pr: ""`.

## 4. Validate and commit

```bash
node scripts/tickets.mjs check
```
- Created during another ticket: commit it **on that ticket's branch** (`docs(TAD-yyy): add TAD-zzz for <reason>`) and add a *Notes* line to TAD-yyy: "Out-of-scope work moved to TAD-zzz."
- Created on its own (backlog work): a branch `tad-zzz-add-ticket`, commit `docs(TAD-zzz): add ticket`, pull request like any other.
- If an existing `todo` ticket must wait for the new one, add the new id to its `blockedBy`.

## Splitting a ticket that is too big

Create the sub-tickets citing the original, share out steps, criteria and tests, make the original depend on them (`blockedBy`) or cancel it with a *Notes* line, and record the split in *Decisions and notes* of `PROGRESS.md`.
