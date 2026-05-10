# WASM API Source Module

## Purpose

This module converts the Rust compiler facade into a small WASM-safe JSON API.

## Current Directory Structure

- `lib.rs`: compile request/response types, compiler pipeline orchestration, and exported memory ABI.

## Key Files

- `lib.rs`: exposes `maodie_alloc`, `maodie_dealloc`, `maodie_compile`, `maodie_response_len`, and `maodie_free_response`.

## Runtime Behaviors

The compile path parses source text, type-checks with the core library, lowers to MIR on success, emits WAT/WASM artifacts, and returns diagnostics, artifacts, and dumps as JSON.

## Integration Notes

The exported ABI intentionally remains low-level so it can be consumed by Node and browsers without `wasm-bindgen`. The TypeScript wrapper owns all public loading and memory safety conventions.
