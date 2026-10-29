/** Height of one clip row in px: `--preview-lines` × 16 px of text + 16 px of padding (DESIGN.md §4.5). */
export const ROW_HEIGHT = 48;

/** Rows rendered above and below the viewport, so a fast scroll never shows a blank gap. */
export const OVERSCAN = 5;

/** The rows to put in the DOM: `start` is the first, `end` is one past the last. */
export interface RowRange {
  start: number;
  end: number;
}

/**
 * Which rows of a fixed-height list to render for the current scroll position: the ones in the
 * viewport plus `overscan` on each side, kept inside `[0, count]`.
 */
export function visibleRange(
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  count: number,
  overscan: number,
): RowRange {
  const maxScroll = Math.max(0, count * rowHeight - viewportHeight);
  const top = Math.min(Math.max(0, scrollTop), maxScroll);
  const first = Math.floor(top / rowHeight);
  const last = Math.ceil((top + viewportHeight) / rowHeight);
  return {
    start: Math.max(0, first - overscan),
    end: Math.min(count, last + overscan),
  };
}
