import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [react(), tsconfigPaths()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./tests/setup.ts"],
    include: ["./app/**/*.test.{ts,tsx}", "./tests/**/*.test.{ts,tsx}"],
    coverage: {
      reporter: ["text", "json", "html"],
      include: ["app/**/*.{ts,tsx}"],
      exclude: ["app/**/*.test.{ts,tsx}", "app/entry.*.tsx"],
    },
  },
});
