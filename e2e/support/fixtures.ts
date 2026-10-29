// Synthetic clips for the end-to-end tests (AGENTS.md §2: never real clip content).
import type { ClipSummary } from "../../src/lib/bindings/ClipSummary";

const NOW = Date.now();
const MINUTE = 60_000;

/** `count` clips, newest first: "Synthetic clip 1" is the newest, one minute apart. */
export function syntheticClips(count: number): ClipSummary[] {
  return Array.from({ length: count }, (_, i) => syntheticClip(i + 1));
}

export function syntheticClip(n: number, over: Partial<ClipSummary> = {}): ClipSummary {
  return {
    id: `clip-${n}`,
    kind: "text",
    preview: `Synthetic clip ${n}`,
    sourceBundle: null,
    sourceName: null,
    pinned: false,
    copyCount: 1,
    lastCopiedAt: NOW - n * MINUTE,
    ...over,
  };
}
