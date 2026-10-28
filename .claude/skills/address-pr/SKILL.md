---
name: address-pr
description: Addresses the human's review of a Tadidy pull request — reads every comment and thread, checks each remark before applying it, fixes with TDD, commits, pushes, answers each thread in English, posts a summary comment. When the human approved, hands over to ticket-update closing. Use when the human gives a pull request URL or number, says "address the review", "traite la revue", "answer the comments", or when a check run locally fails on the branch.
---

# address-pr — one remark, one check, one answer

There is no review bot: the reviewer is the human (ADR 0002), sometimes preceded by an agent review posted with their account by `review-pr` (its body starts with "🤖 Agent review"). Every remark gets the same scrutiny: a reviewer can be wrong, and agreeing out of politeness is a mistake. Only the human's review can approve.

## 1. Read all the feedback

```bash
gh pr view <N> --json title,url,headRefName,state,reviewDecision,reviews,comments
gh api repos/{owner}/{repo}/pulls/<N>/comments --paginate      # inline threads (id, path, line, body, in_reply_to_id)
```

Do not skip threads that look resolved. Ignore only compliments and remarks superseded by a newer one.

- `reviewDecision: APPROVED` (or the human said "approved") and no remark waiting → nothing to fix: go to **`ticket-update`**, *Closing*. Do not merge.
- No review yet → say so and stop.

## 2. Check before applying

For each remark, before touching the code:

- **Is it right?** Check it against the ticket, AGENTS.md, the spec, the prototype and DESIGN.md.
- **Is it in scope?** Out of the ticket's scope → propose a new ticket (`ticket-create`), do not add it here.
- **Does it contradict a settled rule** (AGENTS.md §2–3, the prototype, an ADR)? → answer with the reference instead of applying it; if the human insists, it is their call: apply it and record the decision in `PROGRESS.md` (and a new ADR if it changes one).

Classify: to fix · to discuss (with the argument) · out of scope (with the proposed ticket).

## 3. Fix

```bash
git worktree list                  # the branch already checked out somewhere? work there
gh pr checkout <N>                 # otherwise, in the main folder if it is free
```
If the branch is checked out nowhere and the main folder is busy (`node scripts/worktree.mjs status`), use `node scripts/worktree.mjs add <headRefName>` ([docs/guides/parallel-sessions.md](../../../docs/guides/parallel-sessions.md)).
- A behaviour to change or a bug reported → first a failing test (skill `tdd`).
- A local check failing → `systematic-debugging`, no random fix.
- Changes limited to the feedback; no neighbouring refactor.
- Rerun `scripts/check.sh --cov` and the affected *Verification* commands (`verification-before-completion`).

## 4. Commit, push, answer

In this order:

a) **Commit and push the fixes**
```bash
git status --short && git diff
git add <files>
git commit -m "fix(TAD-xxx): <review feedback addressed>"
git push
```
New commits only: no `--amend`, no force push unless the human asks.

b) **Answer every thread, in English**, in the thread itself:
```bash
gh api repos/{owner}/{repo}/pulls/<N>/comments/<comment id>/replies -f body='✅ Fixed in <sha>: <one line>'
gh api repos/{owner}/{repo}/pulls/<N>/comments/<comment id>/replies -f body='💬 Not changed: <reason, with the AGENTS.md / spec / ticket reference>'
gh api repos/{owner}/{repo}/pulls/<N>/comments/<comment id>/replies -f body='➡️ Out of scope, moved to TAD-zzz.'
```
Remarks made in the review body or as PR comments (no thread) are answered in the summary below.

c) **One summary comment** on the pull request:
```bash
BODY=$(mktemp)
cat > "$BODY" <<'MSG'
Review addressed:
1. ✅ <remark> — fixed in <sha>
2. 💬 <remark> — not changed: <reason>
3. ➡️ <remark> — moved to TAD-zzz
`scripts/check.sh --cov`: <result>
MSG
gh pr comment <N> --body-file "$BODY" && rm "$BODY"
```

d) **Ticket**: a *Notes* line `- YYYY-MM-DD · Review round <n> addressed: <sha list>.` committed on the branch (`chore(TAD-xxx): note review round`).

## 5. Summarize to the human

Pull request URL, remarks fixed, remarks discussed or moved to a ticket, the local check result, and what is expected from them now (another look, or "approved").
