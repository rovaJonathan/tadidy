// State of the clip browser: the search text, the pages of clips it shows, the selection and the
// source-app icons. The core owns the data; this class only mirrors what `list_clips` returns.
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { getAppIcon, listClips, onClipsChanged } from "../api";
import type { ClipQuery } from "../bindings/ClipQuery";
import type { ClipSummary } from "../bindings/ClipSummary";

/** SRC-01: the list follows the typing, 50 ms after the last key. */
const SEARCH_DEBOUNCE_MS = 50;

export class ClipsStore {
  /** What the search field holds, as typed. */
  search = $state("");
  items = $state.raw<ClipSummary[]>([]);
  /** How many clips match the search in all, not only the ones loaded. */
  total = $state(0);
  nextCursor = $state<string | null>(null);
  loading = $state(false);
  /** The first page has arrived: before that, an empty list is not an empty history. */
  loaded = $state(false);
  selectedIndex = $state(0);
  /** Object URLs of the icons found, by bundle id; a bundle id without an icon is absent. */
  readonly iconUrls = new SvelteMap<string, string>();

  /** The search the core is asked for. */
  query = $derived(this.search.trim());

  /** Bumped by every first-page load and by `destroy`: an older answer is dropped. */
  #generation = 0;
  #destroyed = false;
  #debounce: ReturnType<typeof setTimeout> | undefined;
  #frame: number | undefined;
  #unlisten: (() => void) | undefined;
  readonly #requestedIcons = new SvelteSet<string>();

  /** Loads the first page and follows `clips://changed` until `destroy`. */
  async start(): Promise<void> {
    const listening = onClipsChanged(() => this.#scheduleRefresh()).then((unlisten) => {
      if (this.#destroyed) void unlisten();
      else this.#unlisten = unlisten;
    });
    await Promise.all([listening, this.#loadFirstPage(true)]);
  }

  /** Releases the event listener, the pending timers and the icon object URLs. */
  destroy(): void {
    this.#destroyed = true;
    this.#generation++;
    clearTimeout(this.#debounce);
    this.#debounce = undefined;
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    this.#frame = undefined;
    this.#unlisten?.();
    this.#unlisten = undefined;
    for (const url of this.iconUrls.values()) URL.revokeObjectURL(url);
    this.iconUrls.clear();
  }

  setSearch(text: string): void {
    this.search = text;
    clearTimeout(this.#debounce);
    this.#debounce = setTimeout(() => {
      this.#debounce = undefined;
      void this.#loadFirstPage(false);
    }, SEARCH_DEBOUNCE_MS);
  }

  /** Reloads the first page; the selected clip stays selected when it is still there. */
  refresh(): Promise<void> {
    return this.#loadFirstPage(true);
  }

  /** Back to the state of a fresh window: no search, first row selected. */
  reset(): Promise<void> {
    clearTimeout(this.#debounce);
    this.#debounce = undefined;
    this.search = "";
    this.selectedIndex = 0;
    return this.#loadFirstPage(false);
  }

  async loadMore(): Promise<void> {
    const cursor = this.nextCursor;
    if (cursor === null || this.loading) return;
    const generation = this.#generation;
    this.loading = true;
    try {
      const page = await listClips(this.#queryFor(cursor));
      if (generation !== this.#generation) return;
      this.items = [...this.items, ...page.items];
      this.nextCursor = page.nextCursor;
      this.total = page.total;
    } catch {
      // The rows already shown stay; the next scroll asks again.
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  /** Fetches the icon of a source app, once per bundle id. */
  requestIcon(bundleId: string): void {
    if (this.#destroyed || this.#requestedIcons.has(bundleId)) return;
    this.#requestedIcons.add(bundleId);
    getAppIcon(bundleId)
      .then((png) => {
        if (png === null || this.#destroyed) return;
        this.iconUrls.set(bundleId, URL.createObjectURL(new Blob([png], { type: "image/png" })));
      })
      .catch(() => {
        // No icon: the row keeps the generic one.
      });
  }

  async #loadFirstPage(keepSelection: boolean): Promise<void> {
    const generation = ++this.#generation;
    this.loading = true;
    try {
      const page = await listClips(this.#queryFor());
      if (generation !== this.#generation) return;
      const selectedId = keepSelection ? this.items[this.selectedIndex]?.id : undefined;
      this.items = page.items;
      this.total = page.total;
      this.nextCursor = page.nextCursor;
      this.selectedIndex = Math.max(
        0,
        page.items.findIndex((clip) => clip.id === selectedId),
      );
      this.loaded = true;
    } catch {
      // The rows already shown stay.
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  #queryFor(cursor?: string): ClipQuery {
    const query: ClipQuery = {};
    if (this.query) query.search = this.query;
    if (cursor !== undefined) query.cursor = cursor;
    return query;
  }

  /** One reload per animation frame, however many clips were recorded in it. */
  #scheduleRefresh(): void {
    if (this.#destroyed || this.#frame !== undefined) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = undefined;
      void this.refresh();
    });
  }
}
