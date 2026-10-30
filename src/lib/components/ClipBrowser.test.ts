import { emit } from "@tauri-apps/api/event";
import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";
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

  it("points aria-activedescendant at a row that is in the DOM, or at none", async () => {
    fakeCore(() =>
      page(
        Array.from({ length: 50 }, (_, i) => clip(i + 1)),
        null,
        50,
      ),
    );
    stubListHeight(400);
    const { container } = render(ClipBrowser);
    await settle();
    const list = find(container, "#clip-list");

    list.scrollTop = 30 * ROW_HEIGHT;
    await fireEvent.scroll(list);
    expect(container.querySelector("#clip-clip-1")).toBeNull();
    expect(input(container)).not.toHaveAttribute("aria-activedescendant");

    list.scrollTop = 0;
    await fireEvent.scroll(list);
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

  it("keeps #empty-search and its count until the cleared search has reloaded", async () => {
    const held = deferred<ReturnType<typeof page>>();
    let holding = false;
    fakeCore((q) => (q.search ? page([]) : holding ? held.promise : page([clip(1), clip(2)], null, 2)));
    const { container } = render(ClipBrowser);
    await settle();
    await fireEvent.input(input(container), { target: { value: "zzz" } });
    await settle(50);
    expect(container.querySelector("#empty-search")).not.toBeNull();

    holding = true;
    await fireEvent.input(input(container), { target: { value: "" } });
    await settle(50);

    expect(container.querySelector("#empty-history")).toBeNull();
    expect(container.querySelector("#empty-search #empty-search-query")?.textContent).toBe("zzz");
    expect(container.querySelector("#clip-count")?.textContent).toBe("0 results");

    held.resolve(page([clip(1), clip(2)], null, 2));
    await settle();

    expect(container.querySelector("#empty-search")).toBeNull();
    expect(rows(container)).toHaveLength(2);
    expect(container.querySelector("#clip-count")?.textContent).toBe("2 clips");
  });

  it("keeps counting clips, not results, until the first search has loaded", async () => {
    const slow = deferred<ReturnType<typeof page>>();
    fakeCore((q) => (q.search ? slow.promise : page([clip(1)], null, 1000)));
    const { container } = render(ClipBrowser);
    await settle();

    await fireEvent.input(input(container), { target: { value: "c" } });
    await settle(50);

    expect(container.querySelector("#clip-count")?.textContent).toBe("1,000 clips");
    slow.resolve(page([clip(1)], null, 12));
    await settle();
    expect(container.querySelector("#clip-count")?.textContent).toBe("12 results");
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

type Handlers = {
  onpaste: Mock<(id: string) => void>;
  oncopy: Mock<(id: string) => void>;
  onclose: Mock<() => void>;
  onopenpreferences: Mock<() => void>;
  ondisableconfirm: Mock<() => void>;
};

function mountBrowser(props: { confirmBeforeDelete?: boolean } = {}) {
  const handlers: Handlers = {
    onpaste: vi.fn<(id: string) => void>(),
    oncopy: vi.fn<(id: string) => void>(),
    onclose: vi.fn<() => void>(),
    onopenpreferences: vi.fn<() => void>(),
    ondisableconfirm: vi.fn<() => void>(),
  };
  const view = render(ClipBrowser, { ...handlers, ...props });
  return { ...view, ...handlers };
}

function key(target: Element, key: string, modifiers: KeyboardEventInit = {}) {
  return fireEvent.keyDown(target, { key, ...modifiers });
}

function rowAt(container: HTMLElement, index: number): HTMLElement {
  const row = rows(container)[index];
  if (!row) throw new Error(`row ${index} is missing`);
  return row;
}

function selectedIds(container: HTMLElement): (string | undefined)[] {
  return rows(container)
    .filter((row) => row.getAttribute("aria-selected") === "true")
    .map((row) => row.dataset.id);
}

const THREE = () => page([clip(1), clip(2), clip(3)]);

describe("ClipBrowser keyboard", () => {
  it("pastes the second row after typing in the search, ↓ and Return", async () => {
    fakeCore((q) => (q.search === "sel" ? page([clip(7), clip(8), clip(9)]) : THREE()));
    const { container, onpaste } = mountBrowser();
    await settle();
    const field = input(container);

    await fireEvent.input(field, { target: { value: "sel" } });
    await settle(50);
    await key(field, "ArrowDown");
    await key(field, "Enter");

    expect(onpaste).toHaveBeenCalledExactlyOnceWith("clip-8");
  });

  it("moves the selection with ↓ and ↑ and keeps it on the first row going up", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    const field = input(container);

    await key(field, "ArrowDown");
    await settle();
    expect(selectedIds(container)).toEqual(["clip-2"]);
    expect(field.getAttribute("aria-activedescendant")).toBe("clip-clip-2");

    await key(field, "ArrowUp");
    await key(field, "ArrowUp");
    expect(selectedIds(container)).toEqual(["clip-1"]);
  });

  it("does not let ↓ and ↑ move the text cursor of the search", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();

    const notPrevented = await key(input(container), "ArrowDown");

    expect(notPrevented).toBe(false);
  });

  it("loads the next page when ↓ is pressed on the last loaded row", async () => {
    const core = fakeCore((q) => (q.cursor ? page([clip(3), clip(4)], null, 4) : page([clip(1), clip(2)], "2", 4)));
    const { container } = mountBrowser();
    await settle();
    const field = input(container);
    await key(field, "ArrowDown");

    await key(field, "ArrowDown");
    await settle();

    expect(core.queries).toContainEqual({ cursor: "2" });
    expect(selectedIds(container)).toEqual(["clip-3"]);
  });

  it("scrolls the selected row into view, in both directions", async () => {
    fakeCore(() =>
      page(
        Array.from({ length: 10 }, (_, i) => clip(i + 1)),
        null,
        10,
      ),
    );
    stubListHeight(2 * ROW_HEIGHT);
    const { container } = mountBrowser();
    await settle();
    const list = find(container, "#clip-list");
    const field = input(container);

    for (let i = 0; i < 4; i++) await key(field, "ArrowDown");
    await settle();
    expect(list.scrollTop).toBe(3 * ROW_HEIGHT);

    for (let i = 0; i < 4; i++) await key(field, "ArrowUp");
    await settle();
    expect(list.scrollTop).toBe(0);
  });

  it("selects a row on click and keeps the focus in the search", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    const field = input(container);
    field.focus();

    const row = rowAt(container, 2);
    const notPrevented = await fireEvent.mouseDown(row);
    await fireEvent.click(row);

    expect(notPrevented).toBe(false);
    expect(selectedIds(container)).toEqual(["clip-3"]);
    expect(document.activeElement).toBe(field);
  });

  it("pastes a row on double-click", async () => {
    fakeCore(THREE);
    const { container, onpaste } = mountBrowser();
    await settle();

    await fireEvent.dblClick(rowAt(container, 1));

    expect(onpaste).toHaveBeenCalledExactlyOnceWith("clip-2");
  });

  it.each([
    ["⌥Return", "Enter", { altKey: true }],
    ["⌘C", "c", { metaKey: true }],
  ])("%s copies the selected clip", async (_name, name, modifiers) => {
    fakeCore(THREE);
    const { container, oncopy, onpaste } = mountBrowser();
    await settle();
    await key(input(container), "ArrowDown");

    await key(input(container), name, modifiers);

    expect(oncopy).toHaveBeenCalledExactlyOnceWith("clip-2");
    expect(onpaste).not.toHaveBeenCalled();
  });

  it("leaves ⌘C to the search field while part of its text is selected", async () => {
    fakeCore(THREE);
    const { container, oncopy } = mountBrowser();
    await settle();
    const field = input(container);
    field.focus();
    field.value = "hello";
    field.setSelectionRange(0, 3);

    const notPrevented = await key(field, "c", { metaKey: true });

    expect(oncopy).not.toHaveBeenCalled();
    expect(notPrevented).toBe(true);
  });

  it("pastes and copies nothing in an empty history", async () => {
    fakeCore(() => page([]));
    const { container, onpaste, oncopy } = mountBrowser();
    await settle();

    await key(input(container), "Enter");
    await key(input(container), "c", { metaKey: true });

    expect(onpaste).not.toHaveBeenCalled();
    expect(oncopy).not.toHaveBeenCalled();
  });

  it("clears the search on Esc, then closes on the next Esc", async () => {
    fakeCore((q) => (q.search ? page([clip(5)]) : THREE()));
    const { container, onclose } = mountBrowser();
    await settle();
    const field = input(container);
    await fireEvent.input(field, { target: { value: "synthetic" } });
    await settle(50);

    await key(field, "Escape");
    await settle(50);

    expect(field.value).toBe("");
    expect(rows(container)).toHaveLength(3);
    expect(onclose).not.toHaveBeenCalled();

    await key(field, "Escape");

    expect(onclose).toHaveBeenCalledOnce();
  });

  it("gives the focus to the search when a letter is typed with the focus elsewhere", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    input(container).blur();
    expect(document.activeElement).toBe(document.body);

    await key(document.body, "a");

    expect(document.activeElement).toBe(input(container));
  });

  it("selects the text of the search on ⌘F", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    const field = input(container);
    field.value = "abc";
    field.blur();

    const notPrevented = await key(document.body, "f", { metaKey: true });

    expect(notPrevented).toBe(false);
    expect(document.activeElement).toBe(field);
    expect([field.selectionStart, field.selectionEnd]).toEqual([0, 3]);
  });

  it("opens the preferences on ⌘,", async () => {
    fakeCore(THREE);
    const { container, onopenpreferences } = mountBrowser();
    await settle();

    await key(input(container), ",", { metaKey: true });

    expect(onopenpreferences).toHaveBeenCalledOnce();
  });

  it("pastes the match of what was just typed, not the row of the previous list", async () => {
    fakeCore((q) => (q.search ? page([clip(3)]) : THREE()));
    const { container, onpaste } = mountBrowser();
    await settle();
    const field = input(container);

    await fireEvent.input(field, { target: { value: "3" } });
    await key(field, "Enter");
    expect(onpaste).not.toHaveBeenCalled();
    await settle(50);

    expect(onpaste).toHaveBeenCalledExactlyOnceWith("clip-3");
  });

  it("copies the match of what was just typed", async () => {
    fakeCore((q) => (q.search ? page([clip(3)]) : THREE()));
    const { container, oncopy } = mountBrowser();
    await settle();
    const field = input(container);

    await fireEvent.input(field, { target: { value: "3" } });
    await key(field, "Enter", { altKey: true });
    await settle(50);

    expect(oncopy).toHaveBeenCalledExactlyOnceWith("clip-3");
  });

  it("asks to delete the match of what was just typed", async () => {
    const core = fakeCore((q) => (q.search ? page([clip(3)]) : THREE()));
    const { container } = mountBrowser();
    await settle();
    const field = input(container);

    await fireEvent.input(field, { target: { value: "3" } });
    await key(field, "Backspace", { metaKey: true });
    await settle(50);
    await key(find(container, "#dialog-delete-clip"), "Enter");
    await settle();

    expect(core.deleted).toEqual([["clip-3"]]);
  });

  it("does nothing, and does not hang, when the search a Return waits for fails", async () => {
    fakeCore((q) => {
      if (q.search) throw { code: "storage_error", message: "Storage failed." };
      return THREE();
    });
    const { container, onpaste } = mountBrowser();
    await settle();
    const field = input(container);

    await fireEvent.input(field, { target: { value: "3" } });
    await key(field, "Enter");
    await settle(50);

    expect(onpaste).not.toHaveBeenCalled();
  });

  it("ignores the keys of an input method that is composing", async () => {
    fakeCore(THREE);
    const { container, onpaste } = mountBrowser();
    await settle();

    await key(input(container), "Enter", { isComposing: true });

    expect(onpaste).not.toHaveBeenCalled();
  });
});

describe("ClipBrowser deleting", () => {
  it("asks first on ⌘⌫: Return deletes through delete_clips", async () => {
    let items = [clip(1), clip(2), clip(3)];
    const core = fakeCore(() => page(items));
    const { container, ondisableconfirm } = mountBrowser();
    await settle();
    await key(input(container), "ArrowDown");

    await key(input(container), "Backspace", { metaKey: true });

    const dialog = find(container, "#dialog-delete-clip");
    expect(core.deleted).toEqual([]);
    expect(document.activeElement).toBe(find(container, "#btn-delete-confirm"));

    items = [clip(1), clip(3)];
    await key(dialog, "Enter");
    await settle();

    expect(core.deleted).toEqual([["clip-2"]]);
    expect(container.querySelector("#dialog-delete-clip")).toBeNull();
    expect(previews(container)).toEqual(["Synthetic clip 1", "Synthetic clip 3"]);
    expect(selectedIds(container)).toEqual(["clip-3"]);
    expect(document.activeElement).toBe(input(container));
    expect(ondisableconfirm).not.toHaveBeenCalled();
  });

  it("closes only the dialog on Esc: nothing is deleted and the window stays open", async () => {
    const core = fakeCore(THREE);
    const { container, onclose } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });

    await key(find(container, "#dialog-delete-clip"), "Escape");

    expect(container.querySelector("#dialog-delete-clip")).toBeNull();
    expect(core.deleted).toEqual([]);
    expect(onclose).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(input(container));
  });

  it("leaves the other keys to the dialog while it is open", async () => {
    fakeCore(THREE);
    const { container, onpaste } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });

    await key(find(container, "#btn-delete-cancel"), "ArrowDown");
    await key(document.body, "ArrowDown");
    await key(document.body, "c", { metaKey: true });

    expect(selectedIds(container)).toEqual(["clip-1"]);
    expect(onpaste).not.toHaveBeenCalled();
  });

  it("still routes the keys to the dialog when the focus has left it (a click on the dim layer)", async () => {
    const core = fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });
    (document.activeElement as HTMLElement).blur();

    await key(document.body, "Escape");

    expect(container.querySelector("#dialog-delete-clip")).toBeNull();
    expect(core.deleted).toEqual([]);
  });

  it("confirms with Return when the focus has left the dialog", async () => {
    const core = fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });
    (document.activeElement as HTMLElement).blur();

    await key(document.body, "Enter");
    await settle();

    expect(core.deleted).toEqual([["clip-1"]]);
  });

  it("brings Tab back into the dialog when the focus has left it", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });
    (document.activeElement as HTMLElement).blur();

    await key(document.body, "Tab");
    expect(document.activeElement).toBe(find(container, "#chk-dont-ask-delete"));

    (document.activeElement as HTMLElement).blur();
    await key(document.body, "Tab", { shiftKey: true });
    expect(document.activeElement).toBe(find(container, "#btn-delete-cancel"));
  });

  it("deletes and calls ondisableconfirm once when Don’t ask again was ticked", async () => {
    const core = fakeCore(THREE);
    const { container, ondisableconfirm } = mountBrowser();
    await settle();
    await key(input(container), "Backspace", { metaKey: true });

    await fireEvent.click(find(container, "#chk-dont-ask-delete"));
    await fireEvent.click(find(container, "#btn-delete-confirm"));
    await settle();

    expect(core.deleted).toEqual([["clip-1"]]);
    expect(ondisableconfirm).toHaveBeenCalledOnce();
  });

  it("deletes at once, without a dialog, when confirmBeforeDelete is off", async () => {
    const core = fakeCore(THREE);
    const { container } = mountBrowser({ confirmBeforeDelete: false });
    await settle();
    await key(input(container), "ArrowDown");

    await key(input(container), "Backspace", { metaKey: true });
    await settle();

    expect(container.querySelector("#dialog-delete-clip")).toBeNull();
    expect(core.deleted).toEqual([["clip-2"]]);
  });

  it("deletes nothing in an empty history", async () => {
    const core = fakeCore(() => page([]));
    const { container } = mountBrowser();
    await settle();

    await key(input(container), "Backspace", { metaKey: true });

    expect(container.querySelector("#dialog-delete-clip")).toBeNull();
    expect(core.deleted).toEqual([]);
  });
});

describe("ClipBrowser hosting", () => {
  it("reset() empties the search, selects the first row, reloads and focuses the search", async () => {
    const core = fakeCore((q) => (q.search === "sel" ? page([clip(7), clip(8)]) : THREE()));
    const { container, component } = mountBrowser();
    await settle();
    await fireEvent.input(input(container), { target: { value: "sel" } });
    await settle(100);
    await key(input(container), "ArrowDown");
    input(container).blur();
    expect(input(container)).not.toHaveFocus();
    core.queries.length = 0;

    component.reset();
    await settle();

    expect(input(container).value).toBe("");
    expect(core.queries.map((q) => q.search)).toEqual([undefined]);
    expect(previews(container)).toEqual(["Synthetic clip 1", "Synthetic clip 2", "Synthetic clip 3"]);
    expect(selectedIds(container)).toEqual(["clip-1"]);
    expect(input(container)).toHaveFocus();
  });

  it("is the popover layout unless the host says otherwise", async () => {
    fakeCore(THREE);
    const popover = render(ClipBrowser);
    expect(popover.container.querySelector("#clip-browser")).toHaveAttribute("data-layout", "popover");
    popover.unmount();

    const panel = render(ClipBrowser, { layout: "panel" });
    expect(panel.container.querySelector("#clip-browser")).toHaveAttribute("data-layout", "panel");
  });

  it("holds the accessibility banner, in the row between the list and the footer", async () => {
    fakeCore(THREE);
    const { container } = mountBrowser();
    await settle();

    const banner = find(container, "#banner-accessibility");
    expect(banner.previousElementSibling).toBe(find(container, ".list-wrap"));
    expect(banner.nextElementSibling).toBe(find(container, ".browser-footer"));
  });

  it("opens the preferences from the footer button", async () => {
    fakeCore(THREE);
    const { container, onopenpreferences } = mountBrowser();
    await settle();

    await fireEvent.click(find(container, "#btn-preferences"));

    expect(onopenpreferences).toHaveBeenCalledOnce();
  });
});
