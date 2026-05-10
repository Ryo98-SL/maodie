# Language Core Source Module

## Purpose

The source module exports stable language-domain contracts for the rest of the monorepo.

## Current Directory Structure

- `index.ts`: public TypeScript API for diagnostics, source files, spans, and artifacts.

## Key Behaviors

`createSourceFile` normalizes raw text into a shared `SourceFile` object used by compiler and tooling.

## Integration Notes

Add new primitive types here only when they are shared across packages.
