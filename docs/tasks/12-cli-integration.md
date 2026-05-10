# 12 CLI Integration

## 目标

实现 `maodie compile` CLI，把 `.mao` 文件编译为 WASM 或 debug dumps，并输出中文诊断。

## 对应特性

命令行工具链和本地开发入口。

## 前置输入

- 任务 11 的 TS wrapper API 和 TypeScript 类型。
- 当前 `packages/cli` 项目结构。

## 实现范围

- 提供 `maodie compile path/to/main.mao --emit wasm`。
- 支持 emit：`wasm`、`wat`、`ast`、`hir`、`mir`。
- 读取文件、调用 compiler-wasm、打印中文诊断。
- 成功时写出 artifact 或输出到 stdout。
- 失败时返回非零 exit code。

## 不做事项

- 不做包管理器。
- 不做 watch mode。
- 不做项目级 build graph。

## 输出产物

- CLI command implementation。
- CLI 参数和错误输出文档。
- Smoke tests 覆盖成功编译和诊断失败。

## 验收标准

- `maodie compile examples/main.mao --emit wat` 能输出 WAT。
- 空文件或语法错误返回非零 exit code，并显示中文错误码。
- `pnpm test` 成功。

## 完成后验收方式

复验者运行 CLI smoke tests 和 `pnpm test`，手动执行一次成功编译和一次错误源码编译。检查 exit code、stdout/stderr 分流、中文错误码、`--emit` 输出文件或 stdout 行为是否与文档一致。确认 CLI 只调用任务 11 的 TS wrapper，不直接访问 Rust/WASM 内部路径。

## 交接给下一任务

任务 14 可以使用 CLI 作为端到端验收入口。任务 13 不依赖 CLI，但可复用 CLI 示例源码。

## 风险与注意

CLI 不应绕过 TS wrapper 直接访问 Rust/WASM 内部路径，否则 IDE 和 CLI 会产生两套行为。

## 交接记录

状态：已完成。

实现记录：

- `packages/cli` 现在提供 `maodie compile <source.mao> --emit <wasm|wat|ast|hir|mir> [--out <path>]`。
- CLI 只调用 `@maodie/compiler-wasm` 的 TS wrapper，不直接访问 Rust/WASM 内部路径。
- 文本输出 `wat`、`ast`、`hir`、`mir` 默认写入 stdout；`wasm` 默认写入 wrapper artifact 文件名 `module.wasm`；`--out`/`-o` 可把任意 emit 写入指定路径。
- 诊断统一写入 stderr，格式为中文严重级别加稳定错误码，例如 `错误[MD0201]`、`错误[MD0001]`、`警告[MD9001]`。
- WASM API 响应补充 `ast` dump，并把空源码作为 `MD0001` 错误返回，保证 CLI 和后续 IDE 共用同一 wrapper 行为。
- 新增 `examples/main.mao`，可直接用于手动验收命令。
- 新增 CLI smoke tests 覆盖 WAT stdout、WASM 文件输出、语法错误、空文件和 AST/HIR/MIR dump。

验证：

- `cargo fmt --all --check`：通过。
- `pnpm nx run cli:test`：通过。
- `pnpm nx run compiler-wasm:test`：通过。
- `pnpm nx run cli:build`：通过。
- `pnpm test`：通过。
- 手动执行 `node packages/cli/dist/main.js compile examples/main.mao --emit wat`：exit code 0，输出 WAT。
- 手动执行 `node packages/cli/dist/main.js compile /private/tmp/maodie-empty.mao --emit wat`：exit code 1，输出 `错误[MD0001]: Maodie 源文件为空。`。
