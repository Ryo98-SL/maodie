# Rust Crates Module

## Purpose

The Rust crates module owns the Maodie compiler core workspace.

## Current Directory Structure

- `maodie_compiler/`: compiler facade crate and future root of the Rust compile pipeline.
- `maodie_diagnostics/`: shared source file, byte span, diagnostic code, severity, Chinese CLI rendering, and JSON model.
- `maodie_syntax/`: source-level syntax utilities, including the `.mao` lexer, parser, syntax highlight API, and incremental highlight session.
- `maodie_wasm_api/`: WebAssembly ABI crate that wraps the compiler facade and emits JSON compile responses.
- `project.json`: Nx bridge project named `rust` for Cargo build, check, test, lint, format, and WASM build tasks.

## Key Behaviors

Cargo owns Rust crate discovery through the repository-level `Cargo.toml`. Nx invokes Cargo through the `rust` project so root commands can run Rust tasks alongside TypeScript package tasks.

## Integration Notes

Rust crates use the `maodie_*` crate naming convention. Cargo build output stays in the default `target/` directory, which is ignored and declared as the Nx output boundary for cacheable Rust build targets. The TypeScript WASM wrapper loads `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm` in Node by default.
