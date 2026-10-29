import { emit } from "@tauri-apps/api/event";
import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clip, deferred, fakeCore, page } from "../../test/clips";
import { ROW_HEIGHT } from "../virtual-list";
import ClipBrowser from "./ClipBrowser.svelte";

const NOW = Date.UTC(2026, 9, 29, 12, 0, 0);
const MINUTE = 60_000;

/** Lets the pending timers, IPC answers and DOM updates run. */
async function settle(ms = 0) {
  await vi.advanceTimersByTimeAsync(ms);
  await tick();
}

function rows(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(".clip-row")];
}

function previews(container: HTMLElement): (string | undefined)[] {
  return rows(container).map((row) => row.querySelector(".clip-text")?.textContent?.trim());
}

function find<T extends HTMLElement>(container: HTMLElement, selector: string): T {
  const element = container.querySelector<T>(selector);
  if (!element) throw new Error(`${selector} is missing`);
  return element;
}

function input(container: HTMLElement): HTMLInputElement {
  return find<HTMLInputElement>(container, "#search-input");
}

/** jsdom has no layout: give the list the height a real popover would. */
function stubListHeight(height: number) {
  vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(height);
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
  let n = 0;
  URL.createObjectURL = vi.fn(() => `blob:icon-${n++}`);
  URL.revokeObjectURL = vi.fn();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("ClipBrowser rows", () => {
  it("lists the clips newest first with badge, kind icon, preview, app icon and time", async () => {
    fakeCore(() =>
      page(
        [
          clip(1, { preview: "First synthetic text", sourceBundle: "com.example.Editor", lastCopiedAt: NOW }),
          clip(2, { kind: "url", preview: "https://example.test/page", lastCopiedAt: NOW - 2 * MINUTE }),
          clip(3, { kind: "image", preview: null, lastCopiedAt: NOW - 180 * MINUTE }),
        ],
        null,
        3,
      ),
    );

    const { container } = render(ClipBrowser);
    await settle();

    expect(rows(container)).toHaveLength(3);
    expect(previews(container)).toEqual(["First synthetic text", "https://example.test/page", "Image"]);
    expect(rows(container).map((row) => row.dataset.kind)).toEqual(["text", "url", "image"]);
    expect(rows(container).map((row) => row.querySelector(".clip-badge")?.textContent)).toEqual(["⌘1", "⌘2", "⌘3"]);
    expect(rows(container).map((row) => row.querySelector(".clip-time")?.textContent)).toEqual([
      "Just now",
      "2m ago",
      "3h ago",
    ]);
    for (const row of rows(container)) expect(row.querySelector(".clip-tile svg")).not.toBeNull();
    expect(rows(container)[0]?.querySelector("img.app-icon")?.getAttribute("src")).toBe("blob:icon-0");
    expect(container.querySelector("#clip-count")?.textContent).toBe("3 clips");
  });

  it("shows a badge on the first nine rows only", async () => {
    fakeCore(() =>
      page(
        Array.from({ length: 12 }, (_, i) => clip(i + 1)),
        null,
        12,
      ),
    );
    stubListHeight(48 * 12);

    const { container } = render(ClipBrowser);
    await settle();

    const badges = rows(container).map((row) => row.querySelector(".clip-badge")?.textContent);
    expect(badges.slice(0, 9)).toEqual(["⌘1", "⌘2", "⌘3", "⌘4", "⌘5", "⌘6", "⌘7", "⌘8", "⌘9"]);
    expect(badges.slice(9)).toEqual(["", "", ""]);
  });

  it("marks a pinned clip", async () => {
    fakeCore(() => page([clip(1, { pinned: true }), clip(2)], null, 2));

    const { container } = render(ClipBrowser);
    await settle();

    expect(rows(container)[0]?.querySelector(".pin-marker")).not.toBeNull();
    expect(rows(container)[1]?.querySelector(".pin-marker")).toBeNull();
  });

  it("refreshes the relative times every minute", async () => {
    fakeCore(() => page([clip(1, { lastCopiedAt: NOW })]));

    const { container } = render(ClipBrowser);
    await settle();
    expect(container.querySelector(".clip-time")?.textContent).toBe("Just now");

    await settle(MINUTE);

    expect(container.querySelector(".clip-time")?.textContent).toBe("1m ago");
  });

  it("exposes a listbox of options with the selected one marked", async () => {
    fakeCore(() => page([clip(1), clip(2)], null, 2));

    const { container, getByRole, getAllByRole } = render(ClipBrowser);
    await settle();

    expect(getByRole("listbox", { name: "Clips" })).toBe(container.querySelector("#clip-list"));
    const options = getAllByRole("option");
    expect(options.map((option) => option.getAttribute("aria-selected"))).toEqual(["true", "false"]);
    expect(options.map((option) => option.id)).toEqual(["clip-clip-1", "clip-clip-2"]);
    expect(input(container).getAttribute("aria-activedescendant")).toBe("clip-clip-1");
  });
});

describe("ClipBrowser app icons", () => {
  it("shows the generic icon for a bundle id without an icon, and asks once per bundle id", async () => {
    const core = fakeCore(() =>
      page(
        [
          clip(1, { sourceBundle: "com.example.NoIcon" }),
          clip(2, { sourceBundle: "com.example.NoIcon" }),
          clip(3, { sourceBundle: "com.example.Editor" }),
          clip(4),
        ],
        null,
        4,
      ),
    );

    const { container } = render(ClipBrowser);
    await settle();

    expect(core.commands.filter((cmd) => cmd === "get_app_icon")).toHaveLength(2);
    const [first, second, third, fourth] = rows(container);
    expect(first?.querySelector(".app-icon-generic")).not.toBeNull();
    expect(second?.querySelector(".app-icon-generic")).not.toBeNull();
    expect(third?.querySelector("img.app-icon")).not.toBeNull();
    expect(fourth?.querySelector(".app-icon")).toBeNull();
  });

  it("revokes the icon object URLs when it is unmounted", async () => {
    fakeCore(() => page([clip(1, { sourceBundle: "com.example.Editor" })]));
    const { unmount } = render(ClipBrowser);
    await settle();

    unmount();

    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:icon-0");
  });
});

describe("ClipBrowser search", () => {
  it("has the prototype's field: placeholder, shortcut hint, focus on mount", async () => {
    fakeCore(() => page([]));

    const { container } = render(ClipBrowser);
    await settle();

    expect(input(container).placeholder).toBe("Search clips…");
    expect(input(container)).toHaveFocus();
    expect(container.querySelector("#search-shortcut-hint")?.textContent).toBe("⌘⇧V");
    expect(container.querySelector("#btn-clear-search")).toHaveAttribute("hidden");
  });

  it("queries once with the typed text, 50 ms after the last key", async () => {
    const core = fakeCore(() => page([clip(1)]));
    const { container } = render(ClipBrowser);
    await settle();
    core.queries.length = 0;

    for (const text of ["s", "se", "sel"]) {
      await fireEvent.input(input(container), { target: { value: text } });
      await settle(20);
    }
    expect(core.queries).toEqual([]);
    await settle(30);

    expect(core.queries).toEqual([{ search: "sel" }]);
  });

  it("swaps the shortcut hint for a clear button while typing, and clears on click", async () => {
    const core = fakeCore(() => page([clip(1)]));
    const { container } = render(ClipBrowser);
    await settle();

    await fireEvent.input(input(container), { target: { value: "sel" } });
    expect(container.querySelector("#search-shortcut-hint")).toHaveAttribute("hidden");
    expect(container.querySelector("#btn-clear-search")).not.toHaveAttribute("hidden");

    core.queries.length = 0;
    await fireEvent.click(find(container, "#btn-clear-search"));
    await settle(50);

    expect(input(container).value).toBe("");
    expect(core.queries).toEqual([{}]);
    expect(input(container)).toHaveFocus();
  });

  it("counts results instead of clips while a search is active", async () => {
    fakeCore((q) => (q.search ? page([clip(1), clip(2)], null, 2) : page([clip(1), clip(2), clip(3)], null, 487)));
    const { container } = render(ClipBrowser);
    await settle();
    expect(container.querySelector("#clip-count")?.textContent).toBe("487 clips");

    await fireEvent.input(input(container), { target: { value: "clip" } });
    await settle(50);

    expect(container.querySelector("#clip-count")?.textContent).toBe("2 results");
  });
});

describe("ClipBrowser empty states", () => {
  it("shows #empty-history when there is no clip at all", async () => {
    fakeCore(() => page([]));

    const { container } = render(ClipBrowser);
    await settle();

    const empty = container.querySelector("#empty-history");
    expect(empty?.querySelector(".empty-title")?.textContent).toBe("Nothing copied yet.");
    expect(empty?.querySelector(".empty-body")?.textContent).toBe("Press ⌘C anywhere — Tadidy remembers.");
    expect(empty).toHaveAttribute("role", "status");
    expect(container.querySelector("#empty-search")).toBeNull();
    expect(container.querySelector("#clip-count")?.textContent).toBe("0 clips");
  });

  it("shows #empty-search quoting the search when nothing matches", async () => {
    fakeCore((q) => (q.search ? page([]) : page([clip(1)])));
    const { container } = render(ClipBrowser);
    await settle();

    await fireEvent.input(input(container), { target: { value: " xyz " } });
    await settle(50);

    const empty = container.querySelector("#empty-search");
    expect(empty?.querySelector(".empty-title")?.textContent).toBe("No clips match “xyz”");
    expect(empty?.querySelector("#empty-search-query")?.textContent).toBe("xyz");
    expect(empty?.querySelector(".empty-body")?.textContent).toBe("Check the spelling, or try another tab or filter.");
    expect(container.querySelector("#empty-history")).toBeNull();
    expect(container.querySelector("#clip-count")?.textContent).toBe("0 results");
  });

  it("shows no empty state until the first page has arrived", async () => {
    const slow = deferred<ReturnType<typeof page>>();
    fakeCore(() => slow.promise);

    const { container } = render(ClipBrowser);
    await settle();
    expect(container.querySelector("#empty-history")).toBeNull();
    expect(container.querySelector("#empty-search")).toBeNull();

    slow.resolve(page([]));
    await settle();

    expect(container.querySelector("#empty-history")).not.toBeNull();
  });

  it("keeps the previous rows, with no empty state, while a search is loading", async () => {
    const slow = deferred<ReturnType<typeof page>>();
    fakeCore((q) => (q.search ? slow.promise : page([clip(1)])));
    const { container } = render(ClipBrowser);
    await settle();

    await fireEvent.input(input(container), { target: { value: "x" } });
    await settle(50);

    expect(rows(container)).toHaveLength(1);
    expect(container.querySelector("#empty-search")).toBeNull();
    slow.resolve(page([]));
  });
});

describe("ClipBrowser long lists", () => {
  it("keeps about 30 rows in the DOM for 1 000 clips and loads the next page at the end", async () => {
    const first = Array.from({ length: 50 }, (_, i) => clip(i + 1));
    const second = Array.from({ length: 50 }, (_, i) => clip(i + 51));
    const core = fakeCore((q) => (q.cursor ? page(second, "c2", 1000) : page(first, "c1", 1000)));
    stubListHeight(400);

    const { container } = render(ClipBrowser);
    await settle();

    expect(rows(container).length).toBeGreaterThan(0);
    expect(rows(container).length).toBeLessThanOrEqual(30);
    expect(core.queries).toEqual([{}]);
    expect(container.querySelector("#clip-count")?.textContent).toBe("1,000 clips");

    const list = find(container, "#clip-list");
    list.scrollTop = 41 * ROW_HEIGHT;
    await fireEvent.scroll(list);
    await settle();

    expect(core.queries).toEqual([{}, { cursor: "c1" }]);
    expect(rows(container).length).toBeLessThanOrEqual(30);
    expect(previews(container)).toContain("Synthetic clip 45");
  });

  it("goes back to the top when the search changes", async () => {
    const items = Array.from({ length: 50 }, (_, i) => clip(i + 1));
    fakeCore((q) => (q.search ? page([clip(7)], null, 1) : page(items, null, 50)));
    stubListHeight(400);
    const { container } = render(ClipBrowser);
    await settle();
    const list = find(container, "#clip-list");
    list.scrollTop = 30 * ROW_HEIGHT;
    await fireEvent.scroll(list);

    await fireEvent.input(input(container), { target: { value: "7" } });
    await settle(50);

    expect(list.scrollTop).toBe(0);
    expect(rows(container).map((row) => row.dataset.index)).toEqual(["0"]);
  });

  it("positions each row by its index", async () => {
    fakeCore(() => page([clip(1), clip(2), clip(3)]));

    const { container } = render(ClipBrowser);
    await settle();

    expect(rows(container).map((row) => row.style.top)).toEqual(["0px", `${ROW_HEIGHT}px`, `${2 * ROW_HEIGHT}px`]);
    expect(container.querySelector<HTMLElement>("#clip-list > *")?.style.height).toBe(`${3 * ROW_HEIGHT}px`);
  });
});

describe("ClipBrowser live updates", () => {
  it("shows a new clip at the top when the history changes", async () => {
    let items = [clip(1)];
    fakeCore(() => page(items));
    const { container } = render(ClipBrowser);
    await settle(20);

    items = [clip(2, { preview: "Fresh synthetic clip" }), clip(1)];
    await emit("clips://changed", {});
    await settle(32);

    expect(previews(container)).toEqual(["Fresh synthetic clip", "Synthetic clip 1"]);
    expect(container.querySelector("#clip-count")?.textContent).toBe("2 clips");
  });
});
