import { expect, test } from "@playwright/test";
import { mockIpc } from "./support/mock-ipc";

test("the dev harness shows the version from app_version", async ({ page }) => {
  await mockIpc(page, { app_version: "1.2.3" });

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Tadidy" })).toBeVisible();
  await expect(page.getByText("Version 1.2.3")).toBeVisible();
});
