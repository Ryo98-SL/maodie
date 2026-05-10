# CLI Source Module

## Purpose

The CLI source module contains command entry points for local Maodie compiler usage.

## Current Directory Structure

- `index.ts`: public package exports.
- `main.ts`: Node executable entry point.
- `main.test.ts`: v1 acceptance smoke tests that compile checked files under `examples/`.

## Key Behaviors

`main.ts` parses `maodie compile <source.mao> --emit <kind>` and `maodie run <source.mao> [--input <i32>]`, invokes `compileMaodieWasm`, prints diagnostics to stderr, and emits the selected artifact, dump, or captured runtime logs.

Supported emit kinds are `wasm`, `wat`, `ast`, `hir`, and `mir`. Text outputs default to stdout. Binary `wasm` defaults to the wrapper artifact filename unless `--out` is provided. The `run` path instantiates the emitted WASM with `maodie.debug_string` and prints `core.log` messages to stdout.

The v1 smoke path compiles `examples/v1_acceptance.mao`, checks dump output, and verifies `examples/v1_error.mao` returns non-zero with Chinese `MD####` diagnostics.

## Integration Notes

Add subcommands in separate files once command behavior grows beyond this small compile entry point. Keep tests against `runCli` so stdout, stderr, and exit codes stay easy to verify.
