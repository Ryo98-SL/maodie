# Maodie Compiler Crate

## Purpose

`maodie_compiler` is the Rust facade crate for the Maodie compiler core.

## Current Directory Structure

- `Cargo.toml`: crate manifest participating in the repository Rust workspace.
- `src/`: public facade API and crate-level tests.

## Key Behaviors

The crate exposes the Rust compiler facade, core standard library contract, syntax/resolution/type/MIR stages, and v1 WASM backend. `core.log(message: String)` is declared in the compiler-provided core source and recognized by type checking/WASM lowering for minimal `{}` interpolation through `maodie` debug chunk imports.

## Integration Notes

Downstream Rust crates should follow the `maodie_*` naming convention and join the root Cargo workspace by adding themselves to `Cargo.toml`.
