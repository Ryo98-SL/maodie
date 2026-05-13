# WASM API Source Module

## Purpose

This module converts the Rust compiler facade into a small WASM-safe JSON API for compilation and syntax highlighting.

## Current Directory Structure

- `lib.rs`: compile and highlight request/response types, compiler pipeline orchestration, and exported memory ABI.

## Key Files

- `lib.rs`: exposes `maodie_alloc`, `maodie_dealloc`, `maodie_compile`, `maodie_highlight`, `maodie_highlight_session_create`, `maodie_highlight_session_update`, `maodie_highlight_session_reset`, `maodie_highlight_session_dispose`, `maodie_response_len`, `maodie_response_bytes`, and `maodie_free_response`.

## Runtime Behaviors

The compile path parses source text, type-checks with the core library, lowers to MIR on success, emits WAT/WASM artifacts, and returns diagnostics, artifacts, and dumps as JSON.

The highlight path calls `maodie_compiler::syntax::highlight_source` and serializes `HighlightResponse { ok, tokens, diagnostics }`. It preserves lexer diagnostics and does not produce compile artifacts or dumps.

The incremental highlight session path wraps `maodie_compiler::syntax::IncrementalHighlightSession` behind an opaque pointer. Create returns a session handle plus an initial full highlight snapshot. Update requires the caller's current session version and rejects stale versions without mutating the session. Reset replaces the full source snapshot and increments the session version. Dispose drops the Rust session handle; TypeScript owns preventing later calls with that pointer.

## Integration Notes

The exported ABI intentionally remains low-level so it can be consumed by Node and browsers without `wasm-bindgen`. The TypeScript wrapper owns all public loading and memory safety conventions.
