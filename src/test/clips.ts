// Synthetic clips and a fake core for the front tests (AGENTS.md §2: never real clip content).
import { mockIPC } from "@tauri-apps/api/mocks";
import type { ClipQuery } from "../lib/bindings/ClipQuery";
import type { ClipSummary } from "../lib/bindings/ClipSummary";
import type { Page } from "../lib/bindings/Page";

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
 * A fake core: records every `list_clips` query and every command, answers `list_clips` with
 * whatever `answer` returns and `get_app_icon` with a PNG signature, except for a bundle id
 * ending in `.NoIcon`, which has no cached icon.
 */
export function fakeCore(answer: (q: ClipQuery) => Page<ClipSummary> | Promise<Page<ClipSummary>>) {
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
