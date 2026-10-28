---
name: ticket-problem
description: Reports a blocked Tadidy ticket properly — status blocked, a Notes entry with the cause, what was tried and the expected decision with options and a recommendation, work saved on the branch — then moves on to another ready ticket. Use as soon as you are blocked (missing information, contradiction between ticket, prototype and spec, failing dependency, unavailable tool, untestable behaviour, ticket too big) instead of improvising.
---

# ticket-problem — block cleanly rather than guess

A well-written blocker is unblocked in one answer. A vague one costs a meeting.

## When

- The ticket, the prototype and the spec say different things.
- Needed information is missing (a value, a rule, an access).
- A `done` dependency does not do what its closing note says.
- A tool or an API refuses to work after 30 minutes of reasonable attempts.
- The ticket is too big for one session: propose the split in the expected decision.

**Not** a blocker: a red test during TDD, a type error, extra work discovered (→ `ticket-create`).

## Steps

1. **Save the work** on the ticket's branch: `git add -A && git commit -m "wip(TAD-xxx): <state>"` (the `pre-push` hook runs the suite: push only if it passes, otherwise keep the branch local and say so).

2. **Write the blocker** in the ticket's *Notes*:
   ```
   - YYYY-MM-DD · Blocked — decision expected.
     - Cause: <the precise fact, with the file, command or error message>
     - Tried: <each attempt and its result>
     - Decision expected: <the closed question, the options, and your recommendation>
     - Branch: <branch name>
   ```
   Example: "Option A: mock `tauri-nspanel` behind a `PanelHost` trait and test the toggle logic only (recommended, 1 h); option B: wait for an upstream fix."

3. **Mark it**
   ```bash
   node scripts/tickets.mjs set TAD-xxx status=blocked
   git add tickets/TAD-xxx.md && git commit -m "chore(TAD-xxx): mark blocked"
   ```

4. **Back to `main`**: `git checkout main`.

5. **Tell the human** the cause and the decision expected, then propose another ready ticket (`tadidy-session`, step 3).

## Unblocking (when the human decides)

- *Notes*: `- YYYY-MM-DD · Decision: <what was decided, by whom>`.
- `PROGRESS.md`: one line in *Decisions and notes*.
- `node scripts/tickets.mjs set TAD-xxx status=in-progress` (on its branch), or `status=todo` if the work restarts later.
- If the decision changes the ticket (scope, files, criteria), update its body; if it changes the order, update `blockedBy`.
