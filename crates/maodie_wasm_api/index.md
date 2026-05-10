# Maodie WASM API Crate

## Purpose

`maodie_wasm_api` owns the WebAssembly ABI around the Rust compiler facade.

## Current Directory Structure

- `src/`: exported WASM ABI functions and JSON compile contract.
- `Cargo.toml`: crate metadata, `cdylib` output, and compiler/serde dependencies.

## Key Behaviors

The crate exports allocation helpers plus `maodie_compile` for `wasm32-unknown-unknown` builds. Callers pass UTF-8 source text and compile options JSON into WASM memory, then receive a pointer to a response buffer containing JSON.

## Integration Notes

The pointer-level ABI is private to TypeScript wrapper code. CLI and IDE work should depend on `packages/compiler-wasm` instead of importing this crate or duplicating memory handling.
