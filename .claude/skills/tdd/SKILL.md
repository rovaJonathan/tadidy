---
name: tdd
description: Applies Tadidy's red-green-refactor cycle to one acceptance criterion — a test written first and seen failing, test(TAD-xxx) commit, minimal code, feat(TAD-xxx) commit, refactor. Use for every acceptance criterion of a ticket, every bug fix, and whenever you are about to write production code.
---

# tdd — not one line of production code without a red test

## The cycle, for one acceptance criterion

1. **Pick the test** from the ticket's *Tests* section (unit, then integration, then UI). The "Given… when… then…" criterion becomes the test's structure (arrange / act / assert) and its name.

2. **Red** — write the test, nothing else, and run it:
   - Rust unit/integration: `cargo test --manifest-path src-tauri/Cargo.toml <test_name>`
   - Front: `pnpm test -- <file>`; end-to-end: `pnpm e2e <spec>`
   - Repo scripts: `node --test scripts/<file>.test.mjs`

   It **must fail for the right reason**: a wrong assertion or a missing function, not a typo or a broken import. In Rust, a test that does not compile because the function does not exist yet is a valid red; make it compile with a `todo!()` stub if you want to see the assertion fail. A test that already passes proves nothing: strengthen it.
   ```bash
   git add <test files> && git commit -m "test(TAD-xxx): <criterion, one line>"
   ```

3. **Green** — the minimum code to make that test pass. Run it, then the whole suite of the module.
   ```bash
   git add -A && git commit -m "feat(TAD-xxx): <criterion, one line>"
   ```

4. **Refactor** — names, extraction, duplicates; tests stay green. Commit `refactor(TAD-xxx): …` only if there is something.

5. Next criterion.

## Where and how to test

| What | Tool | Where |
|---|---|---|
| Rust logic (watcher, paster, purger, storage, settings, search parsing) | `#[test]`, fakes of the `macos/` traits | `#[cfg(test)] mod tests` next to the code |
| Storage against a real SQLCipher database | `tempfile` folder, fixed test key | `src-tauri/tests/*.rs` |
| IPC commands | call the command function with a test `AppState` | `src-tauri/tests/commands_*.rs` |
| Svelte component, `api.ts` wrapper, keyboard handling | Vitest + Testing Library, IPC mocked with `mockIPC` from `@tauri-apps/api/mocks` | `src/**/x.test.ts` next to the code |
| Screen flow (prototype behaviour) | Playwright against the Vite dev server, IPC mocked | `e2e/*.spec.ts` |
| macOS layer (`src-tauri/src/macos/`) | manual QA steps on a signed build | `docs/guides/manual-qa.md` |
| Repo scripts | `node:test` | `scripts/*.test.mjs` |

## Rules

- At least one test per acceptance criterion; a criterion without a test is not done.
- A bug starts with a test that reproduces it: `test(TAD-xxx): reproduce …`, then `fix(TAD-xxx): …`.
- No test that depends on order, real time (inject a clock), the real pasteboard, the network, the Keychain or the user's data folder (`TADIDY_DATA_DIR` → temporary folder).
- No real clipboard content, secret or personal data in fixtures.
- Coverage ≥ 80 % of lines on changed files; never lower a threshold to pass.
- Red `test(…)` commits are normal on the ticket branch (the `pre-push` hook runs the suite: push after the matching `feat(…)`); the pull request must be green.
