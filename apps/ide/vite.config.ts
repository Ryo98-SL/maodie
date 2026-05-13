import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { defineConfig, type Plugin } from "vite";

// `IDE_BASE` lets CI publish under a sub-path such as `/maodie/` for GitHub Pages
// project sites. Defaults to `/` for local dev / preview.
const base = process.env.IDE_BASE ?? "/";

// Single source of truth for the project logo lives at repo-root `assets/logo.webp`.
// This plugin mirrors it into the IDE's `public/` directory at dev-server start and at
// `vite build`, so the favicon stays in sync without committing a duplicate file.
function syncFaviconFromAssets(): Plugin {
  const src = resolve(__dirname, "../../assets/logo.webp");
  const dest = resolve(__dirname, "public/favicon.webp");
  const copy = () => {
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(src, dest);
  };
  return {
    name: "maodie-sync-favicon",
    buildStart: copy,
    configureServer: copy
  };
}

export default defineConfig({
  root: __dirname,
  base,
  plugins: [syncFaviconFromAssets()],
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
