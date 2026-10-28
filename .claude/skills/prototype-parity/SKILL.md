---
name: prototype-parity
description: Compares a screen implemented in src/ with its reference view in docs/prototype/prototype.html — structure, ids, verbatim texts, states, keyboard behaviour, light and dark themes, DESIGN.md tokens — and lists the gaps to fix. Use for every UI ticket before opening its pull request, when a ticket cites the prototype, or when the human asks "does it match the prototype?".
---

# prototype-parity — the screen must match the prototype

The prototype is the source of truth for every text, id and state (AGENTS.md §1). "Improving" it on your own is out of scope: a gap you think is better is discussed (`ticket-problem`), not shipped.

## 1. Collect the reference

For each `prototype:` entry of the ticket (`"#popover — #search-input"`):
- Read the prototype's HTML for the view and the element: element tree, `id`s, classes, ARIA roles and labels, **texts verbatim** (including ellipsis "…", quotes, shortcut glyphs ⌘⇧⌥).
- Read its JS: states (empty, no results, permission missing, loading, error), keyboard shortcuts, toasts and their texts.
- Read the matching component in [DESIGN.md](../../../DESIGN.md): tokens, sizes, states.

Open it to see it: `open docs/prototype/prototype.html#popover` (the prototype controls switch theme, permission and empty states).

## 2. Look at the implementation

- Run the front with mocked IPC (`pnpm dev`, or the Playwright test) and the app (`pnpm tauri dev`) for the real window.
- Screenshot both, same state, same theme (Playwright `page.screenshot()` for the front; the prototype through headless Chrome:
  `"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --window-size=1440,900 --screenshot=/tmp/proto.png "file://$PWD/docs/prototype/prototype.html#popover"`).

## 3. Compare, line by line

| Check | How |
|---|---|
| Ids | every prototype id of the ticket exists in the component (`grep -rn 'id="search-input"' src/`) |
| Texts | identical, character for character (placeholders, empty states, buttons, toasts, dialog titles) |
| States | every state of the prototype is reachable and tested (empty, no results, banner, dialog) |
| Keyboard | every shortcut of the prototype for this view works and is covered by a test |
| Look | tokens from DESIGN.md (no hard-coded colors), spacing, radius, typography; light **and** dark |
| Accessibility | roles (`listbox`/`option`, `tablist`/`tab`, `dialog`, `menu`), labels, focus order, reduced motion |

## 4. Report

List the gaps as: element · expected (prototype) · actual · fix. Fix them (a behaviour gap = a test first), then fill the **UI ticket checklist** of DESIGN.md in the pull request. A gap kept on purpose is written in the pull request's *Deviations* with its reason and recorded in `PROGRESS.md`.
