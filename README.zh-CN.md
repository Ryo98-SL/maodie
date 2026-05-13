<p align="center">
  <img src="./assets/logo.webp" alt="Maodie 标志" width="160" height="160" />
</p>

# Maodie 编程语言

> English: [README.md](./README.md)

Maodie 是一门**实验性编程语言**，目标是把 **TypeScript 风格的类型系统**、**Rust 风格的语法**和**严格、显式的错误处理体系**结合到同一门编译语言里。仓库以 Nx 单仓库（monorepo）的形式组织，便于编译器、命令行工具、IDE 以及协议契约协同演进，同时保持清晰的项目边界。

## 语言目标

- **类 TypeScript 的类型系统**：结构化类型、泛型、联合类型，以及友好的渐进式类型体验。
- **类 Rust 的语法**：表达式优先的代码块、`fn` / `let mut` / `match`、`trait` 与 `impl`、明确的所有权与可变性提示。
- **严格的错误处理**：拒绝隐式异常；错误通过 `Result<T, E>` / `Option<T>` 流转，用 `?` 传播，由编译器以机器可检的诊断信息呈现。

## 仓库结构

| 路径 | 说明 |
| --- | --- |
| `packages/language-core` | 共享的 source、span、diagnostic、artifact 类型定义。 |
| `packages/compiler` | 公共编译器入口和后续编译流水线。 |
| `packages/compiler-wasm` | 在 Node 与浏览器中加载 Rust WASM 编译器与高亮器的 TypeScript 封装。 |
| `packages/cli` | `maodie` 命令行壳，封装编译器调用。 |
| `packages/ide-protocol` | IDE 客户端与语言服务之间的协议契约。 |
| `apps/ide` | 基于 Vite 的 Web IDE：在 Monaco 中编辑 `.mao` 源码，渲染基于 lexer 的语义高亮与实时诊断，可在内置示例间切换，按需调用 WASM 编译器，并展示 AST/HIR/MIR/WAT 等 dump。 |
| `crates/maodie_compiler` | Cargo workspace 中的 Rust 编译器门面 crate。 |
| `crates/maodie_wasm_api` | 围绕 Rust 编译器门面的低层 WebAssembly ABI。 |
| `docs/tasks` | v1 实现任务手册以及交接规则。 |

## 常用命令

```bash
pnpm install
pnpm build
pnpm typecheck
pnpm test
pnpm rust:check
pnpm ide:dev
pnpm graph
```

Rust 相关任务也可直接通过 Nx 调用，例如 `pnpm nx run rust:build`、`pnpm nx run rust:test`、`pnpm nx run rust:check`、`pnpm nx run rust:wasm-build`。Cargo 的构建产物会留在被忽略的 `target/` 目录中。编译器 WASM 封装默认在 Node 下加载 `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm`；浏览器 IDE 通过 Vite 的 `?url` 资源处理引用同一份产物，覆盖编译与实时高亮的两条 worker 通路。`@maodie/compiler-wasm` 还导出了 `highlightMaodieSource`，可在不执行完整编译流水线的情况下获取词法级别的高亮，同时提供 UTF-8 字节范围与 UTF-16 编辑器范围之间的转换工具。

如需运行 Web IDE 的增量高亮浏览器烟测，先启动 `pnpm ide:dev`，再以远程调试端口启动 Chrome，然后执行 `node tools/ide-highlight-smoke.mjs <ide-url> <chrome-devtools-url>`。烟测脚本通过 Monaco 暴露的 `window.maodieIdeEditor` 测试 API 驱动编辑器，而不是直接操作编辑器 DOM。

共享的语法高亮 fixture 位于 `docs/tasks/highlighting/fixtures/`，用于锁定 Rust / WASM / TS 三端的 token 契约，包含中文标识符、注释、字面量和错误 token 等场景。

当前 v1 通路已具备 Rust 编译器内核、WASM 封装、CLI 壳和浏览器 IDE 编译闭环。构建完成后，`node packages/cli/dist/main.js run examples/hello_world.mao` 会通过 `core.log` 打印 `Hello world`。补全、悬浮信息、跳转定义、多文件索引等完整语言服务能力，留给后续扩展。

## V1 验收

v1 的标准成功 fixture 是 `examples/v1_acceptance.mao`，对应镜像还有 `examples/main.mao` 以及 IDE 的默认源码。IDE 同时提供 Hello World、函数调用、斐波那契等简化示例标签，便于在浏览器工作台演示更小的语言切片。`examples/hello_world.mao` 是 CLI 运行时日志的 fixture。该 v1 fixture 覆盖了函数、局部可变、`if`、`match`、声明、泛型、核心 `Option` / `Result` 以及 `?` 等通过共享 Rust / WASM 编译器走通的能力。

```bash
pnpm rust:test
pnpm build
pnpm test
node packages/cli/dist/main.js compile examples/v1_acceptance.mao --emit wat
node packages/cli/dist/main.js run examples/hello_world.mao
pnpm ide:build
```

v1 的能力支持矩阵、暂缓项以及验证记录见 `docs/v1-acceptance-report.md`。

## V1 任务手册

Maodie v1 的工作被拆分为 `docs/tasks` 下的多个分阶段交接任务。请先阅读 `docs/tasks/README.md`，然后按顺序处理每个任务文件，除非 README 标记某个下游任务可以并行推进。
