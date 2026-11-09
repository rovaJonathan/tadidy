// Synthetic clips and a fake core for the front tests (AGENTS.md §2: never real clip content).
import { mockIPC } from "@tauri-apps/api/mocks";
import type { ClipQuery } from "../lib/bindings/ClipQuery";
import type { ClipSummary } from "../lib/bindings/ClipSummary";
import type { Page } from "../lib/bindings/Page";
import type { StorageStatus } from "../lib/bindings/StorageStatus";

export const READY: StorageStatus = { state: "ready", code: null, canReset: false };
/** A failure that may be temporary (Keychain denied or locked): the history must not be reset. */
export const UNAVAILABLE_TEMPORARY: StorageStatus = { state: "unavailable", code: "storage_error", canReset: false };
export const UNAVAILABLE: StorageStatus = { state: "unavailable", code: "storage_key_missing", canReset: true };

export function clip(n: number, over: Partial<ClipSummary> = {}): ClipSummary {
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

export function page(items: ClipSummary[], nextCursor: string | null = null, total = items.length): Page<ClipSummary> {
  return { items, nextCursor, total };
}

/**
 * A fake core: records every `list_clips` query, every command and the ids of every `delete_clips`, answers `get_storage_status`
 * with `options.storage` (ready by default), `reset_history_database` with `options.resetError` when set,
 * `retry_history_database` with `options.retryError` when set and `list_clips` with
 * whatever `answer` returns and `get_app_icon` with a PNG signature, except for a bundle id
 * ending in `.NoIcon`, which has no cached icon.
 */
export function fakeCore(
  answer: (q: ClipQuery) => Page<ClipSummary> | Promise<Page<ClipSummary>>,
  options: {
    storage?: StorageStatus;
    resetError?: { code: string; message: string };
    retryError?: { code: string; message: string };
  } = {},
) {
  let storage = options.storage ?? READY;
  const queries: ClipQuery[] = [];
  const commands: string[] = [];
  const deleted: string[][] = [];
  mockIPC(
    (cmd, args) => {
      commands.push(cmd);
      if (cmd === "delete_clips") deleted.push((args as { ids: string[] }).ids);
      if (cmd === "list_clips") {
        const q = (args as { q: ClipQuery }).q;
        queries.push(q);
        return answer(q);
      }
      if (cmd === "get_storage_status") return storage;
      if (cmd === "reset_history_database" && options.resetError) throw options.resetError;
      if (cmd === "retry_history_database" && options.retryError) throw options.retryError;
      if (cmd === "get_app_icon") {
        const { bundleId } = args as { bundleId: string };
        if (bundleId.endsWith(".NoIcon")) throw { code: "not_found", message: "No icon is cached for this app." };
        return new Uint8Array([137, 80, 78, 71]).buffer;
      }
      return undefined;
    },
    { shouldMockEvents: true },
  );
  return {
    queries,
    commands,
    deleted,
    /** What `get_storage_status` answers from now on (the core announces it with `storage://changed`). */
    setStorage(status: StorageStatus) {
      storage = status;
    },
  };
}

/** A promise settled by hand, to hold a `list_clips` answer back. */
export function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
