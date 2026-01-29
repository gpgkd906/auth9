import { vitePlugin as remix } from "@remix-run/dev";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [
    remix({
      future: {
        v3_fetcherPersist: true,
        v3_relativeSplatPath: true,
        v3_throwAbortReason: true,
      },
    }),
    tsconfigPaths(),
  ],
  test: {
    environment: "jsdom",
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      thresholds: {
        global: {
          statements: 15,
          branches: 50,
          functions: 50,
          lines: 15,
        },
        "services/**": {
          statements: 90,
          branches: 85,
          functions: 90,
          lines: 90,
        },
      },
      exclude: [
        "node_modules/**",
        "build/**",
        "public/**",
        "**/*.d.ts",
        "**/*.test.ts",
        "**/*.test.tsx",
        "vite.config.ts",
        "tailwind.config.ts",
        "postcss.config.js",
        ".eslintrc.cjs",
      ],
    },
  },
});
