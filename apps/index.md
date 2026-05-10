# Apps Module

## Purpose

The apps module contains runnable Maodie user experiences.

## Current Directory Structure

- `ide/`: Vite-powered Web IDE shell.

## Key Behaviors

Apps should consume reusable logic from `packages/` and avoid owning compiler or language-service behavior directly.

## Integration Notes

Each app owns its own `project.json`, package metadata, and module documentation.
