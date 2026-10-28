---
title: `popover.test.ts` silently overwrote `Popover.test.ts` on macOS
date: 2026-10-28
area: repo
tags: [macos, filesystem, tests, git]
---

## Symptom

A new test file written as `src/windows/popover.test.ts` replaced the committed `src/windows/Popover.test.ts`: `git status` showed the old file as *modified*, no new file appeared, and `pnpm test` stayed green with fewer tests than expected (the component's tests were gone).

## Cause

The default macOS volume is case-insensitive: two names that differ only by case are the same file. The project naming rule (Svelte component `PascalCase.svelte`, its test `PascalCase.test.ts`, an entry module `kebab-case.ts`) makes `Popover.test.ts` and `popover.test.ts` collide.

## Fix

- Never create two files whose names differ only by case. Name the entry module's test after what it tests: `popover-entry.test.ts`.
- After writing a test file, check `git status`: a *modified* file you did not mean to touch means a collision. Compare the number of tests with what you expect, not only the green result.
