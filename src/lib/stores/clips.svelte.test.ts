import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ClipQuery } from "../bindings/ClipQuery";
import type { ClipSummary } from "../bindings/ClipSummary";
import type { Page } from "../bindings/Page";
import { clip, deferred, fakeCore, page } from "../../test/clips";
import { ClipsStore } from "./clips.svelte";

/** A source of clips answering `list_clips` page by page: the cursor is the index of the next clip. */
function pagedBy(source: ClipSummary[]) {
  return (q: ClipQuery) => {
    const from = Number(q.cursor ?? 0);
    const to = from + (q.limit ?? 50);
    return page(source.slice(from, to), to < source.length ? String(to) : null, source.length);
  };
}

const flush = () => vi.advanceTimersByTimeAsync(0);

let store: ClipsStore;

beforeEach(() => {
  vi.useFakeTimers();
  let n = 0;
  URL.createObjectURL = vi.fn(() => `blob:icon-${n++}`);
  URL.revokeObjectURL = vi.fn();
  store = new ClipsStore();
});

afterEach(() => {
  store.destroy();
  vi.useRealTimers();
});

describe("ClipsStore loading", () => {
  it("loads the first page on start, without a search or a cursor", async () => {
    const core = fakeCore(() => page([clip(1), clip(2), clip(3)], null, 3));

    await store.start();

    expect(core.queries).toEqual([{}]);
    expect(store.items.map((c) => c.id)).toEqual(["clip-1", "clip-2", "clip-3"]);
    expect(store.total).toBe(3);
    expect(store.nextCursor).toBeNull();
    expect(store.loaded).toBe(true);
  });

  it("is not loaded before the first page arrives (no empty-state flash)", async () => {
    fakeCore(() => page([]));

    const started = store.start();
    expect(store.loaded).toBe(false);
    await started;

    expect(store.loaded).toBe(true);
  });

  it("appends the next page with the cursor", async () => {
    const core = fakeCore((q) =>
      q.cursor === "c1" ? page([clip(3), clip(4)], null, 4) : page([clip(1), clip(2)], "c1", 4),
    );
    await store.start();

    await store.loadMore();

    expect(core.queries).toEqual([{}, { cursor: "c1" }]);
    expect(store.items.map((c) => c.id)).toEqual(["clip-1", "clip-2", "clip-3", "clip-4"]);
    expect(store.nextCursor).toBeNull();
  });

  it("carries the search into the next page", async () => {
    const core = fakeCore((q) => (q.cursor ? page([clip(9)], null, 2) : page([clip(8)], "c1", 2)));
    await store.start();
    store.setSearch("sel");
    await vi.advanceTimersByTimeAsync(50);

    await store.loadMore();

    expect(core.queries.at(-1)).toEqual({ search: "sel", cursor: "c1" });
  });

  it("does nothing on loadMore without a next cursor", async () => {
    const core = fakeCore(() => page([clip(1)]));
    await store.start();

    await store.loadMore();

    expect(core.queries).toHaveLength(1);
  });

  it("asks for the next page only once while it is loading", async () => {
    const core = fakeCore((q) => (q.cursor ? page([clip(3)], null, 3) : page([clip(1), clip(2)], "c1", 3)));
    await store.start();

    await Promise.all([store.loadMore(), store.loadMore(), store.loadMore()]);

    expect(core.queries.filter((q) => q.cursor)).toHaveLength(1);
    expect(store.items).toHaveLength(3);
  });

  it("keeps the rows it has when the core fails", async () => {
    let fail = false;
    fakeCore(() => {
      if (fail) throw { code: "storage_unavailable", message: "The history database is unavailable." };
      return page([clip(1)]);
    });
    await store.start();

    fail = true;
    await store.refresh();

    expect(store.items.map((c) => c.id)).toEqual(["clip-1"]);
    expect(store.loading).toBe(false);
  });
});

describe("ClipsStore search", () => {
  it("queries once, 50 ms after the last key", async () => {
    const core = fakeCore(() => page([clip(1)]));
    await store.start();
    core.queries.length = 0;

    store.setSearch("s");
    await vi.advanceTimersByTimeAsync(30);
    store.setSearch("se");
    await vi.advanceTimersByTimeAsync(30);
    store.setSearch("sel");
    await vi.advanceTimersByTimeAsync(49);
    expect(core.queries).toEqual([]);

    await vi.advanceTimersByTimeAsync(1);

    expect(core.queries).toEqual([{ search: "sel" }]);
  });

  it("shows what is typed at once, before the query runs", async () => {
    fakeCore(() => page([]));
    await store.start();

    store.setSearch("  sel ");

    expect(store.search).toBe("  sel ");
    expect(store.query).toBe("sel");
  });

  it("goes back to the first row when the search changes", async () => {
    fakeCore((q) => (q.search ? page([clip(7), clip(8)]) : page([clip(1), clip(7), clip(8)])));
    await store.start();
    store.selectedIndex = 2;

    store.setSearch("clip");
    await vi.advanceTimersByTimeAsync(50);

    expect(store.selectedIndex).toBe(0);
  });

  it("drops the answer of a search that was overtaken by a newer one", async () => {
    const slow = deferred<Page<ClipSummary>>();
    fakeCore((q) => (q.search === "a" ? slow.promise : page([clip(2)])));
    await store.start();

    store.setSearch("a");
    await vi.advanceTimersByTimeAsync(50);
    store.setSearch("ab");
    await vi.advanceTimersByTimeAsync(50);
    slow.resolve(page([clip(1)]));
    await flush();

    expect(store.items.map((c) => c.id)).toEqual(["clip-2"]);
  });

  it("does not run a pending search once destroyed", async () => {
    const core = fakeCore(() => page([]));
    await store.start();
    core.queries.length = 0;

    store.setSearch("sel");
    store.destroy();
    await vi.advanceTimersByTimeAsync(100);

    expect(core.queries).toEqual([]);
  });
});

describe("ClipsStore live updates", () => {
  it("reloads the first page when the history changes", async () => {
    let items = [clip(1)];
    fakeCore(() => page(items));
    await store.start();

    items = [clip(2), clip(1)];
    await emit("clips://changed", {});
    await vi.advanceTimersByTimeAsync(20);

    expect(store.items.map((c) => c.id)).toEqual(["clip-2", "clip-1"]);
  });

  it("coalesces a burst of events into one reload per frame", async () => {
    const core = fakeCore(() => page([clip(1)]));
    await store.start();
    core.queries.length = 0;

    for (let i = 0; i < 5; i++) await emit("clips://changed", {});
    await vi.advanceTimersByTimeAsync(20);

    expect(core.queries).toHaveLength(1);
  });

  it("keeps the same clip selected when a new one lands above it", async () => {
    let items = [clip(1), clip(2)];
    fakeCore(() => page(items));
    await store.start();
    store.selectedIndex = 1;

    items = [clip(3), clip(1), clip(2)];
    await store.refresh();

    expect(store.selectedIndex).toBe(2);
    expect(store.items[store.selectedIndex]?.id).toBe("clip-2");
  });

  it("falls back to the first row when the selected clip is gone", async () => {
    let items = [clip(1), clip(2)];
    fakeCore(() => page(items));
    await store.start();
    store.selectedIndex = 1;

    items = [clip(1)];
    await store.refresh();

    expect(store.selectedIndex).toBe(0);
  });

  it("stops listening once destroyed", async () => {
    const core = fakeCore(() => page([clip(1)]));
    await store.start();
    core.queries.length = 0;

    store.destroy();
    await flush();
    await emit("clips://changed", {});
    await vi.advanceTimersByTimeAsync(20);

    expect(core.queries).toEqual([]);
  });

  it("does not leave a listener behind when destroyed before start finished", async () => {
    const core = fakeCore(() => page([clip(1)]));

    const started = store.start();
    store.destroy();
    await started;
    core.queries.length = 0;
    await emit("clips://changed", {});
    await vi.advanceTimersByTimeAsync(20);

    expect(core.queries).toEqual([]);
  });
});

describe("ClipsStore reset", () => {
  it("empties the search, selects the first row and reloads", async () => {
    const core = fakeCore((q) => (q.search ? page([clip(2)]) : page([clip(1), clip(2)])));
    await store.start();
    store.setSearch("clip");
    await vi.advanceTimersByTimeAsync(50);
    store.selectedIndex = 0;
    core.queries.length = 0;

    await store.reset();

    expect(store.search).toBe("");
    expect(store.selectedIndex).toBe(0);
    expect(core.queries).toEqual([{}]);
    expect(store.items.map((c) => c.id)).toEqual(["clip-1", "clip-2"]);
  });

  it("cancels a search that was still waiting for its debounce", async () => {
    const core = fakeCore(() => page([clip(1)]));
    await store.start();
    core.queries.length = 0;

    store.setSearch("sel");
    await store.reset();
    await vi.advanceTimersByTimeAsync(100);

    expect(core.queries).toEqual([{}]);
  });
});

describe("ClipsStore app icons", () => {
  it("fetches an icon once per bundle id and keeps its object URL", async () => {
    const core = fakeCore(() => page([]));

    store.requestIcon("com.example.Editor");
    store.requestIcon("com.example.Editor");
    await flush();

    expect(core.commands.filter((c) => c === "get_app_icon")).toHaveLength(1);
    expect(store.iconUrls.get("com.example.Editor")).toBe("blob:icon-0");
    expect(URL.createObjectURL).toHaveBeenCalledTimes(1);
  });

  it("has no URL for a bundle id without an icon, and does not ask again", async () => {
    const core = fakeCore(() => page([]));

    store.requestIcon("com.example.NoIcon");
    await flush();
    store.requestIcon("com.example.NoIcon");
    await flush();

    expect(store.iconUrls.has("com.example.NoIcon")).toBe(false);
    expect(core.commands.filter((c) => c === "get_app_icon")).toHaveLength(1);
  });

  it("revokes every object URL when destroyed", async () => {
    fakeCore(() => page([]));
    store.requestIcon("com.example.Editor");
    store.requestIcon("com.example.Browser");
    await flush();

    store.destroy();

    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(2);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:icon-0");
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:icon-1");
  });

  it("makes no object URL for an icon that arrives after destroy", async () => {
    fakeCore(() => page([]));

    store.requestIcon("com.example.Editor");
    store.destroy();
    await flush();

    expect(store.iconUrls.size).toBe(0);
    expect(URL.createObjectURL).not.toHaveBeenCalled();
  });

  it("ignores icon requests once destroyed", async () => {
    const core = fakeCore(() => page([]));

    store.destroy();
    store.requestIcon("com.example.Editor");
    await flush();

    expect(core.commands).not.toContain("get_app_icon");
  });
});

const THREE = () => page([clip(1), clip(2), clip(3)]);

describe("ClipsStore selection", () => {
  it("selects the first clip once loaded", async () => {
    fakeCore(() => page([clip(1), clip(2)]));

    await store.start();

    expect(store.selected?.id).toBe("clip-1");
  });

  it("has no selected clip in an empty history", async () => {
    fakeCore(() => page([]));

    await store.start();

    expect(store.selected).toBeUndefined();
  });

  it("moves down and up one row at a time", async () => {
    fakeCore(() => page([clip(1), clip(2), clip(3)]));
    await store.start();

    await store.selectNext();
    await store.selectNext();
    expect(store.selected?.id).toBe("clip-3");

    store.selectPrevious();
    expect(store.selected?.id).toBe("clip-2");
  });

  it("stays on the first row going up and on the last row going down (no wrap)", async () => {
    fakeCore(() => page([clip(1), clip(2)]));
    await store.start();

    store.selectPrevious();
    expect(store.selectedIndex).toBe(0);

    await store.selectNext();
    await store.selectNext();
    expect(store.selectedIndex).toBe(1);
  });

  it("loads the next page at the last loaded row and moves on to its first clip", async () => {
    const core = fakeCore((q) => (q.cursor ? page([clip(3), clip(4)], null, 4) : page([clip(1), clip(2)], "2", 4)));
    await store.start();
    await store.selectNext();

    await store.selectNext();

    expect(core.queries).toEqual([{}, { cursor: "2" }]);
    expect(store.selected?.id).toBe("clip-3");
  });

  it("selects the row that was clicked", async () => {
    fakeCore(() => page([clip(1), clip(2), clip(3)]));
    await store.start();

    store.select(2);

    expect(store.selected?.id).toBe("clip-3");
  });

  it("ignores a row that is not loaded", async () => {
    fakeCore(() => page([clip(1), clip(2)]));
    await store.start();

    store.select(5);
    store.select(-1);

    expect(store.selectedIndex).toBe(0);
  });
});

describe("ClipsStore deleteClip", () => {
  it("deletes through delete_clips and reloads the list", async () => {
    let items = [clip(1), clip(2), clip(3)];
    const core = fakeCore(() => page(items));
    await store.start();
    items = [clip(1), clip(3)];

    await store.deleteClip("clip-2");

    expect(core.deleted).toEqual([["clip-2"]]);
    expect(store.items.map((c) => c.id)).toEqual(["clip-1", "clip-3"]);
    expect(store.total).toBe(2);
  });

  it("keeps the selection at the same index, on the clip that took its place", async () => {
    let items = [clip(1), clip(2), clip(3)];
    fakeCore(() => page(items));
    await store.start();
    store.select(1);
    items = [clip(1), clip(3)];

    await store.deleteClip("clip-2");

    expect(store.selectedIndex).toBe(1);
    expect(store.selected?.id).toBe("clip-3");
  });

  it("selects the new last row when the last one is deleted", async () => {
    let items = [clip(1), clip(2), clip(3)];
    fakeCore(() => page(items));
    await store.start();
    store.select(2);
    items = [clip(1), clip(2)];

    await store.deleteClip("clip-3");

    expect(store.selected?.id).toBe("clip-2");
  });

  it("keeps the selection at the same index when a slow reload is superseded by clips://changed", async () => {
    let items = [clip(1), clip(2), clip(3)];
    fakeCore(() => new Promise((resolve) => setTimeout(() => resolve(page(items)), 30)));
    const started = store.start();
    await vi.advanceTimersByTimeAsync(30);
    await started;
    store.select(1);
    items = [clip(1), clip(3)];

    const deleting = store.deleteClip("clip-2");
    await emit("clips://changed", { removed: ["clip-2"] });
    await vi.advanceTimersByTimeAsync(100);
    await deleting;

    expect(store.selected?.id).toBe("clip-3");
  });

  it("keeps the loaded pages, and the selection's index, when a row past the first page is deleted", async () => {
    const source = Array.from({ length: 100 }, (_, i) => clip(i + 1));
    fakeCore(pagedBy(source));
    await store.start();
    await store.loadMore();
    store.select(60);
    expect(store.selected?.id).toBe("clip-61");
    source.splice(60, 1);

    await store.deleteClip("clip-61");

    expect(store.items).toHaveLength(99);
    expect(store.selectedIndex).toBe(60);
    expect(store.selected?.id).toBe("clip-62");
    expect(store.total).toBe(99);
  });

  it("reloads more than one maximal page when more rows than that were loaded", async () => {
    const source = Array.from({ length: 300 }, (_, i) => clip(i + 1));
    const core = fakeCore(pagedBy(source));
    await store.start();
    while (store.nextCursor !== null) await store.loadMore();
    store.select(250);
    source.splice(250, 1);
    core.queries.length = 0;

    await store.deleteClip("clip-251");

    expect(store.items).toHaveLength(299);
    expect(store.selected?.id).toBe("clip-252");
    expect(core.queries.map((q) => q.limit)).toEqual([200, 99]);
    expect(core.queries.map((q) => q.cursor)).toEqual([undefined, "200"]);
  });

  it("keeps the rows when the core refuses to delete", async () => {
    fakeCore(() => page([clip(1), clip(2)]));
    await store.start();
    mockIPC(
      (cmd) => {
        if (cmd === "delete_clips") throw { code: "storage_error", message: "Storage failed." };
        return page([clip(1), clip(2)]);
      },
      { shouldMockEvents: true },
    );

    await expect(store.deleteClip("clip-1")).resolves.toBeUndefined();

    expect(store.items.map((c) => c.id)).toEqual(["clip-1", "clip-2"]);
  });
});

describe("ClipsStore settled", () => {
  it("is not stale, and returns at once, when the list answers the search", async () => {
    const core = fakeCore(THREE);
    await store.start();
    core.queries.length = 0;

    await store.settled();

    expect(store.stale).toBe(false);
    expect(core.queries).toEqual([]);
  });

  it("flushes the pending search and waits for its first page", async () => {
    fakeCore((q) => (q.search ? page([clip(3)]) : THREE()));
    await store.start();
    store.setSearch("3");
    expect(store.stale).toBe(true);

    await store.settled();

    expect(store.stale).toBe(false);
    expect(store.selected?.id).toBe("clip-3");
  });

  it("waits for a search whose request is already out", async () => {
    const slow = deferred<Page<ClipSummary>>();
    fakeCore((q) => (q.search ? slow.promise : THREE()));
    await store.start();
    store.setSearch("3");
    await vi.advanceTimersByTimeAsync(50);

    let done = false;
    void store.settled().then(() => (done = true));
    await flush();
    expect(done).toBe(false);

    slow.resolve(page([clip(3)]));
    await flush();

    expect(done).toBe(true);
    expect(store.selected?.id).toBe("clip-3");
  });

  it("keeps waiting when a clips://changed reload supersedes the search it waits for", async () => {
    fakeCore((q) => {
      const answer = q.search ? page([clip(3)]) : THREE();
      return new Promise((resolve) => setTimeout(() => resolve(answer), 30));
    });
    const started = store.start();
    await vi.advanceTimersByTimeAsync(30);
    await started;
    store.setSearch("3");

    let staleWhenDone: boolean | undefined;
    void store.settled().then(() => (staleWhenDone = store.stale));
    await emit("clips://changed", {});
    await vi.advanceTimersByTimeAsync(200);

    expect(staleWhenDone).toBe(false);
    expect(store.selected?.id).toBe("clip-3");
  });

  it("returns when the search fails, still stale, instead of waiting forever", async () => {
    fakeCore((q) => {
      if (q.search) throw { code: "storage_error", message: "Storage failed." };
      return THREE();
    });
    await store.start();
    store.setSearch("3");

    await store.settled();

    expect(store.stale).toBe(true);
  });
});
