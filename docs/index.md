# Docs Module

## Purpose

The docs module owns project documentation that guides Maodie language design, implementation sequencing, and contributor handoff.

## Current Directory Structure

- `tasks/`: staged Maodie v1 implementation handbook, including Rust workspace and Nx handoff records.
- `v1-acceptance-report.md`: completed v1 support/deferred capability report and acceptance validation log.
- `core-stdlib.md`: v1 core library contract, including `Option`, `Result`, `Slice`, `String`, and `core.log`.

## Key Files

- `tasks/README.md`: task graph, handoff protocol, and completion rules for Maodie v1.
- `tasks/01-rust-workspace-and-nx-bridge.md`: Rust workspace, crate naming, and Nx bridge entry points.
- `v1-acceptance-report.md`: v1 closure checklist for examples, CLI, IDE, diagnostics, and future scope.

## Main Behaviors

Documentation in this module should reduce implementation ambiguity. Task documents are written as executable briefs: each one states what to build, what to avoid, how to validate it, and how to hand off to the next task.

## Integration Notes

Keep language decisions that affect implementation synchronized with `README.deep.md` and the relevant task file. Rust infrastructure changes must also keep task 01 and direct downstream task inputs aligned.
