# V1 Tasks Module

## Purpose

The v1 tasks module breaks Maodie implementation into staged Markdown handbooks.

## Current Directory Structure

- `README.md`: task overview, dependency model, and shared handoff protocol.
- `01-*.md` through `14-*.md`: implementation tasks in dependency order. Task 01 defines the Rust workspace and Nx project `rust`; task 14 closes v1 with `docs/v1-acceptance-report.md`.

## Key Behaviors

Each task file is the source of truth for that stage. A task starts by reading its own file and upstream handoff records, and ends by updating its own `交接记录`.

Rust task entry points currently flow through Cargo at the repository root and Nx project `rust`, with `rust:check` as the main bridge validation target.

## Integration Notes

When a public interface changes, update every directly affected downstream task before closing the current task.
