# 0002 — One branch `main`, one pull request per ticket

- **Status**: accepted
- **Date**: 2026-10-28

## Context

A one-person project where the code is written by an agent and reviewed by the owner. The owner wants to review every change before it lands, without the overhead of an integration branch.

## Decision

- **`main`** is the only long-lived branch. Each ticket gets a branch `tad-042-<slug>` from an up-to-date `main` and a pull request back to `main`.
- **The human reviews** the pull request on GitHub; the agent addresses the comments (`address-pr`). There is no review bot.
- **The human merges**, with a merge commit or a rebase, **never a squash**: `main`'s history keeps the `test(…)` → `feat(…)` order that proves the TDD.
- Releases are tags `vX.Y.Z` on `main` (skill `release-dmg`).
- No direct push to `main`: the `pre-push` hook refuses it (except the very first push that creates `main` on the remote). On GitHub Free with a private repository there is no server-side protection, so the hook is the protection.

## Consequences

- Nothing reaches `main` without the human's approval.
- The ticket's status changes are commits on its branch (ADR 0003): they reach `main` with the merge.

## Rejected alternatives

- **`develop` + `main`** (as in fluent-test): useful with several people and a deployment environment; for a desktop app released by tags it only adds a promotion step.
- **`main` without pull requests**: faster, but no place to review and comment line by line.
