# Compiler Module

## Purpose

`compiler` owns the public Maodie compile API and the future pipeline from source text to compiled artifacts.

## Current Directory Structure

- `src/`: compiler entry point and tests.
- `project.json`: Nx tasks and dependency metadata.
- `tsconfig.json`: TypeScript project configuration with a reference to `language-core`.

## Key Files

- `src/index.ts`: `compileMaodie` API, compile options, and placeholder IR emission.
- `src/index.test.ts`: contract tests for diagnostics and artifact output.

## Runtime Behaviors

The current implementation creates a `SourceFile`, returns diagnostics for empty input, and emits a placeholder IR artifact for non-empty input.

## Integration Notes

Future lexer, parser, checker, IR, and backend folders should be added under `src/` with matching `index.md` files.
