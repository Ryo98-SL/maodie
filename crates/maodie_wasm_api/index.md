# Maodie WASM API Crate

## Purpose

`maodie_wasm_api` owns the WebAssembly ABI around the Rust compiler facade and syntax highlight entry point.

## Current Directory Structure

- `src/`: exported WASM ABI functions plus JSON compile and highlight contracts.
- `Cargo.toml`: crate metadata, `cdylib` output, and compiler/serde dependencies.

## Key Behaviors

The crate exports allocation helpers plus `maodie_compile` and `maodie_highlight` for `wasm32-unknown-unknown` builds. Callers pass UTF-8 source text and options JSON into WASM memory, then receive a pointer to a response buffer containing JSON.

`maodie_highlight` calls the Rust syntax highlight API directly. It returns `ok`, `tokens`, and `diagnostics` without invoking parse, type-check, MIR lowering, WAT generation, or WASM artifact generation.

## Integration Notes

The pointer-level ABI is private to TypeScript wrapper code. CLI and IDE work should depend on `packages/compiler-wasm` instead of importing this crate or duplicating memory handling.
