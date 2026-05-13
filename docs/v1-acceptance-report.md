# Maodie v1 Acceptance Report

## Scope

Maodie v1 is accepted as a runnable loop from `.mao` source through the Rust compiler, WASM API, TypeScript wrapper, CLI, and browser IDE. The suite validates representative source examples, stable Chinese diagnostics, WAT/WASM artifact emission, debug dumps, and IDE rendering.

## Acceptance Fixtures

- `examples/main.mao` and `examples/v1_acceptance.mao`: canonical success source used by CLI and IDE smoke paths. It covers functions, `let mut` plus assignment, `if`, `match`, `struct`, `enum`, `trait`/`impl`, generic function calls, `Option`, `Result`, and `?`.
- `examples/v1_surface.mao`: additional declaration surface fixture for HIR/dump checks.
- `examples/v1_error.mao`: parser/lexer error fixture that must return a non-zero CLI exit and stable Chinese diagnostics including `MD0101` and `MD0201`.

## Automated Coverage

- CLI acceptance tests compile `examples/v1_acceptance.mao` to WAT and WASM, verify AST/HIR/MIR dump emission, run `examples/hello_world.mao` through `core.log`, and check `examples/v1_error.mao` exit code plus Chinese diagnostic formatting.
- IDE smoke tests compile the browser default source through `@maodie/compiler-wasm`, render success status and dump tabs, render the error fixture diagnostics, and assert the default source stays aligned with `examples/v1_acceptance.mao`.
- Existing Rust and wrapper tests continue to validate the compiler facade, WASM JSON contract, backend WAT/WASM output, and Result/`?` lowering.

## Supported In V1

- Single-source modules with imports from the built-in `core` module.
- Functions, local `let` bindings, `let mut`, assignment, integer and boolean expressions.
- `if` expressions and `match` expressions over literals, wildcard/binding patterns, and enum variant paths.
- Struct, enum, trait, and impl declarations at the checked HIR/type-system level.
- Generic type declarations and generic function calls used by the acceptance fixtures.
- Core `Option<T>`, `Result<T,E>`, `String`, and `Slice<T>` declarations.
- `?` propagation for `Result<T,E>`.
- WAT and WASM artifacts plus AST, HIR, MIR, WAT, and type dumps.
- Runtime logging through `core.log("...")` and minimal `core.log("{}", value)` formatting, lowered to `maodie` debug chunk imports.
- CLI diagnostics with Chinese severity labels and stable `MD####` codes.
- Browser IDE editing, compile status, diagnostics, artifact metadata, and dump tabs.

## Deferred After V1

- Native backend and LLVM integration.
- Async/await, scheduler, and concurrency runtime.
- Package manager, project graph build, multi-file module loading, and dependency resolution.
- Complete standard library beyond the small `core` contract.
- Borrow checker, trait objects, dynamic dispatch, full trait solver, and advanced generic bounds.
- Full managed allocation/GC and rich string or collection runtime.
- Large benchmark suite, release packaging, website, and version distribution.

## Validation Log

- Initial oversized examples failed because v1 `match` patterns currently use enum variant paths rather than `Variant(payload)` syntax. The fixtures were corrected to stay inside the implemented parser/type-checker boundary.
- Initial IDE default smoke failed for the same payload-pattern reason. The default source now matches `examples/v1_acceptance.mao`.
- Running `pnpm build` and `pnpm test` at the same time produced a Cargo `target/` race with missing temporary object/WASM files. Re-running the required commands sequentially passed.
- `cargo fmt --all --check`: passed.
- `pnpm style:guard`: passed.
- `pnpm rust:test`: passed.
- `pnpm build`: passed.
- `pnpm test`: passed.
- `pnpm ide:build`: passed.
- `pnpm nx run cli:test`: passed.
- `pnpm nx run ide:test`: passed.
- `node packages/cli/dist/main.js compile examples/v1_acceptance.mao --emit wasm --out /private/tmp/maodie-v1-acceptance.wasm`: passed.
- `node packages/cli/dist/main.js compile examples/v1_error.mao --emit wat`: returned exit code 1 with `MD0101` and `MD0201`, as expected.
