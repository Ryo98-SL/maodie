# Packages Module

## Purpose

The packages module contains reusable Maodie libraries and tools.

## Current Directory Structure

- `language-core/`: shared language-domain primitives.
- `compiler/`: public compiler API and compile pipeline.
- `compiler-wasm/`: Node/browser TypeScript wrapper around the Rust WASM compiler API.
- `cli/`: command line interface for the compiler.
- `ide-protocol/`: shared IDE and language-service contracts.

## Key Behaviors

Packages should publish stable TypeScript entry points from `src/index.ts` and declare Nx tasks in local `project.json` files.

## Integration Notes

Prefer adding new language behavior to packages before wiring it into apps. CLI and IDE integrations should call `compiler-wasm` rather than duplicating Rust WASM loading or memory handling.
