import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach } from "vitest";

// Every test installs its own IPC handler with mockIPC; none leaks into the next one.
afterEach(() => {
  clearMocks();
});
