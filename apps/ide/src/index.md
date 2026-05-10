# IDE Source Module

## Purpose

The IDE source module owns browser bootstrapping for the Maodie compile workbench.

## Current Directory Structure

- `main.ts`: IDE state, example switching, manual Run/evaluate scheduling, and DOM event wiring.
- `examples.ts`: built-in workbench examples and the default v1 source.
- `state.ts`: shared state and status types for the entry point and renderers.
- `compilerClient.ts`: browser-side wrapper around `@maodie/compiler-wasm`, including the Vite `?url` WASM asset path, readable load failures, and `main` evaluation from emitted WASM.
- `compilerClient.test.ts`: v1 smoke tests for default-source compilation, rendered diagnostics, and evaluation output.
- `view.ts`: Tailwind-based HTML rendering for the workbench shell, editor, example tabs, compile status, artifact metadata, and dump tabs.
- `panels.ts`: diagnostics and evaluation panel rendering.
- `vite-env.d.ts`: Vite asset import typing.
- `tailwind.css`: Tailwind directive entry.

## Key Behaviors

The module renders an editable `workspace/main.mao` document using the v1 acceptance example unless a smoke test passes `?source=` in the URL. The editor offers built-in example tabs for Hello World, function calls, Fibonacci, and the v1 comprehensive source; selecting one replaces the editor contents and waits for Run. Textarea input only updates in-memory source state and invalidates in-flight compile requests, avoiding a full workbench re-render on each keystroke. The workbench fills the browser viewport with `overflow-hidden`; the header is outside the scrolling area, the textarea owns editor scrolling, each right-side panel uses a flex column with a `min-h-0 flex-1 overflow-auto` content region, and side-by-side layout starts at 600px through Tailwind arbitrary breakpoint classes. Clicking Run compiles through the WASM wrapper and evaluates the emitted WASM `main(i32)` export with the current input value. The Evaluation panel appears before Diagnostics, puts `core.log` messages first, then shows call/raw/tag/payload cards and a variant card with both display text and JSON for the v1 `i32` enum encoding. The diagnostics panel shows severity, code, Chinese compiler messages, notes, and source positions. The output panel switches between AST, HIR, MIR, WAT, and types dumps when the compiler response includes them.

## Integration Notes

The current implementation intentionally uses a single-threaded browser compile path and records the limitation in `compilerClient.ts`/package docs. A future Worker can keep the same `compileBrowserSource` contract while moving instantiation and compile calls off the UI thread.
