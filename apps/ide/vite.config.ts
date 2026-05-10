import { resolve } from "node:path";

import { defineConfig } from "vite";

export default defineConfig({
  root: __dirname,
  build: {
    outDir: "../../dist/apps/ide",
    emptyOutDir: true
  },
  resolve: {
    alias: {
      "@maodie/compiler-wasm": resolve(__dirname, "../../packages/compiler-wasm/src/index.ts"),
      "@maodie/ide-protocol": resolve(__dirname, "../../packages/ide-protocol/src/index.ts"),
      "@maodie/language-core": resolve(__dirname, "../../packages/language-core/src/index.ts")
    }
  }
});
