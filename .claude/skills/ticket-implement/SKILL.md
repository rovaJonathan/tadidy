---
name: ticket-implement
description: Carries out the claimed Tadidy ticket end to end — read everything, short plan, TDD criterion by criterion, verification, self-review — then opens the pull request (ticket-update, opening) without asking. Does not mark the ticket done. Use as soon as a ticket is claimed or resumed.
---

# ticket-implement — the conductor

The ticket file contains what is needed: context, scope, steps, files, acceptance criteria, tests, verification, and when they apply IPC, database, interface, dependencies, security, risks, references (spec, prototype). **Guess nothing**: missing or contradictory information is a `ticket-problem`, not an improvisation.

## 1. Read everything (no code yet)

- **The ticket**: `tickets/TAD-xxx.md` in full, including *Notes* (an earlier blocker decision may change the scope).
- **Spec**: every anchor in `spec:` (`docs/spec/<file>.md#<anchor>`), and the related architecture sections.
- **Prototype** (UI tickets): for every `prototype:` entry, open `docs/prototype/prototype.html` at that view and read the HTML and JS of the element: exact texts, ids, states (empty, error, loading), keyboard behaviour. Then the matching components in [DESIGN.md](../../../DESIGN.md).
- **Done dependencies**: the closing note (*Notes*) of each `blockedBy` ticket, to reuse what exists instead of redoing it.
- **Decisions**: *Decisions and notes* in `PROGRESS.md`; [docs/solutions/](../../../docs/solutions/) for traps already met.
- **Conventions**: AGENTS.md §13 (Rust) and §14 (front).

## 2. Write a short plan

In the conversation, 5 to 10 lines: files touched (*Files*), order of the acceptance criteria, matching tests (*Tests*), risks (*Risks*) and their counter-measure, any planned deviation from the ticket. A plan longer than that goes to `docs/plans/YYYY-MM-DD-TAD-xxx-<slug>-plan.md`.

## 3. TDD, criterion by criterion

For each acceptance criterion, skill **`tdd`**: red test → `test(TAD-xxx)` commit → minimal code → green → `feat(TAD-xxx)` commit → refactor.

- Scaffolds: `scaffold-tauri-command` (IPC command), `db-migration` (schema).
- **Check each step as soon as it is done**: `- [ ]` → `- [x]` on that line only of `tickets/TAD-xxx.md`, committed with the work it describes.
- **Dependencies** listed in the ticket: install the **name** at the **latest stable** version (AGENTS.md §2: `pnpm view <pkg> version` then `pnpm add <pkg>@<that>`; `cargo add <crate>`, `cargo add tauri@2` for the Tauri line). Never a version copied from the ticket or memory. Note any dependency not listed in the ticket for the closing note.
- **macOS layer** (`src-tauri/src/macos/`): keep it thin behind a trait, test the logic with fakes, and add the manual steps to `docs/guides/manual-qa.md` under `### TAD-xxx — <title>`.

## 4. Verify

- Run **every** command of the *Verification* section and compare with the expected result.
- Always: `scripts/check.sh --cov` (lint, typecheck, tests, coverage ≥ 80 % on changed files once TAD-002 is done).
- macOS layer: run the manual QA steps on a signed build and note the results for the pull request.
- Skill **`verification-before-completion`**: no "it passes" without the output in front of you.
- UI ticket: skill **`prototype-parity`**, and fill the DESIGN.md UI checklist.

## 5. Open the pull request, then hand over to the human

1. Skill **`self-review`** on the whole diff; fix what it finds (a bug = a test first).
2. Skill **`ticket-update`**, *Opening*: steps checked, status `review`, pull request to `main`. No "Is the ticket OK?" question: the human's review on GitHub replaces it.
3. **Tell the human**, one sentence, no command: "TAD-xxx is done: <what>. Pull request <url> is open for your review; tell me "address the review" when you have commented, or "approved"."
4. Comments → **`address-pr`**. Approval → **`ticket-update`**, *Closing*. The human merges (AGENTS.md §11).

## If it blocks

- A bug or red test whose cause is not obvious: **`systematic-debugging`** before changing anything.
- Missing information, contradiction between ticket / prototype / spec, failing dependency, untestable behaviour, environment broken for more than 30 minutes: **`ticket-problem`**.
- Necessary work outside the scope: **`ticket-create`**, and carry on without it.
