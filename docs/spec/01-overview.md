<a id="01-overview"></a>

# Overview

<a id="problem"></a>

## Problem

macOS keeps only **one** item on the clipboard. Copying overwrites the previous one, and the text, link or image you copied before is lost.

<a id="goals"></a>

## Goals

1. **Lose nothing**: everything you copy (text, rich text, images, files, URLs) is recorded in the history automatically.
2. **Find it fast**: open the history with one shortcut, type a few letters, press Return. Less than 2 seconds from need to paste.
3. **Stay out of the way**: an icon in the menu bar, no icon in the Dock, near-zero CPU use when idle.
4. **Stay private**: encrypted database, no data leaves the machine, old clips expire automatically.

<a id="out-of-scope"></a>

## Out of scope (v1)

- Multi-Mac / iCloud sync
- Mac App Store version (the sandbox prevents auto-paste)
- Localization other than English
- Windows / Linux (Tauri would allow it, but clipboard access and pasting are macOS-specific)
