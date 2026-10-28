# Manual QA

The macOS layer (`src-tauri/src/macos/`: pasteboard, frontmost app, synthetic ⌘V, Accessibility, Keychain, panels) cannot run in a unit test (AGENTS.md §9). Each behaviour it adds is checked by hand with the steps below, **on a signed build** (`pnpm tauri build`, [signing runbook](../runbooks/signing-identity.md)) unless a step says `pnpm tauri dev` is enough. The pull request of the ticket reports the result of its steps (✅ / ❌ + note).

A ticket that adds behaviour to the macOS layer adds its steps here, under its own heading, in the same pull request.

## How to report

In the pull request, section *Verification*:

```
Manual QA (docs/guides/manual-qa.md#tad-004) — signed build, macOS 15.1, Apple Silicon
| Step | Result |
|---|---|
| 1 | ✅ |
| 2 | ❌ the source app is Finder instead of TextEdit → TAD-0xx |
```

## Steps by ticket

<!-- Each ticket adds a "### TAD-xxx — <title>" heading with numbered steps: action → expected result. -->
