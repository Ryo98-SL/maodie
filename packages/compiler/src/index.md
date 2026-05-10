# Compiler Source Module

## Purpose

The compiler source module exposes the initial compile pipeline API.

## Current Directory Structure

- `index.ts`: public compiler API and placeholder compile implementation.
- `index.test.ts`: Vitest coverage for the initial API contract.

## Key Behaviors

`compileMaodie` accepts source text and compile options, returns diagnostics, and produces a deterministic placeholder IR artifact when input is present.

## Integration Notes

Split `index.ts` into dedicated phase modules before adding substantial lexer, parser, checker, or backend logic.
