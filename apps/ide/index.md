# IDE App Module

## Purpose

`apps/ide` owns the first browser-based Maodie IDE shell and compile workbench.

## Current Directory Structure

- `src/`: browser entry point, Monaco editor shell, highlight adapter, Monaco language helpers, example source catalog, compiler client, state types, and UI rendering.
- `index.html`: Vite HTML entry.
- `vite.config.ts`: Vite build and alias configuration.
- `tailwind.config.cjs`: Tailwind utility scanning configuration.
- `postcss.config.cjs`: Tailwind and Autoprefixer PostCSS wiring.
- `project.json`: Nx app tasks.

## Key Files

- `src/main.ts`: owns source state, Monaco lifecycle wiring, example switching, manual Run/evaluate scheduling, and DOM event wiring.
- `src/editor.ts`: creates and destroys the Monaco editor/model, exposes source read/replace APIs, configures restrained IDE behavior, and owns the smoke-test editor hook.
- `src/monacoLanguage.ts`: registers the Maodie Monaco language/theme, maps highlight kinds to semantic token types, converts byte ranges to Monaco ranges, and creates marker data.
- `src/highlightAdapter.ts`: connects a Monaco model to the WASM highlight worker, sends incremental updates or resets, filters stale responses, recovers update protocol errors with a current-document reset, and publishes live lexer diagnostics.
- `src/initialSource.ts`: resolves the default editor document from `window.location.search`, including smoke-test `?source=` input.
- `src/examples.ts`: defines the built-in workbench examples, including Hello World, function calls, Fibonacci, and the v1 acceptance source.
- `src/state.ts`: centralizes IDE state, dump keys, compile status, live lexer status, and evaluation status types.
- `src/compilerClient.ts`: loads `@maodie/compiler-wasm` with the Vite-managed WASM asset URL and evaluates emitted WASM.
- `src/compilerClient.test.ts`: verifies the v1 default source compiles, evaluates, renders diagnostics, and covers Monaco highlight/range adapter behavior.
- `src/view.ts`: renders the editor, diagnostics, evaluation, compile status, artifact summary, and dump tabs.
- `src/panels.ts`: renders diagnostics and evaluation panels used by the main workbench view.
- `src/tailwind.css`: Tailwind directive entry without custom component styles.

## Runtime Behaviors

The IDE shell starts with the v1 acceptance `.mao` example and waits for the user to click Run. Monaco initializes a dedicated highlight worker session immediately and applies returned tokens through a model-scoped semantic token provider plus live lexer diagnostics through Monaco markers. Stale worker responses are ignored, and incremental update protocol errors reset the worker from the current editor document instead of surfacing stale session diagnostics. Run reads the current Monaco model, compiles it through `@maodie/compiler-wasm`, evaluates the generated `main(i32)` export, and captures `core.log` output through `maodie` debug chunk imports, flushing one UI log line on `debug_log_end`. Example tabs replace the current Monaco document without compiling; manual edits update source state and invalidate stale compile/evaluation state without a full workbench re-render, so typing remains continuous. The workbench uses a fixed viewport layout: the top header stays visible, Monaco owns editor scrolling, each right-side panel gives its content area a `min-h-0 flex-1 overflow-auto` scroll region, and the layout switches from stacked to side-by-side at 600px. The right column shows Evaluation above Diagnostics; Diagnostics separates `Live Lexer` from `Last Compile` so real-time lexer errors are not mistaken for parser/typechecker compile output. Evaluation places logs before call/raw cards and renders the v1 enum-encoded return as both `Result.Ok(payload)` text and JSON. The UI also shows source positions, compile/load status, artifact metadata, and debug dumps for AST, HIR, MIR, WAT, and type information when available.

## Integration Notes

Browser WASM is imported from `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm?url`. Vite serves that path in dev and copies it into `dist/apps/ide/assets` during production builds. The highlight adapter passes the same Vite-managed WASM URL to `packages/compiler-wasm/src/highlight.worker.ts`, while compilation currently remains on the main thread; move only `src/compilerClient.ts` behind a Worker when compile latency starts affecting editing. Browser smoke automation uses the narrow `window.maodieIdeEditor` hook rather than Monaco DOM internals.
