---
name: verification-before-completion
description: Forbids saying "it's done", "the tests pass", "it's fixed" or "the build works" without having run, in this same turn, the command that proves it and read its output. Use before any status change, commit, push or pull request description, and whenever you are about to claim a result.
---

# verification-before-completion — proof before claim

Saying it is done without checking does not save time: it hands false information to the reviewer and to the ticket. There is no CI (ADR 0004): your output is the only evidence.

## The rule

**No claim without fresh proof.** If the command was not run in this turn, you cannot say it passes.

1. **Identify** the command that proves the claim (*Verification* section of the ticket, `scripts/check.sh`, a test, a build).
2. **Run it in full**, now, from the repository root.
3. **Read** the whole output: exit code, number of failures, warnings.
4. **Compare** with the expected result.
   - Not as expected → give the real state, with an excerpt of the output.
   - As expected → claim it, quoting the proof ("42 passed, 0 failed", "exit 0").

## What proves what

| Claim | Proof required | Not enough |
|---|---|---|
| "The tests pass" | test output: 0 failed | an earlier run, "it should" |
| "Lint and types are clean" | `pnpm lint`, `pnpm typecheck`, `cargo clippy -D warnings`: 0 error | one of them only |
| "The build works" | `pnpm tauri build`: exit 0 and the bundle exists | `cargo build` green |
| "Coverage is fine" | `scripts/check.sh --cov` output | `cargo test` green |
| "The bug is fixed" | the test that reproduced it passes, and it failed before | the code changed |
| "Auto-paste works" | the manual QA steps, run on a signed build, results noted | it works in `pnpm tauri dev` |
| "Matches the prototype" | `prototype-parity` done, gaps listed | "it looks like it" |
| "Ticket updated" | `node scripts/tickets.mjs show TAD-xxx` read after writing | `set` printed no error |

## Stop signals

- "should", "probably", "normally", "I think it works".
- Celebrating before seeing the output.
- About to commit, push or open a pull request without re-running the commands.
- Relying on a run from before the last change.

## What you write

In the pull request and the closing note: the commands actually run and their actual result. What could not be verified is written as such ("manual QA not run: no signed build on this machine"), never left out.
