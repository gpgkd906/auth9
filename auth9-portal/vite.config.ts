import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [
    tailwindcss(),
    reactRouter(),
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
        "eslint.config.js",
      ],
    },
  },
});
