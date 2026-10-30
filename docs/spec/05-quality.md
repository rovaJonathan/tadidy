<a id="05-quality"></a>

# Quality

<a id="non-functional"></a>

## Non-functional requirements

| Category | Requirement |
|---|---|
| **Performance** | Popover / Quick Panel opens in < 100 ms (pre-created windows). Search < 50 ms over 10,000 clips. Virtualized list (only about 30 DOM nodes rendered). |
| **Resources** | Idle CPU < 0.5% (adaptive polling). RAM < 120 MB (WebKit included). |
| **Reliability** | No clip is lost if the app crashes: write to the database before emitting the event, WAL journal. Versioned and tested schema migrations. |
| **Privacy** | No network calls except the update check (P2, can be turned off). No telemetry. Clip content never appears in the logs. |
| **Compatibility** | macOS 13+, universal arm64 + x86_64 binary. Works with multiple displays, Spaces and full-screen apps. |
| **Accessibility** | VoiceOver on the list (`listbox`/`option` roles), full keyboard navigation, AA contrast, respects "Reduce motion" / "Reduce transparency". |
| **Quality** | `cargo clippy -D warnings`, `cargo fmt`, `svelte-check`, ESLint. Rust unit tests on storage, dedup, purge, transforms and search parsing. Front-end component tests (Vitest). |

<a id="mvp-acceptance"></a>

## MVP acceptance criteria

1. Copy text in any app: it appears at the top of the popover in less than 500 ms.
2. Copy the same text twice: a single clip, moved to the top, `copy_count = 2`.
3. `⌘⇧V` in TextEdit, `↓`, `Return`: the 2nd clip is pasted into TextEdit, which keeps focus.
4. Accessibility permission revoked: `Return` only copies and the banner is shown.
5. With the limit set to 10, an 11th copy deletes the oldest unpinned clip.
6. Typing "sel" filters in less than 50 ms and shows only the clips containing "sel".
7. After restarting the Mac, Tadidy is running (if the option is checked) and the history is intact.
8. Opening `tadidy.db` with `sqlite3` without the key fails ("file is not a database").
9. No icon in the Dock or in `⌘Tab`.
10. Idle CPU < 0.5%, measured over 10 min in Activity Monitor.
