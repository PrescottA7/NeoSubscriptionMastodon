import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // The admin path prefix is only known at deploy time, so the bundle keeps a
  // placeholder that the server substitutes in `index.html` and in asset URLs.
  base: "/__NSM_BASE__",
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:3000",
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/setupTests.ts",
    globals: true,
  },
});
