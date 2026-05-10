# IDE Protocol Module

## Purpose

`ide-protocol` owns shared contracts between the Maodie IDE, future language services, and compiler-facing tools.

## Current Directory Structure

- `src/`: protocol type exports.
- `project.json`: Nx tasks and dependency metadata.
- `tsconfig.json`: TypeScript project configuration.

## Key Files

- `src/index.ts`: editor document state and compile request/response contracts.

## Runtime Behaviors

This package exports types only and should remain runtime-light.

## Integration Notes

Add language-service request and notification types here before wiring them into IDE UI code.
