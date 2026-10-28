<a id="03-interface"></a>

# Interface

<a id="layout"></a>

## Layout: popover / Quick Panel (same component)

```
┌──────────────────────────────────────────────┐
│ 🔍 Search clips…                         ⌘⇧V │
├──────────────────────────────────────────────┤
│  All   Pinned   Snippets      [Txt][Img][🔗]  │
├──────────────────────────────────────────────┤
│ ⌘1 📌 SELECT * FROM users WHERE id = …   VS  │
│ ⌘2 ▸ https://github.com/tauri-apps/…   🧭 2m │
│ ⌘3 ▣ [thumbnail]  Screenshot 1440×900  📷 5m │
│ ⌘4 ¶ Hi John, following up on our…     ✉ 12m │
│ ⌘5 📄 final-report.pdf (+2 files)      📁 1h │
│ …                                            │
├──────────────────────────────────────────────┤
│ 487 clips · ⚠ Auto-paste disabled [Enable] ⚙ │
└──────────────────────────────────────────────┘
```

- Selected row highlighted (system accent color).
- Hovering a row: quick actions on the right (📌 pin, 🗑 delete, ⋯ more).
- Context menu (right click or `⌘K`): Paste · Paste as plain text · Copy · Pin/Unpin · Save as snippet… · Transform ▸ · Open URL / Reveal in Finder · Delete.
- Empty state: "Nothing copied yet. Press ⌘C anywhere — Tadidy remembers."
- Search with no results: `No clips match "xyz"`.

<a id="visual-design"></a>

## Visual design

- macOS translucent material (`popover` / `hudWindow` vibrancy via `window-vibrancy`), 10 px rounded corners.
- System font (`-apple-system`, SF Pro), monospace (SF Mono) for clips detected as code.
- Light/dark theme that follows the system, with an optional override.
- Animations ≤ 120 ms, disabled when "Reduce motion" is on.
- Accessibility: ARIA labels, full keyboard navigation, AA contrast.

<a id="keyboard-shortcuts"></a>

## Keyboard shortcuts (in the panel)

| Key | Action |
|---|---|
| `↑` / `↓` | Previous / next clip |
| `Return` / double-click | Paste |
| `⇧Return` | Paste as plain text |
| `⌥Return` / `⌘C` | Copy only |
| `⌘1` … `⌘9` | Paste the n-th clip |
| `Space` | Preview (when the search is empty or not focused) |
| `⌘P` | Pin / unpin |
| `⌘S` | Save as snippet |
| `⌘⌫` | Delete |
| `⌘O` | Open URL / Reveal in Finder |
| `⌘K` | Actions menu |
| `Tab` / `⇧Tab` | Next / previous tab |
| `⌘F` or any letter | Focus search |
| `Esc` | Clear the search, then close |
| `⌘,` | Preferences |

Global shortcut: `⌘⇧V` (configurable). If registration fails because of a conflict, show an error in the preferences.
