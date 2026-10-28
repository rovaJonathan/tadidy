---
title: A git hook run from a worktree makes the tests write to the real repository
date: 2026-10-28
area: repo
tags: [git, worktree, hooks, tests]
---

## Symptom

`git push` from a worktree: the pre-push hook (`scripts/check.sh --cov`) fails in `tickets.test.mjs` and `worktree.test.mjs` ("re-init: ignored --initial-branch", "could not lock config file …/tadidy/.git/config", "untracked working tree files would be overwritten"). Afterwards, the real repository has `core.bare=true`, `user.name=Test` in its local config, test branches, and the worktree's branch moved onto test commits. The same push from the main folder is fine.

## Cause

Git exports `GIT_DIR` to hooks. From the main folder it is relative (`.git`), so a test that runs git in a temporary folder still finds that folder's own repository. From a worktree it is absolute (`…/tadidy/.git/worktrees/<name>`), so every `git init`, `git config`, `git commit` and `git checkout` of the tests acts on the real repository.

## Fix

- `scripts/check.sh` unsets `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY`, `GIT_PREFIX` before running anything.
- `scripts/worktree.mjs` drops them from the environment of every git command it runs.
- Repair after the incident: `git config --local core.bare false`, unset the local `user.name` / `user.email`, point the branch back to its last real commit (`git reflog <branch>`, `git branch -f`), `git symbolic-ref HEAD` + `git reset` in the worktree, remove the test worktrees and branches.

## How to avoid it

A test or script that runs git in a folder other than the current repository passes `cwd` **and** an environment without those variables. Never rely on `-C` or `cwd` alone: `GIT_DIR` wins over both.
