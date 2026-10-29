<script lang="ts">
  import { untrack } from "svelte";
  import type { ClipSummary } from "../bindings/ClipSummary";
  import { OVERSCAN, ROW_HEIGHT, visibleRange, type RowRange } from "../virtual-list";
  import ClipRow from "./ClipRow.svelte";

  /** The next page is asked for when the last rows of what is loaded come into view. */
  const LOAD_AHEAD = 10;

  interface Props {
    items: ClipSummary[];
    /** How many clips the history holds for this search, loaded or not. */
    total: number;
    selectedIndex: number;
    now: number;
    iconUrls: ReadonlyMap<string, string>;
    /** Changes with the search: the list goes back to the top. */
    resetKey: string;
    /** Told which rows are in the DOM, so `aria-activedescendant` never points at a missing one. */
    onrange: (range: RowRange) => void;
    onneedmore: () => void;
    onneedicon: (bundleId: string) => void;
    onselect: (index: number) => void;
    /** A row was double-clicked. */
    onactivate: (index: number) => void;
  }

  let {
    items,
    total,
    selectedIndex,
    now,
    iconUrls,
    resetKey,
    onrange,
    onneedmore,
    onneedicon,
    onselect,
    onactivate,
  }: Props = $props();

  let list = $state<HTMLElement>();
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  const visible = $derived(visibleRange(scrollTop, viewportHeight, ROW_HEIGHT, items.length, OVERSCAN));
  const rendered = $derived(items.slice(visible.start, visible.end));

  /** Scrolls the least that brings the row at `index` fully into view, keeping the list's padding around it. */
  export function reveal(index: number) {
    if (!list) return;
    const padding = Number.parseFloat(getComputedStyle(list).paddingTop) || 0;
    const top = padding + index * ROW_HEIGHT;
    const bottom = top + ROW_HEIGHT;
    if (top - padding < list.scrollTop) list.scrollTop = top - padding;
    else if (bottom + padding > list.scrollTop + viewportHeight) list.scrollTop = bottom + padding - viewportHeight;
    scrollTop = list.scrollTop;
  }

  $effect.pre(() => {
    onrange(visible);
  });

  $effect(() => {
    for (const clip of rendered) {
      const bundleId = clip.sourceBundle;
      if (bundleId) untrack(() => onneedicon(bundleId));
    }
  });

  $effect(() => {
    if (visible.end > items.length - LOAD_AHEAD) untrack(onneedmore);
  });

  $effect(() => {
    void resetKey;
    untrack(() => {
      if (list) list.scrollTop = 0;
      scrollTop = 0;
    });
  });
</script>

<div
  id="clip-list"
  role="listbox"
  aria-label="Clips"
  bind:this={list}
  bind:clientHeight={viewportHeight}
  onscroll={() => (scrollTop = list?.scrollTop ?? 0)}
>
  <div class="clip-rows" role="presentation" style:height="{items.length * ROW_HEIGHT}px">
    {#each rendered as clip, offset (clip.id)}
      {@const index = visible.start + offset}
      <ClipRow
        {clip}
        {index}
        {total}
        {now}
        selected={index === selectedIndex}
        iconUrl={clip.sourceBundle ? iconUrls.get(clip.sourceBundle) : undefined}
        onselect={() => onselect(index)}
        onactivate={() => onactivate(index)}
      />
    {/each}
  </div>
</div>

<style>
  #clip-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
    outline: none;
    scrollbar-width: thin;
  }

  .clip-rows {
    position: relative;
  }
</style>
