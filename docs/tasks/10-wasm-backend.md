# 10 WASM Backend

## 目标

实现 MIR 到 WAT/WASM 的 v1 后端，能编译基础 `.mao` 程序并验证运行结果。

## 对应特性

WASM-first 编译目标和 v1 可运行闭环的核心后端。

## 前置输入

- 任务 08 的 MIR 数据结构和 lowering API。
- 任务 09 的 core stdlib 与 WASM runtime glue 约定。

## 实现范围

- 设计 MIR 类型到 WASM 类型的映射。
- 生成 WAT debug output 和 WASM binary artifact。
- 支持函数、局部变量、整数/布尔运算、分支、match lowering 结果。
- 支持 core stdlib 所需的最小 runtime glue。
- 建立 golden tests，对源码、MIR、WAT 和运行结果做验证。

## 不做事项

- 不做 LLVM/native backend。
- 不做高级优化。
- 不做完整 GC runtime。

## 输出产物

- WASM codegen crate 或模块。
- Artifact 类型：WAT dump、WASM binary。
- 可运行示例和 golden tests。

## 验收标准

- 一个包含函数、if、match、Result 的 `.mao` 示例能生成 WASM。
- golden test 能验证 WAT 或实际运行结果。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 WASM backend golden tests 和 workspace 测试，至少检查一个成功编译示例的 MIR、WAT、WASM artifact，以及一个运行结果验证。人工确认后端只消费 MIR 和 core runtime 契约，没有回读 AST/HIR。检查任务 11 是否记录最终 artifact 字段、diagnostics 字段和 debug dump 名称。

## 交接给下一任务

任务 11 可以把 compiler facade 暴露为 Rust/WASM API。任务 11 可以信任 backend 已能返回结构化 artifacts 和 diagnostics。

Host logging handoff: `core.log("...")` is recognized by symbol path `core.log` and lowered to the imported function `maodie.debug_string(ptr, len)`. v1 preserves `len` for string literal operands; non-literal `String` logs produce a backend limitation diagnostic and pass length `0`.

## 风险与注意

WASM MVP 对字符串和托管内存支持有限。v1 应先保持运行时策略简单，并把限制写入 artifacts 或诊断文档。

## MIR 输入不变量

- 后端从 `maodie_compiler::mir::MirPackage` 读取输入，不应回读 AST/HIR 表达式形状。
- `MirPackage.functions`、每个函数的 `locals` 和 `blocks` 均按稳定分配顺序排列；dump 和 golden tests 可以依赖这些 id 稳定。
- 每个 `BasicBlock` 包含零个或多个顺序 statement，并最多一个 terminator；可达正常块应以 `Goto`、`Return`、`Branch` 或 `Match` 结束。
- `if` 已表现为 `Branch` terminator；`match` 已表现为 ordered `MirMatchTarget` 列表，pattern 只包含 literal、variant 或 wildcard。
- `?` 已表现为 Result variant `Match`：`Ok` 分支使用 `ProjectVariant` 取得 payload 并继续，`Err` 分支使用 `AggregateVariant` 构造返回值并 `Return`。
- 调用、二元运算、variant aggregate、variant project 均位于 `MirRvalue`；后端只需要消费 local、operand、rvalue 和 terminator，不需要重建表达式树。
- `MirInstantiation` 携带 type checker 记录的泛型替换，作为后续单态化或 specialized codegen 的入口。

## Core runtime 输入约定

- core 标准库由 `maodie_compiler::core` 提供，源码模块名为 `core`。
- `Option<T>`、`Result<T,E>` 和 `Slice<T>` 的源码形状以 `docs/core-stdlib.md` 为准；`Result.Ok` / `Result.Err` 已被 type checker 和 MIR lowering 特殊识别。
- `String` 是编译器内建类型。WASM 边界按 UTF-8 `(ptr: i32, len: i32)` 借用切片表示，字符串字面量应降低为确定性的只读 UTF-8 字节区域。
- `Slice<T>` 的 WASM 表示为 `{ ptr: i32, len: i32 }`，其中 `len` 是元素数量。v1 不要求数组字面量、索引、扩容或集合运行时。
- host module 名为 `maodie`，最小保留 imports 为 `panic(ptr: i32, len: i32)` 和 `debug_string(ptr: i32, len: i32)`。
- 生成的 WASM module 应导出线性内存 `memory`。这些名字同时由 `maodie_compiler::core` 常量公开。

## 交接记录

状态：已完成。

实现记录：

- 新增 `maodie_compiler::wasm` 后端，公开 `compile_mir_to_wasm`、`WasmBackend`、`WasmArtifacts`、`WAT_DUMP_NAME` 和 `WASM_BINARY_NAME`。
- `MirPackage` 现在携带后端所需的类型形状和符号元数据，后端只消费 MIR 包，不回读 AST/HIR。
- WASM v1 布局先把 `i32`、`bool`、字符串/切片句柄和 enum/Result 值映射为 `i32`；一字段 variant 使用低 8 位 tag 和高位 payload 编码。
- 生成 WAT debug dump、WASM binary、线性内存 export，以及 `maodie.panic` / `maodie.debug_string` imports。
- golden test 覆盖源码到 typeck、MIR、WAT、WASM binary，并验证一个包含函数、if、match、Result/`?` 的示例运行结果。
- 当前 v1 诊断会记录托管内存、完整 GC、多字段 enum 完整布局尚未实现的限制。

验证：

- `cargo fmt --all --check`：通过。
- `cargo test --workspace`：通过。
