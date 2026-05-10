# 11 WASM API and TS Wrapper

## 目标

把 Rust 编译器核心编译为 WASM，并提供 TypeScript wrapper，让 Node CLI 和浏览器 IDE 调用同一套编译器 API。

## 对应特性

Rust compiler core 与 TypeScript 工具层的稳定边界。

## 前置输入

- 任务 10 的 compiler facade、diagnostics、artifacts、WAT/WASM 输出。
- 当前 pnpm/Nx/TypeScript workspace。

## 实现范围

- 新增 wasm API crate，导出 `compile(source, options)`。
- 定义 TS wrapper package，例如 `packages/compiler-wasm`。
- 定义 `CompileOptions`、`CompileResponse`、`Diagnostic`、`Artifact` 的 TS 类型。
- 支持 Node 和浏览器加载 wasm。
- Nx target 先构建 Rust wasm，再构建 TS wrapper。

## 不做事项

- 不做 CLI 用户体验。
- 不做 IDE 页面交互。
- 不做多线程 WASM worker 优化。

## 输出产物

- Rust wasm API crate。
- TS wrapper package。
- wasm build artifacts 的 ignored 输出路径和加载约定。
- Node 环境集成测试。

## 验收标准

- TS 测试能加载 wasm 并编译 `.mao` 字符串。
- 返回值包含 `ok`、`diagnostics`、`artifacts`、`dumps`。
- `pnpm build` 和 `pnpm test` 成功。

## 完成后验收方式

复验者运行 Rust wasm 构建、TS wrapper 测试、`pnpm build` 和 `pnpm test`。人工检查 Node 和浏览器加载路径是否都有说明，`CompileOptions` 与 `CompileResponse` 字段是否在 TS 类型和任务 12/13 中一致。用一个 `.mao` 字符串 smoke test 确认 wrapper 返回 diagnostics、artifacts、dumps，而不是只返回原始 wasm 内存指针。

## 交接给下一任务

任务 12 和任务 13 可以并行开始。它们都应只调用 TS wrapper，不直接调用 Rust crate 或复制 wasm 加载逻辑。

## 风险与注意

WASM API 的 JSON shape 是 CLI/IDE 的公共契约。字段命名一旦进入任务 12/13，应通过任务文件同步变更。

## 交接记录

状态：已完成。

实现记录：

- 新增 `crates/maodie_wasm_api`，以 `cdylib`/`rlib` 形式导出低层 WASM ABI：`maodie_alloc`、`maodie_dealloc`、`maodie_compile`、`maodie_response_len`、`maodie_response_bytes`、`maodie_free_response`。
- WASM API 使用 JSON 作为稳定边界，`compile(source, options)` 返回 `ok`、`diagnostics`、`artifacts`、`dumps`，不会向 TS 调用方暴露原始 wasm 内存指针。
- 新增 `packages/compiler-wasm`，公开 `CompileOptions`、`CompileResponse`、`Diagnostic`、`Artifact`、`MaodieCompilerWasm`、`createCompilerWasm`、`compileMaodieWasm`。
- Node 默认加载 ignored Cargo 输出 `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm`；浏览器调用方可传入 `wasmUrl`、`wasmBytes`、`wasmModule` 或已实例化的 `instance`。
- `compiler-wasm` 的 Nx `build`/`test` target 会先运行 `cargo build -p maodie_wasm_api --target wasm32-unknown-unknown`，再构建或测试 TS wrapper。
- artifacts 当前包含文本 `module.wat` 和二进制 `module.wasm`；dumps 当前包含 `hir`、`types`，成功编译时追加 `mir` 和 `wat`。

验证：

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_wasm_api`：通过。
- `pnpm nx run compiler-wasm:typecheck`：通过。
- `pnpm nx run compiler-wasm:test`：通过，Node 集成测试会加载真实 `.wasm` 并编译 `.mao` 字符串。

后续入口：

- 任务 12 CLI 应只调用 `@maodie/compiler-wasm`，使用 `CompileResponse.artifacts` 输出 `wasm`/`wat`，使用 `diagnostics` 打印中文错误。
- 任务 13 IDE 应只调用 `@maodie/compiler-wasm`，在 Vite 开发和生产环境中传入浏览器可访问的 WASM asset URL 或 bytes。
