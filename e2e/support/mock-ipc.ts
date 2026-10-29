import type { Page } from "@playwright/test";
import type { ClipQuery } from "../../src/lib/bindings/ClipQuery";
import type { ClipSummary } from "../../src/lib/bindings/ClipSummary";

/** What each mocked command returns, by command name. */
export type IpcResponses = Record<string, unknown>;

interface MockWindow {
  __ipcCalls: string[];
  __ipcQueries: ClipQuery[];
  __clips: ClipSummary[];
  __browserCalls?: string[];
  __emit: (event: string, payload: unknown) => void;
}

/**
 * Installs a fake `window.__TAURI_INTERNALS__` before the page loads, the same shape
 * `@tauri-apps/api/mocks` installs in unit tests: `invoke` answers from `responses`
 * and rejects any command that is not mocked. Every invoked command name is recorded for
 * `ipcCalls`. Event listeners (`listen`) work and are fed by `emitEvent`.
 *
 * With `clips`, the page also gets a fake history: `list_clips` filters it by the search text
 * and pages it by cursor like the core does (50 by default), and no app has an icon.
 */
export async function mockIpc(
  page: Page,
  responses: IpcResponses,
  options: { clips?: ClipSummary[] } = {},
): Promise<void> {
  await page.addInitScript(
    ({ responses, clips }: { responses: IpcResponses; clips?: ClipSummary[] }) => {
      let nextCallbackId = 0;
      let nextEventId = 0;
      const callbacks = new Map<number, (event: unknown) => void>();
      const listeners = new Map<string, { eventId: number; handler: number }[]>();
      const calls: string[] = [];
      const queries: ClipQuery[] = [];
      const history = clips ?? [];

      Object.defineProperty(window, "__ipcCalls", { value: calls });
      Object.defineProperty(window, "__ipcQueries", { value: queries });
      Object.defineProperty(window, "__clips", { value: history });
      Object.defineProperty(window, "__emit", {
        value: (event: string, payload: unknown) => {
          for (const { eventId, handler } of listeners.get(event) ?? []) {
            callbacks.get(handler)?.({ event, id: eventId, payload });
          }
        },
      });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => undefined },
      });

      function listClips(q: ClipQuery) {
        const needle = (q.search ?? "").trim().toLowerCase();
        const matching = history.filter((clip) => (clip.preview ?? "").toLowerCase().includes(needle));
        const start = q.cursor ? Number(q.cursor) : 0;
        const end = start + (q.limit ?? 50);
        return {
          items: matching.slice(start, end),
          nextCursor: end < matching.length ? String(end) : null,
          total: matching.length,
        };
      }

      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
            calls.push(cmd);
            if (cmd === "plugin:event|listen") {
              const event = args.event as string;
              const eventId = nextEventId++;
              listeners.set(event, [...(listeners.get(event) ?? []), { eventId, handler: args.handler as number }]);
              return eventId;
            }
            if (cmd === "plugin:event|unlisten") return undefined;
            if (clips && cmd === "list_clips") {
              queries.push(args.q as ClipQuery);
              return listClips(args.q as ClipQuery);
            }
            if (clips && cmd === "delete_clips") {
              const ids = args.ids as string[];
              for (const id of ids)
                history.splice(
                  history.findIndex((clip) => clip.id === id),
                  1,
                );
              queueMicrotask(() => (window as unknown as MockWindow).__emit("clips://changed", {}));
              return undefined;
            }
            if (clips && cmd === "get_app_icon")
              throw { code: "not_found", message: "No icon is cached for this app." };
            if (cmd in responses) return responses[cmd];
            throw new Error(`IPC command not mocked: ${cmd}`);
          },
          transformCallback: (callback: (event: unknown) => void) => {
            const id = nextCallbackId++;
            callbacks.set(id, callback);
            return id;
          },
          unregisterCallback: (id: number) => callbacks.delete(id),
          metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
        },
      });
    },
    { responses, clips: options.clips },
  );
}

/** The names of the commands the page has invoked so far, in order. */
export function ipcCalls(page: Page): Promise<string[]> {
  return page.evaluate(() => [...(window as unknown as MockWindow).__ipcCalls]);
}

/** The `q` argument of every `list_clips` call so far, in order (needs `mockIpc` with `clips`). */
export function listClipsQueries(page: Page): Promise<ClipQuery[]> {
  return page.evaluate(() => [...(window as unknown as MockWindow).__ipcQueries]);
}

/** What the dev harness's clip browser told its host so far, e.g. `paste:clip-2` or `close`, in order. */
export function browserCalls(page: Page): Promise<string[]> {
  return page.evaluate(() => [...((window as unknown as MockWindow).__browserCalls ?? [])]);
}

/** Records `clip` as the newest in the fake history, then announces `clips://changed`. */
export function recordClip(page: Page, clip: ClipSummary): Promise<void> {
  return page.evaluate((clip) => {
    const w = window as unknown as MockWindow;
    w.__clips.unshift(clip);
    w.__emit("clips://changed", {});
  }, clip);
}
