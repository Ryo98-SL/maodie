# Compiler WASM Package

## Purpose

`compiler-wasm` owns the TypeScript loading and memory wrapper for the Rust compiler WASM API.

## Current Directory Structure

- `src/`: public wrapper API, compile contract types, and Node smoke tests.
- `project.json`: Nx targets that build the Rust WASM crate before compiling or testing TypeScript.
- `tsconfig.json`: TypeScript project configuration.

## Key Files

- `src/index.ts`: Node/browser WASM loader and `MaodieCompilerWasm` wrapper.
- `src/index.test.ts`: Node integration smoke test that loads the generated `.wasm` artifact.

## Runtime Behaviors

Node defaults to loading `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm`. Browser callers should provide a `wasmUrl`, `wasmBytes`, `wasmModule`, or pre-instantiated `instance` from their bundler or app asset pipeline.

`CompileResponse.dumps` includes `ast`, `hir`, and `types` when parsing/type checking runs. Successful compilation also includes `mir` and `wat`.

## Integration Notes

CLI and IDE tasks should call this package only. They should not duplicate WASM memory handling or depend on the Rust crate output path directly.
