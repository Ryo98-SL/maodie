# Compiler WASM Source Module

## Purpose

This module exposes the stable TypeScript API for compiling and syntax-highlighting `.mao` source through the Rust WASM compiler.

## Current Directory Structure

- `index.ts`: public types, loader helpers, singleton compile/highlight helpers, and memory bridge.
- `ranges.ts`: editor range conversion helpers for highlight token byte ranges.
- `index.test.ts`: integration smoke tests against the locally built WASM artifact.

## Key Behaviors

The wrapper instantiates WASM in Node or browser runtimes, writes source/options into module memory, calls the private pointer-level ABI, and reads JSON response bytes. Compile responses normalize WASM binary artifact content into `Uint8Array`; highlight responses return lexer-backed tokens and diagnostics without artifacts or dumps.

`ranges.ts` converts Rust byte offsets to UTF-16 absolute offsets or 0-based line/character ranges. It throws `RangeError` for out-of-bounds offsets, unordered ranges, or byte offsets that split a UTF-8 code point.

## Integration Notes

Use `createCompilerWasm` when an app needs to control the WASM URL or bytes. Use `compileMaodieWasm` or `highlightMaodieSource` for default Node smoke tests and simple tools.
