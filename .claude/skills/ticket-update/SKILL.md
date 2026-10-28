---
name: ticket-update
description: Keeps a Tadidy ticket's tracking in two steps. Opening, at the end of ticket-implement — steps checked, PROGRESS.md if a decision was taken, status review, branch pushed, pull request to main; the ticket waits for the human's review. Closing, once the human approved — closing note, status done, pushed on the branch, then "Shall I take the next one?". Never merges. Without this skill a ticket is not finished.
---

# ticket-update — a ticket is finished only when its tracking is

The tickets only know a ticket is done through this skill. **No exception**: no "later", no status edited by hand.

Two steps: **opening** (end of `ticket-implement`, no question to the human) and **closing** (after the human's approval). In between, the review is handled by `address-pr`.

## Opening — preconditions (otherwise back to `ticket-implement`)

- Every command of the *Verification* section passes, and `scripts/check.sh --cov` passes.
- `self-review` is done; `prototype-parity` too for a UI ticket; manual QA run for the macOS layer.
- The `test(TAD-xxx)` commits come before the `feat(TAD-xxx)` commits.
- Every `blockedBy` ticket is `done`.

## Opening — steps

1. **Check what remains.** In `tickets/TAD-xxx.md`, every remaining `- [ ]` in *Steps* that is really done → `- [x]`; check the *Acceptance criteria* verified by a test. A step deliberately not done stays unchecked and is explained in the closing note (rare; otherwise it is a `ticket-create`).

2. **`PROGRESS.md`** if a decision was taken during the ticket: one dated line at the top of *Decisions and notes* (`- YYYY-MM-DD · TAD-xxx · <decision and why>`).

3. **Push and open the pull request to `main`**
   ```bash
   git push -u origin HEAD
   BODY=$(mktemp)
   # fill $BODY from .github/pull_request_template.md: ticket link, what changes, criteria,
   # commands actually run with their real result, manual QA results, UI checklist, deviations
   gh pr create --base main --title "TAD-xxx — <ticket title>" --body-file "$BODY"
   ```
   The *Ticket* section links the ticket file on the branch: `https://github.com/<owner>/<repo>/blob/<branch>/tickets/TAD-xxx.md`.

4. **Status and notes**
   ```bash
   node scripts/tickets.mjs set TAD-xxx status=review pr=<pull request url>
   ```
   *Notes*: `- YYYY-MM-DD · Pull request opened: <url>. Waiting for review.` Then
   ```bash
   git add tickets/TAD-xxx.md PROGRESS.md && git commit -m "chore(TAD-xxx): open pull request" && git push
   ```

5. **Tell the human**, without asking "Is it OK?": the pull request is open for their review; "address the review" will run `address-pr`; "approved" will close.

## Closing — preconditions

- The human approved: `gh pr view <N> --json reviewDecision` is `APPROVED`, **or** the human said so in the conversation ("approved", "LGTM", "c'est bon"). Otherwise it is `address-pr`, not closing.
- No review comment left without an answer.
- `scripts/check.sh --cov` passes on the branch as it is now.

## Closing — steps

1. **Closing note** appended to *Notes*:
   ```
   - YYYY-MM-DD · Done.
     - Did: <what was done, 2 to 5 lines>
     - Deviations: <files added or missing, step not done and why, dependency not listed in the ticket; "none" otherwise>
     - Commits: <sha of the last feat(TAD-xxx)>[, <other sha, including review fixes>]
     - Pull request: <url> (approved by <login or "the owner">)
   ```
2. **Status**
   ```bash
   node scripts/tickets.mjs set TAD-xxx status=done completed=$(date +%F)
   git add tickets/TAD-xxx.md && git commit -m "chore(TAD-xxx): close ticket" && git push
   ```
3. **Do not merge.** The human merges, with a merge commit or rebase, not squash (AGENTS.md §11).
4. **Say it is done, then ask about the next one.** `node scripts/tickets.mjs next` (mention the tickets this one unblocks). To the human, no command: "TAD-xxx is closed; the pull request is ready for you to merge. Next is TAD-yyy, <title>. Shall I take it?" Yes → `ticket-claim` (from `main` once merged, or from `main` anyway if the next ticket does not depend on this one). No → stop.

## If the pull request is refused or a bug shows after done

- Small fix in the same pull request: new commits, a *Notes* line with the new sha.
- Reopen: `status=in-progress`, *Notes* "Reopened: <reason>". If `done` tickets depend on it, do not reopen: create a fix ticket (`ticket-create`, citing TAD-xxx).
