import { expect, test } from "@playwright/test";
import { ipcCalls, mockIpc } from "./support/mock-ipc";

test("the popover page loads and Esc hides the window", async ({ page }) => {
  await mockIpc(page, { hide_window: null });

  await page.goto("/popover.html");
  await expect(page.locator("#popover")).toBeVisible();
  await page.keyboard.press("Escape");

  await expect.poll(() => ipcCalls(page)).toEqual(["hide_window"]);
});
