import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

// jsdom has no layout engine and no ResizeObserver, which Svelte's bind:clientHeight needs. This one
// never reports a size: a component test that needs a height stubs `clientHeight` itself.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// Vitest runs without globals, so Testing Library does not unmount by itself; and every test installs
// its own IPC handler with mockIPC: none leaks into the next one.
afterEach(() => {
  cleanup();
  clearMocks();
});
