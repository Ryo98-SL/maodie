# Language Core Module

## Purpose

`language-core` owns shared Maodie language primitives that can be used by compiler, CLI, IDE, and future language-service packages.

## Current Directory Structure

- `src/`: TypeScript source entry point and domain types.
- `project.json`: Nx tasks for build, typecheck, and tests.
- `tsconfig.json`: package TypeScript project configuration.

## Key Files

- `src/index.ts`: source locations, diagnostics, source file, and compile artifact contracts.

## Runtime Behaviors

The package is intentionally dependency-light and has no filesystem or browser runtime behavior.

## Integration Notes

Keep this package stable and low-level. Higher-level compiler phases should depend on these types instead of redefining source spans or diagnostics.
