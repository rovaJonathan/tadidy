---
title: Tauri's mock runtime cannot show or hide a window, and a native menu cannot be built in a unit test
date: 2026-10-28
area: test
tags: [tauri, mock-runtime, muda, tray, macos]
---

## Symptom

- A test that calls `window.hide()` then asserts `!window.is_visible()` fails with the correct code; with the code removed it fails too.
- A test that builds a `Menu` or a `TrayIconBuilder` panics in `muda-…/src/platform_impl/macos/mod.rs`: "`muda::Menu` can only be created on the main thread".

## Cause

- `tauri::test::mock_app()` windows are inert: `show` and `hide` do nothing and `is_visible` always returns `Ok(true)` (`tauri/src/test/mock_runtime.rs`).
- On macOS muda creates `NSMenu` / `NSStatusItem` objects with a `MainThreadMarker`; `cargo test` runs every test on a worker thread, never the main one.

## Fix

- Put the show/hide **decision** behind a small trait (`windows::PopoverWindow`) and test it with a fake; the real window implements the trait, and showing it for real is a manual QA step.
- Keep the menu **model** and the click rules pure (`tray::menu_items`, `tray::toggles_the_popover`) and put the native construction in `src-tauri/src/macos/` (excluded from the coverage gate, AGENTS.md §9), with a manual QA step.
- Do not try to run the test harness on the main thread to get around it: `harness = false` integration tests make every test depend on the AppKit run loop.
