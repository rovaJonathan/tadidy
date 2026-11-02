// State of the clip browser: the search text, the pages of clips it shows, the selection and the
// source-app icons. The core owns the data; this class only mirrors what `list_clips` returns.
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import {
  deleteClips,
  getAppIcon,
  getStorageStatus,
  listClips,
  onClipsChanged,
  onStorageChanged,
  resetHistoryDatabase,
} from "../api";
import type { ClipQuery } from "../bindings/ClipQuery";
import type { ClipSummary } from "../bindings/ClipSummary";
import type { StorageStatus } from "../bindings/StorageStatus";

/** SRC-01: the list follows the typing, 50 ms after the last key. */
const SEARCH_DEBOUNCE_MS = 50;

/** The core's page sizes (`ClipQuery.limit`: 50 by default, 200 at most). */
const DEFAULT_PAGE_SIZE = 50;
const MAX_PAGE_SIZE = 200;

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
  /** What the core last said about the history database (`StorageStatus`); the flags below derive from it. */
  storage = $state<StorageStatus>({ state: "ready", code: null, canReset: false });
  /** The core could not open the history database: nothing is listed until it is reset. */
  storageUnavailable = $derived(this.storage.state === "unavailable");
  /** The last reset did not work: the history is still unavailable and nothing moved. */
  resetFailed = $state(false);
  /** A reset is running (it can wait on a Keychain prompt): the button stays off and a second request is ignored. */
  resetting = $state(false);
  /** The reset may be offered: the key is lost, not only out of reach for now. The core decides (`StorageStatus.canReset`). */
  storageCanReset = $derived(this.storage.state === "unavailable" && this.storage.canReset);
  selectedIndex = $state(0);
  /** Object URLs of the icons found, by bundle id; a bundle id without an icon is absent. */
  readonly iconUrls = new SvelteMap<string, string>();

  /** The clip under the selection, if the history has one. */
  selected = $derived(this.items[this.selectedIndex]);

  /** The search the core is asked for. */
  query = $derived(this.search.trim());
  /** The search `items` and `total` answer: it trails `query` until the reload arrives. */
  loadedQuery = $state("");
  /** The list still shows the answer to an older search: a key that acts on a row must wait (`settled`). */
  stale = $derived(this.query !== this.loadedQuery);

  /** Bumped by every first-page load and by `destroy`: an older answer is dropped. */
  #generation = 0;
  #destroyed = false;
  #debounce: ReturnType<typeof setTimeout> | undefined;
  #firstPage: Promise<void> = Promise.resolve();
  #frame: number | undefined;
  #unlisten: (() => void) | undefined;
  #unlistenStorage: (() => void) | undefined;
  /** A `storage://changed` arrived: it is newer than the status asked for at start. */
  #storageAnnounced = false;
  #markStorageKnown!: () => void;
  /** Settles once the storage status is known (asked for, or announced), or the store is destroyed. */
  readonly storageKnown = new Promise<void>((resolve) => (this.#markStorageKnown = resolve));
  readonly #requestedIcons = new SvelteSet<string>();

  /** Loads the first page and follows `clips://changed` and `storage://changed` until `destroy`. */
  async start(): Promise<void> {
    const listening = onClipsChanged(() => this.#scheduleRefresh()).then((unlisten) => {
      if (this.#destroyed) void unlisten();
      else this.#unlisten = unlisten;
    });
    const listeningStorage = onStorageChanged((status) => this.#applyStorage(status)).then((unlisten) => {
      if (this.#destroyed) void unlisten();
      else this.#unlistenStorage = unlisten;
    });
    // Listening first: an announcement made while the status is asked for must not be missed.
    // A status that cannot be read (or a listener that cannot be set) counts as ready: the list
    // then fails or shows by itself.
    let status: StorageStatus | undefined;
    try {
      await listeningStorage;
      status = await getStorageStatus();
    } catch {
      status = undefined;
    }
    // An announcement made meanwhile is newer than this answer.
    if (status !== undefined && !this.#destroyed && !this.#storageAnnounced) this.#setStorage(status);
    this.#markStorageKnown();
    await Promise.all([listening, this.#loadFirstPage(true)]);
  }

  /** Releases the event listener, the pending timers and the icon object URLs. */
  destroy(): void {
    this.#destroyed = true;
    this.#markStorageKnown();
    this.#generation++;
    clearTimeout(this.#debounce);
    this.#debounce = undefined;
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    this.#frame = undefined;
    this.#unlisten?.();
    this.#unlisten = undefined;
    this.#unlistenStorage?.();
    this.#unlistenStorage = undefined;
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
    // An error from an attempt of an earlier session would show before any new attempt.
    this.resetFailed = false;
    return this.#loadFirstPage(false);
  }

  /** Runs the pending search now and resolves once its first page (or its failure) has landed. */
  async settled(): Promise<void> {
    if (this.#debounce !== undefined) {
      clearTimeout(this.#debounce);
      this.#debounce = undefined;
      void this.#loadFirstPage(false);
    }
    // A reload that starts meanwhile supersedes the one awaited: wait until the promise stops changing.
    let current: Promise<void>;
    do {
      current = this.#firstPage;
      await current;
    } while (current !== this.#firstPage);
  }

  /** Selects the row at `index` when it is loaded. */
  select(index: number): void {
    if (index >= 0 && index < this.items.length) this.selectedIndex = index;
  }

  /** One row up; the first row stays selected. */
  selectPrevious(): void {
    this.select(this.selectedIndex - 1);
  }

  /** One row down; past the last loaded row, the next page is loaded first. */
  async selectNext(): Promise<void> {
    if (this.selectedIndex === this.items.length - 1) await this.loadMore();
    this.select(this.selectedIndex + 1);
  }

  /** Deletes a clip; the selection stays at the same index, on whatever took the clip's place. */
  async deleteClip(id: string): Promise<void> {
    try {
      await deleteClips([id]);
    } catch {
      return; // The rows stay as they are.
    }
    await this.#loadFirstPage(true);
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

  /** Asks the core to start over with an empty history; it announces the result with `storage://changed`. */
  async resetHistory(): Promise<void> {
    if (this.resetting) return;
    this.resetting = true;
    this.resetFailed = false;
    try {
      await resetHistoryDatabase();
    } catch {
      // Still unavailable, nothing moved: the error state says so and the user can retry.
      // A refusal or a failure announces nothing, so read the status: the message and the button
      // are decided from one state. A refusal that lands after the history became ready (another
      // window won) says nothing.
      const status = await getStorageStatus().catch((): undefined => undefined);
      if (status !== undefined && !this.#destroyed) this.#setStorage(status);
      if (this.storageUnavailable && this.storageCanReset) this.resetFailed = true;
    } finally {
      this.resetting = false;
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

  /**
   * With `keepSelection`, the selection follows its clip (when the clip is gone it stays at its index) and the
   * reload brings back as many rows as were loaded, so a list scrolled past the first page keeps its length.
   */
  #loadFirstPage(keepSelection: boolean): Promise<void> {
    this.#firstPage = this.#fetchFirstPage(keepSelection);
    return this.#firstPage;
  }

  #setStorage(status: StorageStatus): void {
    this.storage = status;
    if (!this.storageUnavailable) this.resetFailed = false;
  }

  /** The core announced that the database became usable or unusable. */
  #applyStorage(status: StorageStatus): void {
    if (this.#destroyed) return;
    this.#storageAnnounced = true;
    this.#setStorage(status);
    this.#markStorageKnown();
    if (this.storageUnavailable) {
      // Drop any answer on its way and what is shown: it belongs to a database that is gone.
      this.#generation++;
      this.items = [];
      this.total = 0;
      this.nextCursor = null;
      this.loaded = false;
      this.loading = false;
      this.selectedIndex = 0;
    } else {
      // The reset also announces `clips://changed`: one reload per animation frame for both.
      this.#scheduleRefresh();
    }
  }

  async #fetchFirstPage(keepSelection: boolean): Promise<void> {
    if (this.storageUnavailable) return;
    const generation = ++this.#generation;
    this.loading = true;
    const asked = this.query;
    const wanted = keepSelection ? this.items.length : 0;
    try {
      let page = await listClips(this.#queryFor(undefined, this.#limitFor(wanted)));
      let items = page.items;
      while (items.length < wanted && page.nextCursor !== null) {
        if (generation !== this.#generation) return;
        page = await listClips(this.#queryFor(page.nextCursor, this.#limitFor(wanted - items.length)));
        items = [...items, ...page.items];
      }
      if (generation !== this.#generation) return;
      this.loadedQuery = asked;
      const selectedId = keepSelection ? this.items[this.selectedIndex]?.id : undefined;
      this.items = items;
      this.total = page.total;
      this.nextCursor = page.nextCursor;
      const found = items.findIndex((clip) => clip.id === selectedId);
      const stay = keepSelection ? this.selectedIndex : 0;
      this.selectedIndex = found >= 0 ? found : Math.min(stay, Math.max(0, items.length - 1));
      this.loaded = true;
    } catch {
      // The rows already shown stay.
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  /** The page size that brings back `rows` rows, or none to leave it to the core's default. */
  #limitFor(rows: number): number | undefined {
    return rows > DEFAULT_PAGE_SIZE ? Math.min(rows, MAX_PAGE_SIZE) : undefined;
  }

  #queryFor(cursor?: string, limit?: number): ClipQuery {
    const query: ClipQuery = {};
    if (this.query) query.search = this.query;
    if (cursor !== undefined) query.cursor = cursor;
    if (limit !== undefined) query.limit = limit;
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
