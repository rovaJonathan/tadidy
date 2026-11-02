import { expect, test, type Page } from "@playwright/test";
import { syntheticClips } from "./support/fixtures";
import { emitEvent, ipcCalls, mockIpc } from "./support/mock-ipc";

const UNAVAILABLE = { state: "unavailable", code: "storage_key_missing" };
const READY = { state: "ready", code: null };

/** The clip browser of `url` when the core could not open the history database. */
async function openUnavailable(page: Page, url: string) {
  await mockIpc(
    page,
    {
      get_storage_status: UNAVAILABLE,
      reset_history_database: null,
      hide_window: null,
      open_preferences: null,
      check_accessibility: true,
    },
    { clips: syntheticClips(3) },
  );
  await page.goto(url);
  await expect(page.locator("#storage-unavailable")).toBeVisible();
}

for (const url of ["/popover.html", "/"]) {
  test.describe(`history database unavailable at ${url}`, () => {
    test("shows the error state in place of the list and the empty state, and never lists", async ({ page }) => {
      await openUnavailable(page, url);

      await expect(page.locator(".empty-title")).toHaveText("Your history can’t be opened.");
      await expect(page.locator(".empty-body")).toHaveText(
        "Tadidy can’t unlock its encrypted database, so nothing is being recorded.",
      );
      await expect(page.locator("#btn-reset-history")).toHaveText("Reset history…");
      await expect(page.locator("#empty-history")).toHaveCount(0);
      await expect(page.locator("#clip-list .clip-row")).toHaveCount(0);
      await expect(page.locator("#clip-count")).toHaveText("");
      expect(await ipcCalls(page)).not.toContain("list_clips");
    });

    test("Reset history… asks first, and Cancel resets nothing", async ({ page }) => {
      await openUnavailable(page, url);

      await page.locator("#btn-reset-history").click();

      await expect(page.locator("#dialog-reset-title")).toHaveText("Reset clipboard history?");
      await expect(page.locator("#dialog-reset-desc")).toContainText("can’t be recovered");
      await expect(page.locator("#btn-reset-cancel")).toBeFocused();
      await page.locator("#btn-reset-cancel").click();
      await expect(page.locator("#dialog-reset-history")).toHaveCount(0);
      await expect(page.locator("#storage-unavailable")).toBeVisible();
      expect(await ipcCalls(page)).not.toContain("reset_history_database");
    });

    test("Esc cancels the confirmation and resets nothing", async ({ page }) => {
      await openUnavailable(page, url);
      await page.locator("#btn-reset-history").click();

      await page.keyboard.press("Escape");

      await expect(page.locator("#dialog-reset-history")).toHaveCount(0);
      expect(await ipcCalls(page)).not.toContain("reset_history_database");
    });

    test("confirming resets, then the list comes back when the core announces the new database", async ({ page }) => {
      await openUnavailable(page, url);
      await page.locator("#btn-reset-history").click();

      await page.locator("#btn-reset-confirm").click();

      await expect.poll(() => ipcCalls(page)).toContain("reset_history_database");
      await expect(page.locator("#dialog-reset-history")).toHaveCount(0);
      await emitEvent(page, "storage://changed", READY);
      await expect(page.locator("#storage-unavailable")).toHaveCount(0);
      await expect(page.locator("#clip-list .clip-row")).toHaveCount(3);
      await expect(page.locator("#clip-count")).toHaveText("3 clips");
    });
  });
}
