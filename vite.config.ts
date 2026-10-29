/// <reference types="vitest/config" />
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// Tauri expects a fixed port and must not have its Rust sources watched by Vite.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // One HTML entry per Tauri window (index.html is the dev harness).
  build: {
    rollupOptions: {
      input: { main: "index.html", popover: "popover.html", panel: "panel.html" },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["src/test/setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,svelte}"],
      // main.ts and App.svelte are the dev harness, covered by the end-to-end tests.
      exclude: [
        "src/**/*.test.ts",
        "src/test/**",
        "src/lib/bindings/**",
        "src/main.ts",
        "src/App.svelte",
        "src/vite-env.d.ts",
      ],
      // Not coverage/: coverage/rust.lcov lives there and Vitest cleans its reports directory.
      reportsDirectory: "coverage/front",
      reporter: ["text-summary", "lcov"],
    },
  },
});
