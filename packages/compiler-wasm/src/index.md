# Compiler WASM Source Module

## Purpose

This module exposes the stable TypeScript API for compiling `.mao` source through the Rust WASM compiler.

## Current Directory Structure

- `index.ts`: public types, loader helpers, singleton compile helper, and memory bridge.
- `index.test.ts`: integration smoke test against the locally built WASM artifact.

## Key Behaviors

The wrapper instantiates WASM in Node or browser runtimes, writes source/options into module memory, calls the private pointer-level ABI, reads JSON response bytes, and normalizes WASM binary artifact content into `Uint8Array`.

## Integration Notes

Use `createCompilerWasm` when an app needs to control the WASM URL or bytes. Use `compileMaodieWasm` for default Node smoke tests and simple tools.
