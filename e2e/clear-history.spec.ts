import { expect, test, type Page } from "@playwright/test";
import { syntheticClip, syntheticClips } from "./support/fixtures";
import { emitEvent, ipcArgs, ipcCalls, mockIpc } from "./support/mock-ipc";

const SETTINGS = {
  historySize: 500,
  confirmBeforeDelete: true,
  launchAtLogin: false,
  quickPanelShortcut: "Cmd+Shift+V",
};

/** The popover with five clips, the second one pinned, and the tray menu's Clear history… chosen. */
async function askToClear(page: Page) {
  await mockIpc(
    page,
    { hide_window: null, open_preferences: null, check_accessibility: true },
    {
      clips: [...syntheticClips(5).slice(0, 1), syntheticClip(2, { pinned: true }), ...syntheticClips(5).slice(2)],
      preferences: { settings: { ...SETTINGS }, shortcut: { accelerator: "Cmd+Shift+V", error: null } },
    },
  );
  await page.goto("/popover.html");
  await expect(page.locator("#clip-list .clip-row")).toHaveCount(5);
  await emitEvent(page, "menu://clear-history", null);
  await expect(page.locator("#dialog-clear-history")).toBeVisible();
}

test("Clear history… opens the confirmation with the size of the history and Keep pinned checked", async ({ page }) => {
  await askToClear(page);

  await expect(page.locator("#dialog-clear-title")).toHaveText("Clear clipboard history?");
  await expect(page.locator("#dialog-clear-count")).toHaveText("5 clips");
  await expect(page.locator("#chk-keep-pinned")).toBeChecked();
  await expect(page.locator("#btn-clear-cancel")).toBeFocused();
  expect(await ipcCalls(page)).not.toContain("clear_history");
});

test("confirming with Keep pinned checked clears every clip but the pinned one", async ({ page }) => {
  await askToClear(page);

  await page.locator("#btn-clear-confirm").click();

  await expect.poll(() => ipcArgs(page, "clear_history")).toEqual([{ keepPinned: true }]);
  await expect(page.locator("#dialog-clear-history")).toHaveCount(0);
  await expect(page.locator("#clip-list .clip-row")).toHaveCount(1);
  await expect(page.locator("#clip-count")).toHaveText("1 clip");
  await expect(page.locator("#search-input")).toBeFocused();
});

test("confirming with Keep pinned unchecked empties the history", async ({ page }) => {
  await askToClear(page);

  await page.locator("#chk-keep-pinned").uncheck();
  await page.locator("#btn-clear-confirm").click();

  await expect.poll(() => ipcArgs(page, "clear_history")).toEqual([{ keepPinned: false }]);
  await expect(page.locator("#empty-history")).toBeVisible();
});

test("Cancel and Esc clear nothing and leave the popover open", async ({ page }) => {
  await askToClear(page);

  await page.locator("#btn-clear-cancel").click();
  await expect(page.locator("#dialog-clear-history")).toHaveCount(0);

  await emitEvent(page, "menu://clear-history", null);
  await expect(page.locator("#dialog-clear-history")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("#dialog-clear-history")).toHaveCount(0);

  const calls = await ipcCalls(page);
  expect(calls).not.toContain("clear_history");
  expect(calls).not.toContain("hide_window");
  await expect(page.locator("#clip-list .clip-row")).toHaveCount(5);
  await expect(page.locator("#popover")).toBeVisible();
});
