---
title: In WebKit an absolutely positioned checkbox ignores `inset` and keeps its 12 px size
date: 2026-11-05
area: front
tags: [webkit, wkwebview, css, checkbox, switch, playwright]
---

## Symptom

- In the app (WKWebView), clicking the middle or right of a switch (`.toggle`: an invisible `<input type="checkbox">` over a `.toggle-track`) does nothing. Only its top-left corner, or the `<label for>`, reacts.
- No IPC call, no log: it looks like the core ignored the change. TAD-025 started as "Launch at login cannot be switched on" and was first suspected to be `SMAppService`.
- The Chromium e2e tests pass: Chromium stretches the checkbox to the whole track, and the tests clicked the label or called `box.click()`.

## Cause

WebKit lays a checkbox out like a replaced element with an intrinsic size (12 × 12 px). `position: absolute; inset: 0` does not stretch it; Chromium does.

## Fix

- Give the input an explicit size: `width: 100%; height: 100%` next to `inset: 0` (`src/styles/preferences.css`, DESIGN.md §4.13 *Hit area*).
- Test it with a real mouse click away from the top-left corner, in WebKit: the `webkit` project of `playwright.config.ts` runs `e2e/preferences.spec.ts` (`pnpm exec playwright install webkit` once).
- Anything that depends on layout in the webview (hit areas, sizes) needs a WebKit run, not only Chromium.
