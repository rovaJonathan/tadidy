<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    value: string;
    /** The id of the selected row, for `aria-activedescendant` (focus stays in the field). */
    activeDescendant?: string;
    shortcut?: string;
    onsearch: (text: string) => void;
  }

  let { value, activeDescendant, shortcut = "⌘⇧V", onsearch }: Props = $props();

  let field = $state<HTMLInputElement>();

  onMount(() => field?.focus());

  /** Gives the field the focus, with its text selected when `select` is set. */
  export function focus(select = false) {
    field?.focus();
    if (select) field?.select();
  }

  export function hasFocus(): boolean {
    return document.activeElement === field;
  }

  /** Part of the text is selected: ⌘C copies it instead of the clip. */
  export function hasSelection(): boolean {
    return field !== undefined && field.selectionStart !== field.selectionEnd;
  }

  function clear() {
    onsearch("");
    focus();
  }
</script>

<div class="search-bar">
  <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
    ><circle cx="7" cy="7" r="4.25" /><path d="M10.2 10.2 13.5 13.5" /></svg
  >
  <input
    id="search-input"
    bind:this={field}
    type="text"
    placeholder="Search clips…"
    autocomplete="off"
    spellcheck="false"
    role="combobox"
    aria-expanded="true"
    aria-controls="clip-list"
    aria-autocomplete="list"
    aria-label="Search clips"
    aria-activedescendant={activeDescendant}
    {value}
    oninput={(event) => onsearch(event.currentTarget.value)}
  />
  <button
    type="button"
    id="btn-clear-search"
    class="icon-btn"
    aria-label="Clear search"
    hidden={value === ""}
    onclick={clear}
  >
    <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
      ><circle cx="8" cy="8" r="6.5" fill="currentColor" stroke="none" /><path
        d="m5.8 5.8 4.4 4.4m0-4.4-4.4 4.4"
        stroke="var(--surface-window)"
        stroke-width="1.5"
      /></svg
    >
  </button>
  <kbd id="search-shortcut-hint" title="Open Tadidy from anywhere" hidden={value !== ""}>{shortcut}</kbd>
</div>

<style>
  .search-bar {
    grid-area: search;
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: 44px;
    padding: 0 var(--space-12);
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

  .search-bar > .ico {
    color: var(--color-text-secondary);
  }

  #search-input {
    flex: 1;
    min-width: 0;
    height: 28px;
    border: 0;
    outline: none;
    background: transparent;
    font: inherit;
    font-size: var(--text-search);
    color: var(--color-text-primary);
    padding: 0;
  }

  #search-input::placeholder {
    color: var(--color-text-tertiary);
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: grid;
    place-items: center;
    color: var(--color-text-tertiary);
    padding: 0;
  }

  .icon-btn:hover {
    background: var(--color-fill-hover);
    color: var(--color-text-primary);
  }

  .icon-btn:active {
    background: var(--color-fill-pressed);
  }

  .icon-btn .ico {
    width: 14px;
    height: 14px;
  }

  .icon-btn:focus-visible {
    outline: 3px solid var(--color-focus-ring);
  }

  [hidden] {
    display: none !important;
  }

  kbd {
    font: var(--weight-medium) var(--text-small) / 1 var(--font-sans);
    color: var(--color-text-tertiary);
    background: var(--color-fill);
    border-radius: 4px;
    padding: 3px 5px;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }
</style>
