<!-- Title: TAD-042 — <ticket title>. Base: main. Written in English. Merge with a merge commit or rebase, never squash (ADR 0002). -->

## Ticket

[TAD-xxx — <title>](<GitHub link to tickets/TAD-xxx.md on this branch>) · area: core | ui | full | repo

## What changes

<!-- 2 to 5 lines: what and why, not the list of files. -->

## Acceptance criteria

<!-- Copied from the ticket; check only what a test (or a manual QA step for the macOS layer) verifies. -->
- [ ] …

## Verification

<!-- Commands actually run and their actual result (skill verification-before-completion). There is no CI: this table is the evidence. -->
| Command | Result |
|---|---|
| `scripts/check.sh --cov` | … |
| `…` | … |

<!-- Manual QA (macOS layer only): link to docs/guides/manual-qa.md#tad-xxx, build used, macOS version, step results. -->

## UI checklist

<!-- UI tickets only: paste the "UI ticket checklist" of DESIGN.md, filled in; prototype-parity result. Otherwise: N/A. -->

## Deviations and risks

<!-- Deviation from the ticket, the prototype or the spec, and why; what could not be verified. "None" otherwise. -->

## Checklist

- [ ] `test(TAD-xxx)` commits before `feat(TAD-xxx)`
- [ ] `scripts/check.sh --cov` green (coverage ≥ 80 % on changed files)
- [ ] No secret, clip content, `.env`, certificate or database file in the diff
- [ ] IPC type changed → bindings regenerated and front updated in this pull request (or N/A)
- [ ] Schema changed → migration added and tested, skill `db-migration` (or N/A)
- [ ] Steps checked and status `review` in the ticket file; the closing note and `done` come after the human's approval
