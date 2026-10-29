<script lang="ts">
  import { onMount } from "svelte";
  import { clipCountLabel } from "../format";
  import { ClipsStore } from "../stores/clips.svelte";
  import BrowserFooter from "./BrowserFooter.svelte";
  import ClipList from "./ClipList.svelte";
  import EmptyState from "./EmptyState.svelte";
  import SearchBar from "./SearchBar.svelte";

  const MINUTE_MS = 60_000;

  const store = new ClipsStore();

  /** Rows show their age against this, moved forward every minute. */
  let now = $state(Date.now());

  const selectedId = $derived(store.items[store.selectedIndex]?.id);

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
    activeDescendant={selectedId === undefined ? undefined : `clip-${selectedId}`}
    onsearch={(text) => store.setSearch(text)}
  />

  <div class="list-wrap">
    <ClipList
      items={store.items}
      total={store.total}
      selectedIndex={store.selectedIndex}
      {now}
      iconUrls={store.iconUrls}
      resetKey={store.query}
      onneedmore={() => void store.loadMore()}
      onneedicon={(bundleId) => store.requestIcon(bundleId)}
    />
    {#if store.loaded && store.items.length === 0}
      {#if store.query}
        <EmptyState variant="search" query={store.query} />
      {:else}
        <EmptyState variant="history" />
      {/if}
    {/if}
  </div>

  <BrowserFooter count={clipCountLabel(store.total, store.query !== "")} />
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
