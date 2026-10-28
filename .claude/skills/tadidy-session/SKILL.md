---
name: tadidy-session
description: Starts or resumes a work session on Tadidy — "what should I do today?", "what's next?", "let's continue", "on reprend", "qu'est-ce que je fais aujourd'hui ?". Reads the tickets, proposes ONE ticket (in-progress or in-review first, otherwise the next ready one) and asks "Shall I do it?". No command for the human to type. Use before any code.
---

# tadidy-session — pick up where we left off

The project's memory is not in the conversation: progress is in `tickets/` (read through `scripts/tickets.mjs`), decisions in `PROGRESS.md`. This skill reloads them and decides what comes next. Full rules: [docs/guides/tickets.md](../../../docs/guides/tickets.md).

## Steps

1. **Sync, without disturbing another session**
   ```bash
   node scripts/worktree.mjs prune    # removes the worktrees of merged or closed pull requests
   node scripts/worktree.mjs status   # busy: the main folder is off main or has changes
   git fetch --prune 2>/dev/null || true
   ```
   - **Not busy** → `git checkout main && git pull --rebase` (skip the pull if there is no remote yet).
   - **Busy** → another session may be working there: touch nothing in the main folder. If you resume the ticket checked out there, first ask the human whether another session is still on it. Any other ticket goes to a worktree ([docs/guides/parallel-sessions.md](../../../docs/guides/parallel-sessions.md)), and you say so in step 4.

2. **Read the state**
   ```bash
   node scripts/tickets.mjs next --json
   ```
   `active` = tickets `in-progress` or `review` (the status read on their local `tad-xxx-*` branch); `next` = the first ready `todo` ticket (phase, then priority, then number).

3. **Decide**
   1. **A ticket `in-progress`** → resume it where its branch is checked out (`git worktree list`): its worktree, or the main folder. If the branch is checked out nowhere, `git checkout <branch>` when the main folder is free, `node scripts/worktree.mjs add <branch>` otherwise. Then `ticket-implement` from where the *Steps* checkboxes stop.
   2. **A ticket `review`** → look at its pull request: `gh pr view <url> --json state,reviewDecision,reviews,comments`.
      - Unanswered review comments → propose `address-pr`.
      - `reviewDecision: APPROVED`, or the human said "approved / LGTM" → `ticket-update`, *Closing*.
      - `state: MERGED` while the ticket on `main` is not `done` → something was skipped: say it, propose `ticket-update` *Closing* on a new branch `tad-xxx-close`.
      - Nothing yet → say it is waiting for their review.
      **A ticket in review never blocks the next one**: in the same answer, also propose `next`.
   3. **Otherwise** → propose `next`.
   4. **Nothing ready** → say what blocks (the `blockedBy` chain, `blocked` tickets with their expected decision from *Notes*).
   - If the human names a ticket ("take TAD-009"), check it is ready (`node scripts/tickets.mjs show TAD-009`, every `blockedBy` done) and go to 4 with it.
   - Mention in one line any ticket `blocked` (a decision is waiting: quote it from its *Notes*).

4. **Ask, in the human's language, without commands.** One sentence, then one question. Example: "Today: TAD-004, the pasteboard and workspace layer. TAD-003 is waiting for your review (PR #3). Shall I do TAD-004?" No JSON, no tool or skill names. Wait for yes / no. Yes → `ticket-claim` then `ticket-implement` (or `ticket-implement` directly when resuming an `in-progress` ticket). No → stop.

## Rules

- Never change a ticket's header in this skill: that is `ticket-claim`, `ticket-update` and `ticket-problem`'s job.
- One session = one ticket. For the next one, start from a clean context.
