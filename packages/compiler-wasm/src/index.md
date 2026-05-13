# Compiler WASM Source Module

## Purpose

This module exposes the stable TypeScript API for compiling and syntax-highlighting `.mao` source through the Rust WASM compiler.

## Current Directory Structure

- `index.ts`: public types, loader helpers, singleton compile/highlight helpers, and memory bridge.
- `highlight.worker.ts`: incremental highlight worker protocol types, stale-response helper, and browser/Node-testable request handler.
- `ranges.ts`: editor range conversion helpers for highlight token byte ranges.
- `index.test.ts`: integration smoke tests against the locally built WASM artifact.

## Key Behaviors

The wrapper instantiates WASM in Node or browser runtimes, writes source/options into module memory, calls the private pointer-level ABI, and reads JSON response bytes. Compile responses normalize WASM binary artifact content into `Uint8Array`; highlight responses return lexer-backed tokens and diagnostics without artifacts or dumps.

`MaodieHighlightSession` wraps the WASM session lifecycle. `createHighlightSession` returns an initial full highlight response at session version 0. `update` sends byte-range edits with editor/session versions and only advances the local current response when the WASM response is `ok`. `reset` replaces the full source snapshot and returns a full rehighlight response. `dispose` is safe to call more than once; update/reset after dispose throw a readable error before touching WASM.

`highlight.worker.ts` exposes a protocol for `init`, `update`, `reset`, and `dispose` messages. `createHighlightWorkerRequestHandler` keeps one compiler/session pair in the worker, and `isStaleHighlightWorkerResponse` lets the main thread compare response versions against the current editor state before applying tokens or diagnostics.

`ranges.ts` converts Rust byte offsets to UTF-16 absolute offsets or 0-based line/character ranges. It throws `RangeError` for out-of-bounds offsets, unordered ranges, or byte offsets that split a UTF-8 code point.

## Integration Notes

Use `createCompilerWasm` when an app needs to control the WASM URL or bytes. Use `compileMaodieWasm` or `highlightMaodieSource` for default Node smoke tests and simple tools.
