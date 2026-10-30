<script lang="ts">
  import { onMount } from "svelte";
  import { clipCountLabel } from "../format";
  import { resolveKey, type BrowserAction } from "../keyboard";
  import { ClipsStore } from "../stores/clips.svelte";
  import type { RowRange } from "../virtual-list";
  import BrowserFooter from "./BrowserFooter.svelte";
  import ClipList from "./ClipList.svelte";
  import DeleteClipDialog from "./DeleteClipDialog.svelte";
  import EmptyState from "./EmptyState.svelte";
  import PermissionBanner from "./PermissionBanner.svelte";
  import SearchBar from "./SearchBar.svelte";

  const MINUTE_MS = 60_000;

  /** The host decides what pasting, copying, closing and the preferences do (TAD-016): by default, nothing. */
  interface Props {
    /** Which window hosts the browser: the footer shows its hints in the panel only. */
    layout?: "popover" | "panel";
    /** Ask before deleting a clip (Preferences › Confirm before delete). */
    confirmBeforeDelete?: boolean;
    onpaste?: (id: string) => void;
    oncopy?: (id: string) => void;
    onclose?: () => void;
    onopenpreferences?: () => void;
    /** "Don’t ask again" was ticked in the delete dialog. */
    ondisableconfirm?: () => void;
  }

  const noop = () => undefined;

  let {
    layout = "popover",
    confirmBeforeDelete = true,
    onpaste = noop,
    oncopy = noop,
    onclose = noop,
    onopenpreferences = noop,
    ondisableconfirm = noop,
  }: Props = $props();

  const store = new ClipsStore();

  /** Rows show their age against this, moved forward every minute. */
  let now = $state(Date.now());

  let renderedRange = $state<RowRange>({ start: 0, end: 0 });

  let searchBar = $state<SearchBar>();
  let list = $state<ClipList>();

  /** The clip the delete dialog is about; no dialog while it is `undefined`. */
  let pendingDelete = $state<string>();
  let dialog = $state<DeleteClipDialog>();

  /** Virtualization drops rows far from the viewport: only a row in the DOM can be the active one. */
  const activeId = $derived(
    store.selectedIndex >= renderedRange.start && store.selectedIndex < renderedRange.end
      ? store.items[store.selectedIndex]?.id
      : undefined,
  );

  function onkeydown(event: KeyboardEvent) {
    // Every key goes to an open modal, wherever the focus is (as in the prototype).
    if (pendingDelete !== undefined) {
      dialog?.handleKey(event);
      return;
    }
    const action = resolveKey(event, {
      searchFocused: searchBar?.hasFocus() ?? false,
      searchEmpty: store.search === "",
      inputHasSelection: searchBar?.hasSelection() ?? false,
      dialogOpen: pendingDelete !== undefined,
    });
    if (action === null) return;
    // A typed character is not stopped: it goes on into the field that just got the focus.
    if (action !== "focus-search" || event.metaKey) event.preventDefault();
    run(action, event.metaKey);
  }

  /** Keys that act on the selected row. */
  const ROW_ACTIONS: BrowserAction[] = ["paste", "copy", "delete"];

  function run(action: BrowserAction, command = false, settled = false) {
    // The selected row belongs to the list of an older search: wait for the typed one.
    if (!settled && store.stale && ROW_ACTIONS.includes(action)) {
      void store.settled().then(() => {
        if (!store.stale) run(action, command, true);
      });
      return;
    }
    const id = store.selected?.id;
    switch (action) {
      case "select-next":
        void store.selectNext().then(() => list?.reveal(store.selectedIndex));
        break;
      case "select-previous":
        store.selectPrevious();
        list?.reveal(store.selectedIndex);
        break;
      case "paste":
        if (id !== undefined) onpaste(id);
        break;
      case "copy":
        if (id !== undefined) oncopy(id);
        break;
      case "delete":
        if (id === undefined) break;
        if (confirmBeforeDelete) pendingDelete = id;
        else void store.deleteClip(id);
        break;
      case "clear-search":
        store.setSearch("");
        break;
      case "close":
        onclose();
        break;
      case "focus-search":
        searchBar?.focus(command);
        break;
      case "open-preferences":
        onopenpreferences();
        break;
    }
  }

  /** Back to the state of a fresh window: no dialog, no search, first row selected, list reloaded, search focused. */
  export async function reset(): Promise<void> {
    // A dialog left open would still hold the clip of the previous session.
    pendingDelete = undefined;
    const loaded = store.reset();
    searchBar?.focus();
    await loaded;
    list?.reveal(0);
  }

  function confirmDelete(dontAskAgain: boolean) {
    const id = pendingDelete;
    closeDialog();
    if (dontAskAgain) ondisableconfirm();
    if (id !== undefined) void store.deleteClip(id);
  }

  function closeDialog() {
    pendingDelete = undefined;
    searchBar?.focus();
  }

  function activate(index: number) {
    store.select(index);
    run("paste");
  }

  onMount(() => {
    void store.start();
    const tick = setInterval(() => (now = Date.now()), MINUTE_MS);
    return () => {
      clearInterval(tick);
      store.destroy();
    };
  });
</script>

<svelte:window {onkeydown} />

<div id="clip-browser" class="clip-browser" data-layout={layout}>
  <SearchBar
    bind:this={searchBar}
    value={store.search}
    activeDescendant={activeId === undefined ? undefined : `clip-${activeId}`}
    onsearch={(text) => store.setSearch(text)}
  />

  <div class="list-wrap">
    <ClipList
      bind:this={list}
      items={store.items}
      total={store.total}
      selectedIndex={store.selectedIndex}
      {now}
      iconUrls={store.iconUrls}
      onrange={(range) => (renderedRange = range)}
      resetKey={store.query}
      onneedmore={() => void store.loadMore()}
      onneedicon={(bundleId) => store.requestIcon(bundleId)}
      onselect={(index) => store.select(index)}
      onactivate={activate}
    />
    {#if store.loaded && store.items.length === 0}
      {#if store.loadedQuery}
        <EmptyState variant="search" query={store.loadedQuery} />
      {:else}
        <EmptyState variant="history" />
      {/if}
    {/if}
  </div>

  <PermissionBanner />

  <BrowserFooter count={clipCountLabel(store.total, store.loadedQuery !== "")} onpreferences={onopenpreferences} />

  {#if pendingDelete !== undefined}
    <div id="modal-layer">
      <DeleteClipDialog bind:this={dialog} onconfirm={confirmDelete} oncancel={closeDialog} />
    </div>
  {/if}
</div>

<style>
  .clip-browser {
    position: relative;
    height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr) auto auto;
    grid-template-areas: "search" "list" "banner" "footer";
  }

  /* The prototype's dim layer over the browser while a dialog is open. */
  #modal-layer {
    position: absolute;
    inset: 0;
    z-index: 300;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.06);
  }

  .list-wrap {
    grid-area: list;
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
