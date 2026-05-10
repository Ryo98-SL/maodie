# Maodie Compiler Source

## Purpose

This source directory contains the public Rust compiler facade.

## Current Directory Structure

- `lib.rs`: minimal facade API used to prove Cargo and Nx can build and test the Rust workspace.

## Key Behaviors

The facade reports stable workspace metadata only. Task 01 deliberately avoids lexer, parser, diagnostics, WASM API, or language semantics.

## Integration Notes

Keep public additions small and covered by Rust unit tests so `cargo test --workspace` remains the primary Rust validation entry point.
