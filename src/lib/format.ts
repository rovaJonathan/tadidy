const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** A clip's age as the rows show it: "Just now", "2m ago", "3h ago", "Yesterday", "4d ago". */
export function relativeTime(ms: number, now: number): string {
  const age = Math.max(0, now - ms);
  if (age < MINUTE) return "Just now";
  if (age < HOUR) return `${Math.floor(age / MINUTE)}m ago`;
  if (age < DAY) return `${Math.floor(age / HOUR)}h ago`;
  if (age < 2 * DAY) return "Yesterday";
  return `${Math.floor(age / DAY)}d ago`;
}

/** The footer's count: "1,000 clips", or "3 results" while a search is active. */
export function clipCountLabel(total: number, searching = false): string {
  const noun = searching ? "result" : "clip";
  return `${total.toLocaleString("en-US")} ${noun}${total === 1 ? "" : "s"}`;
}
