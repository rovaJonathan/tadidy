---
name: ticket-claim
description: Claims the Tadidy ticket the human just accepted ("yes", "do it", "go") after "Shall I do it?" — checks it is ready, creates its branch from main, sets status in-progress and commits the claim on the branch. Refuses a ticket whose blockedBy is not done or that is blocked/canceled. The human types no command.
---

# ticket-claim — take one ticket, cleanly

Rules: [docs/guides/tickets.md](../../../docs/guides/tickets.md).

## Steps

1. **Start clean and up to date**
   ```bash
   git status --short                 # nothing pending; otherwise finish or stash it first
   git checkout main && git pull --rebase   # skip the pull if there is no remote yet
   ```

2. **One ticket: the one proposed by `tadidy-session` or named by the human.** Not another one.

3. **Check just before writing**: `node scripts/tickets.mjs show TAD-xxx`
   - effective status `todo` (not `blocked`, `canceled`, or already claimed on a branch: `source` is `working tree`);
   - every `blockedBy` ticket is `done` (`node scripts/tickets.mjs show <dep>`).
   A false condition → write nothing, say it in one sentence, go back to `tadidy-session`.

4. **Create the branch** `tad-xxx-<short-slug>` (3–5 words of the title, kebab-case):
   ```bash
   git checkout -b tad-004-pasteboard-workspace-layer
   ```

5. **Claim**
   ```bash
   node scripts/tickets.mjs set TAD-004 status=in-progress branch=tad-004-pasteboard-workspace-layer started=$(date +%F)
   ```
   Append to the ticket's *Notes*: `- YYYY-MM-DD · Claimed, branch tad-004-pasteboard-workspace-layer.`
   ```bash
   git add tickets/TAD-004.md && git commit -m "chore(TAD-004): claim ticket"
   ```

6. Continue with **`ticket-implement`**.

## Releasing a ticket

To give a ticket up: `node scripts/tickets.mjs set TAD-xxx status=todo branch="" started=""`, a *Notes* line saying why, commit on the branch, then keep or delete the branch with the human's agreement.
