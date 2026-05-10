import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    globals: true,
    passWithNoTests: true
  },
  resolve: {
    alias: {
      "@maodie/language-core": new URL("./packages/language-core/src/index.ts", import.meta.url).pathname,
      "@maodie/compiler": new URL("./packages/compiler/src/index.ts", import.meta.url).pathname,
      "@maodie/compiler-wasm": new URL("./packages/compiler-wasm/src/index.ts", import.meta.url).pathname,
      "@maodie/ide-protocol": new URL("./packages/ide-protocol/src/index.ts", import.meta.url).pathname
    }
  }
});
