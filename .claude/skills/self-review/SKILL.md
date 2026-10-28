---
name: self-review
description: Reviews the whole diff of a Tadidy ticket before opening its pull request — acceptance criteria covered by tests, TDD commit order, scope, security and privacy (no clip content in logs, no secret, webview untrusted), Rust and front conventions, prototype parity. Use after the verification commands and before ticket-update, or when the human asks "review it", "is it ready?".
---

# self-review — review yourself like a demanding reviewer

The human reviews next. Everything this checklist catches is a comment they do not have to write.

## 1. Gather the diff

```bash
git diff --stat main...HEAD
git log --oneline main..HEAD             # test(…) before feat(…)
git diff main...HEAD
```
And the ticket: `tickets/TAD-xxx.md` (*Tests*, *Files*, *Security*, *Risks*).

## 2. Checklist

For each line, note ✓ or the gap and its fix.

**Ticket**
- [ ] Every acceptance criterion has at least one test that would fail if the criterion were false.
- [ ] The tests announced in *Tests* exist (unit, integration, UI, manual steps).
- [ ] `test(TAD-xxx)` commits precede the matching `feat(TAD-xxx)`.
- [ ] The files of *Files* are created or modified; any extra or missing file is justified.
- [ ] Nothing from *Out of scope* was done; no "bonus".
- [ ] Every point of *Security* and *Risks* is handled (counter-measure in place, tested when possible).

**Privacy and security** (AGENTS.md §2)
- [ ] No clip content in logs, errors, panics, test names, snapshots or screenshots: `git diff main...HEAD | grep -nE 'println!|dbg!|eprintln!|console\.log|tracing::(info|debug|trace|warn|error)!'` and read every hit.
- [ ] No secret, key, certificate, `.env` or database file: `git diff main...HEAD | grep -iE "secret|password|token|api[_-]?key|BEGIN .*PRIVATE|PRAGMA key"` and read every hit.
- [ ] The database key never leaves Rust; nothing sensitive is sent to the webview beyond what the command needs.
- [ ] New IPC command: input validated in Rust, errors as `AppError { code, message }`, capability granted to the minimum.
- [ ] SQL only with parameters; no `{@html}` in Svelte on clip content; file URLs and links opened only through Rust after validation.
- [ ] No network call added.

**Quality**
- [ ] `scripts/check.sh --cov` green; coverage ≥ 80 % on changed files.
- [ ] No `unwrap()`/`expect()` outside tests and setup; every `unsafe` in `src/macos/` with a `// SAFETY:` comment.
- [ ] No `any`, `@ts-ignore`, `console.log`, `test.only`, `.skip`, `#[ignore]`, commented-out code or TODO without a ticket.
- [ ] `macos/` stays thin: logic is in services behind traits and tested with fakes; manual QA steps added.
- [ ] IPC type changed → bindings regenerated and committed, front updated in the same pull request.
- [ ] Schema changed → migration appended and tested (`db-migration`).
- [ ] Names, places and formats follow AGENTS.md §13–15 and [GLOSSARY.md](../../../GLOSSARY.md).

**UI**
- [ ] `prototype-parity` done, gaps fixed or recorded; DESIGN.md UI checklist filled.
- [ ] Empty, error and loading states; full keyboard path; ARIA roles and labels; light and dark; reduced motion.

## 3. Conclude

- Fix every gap (a bug = first a test that reproduces it), rerun the verification commands.
- Summarize for the human in 3 to 6 lines: what is covered, what was fixed, the remaining accepted gaps (they go in the pull request's *Deviations* and the closing note).
- Then `ticket-update`, *Opening*.
