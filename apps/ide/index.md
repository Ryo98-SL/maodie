# IDE App Module

## Purpose

`apps/ide` owns the first browser-based Maodie IDE shell and compile workbench.

## Current Directory Structure

- `src/`: browser entry point, example source catalog, compiler client, state types, and UI rendering.
- `index.html`: Vite HTML entry.
- `vite.config.ts`: Vite build and alias configuration.
- `tailwind.config.cjs`: Tailwind utility scanning configuration.
- `postcss.config.cjs`: Tailwind and Autoprefixer PostCSS wiring.
- `project.json`: Nx app tasks.

## Key Files

- `src/main.ts`: owns source state, example switching, compile/evaluate scheduling, and DOM event wiring.
- `src/examples.ts`: defines the built-in workbench examples, including Hello World, function calls, Fibonacci, and the v1 acceptance source.
- `src/state.ts`: centralizes IDE state, dump keys, compile status, and evaluation status types.
- `src/compilerClient.ts`: loads `@maodie/compiler-wasm` with the Vite-managed WASM asset URL and evaluates emitted WASM.
- `src/compilerClient.test.ts`: verifies the v1 default source compiles, evaluates, and error diagnostics render.
- `src/view.ts`: renders the editor, diagnostics, evaluation, compile status, artifact summary, and dump tabs.
- `src/panels.ts`: renders diagnostics and evaluation panels used by the main workbench view.
- `src/tailwind.css`: Tailwind directive entry without custom component styles.

## Runtime Behaviors

The IDE shell starts with the v1 acceptance `.mao` example, compiles through `@maodie/compiler-wasm`, evaluates the generated `main(i32)` export, captures `core.log` output through the `maodie.debug_string` host import, and refreshes diagnostics after edits. The editor header includes example tabs that replace the current source and immediately recompile, while manual edits move the workbench into a custom-source state. The UI shows source positions, compile/load status, evaluation raw/tag/payload/log output, artifact metadata, and debug dumps for AST, HIR, MIR, WAT, and type information when available.

## Integration Notes

Browser WASM is imported from `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm?url`. Vite serves that path in dev and copies it into `dist/apps/ide/assets` during production builds. Compilation currently runs on the main thread; move only `src/compilerClient.ts` behind a Worker when compile latency starts affecting editing.
