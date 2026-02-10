import { defineConfig } from "tsup";

export default defineConfig({
  entry: [
    "src/index.ts",
    "src/middleware/express.ts",
    "src/middleware/next.ts",
    "src/middleware/fastify.ts",
    "src/testing.ts",
  ],
  format: ["esm", "cjs"],
  dts: true,
  sourcemap: true,
  clean: true,
  target: "es2022",
  external: ["express", "fastify", "next"],
});
