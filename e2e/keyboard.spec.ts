import { expect, test, type Page } from "@playwright/test";
import { syntheticClips } from "./support/fixtures";
import { browserCalls, ipcCalls, mockIpc } from "./support/mock-ipc";

const rows = (page: Page) => page.locator("#clip-list .clip-row");
const selected = (page: Page) => page.locator("#clip-list .clip-row[aria-selected='true']");

async function openBrowser(page: Page, count = 30) {
  await mockIpc(page, { app_version: "1.2.3" }, { clips: syntheticClips(count) });
  await page.goto("/");
  await expect(rows(page).first()).toBeVisible();
  await expect(page.locator("#search-input")).toBeFocused();
}

test("typing, ↓ and Return paste the second match", async ({ page }) => {
  await openBrowser(page);

  await page.keyboard.type("clip 1");
  await expect(rows(page)).toHaveCount(11);
  await page.keyboard.press("ArrowDown");
  await expect(selected(page)).toContainText("Synthetic clip 10");
  await page.keyboard.press("Enter");

  expect(await browserCalls(page)).toEqual(["paste:clip-10"]);
});

test("⌥Return and ⌘C copy the selected clip, a double-click pastes", async ({ page }) => {
  await openBrowser(page);

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Alt+Enter");
  await page.keyboard.press("Meta+c");
  await rows(page).nth(3).dblclick();

  expect(await browserCalls(page)).toEqual(["copy:clip-2", "copy:clip-2", "paste:clip-4"]);
  await expect(page.locator("#search-input")).toBeFocused();
});

test("↑ stays on the first row and the selection follows the keys down the list", async ({ page }) => {
  await openBrowser(page, 100);

  await page.keyboard.press("ArrowUp");
  await expect(selected(page)).toContainText("Synthetic clip 1");
  for (let i = 0; i < 20; i++) await page.keyboard.press("ArrowDown");

  await expect(selected(page)).toContainText("Synthetic clip 21");
  await expect(selected(page)).toBeInViewport({ ratio: 1 });
});

test("⌘⌫ asks first: Esc cancels, Return deletes", async ({ page }) => {
  await openBrowser(page, 5);
  await page.keyboard.press("ArrowDown");

  await page.keyboard.press("Meta+Backspace");
  const dialog = page.locator("#dialog-delete-clip");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("#dialog-delete-title")).toHaveText("Delete this clip?");
  await expect(page.locator("#btn-delete-confirm")).toBeFocused();
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(rows(page)).toHaveCount(5);
  await expect(page.locator("#search-input")).toBeFocused();
  expect(await browserCalls(page)).toEqual([]);

  await page.keyboard.press("Meta+Backspace");
  await page.keyboard.press("Enter");

  await expect(dialog).toHaveCount(0);
  await expect(rows(page)).toHaveCount(4);
  await expect(page.locator("#clip-list")).not.toContainText("Synthetic clip 2");
  await expect(selected(page)).toContainText("Synthetic clip 3");
  expect(await ipcCalls(page)).toContain("delete_clips");
});

test("Esc clears the search, then closes", async ({ page }) => {
  await openBrowser(page);
  await page.keyboard.type("clip 2");
  await expect(rows(page)).toHaveCount(11);

  await page.keyboard.press("Escape");

  await expect(page.locator("#search-input")).toHaveValue("");
  await expect(rows(page).first()).toContainText("Synthetic clip 1");
  expect(await browserCalls(page)).toEqual([]);

  await page.keyboard.press("Escape");

  expect(await browserCalls(page)).toEqual(["close"]);
});

test("a letter typed away from the search lands in it", async ({ page }) => {
  await openBrowser(page);
  await page.locator("#search-input").blur();

  await page.keyboard.press("a");

  await expect(page.locator("#search-input")).toBeFocused();
  await expect(page.locator("#search-input")).toHaveValue("a");
});

test("⌘, opens the preferences", async ({ page }) => {
  await openBrowser(page);

  await page.keyboard.press("Meta+,");

  expect(await browserCalls(page)).toEqual(["open-preferences"]);
});
