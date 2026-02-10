import { defineConfig } from "vitest/config";
import { resolve } from "path";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/**/*.test.ts", "src/index.ts"],
      reporter: ["text", "html", "json"],
      reportsDirectory: "./coverage",
    },
  },
  resolve: {
    alias: {
      "@auth9/core": resolve(__dirname, "../core/src/index.ts"),
    },
  },
});
