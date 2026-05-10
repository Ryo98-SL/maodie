# Maodie Program Language

Maodie is a planned compiled programming language with a companion IDE. This repository starts as an Nx-powered monorepo so the compiler, command line tooling, IDE shell, and shared protocol contracts can evolve together without losing clear project boundaries.

## Projects

- `packages/language-core`: shared source, span, diagnostic, and artifact types.
- `packages/compiler`: the public compiler entry point and future compile pipeline.
- `packages/compiler-wasm`: TypeScript wrapper that loads the Rust WASM compiler in Node or browsers.
- `packages/cli`: the `maodie` command line shell around the compiler.
- `packages/ide-protocol`: shared contracts between IDE clients and language services.
- `apps/ide`: a Vite-powered Web IDE that edits `.mao` source, switches between built-in examples, loads the WASM compiler, and shows diagnostics plus AST/HIR/MIR/WAT dumps.
- `crates/maodie_compiler`: Rust compiler facade crate in the Cargo workspace.
- `crates/maodie_wasm_api`: low-level WebAssembly ABI around the Rust compiler facade.
- `docs/tasks`: v1 implementation task handbook with handoff rules.

## Common Tasks

```bash
pnpm install
pnpm build
pnpm typecheck
pnpm test
pnpm rust:check
pnpm ide:dev
pnpm graph
```

Rust tasks can also be invoked through Nx directly, for example `pnpm nx run rust:build`, `pnpm nx run rust:test`, `pnpm nx run rust:check`, and `pnpm nx run rust:wasm-build`. Cargo build artifacts stay in the ignored `target/` directory. The compiler WASM wrapper builds and loads `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm` by default in Node; the browser IDE imports that same build output through Vite's `?url` asset handling.

The current v1 path has a Rust compiler core, WASM wrapper, CLI shell, and browser IDE compile loop. After building, `node packages/cli/dist/main.js run examples/hello_world.mao` prints `Hello world` through `core.log`. Full language-service features such as completion, hover, jump-to-definition, and multi-file indexing remain future extension points.

## V1 Acceptance

The canonical success fixture is `examples/v1_acceptance.mao`, mirrored by `examples/main.mao` and the IDE default source. The IDE also includes simpler tabs for Hello World, function calls, and Fibonacci so the browser workbench can demonstrate smaller language slices. `examples/hello_world.mao` is the CLI runtime logging fixture. The v1 fixture exercises functions, local mutation, `if`, `match`, declarations, generics, core `Option`/`Result`, and `?` through the shared Rust/WASM compiler.

```bash
pnpm rust:test
pnpm build
pnpm test
node packages/cli/dist/main.js compile examples/v1_acceptance.mao --emit wat
node packages/cli/dist/main.js run examples/hello_world.mao
pnpm ide:build
```

The v1 support/deferred capability list and validation notes live in `docs/v1-acceptance-report.md`.

## V1 Task Handbook

The Maodie v1 work is split into staged handoff tasks under `docs/tasks`. Start with `docs/tasks/README.md`, then follow each task file in order unless the README marks a downstream task as parallel-ready.
