/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed port and must see Rust errors in the terminal.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  build: { target: "es2022" },
  // Component tests (jsdom) mount the client build of Svelte, not its server one.
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
  test: { include: ["src/**/*.test.ts"], passWithNoTests: true, setupFiles: ["src/test-setup.ts"] },
});
