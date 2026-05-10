# Compiler WASM Package

## Purpose

`compiler-wasm` owns the TypeScript loading and memory wrapper for the Rust compiler and syntax highlight WASM API.

## Current Directory Structure

- `src/`: public wrapper API, compile/highlight contract types, and Node smoke tests.
- `project.json`: Nx targets that build the Rust WASM crate before compiling or testing TypeScript.
- `tsconfig.json`: TypeScript project configuration.

## Key Files

- `src/index.ts`: Node/browser WASM loader and `MaodieCompilerWasm` wrapper.
- `src/ranges.ts`: UTF-8 byte range to UTF-16 editor offset and line/character conversion helpers.
- `src/index.test.ts`: Node integration smoke tests that load the generated `.wasm` artifact, verify shared highlight fixtures, and run the final syntax-highlighting acceptance smoke through the TS wrapper.

## Runtime Behaviors

Node defaults to loading `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm`. Browser callers should provide a `wasmUrl`, `wasmBytes`, `wasmModule`, or pre-instantiated `instance` from their bundler or app asset pipeline.

`CompileResponse.dumps` includes `ast`, `hir`, and `types` when parsing/type checking runs. Successful compilation also includes `mir` and `wat`.

`HighlightResponse` includes `ok`, `tokens`, and `diagnostics`. The wrapper exposes both `MaodieCompilerWasm.highlight` and `highlightMaodieSource`; neither path normalizes artifacts because highlight responses never include compile output.

Highlight token ranges remain Rust UTF-8 byte ranges. Use `byteRangeToUtf16Range` or `byteRangeToUtf16LineColumnRange` before handing tokens to editor APIs that expect UTF-16 offsets or 0-based line/character positions.

## Integration Notes

CLI and IDE tasks should call this package only. They should not duplicate WASM memory handling or depend on the Rust crate output path directly.
