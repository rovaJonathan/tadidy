# Parallel sessions: one worktree per extra session

Several sessions (agents or humans) can work at the same time, each on its own ticket or pull request review. They must never share a working folder: a session that switches branch or edits files under another one breaks its work silently. A session that starts while the main folder is busy therefore works in a **git worktree**: a second folder of the same repository, on its own branch, sharing commits and branches with the main folder.

## Is the main folder busy?

```bash
node scripts/worktree.mjs status      # { main, branch, dirty, busy }
```

The main folder is **busy** when it is not on `main` or has uncommitted changes: another session is probably working there. Then:

| What this session does | Where it works |
|---|---|
| The ticket checked out in the main folder | Nothing: another session owns it (`tickets.mjs next` lists it under `taken`). Propose the next ticket. Resume it in the main folder only if the human names it. |
| Another ticket | `node scripts/worktree.mjs add <branch>` → `../tadidy-worktrees/<branch>` |
| A pull request review (`review-pr`) | `node scripts/worktree.mjs review <N>` → `../tadidy-worktrees/review-pr-<N>`, always, even when the main folder is free |
| A ticket whose branch already has a worktree | That worktree (`git worktree list`) |
| A release (`release-dmg`) | Only in the main folder on a clean `main`. If it is busy, wait. |

Not busy → work in the main folder as usual.

## Working in a worktree

- `add` creates the branch from an up-to-date `origin/main` (no `git checkout main` needed) and runs `pnpm install`. The first Rust build in a worktree compiles everything: each worktree has its own `src-tauri/target/`.
- **Every command and every file path targets the worktree**: `cd ../tadidy-worktrees/<branch>` first, absolute paths for the editing tools. Never `git checkout` in the main folder: it belongs to the other session.
- The skills apply unchanged inside the worktree. `ticket-claim` creates the branch with `add` instead of `git checkout -b`; the claim note says the ticket is in a worktree.
- `node scripts/tickets.mjs` sees every local branch, worktree or not: ticket statuses stay correct.
- Hooks run in worktrees only because `core.hooksPath` is absolute (README, *Set up a clone*). `add` and `review` refuse to run otherwise. A relative path would skip every hook silently.
- Hooks run from a worktree export an absolute `GIT_DIR`. `scripts/check.sh` and `worktree.mjs` drop it, and any new script or test that runs git in another folder must drop it too ([solution](../solutions/repo/2026-10-28-git-dir-from-hooks-in-worktrees.md)).
- **End-to-end tests use port 1420 and reuse any server already running there.** Two worktrees running `pnpm e2e` at the same time would test each other's code. Before `pnpm e2e`, check that nothing else serves the port: `lsof -iTCP:1420 -sTCP:LISTEN`.

## Cleaning up

```bash
node scripts/worktree.mjs prune          # at the start of every session (tadidy-session)
node scripts/worktree.mjs remove <name>  # one worktree, e.g. review-pr-12
```

`prune` removes review worktrees and the worktrees whose pull request is merged or closed, but never one with uncommitted changes. The branches stay.
