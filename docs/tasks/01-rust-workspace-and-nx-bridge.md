# 01 Rust Workspace and Nx Bridge

## 目标

建立 Rust 编译器核心的 workspace，并让 Nx 能统一编排 Cargo build、test、wasm 构建和现有 TypeScript 项目任务。

## 对应特性

工具链基础设施。这个任务不实现语言语义，只建立后续 Rust 编译器任务的工程地基。

## 前置输入

- `docs/tasks/README.md` 的任务顺序和交接协议。
- 当前 Nx/pnpm/TypeScript monorepo 配置。

## 实现范围

- 新增 `crates/` Rust workspace，包含最小 compiler facade crate 和 workspace-level Cargo 配置。
- 为 Rust workspace 配置统一格式、lint、test、build 命令。
- 在 Nx 中暴露 Rust 相关 target，例如 `rust:build`、`rust:test`、`rust:check`。
- 明确 Rust crate 命名约定：`maodie_*`。
- 记录 Cargo 与 Nx 的缓存边界和输出目录。

## 不做事项

- 不实现 lexer、parser、诊断模型或 WASM API。
- 不引入 LLVM/native backend。
- 不迁移 TypeScript CLI/IDE 行为。

## 输出产物

- Rust workspace 基础文件。
- 最小 Rust crate，可被 Cargo build/test。
- Nx target，可从根目录统一执行 Rust 检查。
- 更新后的 README、README.deep、相关 `index.md`。

## 验收标准

- `cargo test --workspace` 成功。
- `pnpm nx run rust:check` 或等效 Nx target 成功。
- `pnpm build` 不因 Rust workspace 加入而失败。
- 文档说明如何从 Nx 进入 Rust 任务。

## 完成后验收方式

复验者从干净终端执行 Rust 和 Nx 两条入口命令，确认 Cargo 与 Nx 都能发现同一组 Rust crate。检查根 README、README.deep 和相关 `index.md` 是否记录了 Rust workspace、Nx target 名称和输出目录。打开任务 02，确认其 `前置输入` 中提到的 crate 命名和 target 名称与实际实现一致。

## 交接给下一任务

任务 02 可以假设 Rust workspace 已存在，且可以新增诊断/source model crate。任务 02 必须读取本文件的 `交接记录`，确认 crate 命名、Nx target 名称和 Cargo workspace 布局。

## 风险与注意

Cargo 和 Nx 的缓存模型不同，避免让 Nx target 写入不可预测路径。Rust 输出应保持在 Cargo 默认 target 或明确的 ignored 目录中。

## 交接记录

状态：已完成。

完成摘要：

- 新增根 `Cargo.toml` 作为 Rust workspace，当前成员为 `crates/maodie_compiler`。
- 新增最小 Rust compiler facade crate `maodie_compiler`，提供 `CompilerFacade` 元数据 API 和单元测试。
- 新增 Nx 项目 `rust`，从 `crates/project.json` 暴露 `build`、`test`、`check`、`fmt`、`lint`、`wasm-build` targets。
- 更新根 README、README.deep、`index.md`、`docs/index.md`、`docs/tasks/index.md`，记录 Rust workspace、Nx target 名称、crate 命名约定和输出目录。
- 更新任务 02 的前置输入，明确可依赖的 crate 名称和 Nx/Cargo target。

公共接口变更：

- Rust crate 命名约定为 `maodie_*`。
- 当前 facade crate 为 `maodie_compiler`。
- Cargo workspace 入口为仓库根 `Cargo.toml`，Rust crate 源码位于 `crates/`。
- Nx 入口项目名为 `rust`；主要验证命令为 `pnpm nx run rust:check`。
- Cargo/Nx 输出边界为 ignored `target/`；TS package build 仍使用各自 `dist/` 或根 `dist/`。

测试命令结果：

- `cargo test --workspace`：通过。
- `pnpm nx run rust:check`：通过。
- `pnpm build`：通过。
- `pnpm nx run rust:wasm-build`：通过。首次执行前本机缺少 `wasm32-unknown-unknown` target，已通过 `rustup target add wasm32-unknown-unknown` 安装后复验通过。

已知限制：

- `rust:wasm-build` 只验证 Rust crate 能面向 `wasm32-unknown-unknown` 编译；本任务不提供 WASM API 或 JS/TS wrapper。
- 本任务不实现 lexer、parser、诊断模型、WASM API 或 TS 行为迁移。

下一任务入口：

- 任务 02 从根 `Cargo.toml` 和 `crates/maodie_compiler` 开始，按 `maodie_*` 约定新增 diagnostics/source model crate，并继续使用 `cargo test --workspace` 与 `pnpm nx run rust:check` 验收。
