import type { Page } from "@playwright/test";

/** What each mocked command returns, by command name. */
export type IpcResponses = Record<string, unknown>;

/**
 * Installs a fake `window.__TAURI_INTERNALS__` before the page loads, the same shape
 * `@tauri-apps/api/mocks` installs in unit tests: `invoke` answers from `responses`
 * and rejects any command that is not mocked.
 */
export async function mockIpc(page: Page, responses: IpcResponses): Promise<void> {
  await page.addInitScript((responses: IpcResponses) => {
    let nextCallbackId = 0;
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: async (cmd: string) => {
          if (cmd in responses) return responses[cmd];
          throw new Error(`IPC command not mocked: ${cmd}`);
        },
        transformCallback: () => nextCallbackId++,
        unregisterCallback: () => undefined,
        metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
      },
    });
  }, responses);
}
