import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ClipQuery } from "../bindings/ClipQuery";
import type { ClipSummary } from "../bindings/ClipSummary";
import type { Page } from "../bindings/Page";
import { ClipsStore } from "./clips.svelte";

function clip(n: number, over: Partial<ClipSummary> = {}): ClipSummary {
  return {
    id: `clip-${n}`,
    kind: "text",
    preview: `Synthetic clip ${n}`,
    sourceBundle: null,
    sourceName: null,
    pinned: false,
    copyCount: 1,
    lastCopiedAt: 1_700_000_000_000 - n,
    ...over,
  };
}

function page(items: ClipSummary[], nextCursor: string | null = null, total = items.length): Page<ClipSummary> {
  return { items, nextCursor, total };
}

/** A fake core: records every `list_clips` query and answers with whatever `answer` returns. */
function fakeCore(answer: (q: ClipQuery) => Page<ClipSummary> | Promise<Page<ClipSummary>>) {
  const queries: ClipQuery[] = [];
  const commands: string[] = [];
  mockIPC(
    (cmd, args) => {
      commands.push(cmd);
      if (cmd === "list_clips") {
        const q = (args as { q: ClipQuery }).q;
        queries.push(q);
        return answer(q);
      }
      if (cmd === "get_app_icon") {
        const { bundleId } = args as { bundleId: string };
        if (bundleId.endsWith(".NoIcon")) throw { code: "not_found", message: "No icon is cached for this app." };
        return new Uint8Array([137, 80, 78, 71]).buffer;
      }
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return { queries, commands };
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
    const slow = Promise.withResolvers<Page<ClipSummary>>();
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
