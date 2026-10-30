import { expect, test, type Page } from "@playwright/test";
import { syntheticClips } from "./support/fixtures";
import { emitEvent, ipcArgs, ipcCalls, mockIpc } from "./support/mock-ipc";

const WINDOWS = [
  { label: "popover", page: "/popover.html", section: "#popover" },
  { label: "panel", page: "/panel.html", section: "#quick-panel" },
] as const;

const SETTINGS = {
  historySize: 500,
  confirmBeforeDelete: true,
  launchAtLogin: false,
  quickPanelShortcut: "Cmd+Shift+V",
};

async function open(page: Page, url: string, responses: Record<string, unknown> = {}) {
  await mockIpc(
    page,
    { hide_window: null, open_preferences: null, check_accessibility: true, paste_clip: "pasted", ...responses },
    {
      clips: syntheticClips(30),
      preferences: { settings: { ...SETTINGS }, shortcut: { accelerator: "Cmd+Shift+V", error: null } },
    },
  );
  await page.goto(url);
  await expect(page.locator("#clip-list .clip-row").first()).toBeVisible();
  await expect(page.locator("#search-input")).toBeFocused();
}

for (const { label, page: url, section } of WINDOWS) {
  test.describe(label, () => {
    test("renders the shared browser", async ({ page }) => {
      await open(page, url);

      await expect(page.locator(section)).toBeVisible();
      for (const id of ["#search-input", "#clip-list", "#clip-count", "#btn-preferences"]) {
        await expect(page.locator(`${section} ${id}`)).toBeVisible();
      }
      await expect(page.locator("#clip-count")).toHaveText("30 clips");
    });

    test("fills its window: the list scrolls and the footer stays in view", async ({ page }) => {
      await open(page, url);

      await expect(page.locator("#btn-preferences")).toBeInViewport({ ratio: 1 });
      await expect(page.locator("#clip-count")).toBeInViewport({ ratio: 1 });
    });

    test("search, ↓ and Return paste the second match in the default mode", async ({ page }) => {
      await open(page, url);

      await page.keyboard.type("clip 1");
      await expect(page.locator("#clip-list .clip-row")).toHaveCount(11);
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Enter");

      await expect.poll(() => ipcArgs(page, "paste_clip")).toEqual([{ id: "clip-10", mode: "default" }]);
    });

    test("Esc hides the window", async ({ page }) => {
      await open(page, url);

      await page.keyboard.press("Escape");

      await expect.poll(async () => (await ipcCalls(page)).filter((cmd) => cmd === "hide_window")).toHaveLength(1);
    });

    test("⚙ and ⌘, open the preferences", async ({ page }) => {
      await open(page, url);

      await page.locator("#btn-preferences").click();
      await page.keyboard.press("Meta+,");

      await expect.poll(async () => (await ipcCalls(page)).filter((cmd) => cmd === "open_preferences")).toHaveLength(2);
    });

    test("a paste that only copied keeps the window open and shows the banner", async ({ page }) => {
      await open(page, url, { paste_clip: "copied_only", check_accessibility: false });
      await expect(page.locator("#banner-accessibility")).toBeVisible();

      await page.keyboard.press("Enter");

      await expect.poll(() => ipcArgs(page, "paste_clip")).toHaveLength(1);
      expect(await ipcCalls(page)).not.toContain("hide_window");
      await expect(page.locator("#banner-accessibility")).toBeVisible();
    });

    test("showing the window again starts afresh, for its own label only", async ({ page }) => {
      await open(page, url);
      await page.keyboard.type("clip 2");
      await expect(page.locator("#clip-list .clip-row")).toHaveCount(11);
      await page.keyboard.press("ArrowDown");

      await emitEvent(page, "panel://shown", { window: label === "panel" ? "popover" : "panel" });
      await expect(page.locator("#search-input")).toHaveValue("clip 2");

      await emitEvent(page, "panel://shown", { window: label });
      await expect(page.locator("#search-input")).toHaveValue("");
      await expect(page.locator("#clip-list .clip-row[aria-selected='true']")).toContainText("Synthetic clip 1");
      await expect(page.locator("#search-input")).toBeFocused();
    });
  });
}
