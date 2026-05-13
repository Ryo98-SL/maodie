# 09 Core Stdlib

## 目标

实现 Maodie v1 core 标准库的最小契约，支撑基础类型、`Option`、`Result`、字符串/数组最小能力和 WASM runtime glue。

## 对应特性

语言基础库和运行时边界。

## 前置输入

- 任务 07 的 Option/Result 语义。
- 任务 08 的 MIR 对运行时和内建类型的需求。

## 实现范围

- 定义 core 标准库源码或编译器内建库加载方式。
- 提供 `Option<T>`、`Result<T,E>`、`Ok`、`Err`、`Some`、`None`。
- 明确 `String` 的 v1 表示和 WASM 边界限制。
- 明确数组或切片的 v1 最小能力。
- 定义 WASM host imports/runtime glue 的最小接口。

## 不做事项

- 不做完整 IO、文件系统、网络、时间、路径库。
- 不做包管理器和第三方依赖。
- 不做高性能 GC 或复杂内存优化。

## 输出产物

- core stdlib 文件或内建 crate。
- stdlib 类型与编译器内建语义对齐文档。
- core stdlib 测试示例。

## 验收标准

- `Option`/`Result` 作为普通源码或稳定内建被编译器识别。
- 字符串字面量在 WASM 后端有明确表示策略。
- core 示例通过类型检查并能进入 MIR。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 core stdlib 测试和 workspace 测试，检查 `Option`、`Result`、字符串字面量和数组最小能力示例。人工确认 core 类型与任务 07 的编译器已知语义完全同名同形，且 WASM runtime glue 的 imports/exports 约定写入任务 10。确认没有把 IO、文件系统或包管理扩进 v1。

## 交接给下一任务

任务 10 可以依赖 core stdlib 的运行时约定生成 WASM。任务 10 不需要重新定义字符串、Result 或 host import 表示。

## 风险与注意

如果 core 类型既是源码库又被编译器特殊识别，必须记录特殊识别点，避免后续维护者误以为它完全普通。

## 交接记录

状态：已完成。

实现记录：

- 新增 `maodie_compiler::core` 模块，提供 `CORE_SOURCE`、`core_source()`、`resolve_source_with_core()`、`check_source_with_core()` 等显式 core 加载 API。
- core 源码模块名为 `core`，声明 `Option<T> { Some(T), None }`、`Result<T,E> { Ok(T), Err(E) }`、`Slice<T> { ptr: i32, len: i32 }` 和 host-backed `fn log(message: String) -> unit;`。
- `String` 保持为编译器内建类型；字符串字面量在 MIR 中保持 literal 常量，WASM 边界按 UTF-8 `(ptr: i32, len: i32)` 借用切片表示。
- `Result` 的特殊识别点保持和任务 07/08 对齐：类型检查和 MIR lowering 均按最终名称 `Result`、变体 `Ok`/`Err`、两个泛型参数识别。
- WASM host glue 最小约定写入 `docs/core-stdlib.md` 和 `docs/tasks/10-wasm-backend.md`，并在 `maodie_compiler::core` 中公开 `maodie`、`panic`、`debug_string`、`memory` 常量，供任务 10 使用。`core.log("...")` 由任务 10 的 WASM backend 降到 `debug_string(ptr, len)`。
- 任务 15 已扩展 `core.log` 为 debug chunk imports 和最小 `{}` 插值；读取当前日志契约时以 `docs/core-stdlib.md` 和 `docs/tasks/15-core-log-formatting.md` 为准。

验证：

- core 测试覆盖 import 解析、Option/Result/String/Slice/log 类型检查、core `Result` 的 `?` MIR lowering、`core.log("Hello world")` host lowering 和 WASM glue 常量。
- `cargo test --workspace` 通过。
