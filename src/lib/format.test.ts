import { describe, expect, it } from "vitest";
import { clipCountLabel, relativeTime } from "./format";

const NOW = Date.UTC(2026, 9, 29, 12, 0, 0);
const MINUTE = 60_000;

describe("relativeTime", () => {
  it("says 'Just now' under a minute", () => {
    expect(relativeTime(NOW, NOW)).toBe("Just now");
    expect(relativeTime(NOW - 59_999, NOW)).toBe("Just now");
  });

  it("counts minutes under an hour", () => {
    expect(relativeTime(NOW - MINUTE, NOW)).toBe("1m ago");
    expect(relativeTime(NOW - 59 * MINUTE, NOW)).toBe("59m ago");
  });

  it("counts hours under a day", () => {
    expect(relativeTime(NOW - 60 * MINUTE, NOW)).toBe("1h ago");
    expect(relativeTime(NOW - 1439 * MINUTE, NOW)).toBe("23h ago");
  });

  it("says 'Yesterday' for the following 24 hours", () => {
    expect(relativeTime(NOW - 1440 * MINUTE, NOW)).toBe("Yesterday");
    expect(relativeTime(NOW - 2879 * MINUTE, NOW)).toBe("Yesterday");
  });

  it("counts days from two days on", () => {
    expect(relativeTime(NOW - 2880 * MINUTE, NOW)).toBe("2d ago");
    expect(relativeTime(NOW - 10 * 1440 * MINUTE, NOW)).toBe("10d ago");
  });

  it("treats a clip dated after 'now' (clock skew) as just now", () => {
    expect(relativeTime(NOW + 5 * MINUTE, NOW)).toBe("Just now");
  });
});

describe("clipCountLabel", () => {
  it("counts clips with a thousands separator", () => {
    expect(clipCountLabel(487)).toBe("487 clips");
    expect(clipCountLabel(1000)).toBe("1,000 clips");
    expect(clipCountLabel(0)).toBe("0 clips");
  });

  it("uses the singular for one clip", () => {
    expect(clipCountLabel(1)).toBe("1 clip");
  });

  it("counts results while a search is active", () => {
    expect(clipCountLabel(3, true)).toBe("3 results");
    expect(clipCountLabel(1, true)).toBe("1 result");
    expect(clipCountLabel(0, true)).toBe("0 results");
  });
});
