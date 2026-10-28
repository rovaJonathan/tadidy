# Tadidy — Design system

> Visual system for the Tadidy front end (Svelte 5 + TypeScript in a Tauri 2 WebKit webview).
> Reference prototype: [`docs/prototype/prototype.html`](docs/prototype/prototype.html). Open it in a browser.

## 1. Purpose

Two artifacts define the UI. Neither one replaces the other.

| Artifact | Is the source of truth for |
|---|---|
| `docs/prototype/prototype.html` | **What** is on screen: every view, every UI string (verbatim), every element `id`, every state (empty, error, loading, disabled, permission missing), keyboard behaviour. Tickets cite it as `docs/prototype/prototype.html#popover — #search-input`. |
| `DESIGN.md` (this file) | **How** it looks and behaves: design tokens, window types, component anatomy and states, keyboard and focus rules, accessibility, icons, copy rules. |

If the two disagree, the prototype wins for texts, ids and states, and this file wins for token values. Fix the one that is wrong in the same PR.

**Prototype deep links.** Views: `#popover`, `#quick-panel`, `#clip-preview`, `#tray-menu`, `#preferences`, `#prefs-general`, `#prefs-capture`, `#prefs-shortcuts`, `#prefs-appearance`, `#prefs-storage`, `#prefs-about`, `#onboarding`, `#onboarding-welcome`, `#onboarding-accessibility`, `#onboarding-setup`, `#closed`. States: `#banner-accessibility`, `#empty-history`, `#empty-search`, `#dialog-delete-clip`, `#dialog-clear-history`, `#snippet-editor`, `#clip-context-menu`. Query parameters: `?theme=light|dark`, `?ax=0` (Accessibility not granted), `?empty=1` (empty history). Example: `prototype.html?theme=dark&ax=0#popover`.

Everything prefixed `proto-` in the prototype (toolbar, fake desktop, menu bar, background TextEdit window, `--proto-*` variables) is scaffolding. Do **not** port it.

## 2. Design tokens

Tokens are CSS custom properties. They live in `src/styles/tokens.css`, which is a copy of section 1 of the prototype's `<style>` block. Components use tokens only. **No hard-coded colours, radii, shadows or durations in components.**

The theme is set by `data-theme="light" | "dark"` on `<html>`. It follows the system by default (`prefers-color-scheme`), and Preferences › Appearance › Theme can override it.

### 2.1 Colour

| Token | Light | Dark | Use |
|---|---|---|---|
| `--color-accent` | `#007AFF` | `#0A84FF` | Primary buttons, toggles on, active menu item, checkbox on |
| `--color-accent-text` | `#0062CC` | `#4DA3FF` | Accent-coloured text (links, pressed chips, "New snippet"). AA on window surfaces |
| `--color-accent-tint` | `rgba(0,122,255,.14)` | `rgba(10,132,255,.26)` | Background of a pressed filter chip |
| `--color-on-accent` | `#FFFFFF` | `#FFFFFF` | Text and icons on accent |
| `--color-selection` | `#0063E1` | `#0058D0` | Selected clip row, selected list item (macOS *selectedContentBackgroundColor*) |
| `--color-selection-text` | `#FFFFFF` | `#FFFFFF` | Primary text on selection |
| `--color-selection-text-secondary` | `rgba(255,255,255,.78)` | same | Badge, meta, second line on selection |
| `--color-focus-ring` | `rgba(0,122,255,.45)` | `rgba(10,132,255,.55)` | 3 px keyboard focus ring |
| `--color-text-primary` | `rgba(0,0,0,.85)` | `rgba(255,255,255,.88)` | Body text |
| `--color-text-secondary` | `rgba(0,0,0,.62)` | `rgba(255,255,255,.62)` | Descriptions, meta, inactive tabs |
| `--color-text-tertiary` | `rgba(0,0,0,.45)` | `rgba(255,255,255,.42)` | Placeholders, ⌘1–⌘9 badges, menu shortcuts. Decorative or duplicated info only (below AA for body text) |
| `--color-text-disabled` | `rgba(0,0,0,.25)` | `rgba(255,255,255,.25)` | Disabled labels and menu items |
| `--color-separator` | `rgba(0,0,0,.10)` | `rgba(255,255,255,.10)` | Hairlines (0.5 px) |
| `--color-border` | `rgba(0,0,0,.14)` | `rgba(255,255,255,.14)` | Image outlines, keycap base |
| `--color-fill` | `rgba(0,0,0,.05)` | `rgba(255,255,255,.07)` | Kind tile, segmented track, kbd |
| `--color-fill-hover` | `rgba(0,0,0,.06)` | `rgba(255,255,255,.08)` | Row / button hover |
| `--color-fill-pressed` | `rgba(0,0,0,.10)` | `rgba(255,255,255,.14)` | Pressed state, empty bar track |
| `--color-control-bg` | `#FFFFFF` | `rgba(255,255,255,.16)` | Push buttons, popup buttons, fields, steppers |
| `--color-segment-selected` | `#FFFFFF` | `rgba(255,255,255,.22)` | Selected segment (tabs, theme picker) |
| `--color-toggle-off` | `rgba(0,0,0,.10)` | `rgba(255,255,255,.16)` | Switch track when off |
| `--color-destructive` | `#FF3B30` | `#FF453A` | Destructive filled button |
| `--color-destructive-text` | `#D70015` | `#FF6961` | Destructive text button, field errors |
| `--color-warning` | `#FF9500` | `#FF9F0A` | Warning glyph (permission banner, export warning) |
| `--color-warning-bg` | `rgba(255,149,0,.14)` | `rgba(255,159,10,.18)` | Permission banner background |
| `--color-success` | `#34C759` | `#30D158` | Success fill |
| `--color-success-text` | `#248A3D` | `#30D158` | "Accessibility enabled ✓", encryption "On" |
| `--color-pin` | `#FF9500` | `#FF9F0A` | Pin marker in rows |
| `--color-match-bg` | `rgba(255,204,0,.50)` | `rgba(255,214,10,.38)` | Search match highlight (`<mark>`) |
| `--color-match-bg-selected` | `rgba(255,255,255,.30)` | same | Highlight inside a selected row |
| `--color-toast-bg` | `rgba(30,30,30,.86)` | `rgba(60,60,62,.92)` | Toast (HUD) |
| `--color-toast-text` | `#FFFFFF` | `#FFFFFF` | Toast text |
| `--color-code-keyword` | `#AD3DA4` | `#FF7AB2` | Preview syntax colours (Xcode default theme) |
| `--color-code-string` | `#C41A16` | `#FF8170` | |
| `--color-code-number` | `#1C00CF` | `#D9C97C` | |
| `--color-code-comment` | `#5D6C79` | `#7F8C98` | Also used for SQL comments and shell flags |

### 2.2 Materials, surfaces, blur

In the app, translucency comes from **native vibrancy** (`window-vibrancy`, see §3). The webview background is then `transparent`, and the `--material-*` values are only the **fallback**: they are used in the browser prototype and when *Reduce transparency* is on.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--material-popover` | `rgba(246,246,246,.80)` | `rgba(38,38,40,.76)` | Popover window |
| `--material-hud` | `rgba(248,248,248,.84)` | `rgba(32,32,34,.80)` | Quick Panel |
| `--material-menu` | `rgba(242,242,242,.84)` | `rgba(44,44,46,.82)` | In-webview menus (context menu, apps menu) |
| `--material-sidebar` | `rgba(232,232,234,.82)` | `rgba(40,40,42,.80)` | Preferences sidebar |
| `--material-alert` | `rgba(240,240,240,.94)` | `rgba(50,50,52,.94)` | Alerts |
| `--surface-window` | `#F5F5F5` | `#1E1E1E` | Preferences content, onboarding, sheets |
| `--surface-group` | `#FFFFFF` | `rgba(255,255,255,.05)` | Grouped form boxes |
| `--surface-group-border` | `rgba(0,0,0,.08)` | `rgba(255,255,255,.07)` | Border of grouped boxes |
| `--surface-overlay` | `rgba(250,250,250,.97)` | `rgba(36,36,38,.97)` | Preview overlay in the popover |
| `--surface-preview` | `rgba(0,0,0,.025)` | `rgba(255,255,255,.03)` | Preview column in the Quick Panel |
| `--blur-material` | `blur(30px) saturate(180%)` | same | `backdrop-filter` fallback for windows |
| `--blur-menu` | `blur(20px) saturate(180%)` | same | `backdrop-filter` for menus and toasts |

With `prefers-reduced-transparency: reduce`, materials become opaque (`#F2F2F2` / `#262628` for the popover, etc. See the prototype) and the blurs become `none`. Native vibrancy must also be switched off (see §7).

### 2.3 Typography

Font stacks:

- `--font-sans`: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", Helvetica, Arial, sans-serif`
- `--font-mono`: `ui-monospace, "SF Mono", SFMono-Regular, Menlo, Monaco, monospace`

Never bundle or load web fonts.

| Token (size / leading) | Value | Use |
|---|---|---|
| `--text-caption` / `--leading-caption` | 10 / 13 px | Stack count on file tiles |
| `--text-small` / `--leading-small` | 11 / 14 px | Meta, badges, descriptions, footer, alert body |
| `--text-callout` / `--leading-callout` | 12 / 15 px | Tabs, chips, banner, second line of a row, toast |
| `--text-body` / `--leading-body` | 13 / 16 px | Default UI text (macOS body) |
| `--text-code` / `--leading-code` | 12 / 16 px | Code clips (mono) |
| `--text-headline` / `--leading-headline` | 15 / 20 px | Prefs header, empty-state title, sheet title |
| `--text-search` | 15 px | Popover search field |
| `--text-search-lg` | 18 px | Quick Panel search field |
| `--text-title` / `--leading-title` | 17 / 22 px | Reserved |
| `--text-title-lg` / `--leading-title-lg` | 22 / 28 px | Onboarding titles, About app name |

Weights: `--weight-regular` 400, `--weight-medium` 500, `--weight-semibold` 600, `--weight-bold` 700. Use tabular numbers (`font-variant-numeric: tabular-nums`) for counts, times, badges and sizes.

### 2.4 Spacing, radii, sizes

Spacing: `--space-2` 2px · `--space-4` 4 · `--space-6` 6 · `--space-8` 8 · `--space-10` 10 · `--space-12` 12 · `--space-16` 16 · `--space-20` 20 · `--space-24` 24 · `--space-32` 32. Do not use any other values.

| Radius token | Value | Use |
|---|---|---|
| `--radius-xs` | 3 px | Small inner marks |
| `--radius-sm` | 5 px | Segments, row action buttons, fields, stepper |
| `--radius-md` | 6 px | Clip rows, tiles, chips, push and popup buttons |
| `--radius-lg` | 8 px | Menus, grouped boxes, banner |
| `--radius-window` | 10 px | Popover, Quick Panel, Preferences, onboarding |
| `--radius-alert` | 12 px | Alerts, snippet editor sheet |
| `--radius-pill` | 999 px | Toast |

| Size token | Value |
|---|---|
| `--popover-width` × `--popover-height` | 380 × 520 px |
| `--panel-width` × `--panel-height` | 640 × 460 px |
| `--preview-width` | 272 px (Quick Panel preview column) |
| `--prefs-width` × `--prefs-height` | 640 × 480 px |
| `--prefs-sidebar-width` | 184 px |
| `--onboarding-width` × `--onboarding-height` | 520 × 440 px |
| `--tile-size` | 28 px (20 px in compact mode) |
| `--preview-lines` | 2 (Preferences › Preview lines, 1–4; compact mode forces 1) |

### 2.5 Shadows

| Token | Light | Dark |
|---|---|---|
| `--shadow-window` | `0 0 0 .5px rgba(0,0,0,.18), 0 12px 40px rgba(0,0,0,.22), 0 2px 8px rgba(0,0,0,.08)` | `0 0 0 .5px rgba(0,0,0,.85), inset 0 0 0 .5px rgba(255,255,255,.14), 0 12px 40px rgba(0,0,0,.50)` |
| `--shadow-menu` | `0 0 0 .5px rgba(0,0,0,.15), 0 8px 24px rgba(0,0,0,.18)` | `0 0 0 .5px rgba(0,0,0,.85), inset 0 0 0 .5px rgba(255,255,255,.14), 0 8px 24px rgba(0,0,0,.45)` |
| `--shadow-alert` | `0 0 0 .5px rgba(0,0,0,.15), 0 20px 50px rgba(0,0,0,.28)` | `0 0 0 .5px rgba(0,0,0,.85), inset 0 0 0 .5px rgba(255,255,255,.14), 0 20px 50px rgba(0,0,0,.55)` |
| `--shadow-control` | `0 0 0 .5px rgba(0,0,0,.14), 0 .5px 1.5px rgba(0,0,0,.14)` | `0 0 0 .5px rgba(0,0,0,.35), inset 0 .5px 0 rgba(255,255,255,.10)` |
| `--shadow-segment` | `0 0 0 .5px rgba(0,0,0,.06), 0 1px 2px rgba(0,0,0,.14)` | `0 1px 2px rgba(0,0,0,.30)` |
| `--shadow-toast` | `0 4px 14px rgba(0,0,0,.25)` | `0 4px 14px rgba(0,0,0,.45)` |

In the app, macOS draws the shadow of native windows. `--shadow-window` is used only in the prototype and for in-webview surfaces.

### 2.6 Motion

| Token | Value | Use |
|---|---|---|
| `--duration-fast` | 80 ms | Hover and pressed backgrounds, chip toggles |
| `--duration-base` | 120 ms | Toggle knob, toast in/out |
| `--ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | All transitions |

Motion rules:

- No animation is longer than 120 ms (spec §4.2).
- Windows, menus and dialogs appear **instantly**. Do not fade or scale them.
- List selection does not animate.
- With `prefers-reduced-motion: reduce`, both durations become `0ms`. The only exception is the onboarding spinner, which slows down to 2.4 s per turn but keeps turning, because it carries state.

## 3. Window types

| Window | Tauri label | Size | Native material (`window-vibrancy`) | Notes |
|---|---|---|---|---|
| Popover | `popover` | 380 × 520 | `NSVisualEffectMaterial::Popover`, state `Active`, radius 10 | Borderless, transparent, under the tray icon (`TrayBottomCenter`), 5 px below the menu bar, kept ≥ 8 px from the screen edge. Hides on blur and on `Esc`. Prototype: `#popover`. |
| Quick Panel | `panel` | 640 × 460 | `NSVisualEffectMaterial::HudWindow`, state `Active`, radius 10 | Non-activating `NSPanel`. Search field 18 px, preview column on the right (272 px), visible by default and toggled with Space. Position follows Preferences › Quick Panel position (default: under the mouse pointer. The prototype centres it). Prototype: `#quick-panel`, `#clip-preview`. |
| Preferences | `settings` | 640 × 480 | Sidebar only: `NSVisualEffectMaterial::Sidebar`. Content: opaque `--surface-window` | Standard titled window with a transparent title bar (traffic lights over the sidebar), and no zoom. 184 px sidebar plus content, 52 px content header. Prototype: `#preferences`. |
| Onboarding | `onboarding` | 520 × 440 | None (opaque `--surface-window`) | Standard window. Only the close button is enabled, and it is not resizable. Prototype: `#onboarding`. |
| Tray menu | — | native | Native `NSMenu` (not a webview) | Built in Rust (`tray.rs`). The prototype `#tray-menu` defines texts, order, separators and shortcuts only. |
| Alerts | — | 260 wide | `--material-alert` | Rendered in the current webview, centred over the window that triggered them (popover, panel or settings). Clear history from the tray uses the settings window or a native `NSAlert` with the same texts. |

## 4. Components

Each component lists its anatomy, its states and the prototype id(s). The Svelte component names match spec §6.9.

### 4.1 ClipBrowser (container shared by the popover and the Quick Panel)

- **Anatomy.** A grid with these areas: `search` / `toolbar` / `list` (+ `preview` in the Quick Panel) / `banner` / `footer`. There is one component, with two layouts: `data-layout="popover" | "panel"`.
- **Popover layout.** Filter chips are icon-only (labels visually hidden). The preview is an overlay over the list. The footer shows no hints.
- **Panel layout.** Chips show their labels. The preview is a 272 px column. The footer shows the hints "↩ Paste · ⇧↩ Plain text · ⌥↩ Copy · ⌘K Actions".
- **Prototype:** `#clip-browser` (re-parented into `#popover` or `#quick-panel`).

### 4.2 SearchBar

- **Anatomy.** Magnifier glyph (secondary) · borderless input · clear button (shown only when there is text) · `kbd` hint with the global shortcut (hidden while typing).
- **Placeholder:** "Search clips…".
- **Input height:** 44 px (popover) or 52 px (panel).
- **States.**
  - Empty: placeholder and shortcut hint.
  - Typing: clear button visible. The list filters live (debounce 50 ms) and matches are highlighted.
  - Focused: no focus ring, because the field is the window's primary control.
- The input is a `combobox` that controls `#clip-list` through `aria-activedescendant`.
- **Prototype:** `#search-input`, `#btn-clear-search`, `#search-shortcut-hint`.

### 4.3 Tabs

- **Anatomy.** A segmented control (`role="tablist"`) with All · Pinned · Snippets. The track uses `--color-fill`. The selected segment uses `--color-segment-selected` and `--shadow-segment`.
- **States.** Default (secondary text) · hover (primary text) · selected · keyboard (Tab / ⇧Tab cycles through the tabs). The tabs are `tabindex="-1"`, so focus stays in the search field.
- **Tab content.**
  - **All** = history, with pinned clips on top when "Show pinned on top" is on. Snippets appear in All only as search results.
  - **Pinned** = all pinned clips, including snippets.
  - **Snippets** = snippets grouped by collection, with a "New snippet" row on top.
- **Prototype:** `#tab-all`, `#tab-pinned`, `#tab-snippets`, `#btn-new-snippet`.

### 4.4 KindFilterChips

- **Anatomy.** 22 px chips, each with a 14 px kind glyph and a label: Text · Rich · Images · Files · Links. In the popover the chips are 24 px wide and icon-only; the label stays available to VoiceOver and as a tooltip.
- **Behaviour.** Multi-select toggles (`aria-pressed`). When none are pressed, every kind is shown. Colour clips count as Text.
- **States.** Default · hover (`--color-fill-hover`) · pressed (`--color-accent-tint` background, `--color-accent-text`) · focus-visible ring.
- **Prototype:** `#filter-text`, `#filter-rich`, `#filter-image`, `#filter-file`, `#filter-link`.

### 4.5 ClipRow

- **Anatomy.** A grid `20px | tile | 1fr | auto`, made of these parts:
  1. `⌘1`–`⌘9` badge (first nine visible rows only, hidden when Quick paste is off).
  2. Tile (28 px, `--radius-md`). It shows one of: a kind glyph (text, code, rich, link, file, with a stack count badge when there are several files), the image thumbnail (cover-cropped), or a colour swatch.
  3. Content. Text and code: up to `--preview-lines` lines; code uses mono at 12 px. Image: name, then "W × H · FORMAT · SIZE". File: "name (+N files)", then the folder. Link: host in primary text, path in secondary. Colour: hex in mono, then `rgb()`. Snippet: bold title, then a one-line content preview.
  4. Meta: pin marker (`--color-pin`), source app icon (14 px), relative time ("Just now", "2m ago", "3h ago", "Yesterday", "4d ago"). Snippets show their collection name instead of the time outside the Snippets tab.
  5. Hover actions that replace the meta: pin / unpin, delete, more (opens the context menu).
- **States.**
  - Default.
  - Hover: `--color-fill-hover` and the hover actions.
  - Selected: `--color-selection` background, white text; secondary parts use `--color-selection-text-secondary` and highlights use `--color-match-bg-selected`.
  - Selected + hover: the action buttons turn white.
  - Compact: one line and a 20 px tile.
  - "Show app icons" off: the app icon is hidden.
  - Disabled: not applicable.
- **Rules.**
  - `role="option"`, `id="clip-<id>"`, `data-id`, `data-kind` (`text|rich|image|file|url|color`).
  - The action buttons are `tabindex="-1"`. Keyboard users reach the same actions through ⌘K.
  - Double-click pastes. Right-click opens the context menu.
- **Prototype:** `.clip-row` inside `#clip-list`.

### 4.6 PermissionBanner

- **Anatomy.** A warning glyph (`--color-warning`), the text "Auto-paste disabled — clips are copied only.", and a small "Enable" button. Background `--color-warning-bg`, radius 8, inset 8 px, placed above the footer.
- **States.** Hidden when Accessibility is granted. Visible otherwise, and it updates live on `accessibility://changed`.
- "Enable" calls `open_accessibility_settings()`.
- **Prototype:** `#banner-accessibility`, `#btn-enable-accessibility`.

### 4.7 EmptyState

- **Anatomy.** A centred 44 px circle with a glyph, a title (15 px semibold) and a body (secondary). There is an optional action button.
- **Variants.**

| Id | Title | Body |
|---|---|---|
| `#empty-history` | "Nothing copied yet." | "Press ⌘C anywhere — Tadidy remembers." |
| `#empty-search` | No clips match “`<query>`” (query in `#empty-search-query`). Variant when only filters are active: "No clips match these filters" | "Check the spelling, or try another tab or filter." |
| `#empty-pinned` | "No pinned clips" | Text as in the prototype |
| `#empty-snippets` | "No snippets yet" | Text as in the prototype, plus a "New snippet" button |

- **Loading.** The list must render in < 100 ms, so there is no spinner. If a page is still loading, keep the previous rows. Never flash an empty state before the first page arrives.

### 4.8 ContextMenu

- **Anatomy.** A menu at least 220 px wide. Padding 5 px, items 22 px high with a 4 px radius, shortcuts in tertiary text on the right, separators inset 10 px. Background `--material-menu`, radius 8, `--shadow-menu`.
- **Items:**
  1. Paste ↩
  2. Paste as plain text ⇧↩
  3. Copy ⌘C
  4. *(separator)*
  5. Pin / Unpin ⌘P
  6. Save as snippet… ⌘S
  7. Transform ▸. The submenu has: UPPERCASE · lowercase · Title Case · Trim whitespace · *(separator)* · Format JSON · Minify JSON · *(separator)* · URL encode · URL decode · Base64 encode · Base64 decode.
  8. *(separator)*
  9. Open URL / Reveal in Finder ⌘O. Shown only for links and files.
  10. *(separator)*
  11. Delete ⌘⌫
- **States.**
  - Item: default · active (hover or arrow keys: `--color-accent` background, white text) · disabled (`--color-text-disabled`).
  - Disabled items: Transform and Paste as plain text for images and files; the JSON transforms when the clip is not JSON.
- **Opening.** ⌘K opens the menu below the selected row with the first item active. Right-click opens it at the pointer with no item active.
- **Prototype:** `#clip-context-menu`, items `#ctx-*`, submenu `#ctx-transform-menu`.

### 4.9 Dialog (alert)

- **Anatomy.** 260 px wide, centred. App icon 56 px · bold 13 px title · 11 px message · optional checkbox · buttons stacked full width (28 px high). Radius 12, `--material-alert`.
- **Variants.**
  - `#dialog-delete-clip`: "Delete this clip?" / "This can’t be undone." / checkbox `#chk-dont-ask-delete` "Don’t ask again" / buttons **Delete** (`.btn-destructive`, default, Return) and **Cancel** (Esc). Checking "Don’t ask again" turns off Preferences › Confirm before delete.
  - `#dialog-clear-history`: "Clear clipboard history?" / "`N` clips will be permanently deleted from this Mac. This can’t be undone." / checkbox `#chk-keep-pinned` "Keep pinned clips and snippets" (checked) / buttons **Clear history** (destructive text, not default) and **Cancel** (primary, default, Return).
- **Behaviour.** `role="alertdialog"` with `aria-modal`. Focus is trapped inside and lands on the default button. Esc cancels.

### 4.10 Toast

- **Anatomy.** A HUD pill centred 44 px above the footer: check glyph, then 12 px medium text. It slides in over 6 px and fades in 120 ms, then auto-hides after 1.6 s.
- **Texts (verbatim):** "Pasted", "Pasted as plain text", "Copied", "Copied — auto-paste disabled", "Copied as plain text — auto-paste disabled", "Pinned", "Unpinned", "Snippets are always pinned", "Clip deleted", "Snippet saved", "Opened in browser", "Revealed in Finder", "Transformed and pasted", "Transformed and copied — auto-paste disabled", "Opening System Settings…".
- In the real app, a successful paste closes the window, so you only see the toast for copy-only actions and for actions that keep the window open (pin, delete, save).
- `role="status"`, `aria-live="polite"`.
- **Prototype:** `#toast`, `#toast-text`.

### 4.11 Preview

- **Anatomy.**
  - Header: kind summary (for example "Code · 57 characters", "Image · 1440 × 900", "3 files"), a `Space` kbd and a close button.
  - Body:
    - Text: pre-wrapped.
    - Code: mono with basic syntax colours.
    - Rich text: rendered HTML.
    - Image: large, contained.
    - Files: list with the folder and "Reveal in Finder".
    - Link: host, full URL and "Open URL".
    - Colour: big swatch, then Hex / RGB / HSL.
  - Metadata list: Source (app icon and name, or "Created in Tadidy") · Collection (snippets) · Copied · Times copied · Size.
- **States.**
  - Popover: closed by default. Space opens it as an overlay and Esc / Space closes it.
  - Quick Panel: open by default, and Space toggles it.
  - "No clip selected".
- **Prototype:** `#clip-preview`, `#clip-preview-title`, `#clip-preview-body`, `#clip-preview-meta`, `#btn-close-preview`.

### 4.12 PreferenceRow

- **Anatomy.** Inside a `.pref-group` (`--surface-group`, 0.5 px border, radius 8), each row is ≥ 38 px high with 12 px horizontal padding. The label (13 px) and an optional description (11 px secondary) sit on the left, the control on the right. Rows are separated by hairlines. Block rows put full-width content (checkbox grid, list, usage bar) under the label.
- **States.** Default · disabled (`.is-disabled`: disabled label and control, used for P2 rows such as "Snippet shortcuts").
- Footnotes (`.pref-footnote`) sit under a group, for example the export warning.
- **Prototype:** `#prefs-general` … `#prefs-about`. Every control has a `pref-*` id.

### 4.13 Toggle

- **Anatomy.** A 32 × 18 track with a 16 px white knob. Off: `--color-toggle-off`. On: `--color-accent`. The knob moves 14 px in 120 ms.
- **Markup.** `<input type="checkbox" role="switch">` with a visible `<label for>`.
- **States.** Off · on · focus-visible (3 px ring) · disabled (45 % opacity).

### 4.14 Other controls

- **Push button** (`.btn`): 22 px, radius 6, `--color-control-bg` + `--shadow-control`.
  - Variants: `.btn-primary` (accent fill), `.btn-destructive` (red fill), `.btn-destructive-text` (red label), `.btn-small` (20 px), `.btn-large` (28 px).
  - Pressed: `--color-fill-pressed`. Disabled: disabled text colour.
- **Popup button** (`.select-wrap > select`): 22 px, with a 16 px accent square holding white up/down chevrons on the right.
- **Stepper**: a value box (`role="spinbutton"`, ↑ / ↓ keys) plus a 15 px up/down arrow pair.
- **Checkbox**: 14 px, radius 3.5. When checked: accent fill with a white check.
- **Text field**: 22 px, radius 5. When focused: a 3 px focus ring. `.is-code` switches to mono.

### 4.15 ShortcutRecorder

- **Anatomy.** A 128 × 22 control that shows the combination with glyphs (default `⌘⇧V`), with a "restore default" button (ⓧ) on the right.
- **States.**
  - Idle.
  - Recording: placeholder "Type shortcut…" with a 2 px accent ring. Esc cancels. The combination needs ⌘, ⌃ or ⌥.
  - Error: 2 px destructive ring, and `#shortcut-error` shows "This shortcut is already used by macOS or another app. Choose another one." This covers OS-reserved shortcuts and registration failures.
  - Focus-visible.
- **Glyph order:** ⌃ ⌥ ⌘ ⇧ then the key, which matches the spec's `⌘⇧V`.
- After a change, update every place that shows the shortcut: the search hint, the onboarding keycaps and the desktop hint.
- **Prototype:** `#shortcut-recorder`, `#shortcut-keys`, `#btn-shortcut-reset`, `#shortcut-error`.

### 4.16 ExcludedAppsList

- **Anatomy.** A bordered list (`role="listbox"`, 112 px tall, 24 px rows: app icon 16, name, bundle id in tertiary text on the right), with a bottom bar of + / − buttons.
- Defaults: 1Password, Bitwarden, Keychain Access, Passwords.
- **Adding.** "+" opens a menu titled "Running apps" (from `list_running_apps()`), followed by "Choose from Applications…".
- **Removing.** "−" removes the selected app. It is disabled when nothing is selected. ⌫ works on the focused list too.
- **States.** Item: default · selected (selection colours). The − button: disabled.
- **Prototype:** `#excluded-apps-list`, `#btn-add-excluded-app`, `#btn-remove-excluded-app`, `#excluded-apps-menu`.

### 4.17 SnippetEditor

- **Anatomy.** A 420 px sheet with a title and a subtitle, then a form grid with right-aligned labels. Fields:
  - Title (`#snippet-title`, placeholder "e.g. Email signature").
  - Collection popup (`#snippet-collection`): No collection, then the existing collections, then "New collection…". Choosing "New collection…" shows `#snippet-new-collection`.
  - Content textarea (`#snippet-content`, mono for code).
- Buttons: Cancel (`#btn-snippet-cancel`) and Save (`#btn-snippet-save`, primary).
- **Title variants:** "Save as snippet" (from a clip), "New snippet" (from scratch), "Edit snippet".
- **States.**
  - Save is disabled until there is a title (and content, for a new snippet).
  - The content field is disabled for image and file clips.
  - ⌘↩ saves. Esc cancels.
- **Prototype:** `#snippet-editor`.

### 4.18 Onboarding steps

There are three steps (`#onboarding-welcome`, `#onboarding-accessibility`, `#onboarding-setup`), with step dots and a footer: Back (hidden on step 1) · Continue · "Start using Tadidy" (step 3 only).

On the accessibility step:

- The live status `#ax-status` shows a spinner and "Waiting for permission…" until access is granted, then "Accessibility enabled ✓" (`--color-success-text`).
- "Open System Settings" is disabled once access is granted.
- While access is missing, the Continue button reads "Continue without auto-paste".

## 5. Keyboard and focus rules

- **Scope.** Only the visible window handles keys. Shortcuts of the clip browser never fire in Preferences or onboarding. When a dialog is open, it gets all keys. When a menu is open, it gets the arrow keys, Return and Esc.
- **Clip browser.** The search field keeps focus. The list never takes DOM focus. Selection is virtual (`aria-activedescendant`).
- **Clip browser keys** (spec §5):

| Key | Action |
|---|---|
| ↑ / ↓ | Move the selection (clamped, scrolls to the nearest row) |
| ↩ | Paste |
| ⇧↩ | Paste as plain text (inverts the setting when "Always paste as plain text" is on) |
| ⌥↩ or ⌘C | Copy. ⌘C copies the search text instead when some of it is selected. |
| ⌘1–⌘9 | Paste the n-th visible clip |
| ⌘P | Pin / unpin |
| ⌘S | Save as snippet… |
| ⌘⌫ | Delete (asks first unless "Confirm before delete" is off) |
| ⌘O | Open the URL or reveal in Finder |
| ⌘K or right-click | Open the context menu |
| Space | Toggle the preview when the search field is empty or not focused |
| Tab / ⇧Tab | Next / previous tab |
| ⌘F or any printable key | Focus the search field (the typed character is kept) |
| Esc | Close the preview overlay, then clear the search, then close the window |
| ⌘, | Open Preferences |

- **Menus.** ↑ / ↓ skip disabled items and wrap around. → opens a submenu and ← closes it. ↩ or Space activates the item. Esc closes the submenu, then the menu.
- **Dialogs.** Focus is trapped. Initial focus goes to the default button (or the first field in the snippet editor). ↩ triggers the default button. Esc cancels. When the dialog closes, focus returns to the search field.
- **Preferences.** Sidebar tabs use roving tabindex and switch with ↑ / ↓. Steppers respond to ↑ / ↓. ⌘W closes the window.
- **Focus ring.** `outline: 3px solid var(--color-focus-ring)` on `:focus-visible` only, never on mouse focus.

## 6. Icons

- **Style.** SF-Symbols-like inline SVGs on a 16 × 16 view box: 1.4 px stroke, round caps and joins, `currentColor`. Default size 16 px; 14 px in chips and row actions; 12–13 px in menus and footnotes.
- **Set** (in the prototype's `ICONS` map plus static markup): search, text, code, rich, image, file, link, pin, pin (filled), trash, more (•••), gear, check, copy, external, folder, bookmark (snippet), warning (filled triangle), plus, minus, lock, keyboard, appearance, info, accessibility, clipboard. In production, export them as `src/lib/icons/*.svg` or as Svelte components with the same names.
- **Tray icon.** A clipboard with three history lines, drawn on an 18 × 18 view box with a 1.4 px stroke. Ship it as a **template image** (`icons/tray-template.png` @1x and `@2x`, black on transparent, with `icon_as_template(true)`) so macOS tints it for light and dark menu bars and for the highlighted state. Never colour it.
- **App icon.** Blue squircle (`#5A8DFF` → `#3346D3`) with a white clipboard and an orange clock badge (`#FF9F0A`). It is used in alerts (56 px), onboarding (80 px) and About (96 px).
- **Source app icons.** Real app icons from `app_icons` (cached PNG), shown at 14 px in rows, 12 px in the preview metadata and 16 px in lists. The coloured glyphs in the prototype are stand-ins.

## 7. Accessibility

- **VoiceOver roles.**
  - Clip list: `listbox` / `option`, with `aria-selected`, and an `aria-label` that describes the clip ("Code: SELECT…, from Visual Studio Code, copied 1 minute ago, pinned, Command 1").
  - Snippet collections: `group` with `aria-labelledby`.
  - Tabs: `tablist` / `tab`.
  - Filter chips: toggle buttons (`aria-pressed`).
  - Menus: `menu` / `menuitem`, with `aria-disabled` and `aria-haspopup` on Transform.
  - Alerts: `alertdialog`. Sheets and windows: `dialog`.
  - Banner, toast and the onboarding status: `status`. Switches: `role="switch"`. Steppers: `spinbutton`.
  - Icon-only buttons always have an `aria-label`.
- **Contrast (AA).** Body and secondary text pass 4.5:1 on every surface in both themes, white on `--color-selection` included. Tertiary text is limited to placeholders, shortcut hints and duplicated meta. Never use it for information that exists nowhere else.
- **Reduce motion.** Durations drop to 0 (see §2.6).
- **Reduce transparency.** When `prefers-reduced-transparency` (or `NSWorkspace.accessibilityDisplayShouldReduceTransparency`) is set:
  - The `--material-*` values become opaque.
  - `backdrop-filter` is set to none.
  - Rust calls `clear_vibrancy` and sets an opaque window background.
- **Increase contrast.** Not covered by tokens in v1. Separators must remain visible at 0.10 alpha. Revisit if users report problems.
- **Full keyboard access.** Every action is reachable without a mouse (see §5).

## 8. Copy rules

- **Language.** The UI is in English only (US spelling: "color", "center").
- **Case.** Sentence case everywhere: buttons, menu items, titles, tabs ("Clear history…", "Save as snippet…", "Check for updates…"). Proper names keep their case: Tadidy, System Settings, Accessibility, Finder, Keychain.
- **Ellipsis.** Use "…" (U+2026, never three dots) in two cases:
  - An action that opens a dialog or needs more input: "Preferences…", "Clear history…", "Save as snippet…", "Export…", "Import…", "Edit…".
  - A progress state: "Waiting for permission…", "Checking for updates…", "Type shortcut…".
  - Actions that complete immediately take no ellipsis: "Paste", "Delete", "Quit Tadidy".
- **Typography.**
  - Typographic apostrophes and quotes: ’ “ ” (for example "can’t", "Don’t ask again", No clips match “xyz”).
  - Em dash with spaces for asides: "Auto-paste disabled — clips are copied only."
  - "×" for dimensions: 1440 × 900.
- **Shortcut glyphs.**
  - Modifiers: ⌘ Command, ⇧ Shift, ⌥ Option, ⌃ Control.
  - Keys: ↩ Return, ⌫ Delete, ⎋ Esc, ⇥ Tab, Space.
  - Order modifiers as in the spec (`⌘⇧V`) and write them with no spaces or "+".
  - In body prose, keys may be named: "Press ⌘C anywhere".
- **Relative time.** "Just now", "2m ago", "3h ago", "Yesterday", "4d ago" in rows. Long forms ("2 minutes ago") in the preview and in VoiceOver labels.
- **Counts.** "487 clips", "1 result" / "3 results", "6 snippets". Use thousands separators ("1,000 clips").
- **Tone.** Short, calm, no exclamation marks. Errors say what happened and what to do next.

## 9. UI ticket checklist

Copy this checklist into every UI ticket and PR, and tick every box before review.

- [ ] Element ids match the prototype exactly (`docs/prototype/prototype.html#…`). No id was renamed or dropped.
- [ ] All texts are copied verbatim from the prototype: typographic quotes and apostrophes, "…", shortcut glyphs, capitalisation.
- [ ] Every state is implemented: default, hover, selected, focus-visible, disabled, empty (`#empty-*`), error (for example `#shortcut-error`), loading (no empty-state flash), permission missing (`#banner-accessibility`, `?ax=0`).
- [ ] Keyboard paths work as in §5: shortcuts are scoped to the visible window, focus stays in search, Esc order is right, dialogs trap focus, menus support the arrow keys.
- [ ] Mouse paths work: hover actions, right-click menu, double-click to paste, click outside closes.
- [ ] Checked in both light and dark (`?theme=light`, `?theme=dark`) and with the system appearance switched live.
- [ ] Reduced motion: no transition longer than 0 ms when it is on. Nothing animates longer than 120 ms when it is off.
- [ ] Reduced transparency: opaque materials and no blur. Vibrancy is cleared on the native side.
- [ ] VoiceOver: roles and `aria-*` as in §7, labels on icon-only buttons, the selected clip is announced, status messages are polite.
- [ ] Contrast AA is met. Tertiary text is used only for duplicated or decorative information.
- [ ] Only tokens are used: no hard-coded colours, radii, spacing, shadows or durations (`grep -E '#[0-9a-fA-F]{3,6}|rgba?\(' src/lib/components` returns nothing outside `tokens.css`).
- [ ] The window size, material and position match §3.
- [ ] A screenshot of the implementation next to the prototype, in light and dark, is attached to the PR.
