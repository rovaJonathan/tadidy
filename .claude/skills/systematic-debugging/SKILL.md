---
name: systematic-debugging
description: Four-phase debugging — cause first, fix second — for an unexpected red test, a build or link error, a wrong behaviour, a window or permission problem on macOS, a hook that fails. Use as soon as the cause is not obvious, especially when in a hurry, when a "quick fix" looks obvious, or after a fix that did not work.
---

# systematic-debugging — the cause before the fix

Fixing a symptom is failing later. First, search [docs/solutions/](../../../docs/solutions/): the problem may already be solved.

## Phase 1 — Understand (no change)

1. **Read the whole error**: message, backtrace (`RUST_BACKTRACE=1`), line numbers, the warnings before it.
2. **Reproduce reliably**: the exact command, from the repository root. Not reproducible → collect more data before acting (`RUST_LOG=tadidy=debug`, Console.app for the app's logs, Web Inspector for the webview).
3. **Look at what changed**: `git diff`, `git log --oneline main..HEAD`, dependencies (`Cargo.lock`, `pnpm-lock.yaml`), toolchain, macOS version, **the code signature** (a rebuild with another signature resets Accessibility and Keychain access: [runbook](../../../docs/runbooks/signing-identity.md)).
4. **Trace the data back**: where the wrong value comes from, who called with it, up to the source.

## Phase 2 — Compare

1. Find a case that works (a module, a test, a neighbouring command).
2. Read the documentation of the tool for that point in full (Tauri, AppKit, SQLCipher), not diagonally.
3. List **all** differences, even those that "cannot" matter (main thread vs background thread, activation policy, window level, sandbox, signed vs unsigned).

## Phase 3 — One hypothesis, one test

1. Write the hypothesis: "I think X is the cause because Y."
2. Test it with the smallest possible change, one variable at a time.
3. Confirmed → phase 4. Refuted → new hypothesis; **do not stack fixes**.

## Phase 4 — Fix

1. **A failing test** that reproduces the bug (`test(TAD-xxx): reproduce …`) — mandatory before the fix. For the macOS layer, reproduce it in the logic behind the trait, or add a manual QA step.
2. **One fix**, on the identified cause, no "while I'm at it".
3. **Verify**: the test passes, the rest of the suite too (`verification-before-completion`). Commit `fix(TAD-xxx): …`.
4. **If it does not work**: stop. Fewer than three attempts → back to phase 1. Three or more → question the approach, and use `ticket-problem` if the decision goes beyond the ticket.

## Capitalize

If the problem may come back (tool trap, obscure error, workaround), write `docs/solutions/<category>/YYYY-MM-DD-<slug>.md`: Symptom · Cause · Fix · How to avoid it.

## Stop signals

"A quick fix now, I'll investigate later", "let's try changing X", "I don't understand everything but it should work", "one more try" after two failures → back to phase 1.
