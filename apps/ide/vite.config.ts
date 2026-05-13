import { resolve } from "node:path";

import { defineConfig } from "vite";

// `IDE_BASE` lets CI publish under a sub-path such as `/maodie/` for GitHub Pages
// project sites. Defaults to `/` for local dev / preview.
const base = process.env.IDE_BASE ?? "/";

export default defineConfig({
  root: __dirname,
  base,
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
