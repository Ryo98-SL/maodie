# Maodie Program Language Deep README

## Repository Shape

This repository is an Nx monorepo using pnpm workspaces. Project configuration lives next to each package or app in `project.json`, while shared task behavior lives in `nx.json`.

Root package scripts run Nx with `NX_DAEMON=false` and `NX_ISOLATE_PLUGINS=false`. This keeps project graph calculation in-process, which is friendlier to the current local sandbox while preserving Nx task orchestration and caching.

The Rust compiler core is a Cargo workspace rooted at the repository `Cargo.toml`. Rust crates live under `crates/` and use the `maodie_*` naming convention; task 01 establishes `crates/maodie_compiler` as the minimal facade crate.

## Task Orchestration

- `build`: compiles each project and runs dependency builds first through Nx `dependsOn`.
- `typecheck`: validates project TypeScript using the same project graph ordering.
- `test`: runs Vitest where tests exist and allows empty projects to keep new modules cheap.
- `dev`: currently exists on `apps/ide` for local Vite development.
- `rust:build`, `rust:test`, `rust:check`, and `rust:wasm` are package scripts that delegate to Nx project `rust`.

Nx exposes the Rust bridge as project `rust` from `crates/project.json`:

- `pnpm nx run rust:build`: `cargo build --workspace`.
- `pnpm nx run rust:test`: `cargo test --workspace`.
- `pnpm nx run rust:check`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo test --workspace`.
- `pnpm nx run rust:wasm-build`: `cargo build --workspace --target wasm32-unknown-unknown`.

Cargo owns crate-level incremental artifacts and writes them to the ignored `target/` directory. Nx treats `target/` as the Rust build output boundary and does not try to relocate Cargo's own cache. The WASM target requires the Rust toolchain component `wasm32-unknown-unknown`.

## Project Boundaries

- `packages/language-core` should stay dependency-light and contain stable language-domain primitives.
- `packages/compiler` owns the compile pipeline API and depends on `language-core`.
- `packages/compiler-wasm` owns the TypeScript wrapper over the Rust WASM compiler ABI. It exposes compile and lexer-backed highlight helpers while sharing the same Node/browser WASM loader. Node tools may use its default ignored Cargo output path, while browser apps should provide their own asset URL, bytes, module, or instance.
- `packages/cli` owns process, filesystem, and terminal behavior for the command line.
- `packages/ide-protocol` owns contracts shared by UI and future language-service code.
- `apps/ide` owns browser UI, built-in workbench examples, Monaco editor integration, semantic-token highlighting, live marker diagnostics, and manual Run orchestration, and depends on `@maodie/compiler-wasm` for browser-side compilation and lexer-backed highlight worker sessions instead of copying language behavior.
- `crates/maodie_compiler` owns the Rust compiler facade and is the first Rust entry point for future compiler-core work.
- `crates/maodie_wasm_api` owns the private pointer-level WASM ABI that returns the public JSON compile and highlight responses consumed by `packages/compiler-wasm`.

Library packages emit build output into local `dist/` folders so package `exports`, `main`, and CLI `bin` entries remain valid when workspace packages are linked by pnpm.

## Documentation Convention

Each module folder has an `index.md` describing purpose, layout, and integration boundaries. When a direct child directory is added inside a module, its own `index.md` should be created or updated in the same change.

The v1 task handbook lives in `docs/tasks`. Each task file is both an implementation brief and a handoff contract. A task is not complete until its `交接记录` section names completed outputs, public interface changes, validation commands, known limits, and the next task entry point.

## Future Implementation Slots

The compiler package is ready for dedicated folders such as `lexer`, `parser`, `checker`, `ir`, and `backend` once the language design is finalized. The IDE app is ready to connect to a language-service layer through `packages/ide-protocol`.

The current v1 route is Rust compiler core, hand-written parser, AST/HIR/MIR internal IR, WASM-first backend, TypeScript wrapper packages, and browser IDE integration.

## V1 Acceptance Suite

Task 14 closes v1 with checked examples and smoke tests instead of new language features. `examples/v1_acceptance.mao` is the canonical success fixture and is mirrored by `examples/main.mao` plus `apps/ide/src/examples.ts`'s `defaultSource`. The IDE source catalog also carries simpler Hello World, function-call, and Fibonacci examples for tabbed workbench switching. `examples/v1_surface.mao` keeps declaration-surface coverage visible, and `examples/v1_error.mao` locks stable Chinese diagnostics.

The acceptance report lives at `docs/v1-acceptance-report.md`. Keep it synchronized when v1 support changes, especially around supported language surface, deferred capabilities, and manual validation commands. `examples/hello_world.mao` is the smallest runtime logging fixture and uses `core.log("Hello world")`.

## WASM Compiler Boundary

Task 11 exposes the compiler through `@maodie/compiler-wasm`. Its public response shape is `CompileResponse` with `ok`, `diagnostics`, `artifacts`, and `dumps`. Artifacts currently include `module.wat` as text and `module.wasm` as `Uint8Array`. Dumps currently use `ast`, `hir`, `types`, `mir`, and `wat` keys when compilation reaches those stages.

The same WASM boundary exposes `highlightMaodieSource` and `MaodieCompilerWasm.highlight` for syntax highlighting. Highlight responses contain `ok`, `tokens`, and `diagnostics` only; they call the Rust syntax highlight API and do not parse, type-check, lower, emit artifacts, or populate dumps.

Syntax highlight fixtures live in `docs/tasks/highlighting/fixtures/`. `syntax-highlight.mao` and `syntax-highlight.tokens.json` are shared by Rust and TS tests to keep keyword, identifier, literal, comment, Chinese identifier, and error-token ranges aligned across runtime boundaries.

`packages/compiler-wasm/src/ranges.ts` converts Rust UTF-8 byte ranges into editor-facing UTF-16 absolute offsets or 0-based line/character positions. Adapters should run highlight tokens through these helpers before handing ranges to browser editors, VSCode, or other UTF-16 APIs.

The source-level `core.log(message: String) -> unit` function is host-backed and compiler-recognized for minimal formatting. WASM codegen lowers `core.log("Hello world")` and formatted calls such as `core.log("value is {}", value)` to debug chunk imports: `debug_string`, `debug_i32`, `debug_bool`, and `debug_log_end`. CLI `maodie run` and the browser IDE evaluation host collect chunks until `debug_log_end` and show one resulting log line.

The Rust crate `maodie_wasm_api` exports `maodie_alloc`, `maodie_dealloc`, `maodie_compile`, `maodie_highlight`, `maodie_response_len`, `maodie_response_bytes`, and `maodie_free_response`. That ABI is intentionally not a public app contract; downstream packages should call the TypeScript wrapper.

`apps/ide` imports `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm?url` through Vite. In dev, Vite serves that imported workspace asset; in production, Vite copies it into `dist/apps/ide/assets` and rewrites the runtime URL. Monaco passes the same URL to the highlight worker session for live lexer-backed semantic tokens and marker diagnostics. Stale highlight worker responses are filtered by editor/session version, and update protocol errors recover with a current-document reset so version mismatch diagnostics do not overwrite the latest live lexer state. Task 13 keeps compilation on the browser main thread and records that limitation in the IDE module docs so a later worker transport can replace only the compiler client boundary.

`tools/ide-highlight-smoke.mjs` is the final Web IDE incremental highlighting browser smoke harness. It expects a running IDE dev server and a Chrome DevTools endpoint, drives Monaco through `window.maodieIdeEditor`, then verifies default/source-injected highlighting, live lexer diagnostics, example switching, Run compilation, continuous-input responsiveness, and stale worker response settling.
