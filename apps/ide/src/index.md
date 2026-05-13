# IDE Source Module

## Purpose

The IDE source module owns browser bootstrapping for the Maodie compile workbench.

## Current Directory Structure

- `main.ts`: IDE state, Monaco lifecycle wiring, live lexer diagnostics wiring, example switching, manual Run/evaluate scheduling, and DOM event wiring.
- `editor.ts`: Monaco editor/model setup with document initialization, restrained IDE options, source read/replace helpers, worker lifecycle hookup, and smoke-test API exposure.
- `monacoLanguage.ts`: Maodie Monaco language registration, theme, semantic token legend, byte range conversion, marker conversion, and model token storage.
- `highlightAdapter.ts`: Monaco highlight worker adapter, incremental update/reset scheduling, stale response checks, update-error reset recovery, semantic token application, marker application, and live lexer state publication.
- `initialSource.ts`: pure helper for default source selection and smoke-test `?source=` injection.
- `examples.ts`: built-in workbench examples and the default v1 source.
- `state.ts`: shared state and status types for the entry point and renderers, including live lexer diagnostics state.
- `compilerClient.ts`: browser-side wrapper around `@maodie/compiler-wasm`, including the Vite `?url` WASM asset path, readable load failures, and `main` evaluation from emitted WASM.
- `compilerClient.test.ts`: v1 smoke tests for default-source compilation, rendered diagnostics, evaluation output, and Monaco highlight adapter mapping/range behavior.
- `view.ts`: Tailwind-based HTML rendering for the workbench shell, editor, example tabs, compile status, artifact metadata, and dump tabs.
- `panels.ts`: diagnostics and evaluation panel rendering.
- `vite-env.d.ts`: Vite asset import typing.
- `tailwind.css`: Tailwind directive entry.

## Key Behaviors

The module renders an editable `workspace/main.mao` Monaco document using the v1 acceptance example unless a smoke test passes `?source=` in the URL. The editor starts a WASM highlight worker session and applies returned token data through Monaco semantic tokens after stale editor or session responses are filtered out. Live lexer diagnostics are written as Monaco model markers and also rendered in the Diagnostics panel. If an incremental `update` response reports a protocol/session error, the adapter resets the worker session from the current Monaco model so an old version mismatch cannot become a user-facing live lexer diagnostic. The editor offers built-in example tabs for Hello World, function calls, Fibonacci, and the v1 comprehensive source; selecting one replaces the Monaco model, clears old compile/evaluation state, and waits for Run. Monaco change events only update in-memory source state, schedule highlight worker updates, and invalidate in-flight compile requests, avoiding a full workbench re-render on each keystroke. The workbench fills the browser viewport with `overflow-hidden`; the header is outside the scrolling area, Monaco owns editor scrolling, each right-side panel uses a flex column with a `min-h-0 flex-1 overflow-auto` content region, and side-by-side layout starts at 600px through Tailwind arbitrary breakpoint classes. Clicking Run reads the current Monaco document, compiles through the WASM wrapper, and evaluates the emitted WASM `main(i32)` export with the current input value, collecting `core.log` debug chunks into one UI log line per `debug_log_end`. The Evaluation panel appears before Diagnostics, puts `core.log` messages first, then shows call/raw/tag/payload cards and a variant card with both display text and JSON for the v1 `i32` enum encoding. The diagnostics panel labels `Live Lexer` separately from `Last Compile`; live diagnostics come from the highlight worker and compile diagnostics remain tied to the last Run result. The output panel switches between AST, HIR, MIR, WAT, and types dumps when the compiler response includes them.

## Integration Notes

The current implementation intentionally uses a single-threaded browser compile path and records the limitation in `compilerClient.ts`/package docs. Syntax highlighting already runs through the compiler-wasm highlight worker and uses the same Vite-managed WASM URL as compile. A future compile Worker can keep the same `compileBrowserSource` contract while moving instantiation and compile calls off the UI thread. Browser smoke automation uses `window.maodieIdeEditor` for source replacement, insertion, semantic token counts, and live marker counts so tests do not depend on Monaco internal DOM.
