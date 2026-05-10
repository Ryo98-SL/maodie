# Repository Module

## Purpose

The repository module owns shared workspace configuration for the Maodie language monorepo.

## Current Directory Structure

- `apps/`: browser-facing applications, starting with the IDE shell.
- `crates/`: Rust compiler-core Cargo workspace crates.
- `docs/`: project documentation and staged v1 task handbooks.
- `packages/`: reusable Maodie packages for compiler, CLI, core language types, and IDE contracts.
- `tools/`: local maintenance scripts that support repository checks.
- `Cargo.toml`: Rust workspace manifest for `maodie_*` compiler crates.
- `nx.json`: Nx task graph and caching configuration.
- `pnpm-workspace.yaml`: pnpm workspace package discovery.
- `tsconfig.base.json`: shared TypeScript compiler options and path aliases.

## Key Behaviors

Nx coordinates project tasks through each project's `project.json`. pnpm owns package installation and workspace linking. Cargo owns Rust crate discovery and writes build artifacts to the ignored `target/` directory.

## Integration Notes

Keep root configuration changes small and document any new shared scripts here and in `README.deep.md`. Rust Nx targets live on project `rust`, with `rust:check` as the main root validation entry point. For implementation planning, update `docs/tasks` instead of burying handoff details in chat.
