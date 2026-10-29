import { describe, expect, it } from "vitest";
import { visibleRange } from "./virtual-list";

describe("visibleRange", () => {
  it("covers the viewport from the top, plus the overscan below", () => {
    // 200 px / 50 px = rows 0..3 are in view; 2 more rows of overscan below.
    expect(visibleRange(0, 200, 50, 1000, 2)).toEqual({ start: 0, end: 6 });
  });

  it("starts above the first partly visible row, minus the overscan", () => {
    // scrollTop 125 → the first row in view is row 2 (100..150).
    expect(visibleRange(125, 200, 50, 1000, 2)).toEqual({ start: 0, end: 9 });
    expect(visibleRange(1000, 200, 50, 1000, 2)).toEqual({ start: 18, end: 26 });
  });

  it("stops at the last row", () => {
    expect(visibleRange(900, 200, 50, 20, 3)).toEqual({ start: 15, end: 20 });
  });

  it("is empty when there is no row", () => {
    expect(visibleRange(0, 200, 50, 0, 3)).toEqual({ start: 0, end: 0 });
  });

  it("keeps the range inside the list when the scroll position is past the end", () => {
    expect(visibleRange(5000, 200, 50, 10, 2)).toEqual({ start: 4, end: 10 });
  });

  it("treats a negative scroll position (rubber-band) as the top", () => {
    expect(visibleRange(-40, 200, 50, 1000, 0)).toEqual({ start: 0, end: 4 });
  });

  it("renders only the overscan before the viewport is measured", () => {
    expect(visibleRange(0, 0, 50, 3, 5)).toEqual({ start: 0, end: 3 });
  });
});
