import { defineConfig, devices } from "@playwright/test";

// End-to-end flows run against the Vite dev server with the IPC mocked (AGENTS.md §14).
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: true,
  reporter: "list",
  use: {
    baseURL: "http://localhost:1420",
    trace: "retain-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    // The app runs in WKWebView: what only WebKit lays out differently is checked there too.
    { name: "webkit", use: { ...devices["Desktop Safari"] }, testMatch: /preferences\.spec\.ts/ },
  ],
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:1420",
    reuseExistingServer: true,
  },
});
