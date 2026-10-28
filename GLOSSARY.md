# Glossary

Domain vocabulary. Use these words in code, UI, tickets and commits.

| Term | Definition |
|---|---|
| **Clip** | One item of the history, matching one copy. It can hold several *representations*. |
| **Representation** | One data format of a clip, identified by its UTI (`public.utf8-plain-text`, `public.rtf`, `public.html`, `public.png`, `public.file-url`…). |
| **Pinned** | A clip protected from the purge and from the history limit, shown at the top or in a dedicated tab. |
| **Snippet** | A pinned clip with a **title**, filed in a **collection**. It can be created by hand, without copying anything first. |
| **Collection** | A named group of snippets (e.g. "Emails", "SQL"). |
| **Popover** | The window that opens under the menu bar icon when you click it. |
| **Quick Panel** | A floating panel opened by a global shortcut, keyboard-driven, that does not steal focus from the active app. |
| **Source app** | The app in the foreground at the moment of the copy. |

Full specification: [docs/spec/](docs/spec/README.md).

## Words to avoid

| Say | Not | Note |
|---|---|---|
| clip | item, entry, element, copy (as a noun) | "copy" is only a verb or the "Copy only" action. |
| pinned, pin / unpin | favorite, starred, bookmarked | |
| snippet | template, macro, saved text | A snippet is always a pinned clip with a title, in a collection. |
| collection | folder, group, category, tag | |
| representation | format, flavor | The UTI identifies it (`uti` column). |
| Quick Panel | popup, window, palette, HUD | Opened by the global shortcut. |
| popover | popup, dropdown, menu | Opened by clicking the menu bar icon. The right-click menu is the "native menu". |
| source app | origin app, app of origin, from-app | |
