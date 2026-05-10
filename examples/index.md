# Examples Module

## Purpose

`examples` contains checked `.mao` programs used by CLI, IDE, and v1 acceptance smoke tests.

## Current Directory Structure

- `main.mao`: representative v1 acceptance program for manual CLI and IDE checks.
- `hello_world.mao`: smallest runtime logging fixture; `maodie run` prints `Hello world` through `core.log`.
- `v1_acceptance.mao`: the canonical v1 end-to-end success fixture.
- `v1_surface.mao`: syntax and type-system surface fixture covering declarations, generics, trait/impl, Option/Result, and `?`.
- `v1_error.mao`: stable error fixture for Chinese diagnostic and exit-code checks.

## Key Behaviors

`main.mao` and `v1_acceptance.mao` intentionally match so common commands keep exercising the full v1 closure. `hello_world.mao` keeps the host logging path small and direct. Tests read these files directly instead of keeping hidden copies of the same source.

The v1 examples stay inside the accepted language boundary. They do not imply package management, async, native codegen, or a full standard library.
