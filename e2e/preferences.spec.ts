import { expect, test, type Page } from "@playwright/test";
import { mockIpc, settingsPatches, type FakePreferences } from "./support/mock-ipc";

function preferences(over: Partial<FakePreferences> = {}): FakePreferences {
  return {
    settings: { historySize: 500, confirmBeforeDelete: true, launchAtLogin: false, quickPanelShortcut: "Cmd+Shift+V" },
    shortcut: { accelerator: "Cmd+Shift+V", error: null },
    ...over,
  };
}

async function open(page: Page, over: Partial<FakePreferences> = {}) {
  await mockIpc(page, {}, { preferences: preferences(over) });
  await page.goto("/settings.html");
  await expect(page.locator("#pref-launch-at-login")).toBeAttached();
}

const CONFLICT = "This shortcut is already used by macOS or another app. Choose another one.";

test("the window has the General and Shortcuts tabs and nothing else", async ({ page }) => {
  await open(page);

  await expect(page.locator('[role="tab"]')).toHaveText(["General", "Shortcuts"]);
  await expect(page.locator(".prefs-panel")).toHaveCount(2);
  await expect(page.locator("#prefs-general")).toBeVisible();
  await expect(page.locator("#prefs-shortcuts")).toBeHidden();
  await expect(page.locator("#prefs-title")).toHaveText("General");
});

test("Launch at login, History size and Confirm before delete send their setting", async ({ page }) => {
  await open(page);

  await page.locator("label[for='pref-launch-at-login']").click();
  await page.locator("#pref-history-size").selectOption("10");
  await page.locator("#pref-history-size").selectOption({ label: "Unlimited" });
  await page.locator("#pref-confirm-delete").evaluate((box: HTMLInputElement) => box.click());

  await expect
    .poll(() => settingsPatches(page))
    .toEqual([{ launchAtLogin: true }, { historySize: 10 }, { historySize: 0 }, { confirmBeforeDelete: false }]);
});

test("a login item the system refuses unchecks the box and shows the error", async ({ page }) => {
  await open(page, { refuse: { code: "login_item_failed", message: "Launch at login could not be changed." } });

  await page.locator("#pref-launch-at-login").evaluate((box: HTMLInputElement) => box.click());

  await expect(page.locator("#pref-launch-at-login-error")).toBeVisible();
  await expect(page.locator("#pref-launch-at-login-error")).toHaveText("Launch at login could not be changed.");
  await expect(page.locator("#pref-launch-at-login")).not.toBeChecked();
});

test("↓ and ↑ move between the tabs", async ({ page }) => {
  await open(page);

  await page.locator("#prefs-tab-general").focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.locator("#prefs-title")).toHaveText("Shortcuts");
  await expect(page.locator("#prefs-shortcuts")).toBeVisible();
  await expect(page.locator("#prefs-tab-shortcuts")).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(page.locator("#prefs-title")).toHaveText("General");
});

test("recording ⌃⌥V saves Ctrl+Alt+V; V alone and ⇧ alone are refused", async ({ page }) => {
  await open(page);
  await page.locator("#prefs-tab-shortcuts").click();
  await expect(page.locator("#shortcut-keys")).toHaveText("⌘⇧V");

  await page.locator("#shortcut-recorder").click();
  await expect(page.locator("#shortcut-keys")).toHaveText("Type shortcut…");
  await page.keyboard.press("KeyV");
  await page.keyboard.press("Shift");
  await expect(page.locator("#shortcut-keys")).toHaveText("Type shortcut…");
  await page.keyboard.press("Control+Alt+KeyV");

  await expect(page.locator("#shortcut-keys")).toHaveText("⌃⌥V");
  expect(await settingsPatches(page)).toEqual([{ quickPanelShortcut: "Ctrl+Alt+V" }]);
  await expect(page.locator("#shortcut-error")).toBeHidden();
});

test("a shortcut macOS keeps shows the conflict error and is not sent", async ({ page }) => {
  await open(page);
  await page.locator("#prefs-tab-shortcuts").click();

  await page.locator("#shortcut-recorder").click();
  await page.keyboard.press("Meta+Shift+Digit4");

  await expect(page.locator("#shortcut-error")).toBeVisible();
  await expect(page.locator("#shortcut-error")).toHaveText(CONFLICT);
  await expect(page.locator("#shortcut-recorder")).toHaveClass(/has-error/);
  expect(await settingsPatches(page)).toEqual([]);
});

test("a shortcut the core refuses shows the conflict error", async ({ page }) => {
  await open(page, { refuse: { code: "shortcut_unavailable", message: "taken" } });
  await page.locator("#prefs-tab-shortcuts").click();

  await page.locator("#shortcut-recorder").click();
  await page.keyboard.press("Control+Alt+KeyV");

  await expect(page.locator("#shortcut-error")).toBeVisible();
  await expect(page.locator("#shortcut-error")).toHaveText(CONFLICT);
  expect(await settingsPatches(page)).toEqual([{ quickPanelShortcut: "Ctrl+Alt+V" }]);
});

test("a shortcut that failed at startup shows the conflict error when the tab opens", async ({ page }) => {
  await open(page, { shortcut: { accelerator: "Cmd+Shift+V", error: "unavailable" } });

  await page.locator("#prefs-tab-shortcuts").click();

  await expect(page.locator("#shortcut-error")).toBeVisible();
  await expect(page.locator("#shortcut-error")).toHaveText(CONFLICT);
  await expect(page.locator("#shortcut-keys")).toHaveText("⌘⇧V");
});

test("Escape cancels a recording and Restore default brings ⌘⇧V back", async ({ page }) => {
  await open(page, {
    settings: { historySize: 500, confirmBeforeDelete: true, launchAtLogin: false, quickPanelShortcut: "Ctrl+Alt+V" },
    shortcut: { accelerator: "Ctrl+Alt+V", error: null },
  });
  await page.locator("#prefs-tab-shortcuts").click();
  await expect(page.locator("#shortcut-keys")).toHaveText("⌃⌥V");

  await page.locator("#shortcut-recorder").click();
  await page.keyboard.press("Escape");
  await expect(page.locator("#shortcut-keys")).toHaveText("⌃⌥V");
  await page.locator("#btn-shortcut-reset").click();

  await expect(page.locator("#shortcut-keys")).toHaveText("⌘⇧V");
  expect(await settingsPatches(page)).toEqual([{ quickPanelShortcut: "Cmd+Shift+V" }]);
});
