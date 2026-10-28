---
name: review-pr
description: Reviews a Tadidy pull request as an independent second reader — reads the pull request and its ticket, runs the checks in a dedicated worktree, reviews the diff for bugs and against AGENTS.md and the self-review checklist, verifies every finding, then posts one GitHub review (comment only) with inline comments that address-pr can process. Use when the human says "review PR #12", "fais la revue de la PR", "code review", or gives a pull request URL to review.
---

# review-pr — an independent second reader

`self-review` is the author checking their own diff. This skill is a second reader who did not write it, so it must not inherit the author's assumptions. If this session wrote the pull request, stop and ask the human to start a new session for the review.

The review is posted with the human's GitHub account, so:
- the event is always `COMMENT`, never `APPROVE` or `REQUEST_CHANGES`, and the body says it comes from the agent;
- this skill never merges, never pushes to the pull request's branch, and never changes the ticket. Fixes are the author session's job (`address-pr`).

## 1. Read

```bash
gh pr view <N> --json number,title,url,state,isDraft,headRefName,headRefOid,body,files,commits
gh pr diff <N>
gh api repos/{owner}/{repo}/pulls/<N>/comments --paginate      # earlier threads: do not repeat them
```
- `state` is not `OPEN` → say so and stop.
- The ticket: `tickets/TAD-xxx.md` from the branch name `tad-xxx-*`, read in the review worktree (step 2): *Scope*, *Out of scope*, *Acceptance criteria*, *Tests*, *Verification*, *Security*, *Risks*, and the spec anchors it cites.

## 2. Run the checks in a review worktree

Always in a worktree, even when the main folder is free ([docs/guides/parallel-sessions.md](../../../docs/guides/parallel-sessions.md)):
```bash
node scripts/worktree.mjs review <N>          # ../tadidy-worktrees/review-pr-<N>, detached on the pull request head
cd ../tadidy-worktrees/review-pr-<N>
git log --oneline --reverse origin/main..HEAD   # test(…) before feat(…) for each criterion
scripts/check.sh --cov
```
Then every command of the ticket's *Verification* section, except the ones that need a human at the screen (e.g. "no Dock icon"). Before `pnpm e2e`, check that nothing else serves port 1420 (`lsof -iTCP:1420 -sTCP:LISTEN`): otherwise the tests run against another worktree's code. Note each command with its result, and compare with the results claimed in the pull request description.

## 3. Review

Read the whole diff in the worktree, with the surrounding code, not just the hunks. In this order:

1. **Correctness**: bugs, edge cases, error paths, wrong behaviour compared with the spec anchors of the ticket, threads (watcher thread, AppKit main thread), SQL, IPC types out of sync with the front.
2. **The ticket**: every acceptance criterion has a test that would fail if it were false; TDD commit order; nothing from *Out of scope*; the *Files* and *Risks* handled.
3. **The rules**: the checklist of [self-review §2](../self-review/SKILL.md) (privacy and security, quality, UI), AGENTS.md §2, §13, §14.
4. **The tests themselves**: they check behaviour, not implementation details; none passes trivially; fakes are used behind the `macos/` traits.
5. **The pull request description**: commands actually run, manual QA, deviations explained.

Do not comment on what `cargo fmt`, Prettier or the linters already enforce, nor on taste.

## 4. Verify every finding before posting it

A wrong remark costs the author a round trip. For each finding:
- re-read the code at that line in the worktree, and the code that calls it;
- reproduce it when possible: a failing test or a command, run locally, never committed;
- keep it only if you can say *what goes wrong, with which input*. Otherwise drop it, or ask it as a question.

Label each one: **blocking** (a bug, an acceptance criterion not met, a rule of AGENTS.md §2 broken, a failing check) · **should-fix** · **nit** (few).

## 5. Post one review

One review, in English, inline comments on lines of the diff (`side: RIGHT`). A finding on a line outside the diff goes in the body as `path:line`.
```bash
REVIEW=$(mktemp)
cat > "$REVIEW" <<'JSON'
{
  "commit_id": "<headRefOid>",
  "event": "COMMENT",
  "body": "🤖 Agent review (skill `review-pr`): comments only, the decision stays with the owner.\n\n**Verdict**: changes needed, 1 blocking.\n\n**Checks run on <sha7>**\n- `scripts/check.sh --cov` → ✅\n- `pnpm e2e e2e/smoke.spec.ts` → ❌ <one line>\n\n**Findings**: 1 blocking · 2 should-fix · 1 nit (inline).",
  "comments": [
    { "path": "src-tauri/src/watcher.rs", "line": 42, "side": "RIGHT",
      "body": "**blocking**: <what goes wrong, with which input>.\n\n<how it was verified>\n\nSuggestion: <fix, or a test that reproduces it>" }
  ]
}
JSON
gh api repos/{owner}/{repo}/pulls/<N>/reviews --method POST --input "$REVIEW" && rm "$REVIEW"
```
- HTTP 422 on a comment → its line is not in the diff: move it to the body and post again.
- No finding → still post the review, with the checks and "No finding."
- Never quote a secret or clip content in a comment.

## 6. Clean up and report

```bash
cd <main folder> && node scripts/worktree.mjs remove review-pr-<N>
```
Tell the human, in their language: the pull request URL, the verdict, the number of findings per label with the blocking ones in one line each, and the checks that failed. Next step: the ticket's session runs `address-pr` on these comments, then the human makes the final review and merges.
