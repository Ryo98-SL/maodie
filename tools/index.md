# Tools Module

## Purpose

The tools module contains local repository maintenance scripts.

## Current Directory Structure

- `ide-highlight-smoke.mjs`: connects to an already-running Chrome DevTools endpoint and runs the Web IDE final highlight smoke suite against a local IDE URL.
- `style-guard.mjs`: validates the initial documentation and workspace skeleton.

## Key Behaviors

Scripts here are invoked from root package scripts or final acceptance notes. `ide-highlight-smoke.mjs` is intentionally project-specific because it documents the Web IDE browser smoke contract while the IDE app does not yet own end-to-end browser test infrastructure.

## Integration Notes

When a script becomes package-specific, move it into that package and expose it through the package's Nx targets.
