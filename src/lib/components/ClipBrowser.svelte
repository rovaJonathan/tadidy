<script lang="ts">
  import { onMount } from "svelte";
  import { clipCountLabel } from "../format";
  import { ClipsStore } from "../stores/clips.svelte";
  import type { RowRange } from "../virtual-list";
  import BrowserFooter from "./BrowserFooter.svelte";
  import ClipList from "./ClipList.svelte";
  import EmptyState from "./EmptyState.svelte";
  import SearchBar from "./SearchBar.svelte";

  const MINUTE_MS = 60_000;

  const store = new ClipsStore();

  /** Rows show their age against this, moved forward every minute. */
  let now = $state(Date.now());

  let renderedRange = $state<RowRange>({ start: 0, end: 0 });

  /** Virtualization drops rows far from the viewport: only a row in the DOM can be the active one. */
  const activeId = $derived(
    store.selectedIndex >= renderedRange.start && store.selectedIndex < renderedRange.end
      ? store.items[store.selectedIndex]?.id
      : undefined,
  );

  onMount(() => {
    void store.start();
    const tick = setInterval(() => (now = Date.now()), MINUTE_MS);
    return () => {
      clearInterval(tick);
      store.destroy();
    };
  });
</script>

<div id="clip-browser" class="clip-browser" data-layout="popover">
  <SearchBar
    value={store.search}
    activeDescendant={activeId === undefined ? undefined : `clip-${activeId}`}
    onsearch={(text) => store.setSearch(text)}
  />

  <div class="list-wrap">
    <ClipList
      items={store.items}
      total={store.total}
      selectedIndex={store.selectedIndex}
      {now}
      iconUrls={store.iconUrls}
      onrange={(range) => (renderedRange = range)}
      resetKey={store.query}
      onneedmore={() => void store.loadMore()}
      onneedicon={(bundleId) => store.requestIcon(bundleId)}
    />
    {#if store.loaded && store.items.length === 0}
      {#if store.loadedQuery}
        <EmptyState variant="search" query={store.loadedQuery} />
      {:else}
        <EmptyState variant="history" />
      {/if}
    {/if}
  </div>

  <BrowserFooter count={clipCountLabel(store.total, store.loadedQuery !== "")} />
</div>

<style>
  .clip-browser {
    position: relative;
    height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr) auto;
    grid-template-areas: "search" "list" "footer";
  }

  .list-wrap {
    grid-area: list;
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
