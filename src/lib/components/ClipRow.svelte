<script lang="ts">
  import type { ClipKind } from "../bindings/ClipKind";
  import type { ClipSummary } from "../bindings/ClipSummary";
  import { relativeTime } from "../format";
  import { ROW_HEIGHT } from "../virtual-list";

  interface Props {
    clip: ClipSummary;
    /** Position in the whole list (not in the rendered slice). */
    index: number;
    /** Size of the whole list, for `aria-setsize`: only a slice of it is in the DOM. */
    total: number;
    selected: boolean;
    now: number;
    /** Object URL of the source app's icon, when the core has one. */
    iconUrl: string | undefined;
    onselect: () => void;
    onactivate: () => void;
  }

  let { clip, index, total, selected, now, iconUrl, onselect, onactivate }: Props = $props();

  /** Glyph paths of the prototype's 16 × 16 icon set, by clip kind. */
  const KIND_GLYPHS: Record<ClipKind, string[]> = {
    text: ["M3 4h10M3 7h10M3 10h10M3 13h6"],
    rich: ["M1.8 12.5 5.2 3.5h.6l3.4 9M3 9.6h5", "M11 6.5h3.2M11 9.5h3.2M11 12.5h3.2"],
    image: [
      "M4 3h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
      "M5.75 5.4a1.1 1.1 0 1 0 0 2.2 1.1 1.1 0 0 0 0-2.2z",
      "m2.5 11.5 3.2-3 2.6 2.3 2.2-1.8 3 2.8",
    ],
    file: ["M4 2.5h4.75L12 5.75v7.75H4z", "M8.5 2.5v3.5H12"],
    url: [
      "M6.8 9.2a2.6 2.6 0 0 0 3.7 0l2-2a2.6 2.6 0 0 0-3.7-3.7l-.9.9",
      "M9.2 6.8a2.6 2.6 0 0 0-3.7 0l-2 2a2.6 2.6 0 0 0 3.7 3.7l.9-.9",
    ],
    color: ["M8 3.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9z"],
  };

  const PIN_PATHS = [
    "M9.5 2.2 13.8 6.5l-1.3.6-2.4 2.4-.3 2.9-1.1 1.1L2.5 7.2l1.1-1.1 2.9-.3 2.4-2.4z",
    "M5.5 10.5 2.5 13.5",
  ];

  /** An image has no text: the row says what it is, as the prototype does for an unnamed one. */
  const text = $derived(clip.preview ?? (clip.kind === "image" ? "Image" : ""));
</script>

<!-- The keys are handled for the whole browser (`resolveKey`); a click only adds the mouse. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="clip-row"
  class:is-selected={selected}
  role="option"
  id="clip-{clip.id}"
  data-id={clip.id}
  data-kind={clip.kind}
  data-index={index}
  aria-selected={selected}
  aria-posinset={index + 1}
  aria-setsize={total}
  style:top="{index * ROW_HEIGHT}px"
  style:height="{ROW_HEIGHT}px"
  tabindex="-1"
  onmousedown={(event) => event.preventDefault()}
  onclick={onselect}
  ondblclick={onactivate}
>
  <span class="clip-badge" aria-hidden="true">{index < 9 ? `⌘${index + 1}` : ""}</span>
  <span class="clip-tile">
    <svg class="ico" viewBox="0 0 16 16" aria-hidden="true">
      {#each KIND_GLYPHS[clip.kind] as path (path)}<path d={path} />{/each}
    </svg>
  </span>
  <div class="clip-content"><div class="clip-text">{text}</div></div>
  <div class="clip-meta">
    {#if clip.pinned}
      <span class="pin-marker" title="Pinned">
        <svg class="ico" viewBox="0 0 16 16" aria-hidden="true">
          {#each PIN_PATHS as path (path)}<path d={path} fill="currentColor" />{/each}
        </svg>
      </span>
    {/if}
    {#if iconUrl}
      <img class="app-icon" src={iconUrl} width="14" height="14" alt="" />
    {:else if clip.sourceBundle}
      <svg class="app-icon app-icon-generic" viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <rect x="1" y="1" width="14" height="14" rx="3.4" />
        <path d="M5 8h6M8 5v6" />
      </svg>
    {/if}
    <span class="clip-time">{relativeTime(clip.lastCopiedAt, now)}</span>
  </div>
</div>

<style>
  .clip-row {
    position: absolute;
    left: 0;
    right: 0;
    box-sizing: border-box;
    display: grid;
    align-items: center;
    grid-template-columns: 20px var(--tile-size) minmax(0, 1fr) auto;
    column-gap: var(--space-8);
    padding: 7px var(--space-8) 7px var(--space-4);
    border-radius: var(--radius-md);
    user-select: none;
  }

  .clip-row:hover {
    background: var(--color-fill-hover);
  }

  .clip-row.is-selected {
    background: var(--color-selection);
    color: var(--color-selection-text);
  }

  .clip-badge {
    font-size: var(--text-small);
    color: var(--color-text-tertiary);
    text-align: right;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }

  .clip-tile {
    width: var(--tile-size);
    height: var(--tile-size);
    border-radius: var(--radius-md);
    background: var(--color-fill);
    color: var(--color-text-secondary);
    display: grid;
    place-items: center;
    overflow: hidden;
  }

  .ico {
    width: 16px;
    height: 16px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .clip-content {
    min-width: 0;
  }

  .clip-text {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: var(--preview-lines);
    line-clamp: var(--preview-lines);
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .clip-meta {
    align-self: start;
    margin-top: 1px;
    min-width: 76px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 5px;
    font-size: var(--text-small);
    color: var(--color-text-secondary);
    white-space: nowrap;
  }

  .clip-time {
    font-variant-numeric: tabular-nums;
  }

  .pin-marker {
    color: var(--color-pin);
    display: inline-flex;
  }

  .pin-marker .ico {
    width: 12px;
    height: 12px;
  }

  .app-icon {
    flex: none;
    display: block;
  }

  .app-icon-generic rect {
    fill: var(--color-fill-pressed);
  }

  .app-icon-generic path {
    fill: none;
    stroke: var(--color-text-tertiary);
    stroke-width: 1.4;
    stroke-linecap: round;
  }

  .clip-row.is-selected .clip-badge,
  .clip-row.is-selected .clip-meta,
  .clip-row.is-selected .pin-marker {
    color: var(--color-selection-text-secondary);
  }

  .clip-row.is-selected .clip-tile {
    background: color-mix(in srgb, var(--color-selection-text) 18%, transparent);
    color: var(--color-selection-text);
  }
</style>
