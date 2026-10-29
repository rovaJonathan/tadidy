import { expect, test } from "@playwright/test";
import { syntheticClip, syntheticClips } from "./support/fixtures";
import { listClipsQueries, mockIpc, recordClip } from "./support/mock-ipc";

const rows = (page: import("@playwright/test").Page) => page.locator("#clip-list .clip-row");

test("the browser lists the newest clips first with the count", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(3) });

  await page.goto("/");

  await expect(rows(page)).toHaveCount(3);
  await expect(rows(page).first()).toContainText("Synthetic clip 1");
  await expect(rows(page).first().locator(".clip-badge")).toHaveText("⌘1");
  await expect(rows(page).first().locator(".clip-time")).toHaveText("1m ago");
  await expect(page.locator("#clip-count")).toHaveText("3 clips");
  await expect(page.locator("#search-input")).toBeFocused();
});

test("typing in the search narrows the list to the matching clips", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(30) });
  await page.goto("/");
  await expect(rows(page).first()).toBeVisible();

  await page.locator("#search-input").pressSequentially("clip 2");

  await expect(rows(page)).toHaveCount(11);
  await expect(page.locator("#clip-count")).toHaveText("11 results");
  const queries = await listClipsQueries(page);
  expect(queries.filter((q) => q.search !== undefined).at(-1)).toEqual({ search: "clip 2" });
});

test("a search with no result shows #empty-search quoting it", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(5) });
  await page.goto("/");

  await page.locator("#search-input").fill("xyz");

  await expect(page.locator("#empty-search")).toBeVisible();
  await expect(page.locator("#empty-search .empty-title")).toHaveText("No clips match “xyz”");
  await expect(rows(page)).toHaveCount(0);

  await page.locator("#btn-clear-search").click();

  await expect(page.locator("#empty-search")).toHaveCount(0);
  await expect(rows(page)).toHaveCount(5);
});

test("an empty history shows #empty-history", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: [] });

  await page.goto("/");

  await expect(page.locator("#empty-history")).toBeVisible();
  await expect(page.locator("#empty-history .empty-title")).toHaveText("Nothing copied yet.");
  await expect(page.locator("#clip-count")).toHaveText("0 clips");
});

test("1 000 clips keep about 30 rows in the DOM and scrolling loads the next page", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(1000) });
  await page.goto("/");
  await expect(rows(page).first()).toBeVisible();
  await expect(page.locator("#clip-count")).toHaveText("1,000 clips");
  expect(await rows(page).count()).toBeLessThanOrEqual(30);

  await page.locator("#clip-list").evaluate((list) => (list.scrollTop = list.scrollHeight));

  await expect.poll(async () => (await listClipsQueries(page)).map((q) => q.cursor)).toContain("50");
  expect(await rows(page).count()).toBeLessThanOrEqual(30);
  await expect(page.locator("#clip-list .clip-row", { hasText: "Synthetic clip 50" })).toBeVisible();
});

test("a recorded clip appears at the top without any action", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(2) });
  await page.goto("/");
  await expect(rows(page)).toHaveCount(2);

  await recordClip(page, syntheticClip(0, { preview: "Fresh synthetic clip", lastCopiedAt: Date.now() }));

  await expect(rows(page)).toHaveCount(3);
  await expect(rows(page).first()).toContainText("Fresh synthetic clip");
  await expect(rows(page).first().locator(".clip-time")).toHaveText("Just now");
});
