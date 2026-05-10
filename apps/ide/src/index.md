# IDE Source Module

## Purpose

The IDE source module owns browser bootstrapping for the Maodie compile workbench.

## Current Directory Structure

- `main.ts`: IDE state, example switching, debounced compile/evaluate scheduling, and DOM event wiring.
- `examples.ts`: built-in workbench examples and the default v1 source.
- `state.ts`: shared state and status types for the entry point and renderers.
- `compilerClient.ts`: browser-side wrapper around `@maodie/compiler-wasm`, including the Vite `?url` WASM asset path, readable load failures, and `main` evaluation from emitted WASM.
- `compilerClient.test.ts`: v1 smoke tests for default-source compilation, rendered diagnostics, and evaluation output.
- `view.ts`: Tailwind-based HTML rendering for the workbench shell, editor, example tabs, compile status, artifact metadata, and dump tabs.
- `panels.ts`: diagnostics and evaluation panel rendering.
- `vite-env.d.ts`: Vite asset import typing.
- `tailwind.css`: Tailwind directive entry.

## Key Behaviors

The module renders an editable `workspace/main.mao` document using the v1 acceptance example unless a smoke test passes `?source=` in the URL. The editor offers built-in example tabs for Hello World, function calls, Fibonacci, and the v1 comprehensive source; selecting one replaces the editor contents and immediately recompiles. Source changes trigger a debounced compile through the WASM wrapper and clear the selected example marker. Successful compiles also evaluate the emitted WASM `main(i32)` export with the current input value, showing raw `i32` output plus the v1 tag/payload decoding and `core.log` messages captured from `maodie.debug_string`. The diagnostics panel shows severity, code, Chinese compiler messages, notes, and source positions. The output panel switches between AST, HIR, MIR, WAT, and types dumps when the compiler response includes them.

## Integration Notes

The current implementation intentionally uses a single-threaded browser compile path and records the limitation in `compilerClient.ts`/package docs. A future Worker can keep the same `compileBrowserSource` contract while moving instantiation and compile calls off the UI thread.
