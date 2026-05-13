# 15 Core Log Formatting

## 目标

实现 `core.log` 的最小 `{}` 插值打印，让调试代码可以写 `log("value is {}", value)` 并在 CLI/IDE 中得到一条完整日志。

## 对应特性

core 标准库日志格式化、直接 `String` 值 WASM 表示、CLI/IDE runtime log 捕获。

## 前置输入

- 任务 09 的 core stdlib 契约。
- 任务 10 的 WASM backend 与 host import 约定。
- 任务 12/13 的 CLI 和 IDE WASM evaluation host。

## 实现范围

- `core.log` 保持源码声明 `fn log(message: String) -> unit;`，但 type checker 对 `core.log` 做专用变长参数识别。
- 格式串必须是字符串字面量；只识别 `{}`；占位符数量必须等于后续参数数量。
- 插值参数支持直接 `i32`、`bool`、`String`。
- 直接 `String` WASM 值使用 packed `i64`：低 32 位指针，高 32 位字节长度。
- WASM backend 生成 `debug_string`、`debug_i32`、`debug_bool`、`debug_log_end` chunk calls。
- CLI/IDE host 收集 chunks，并在 `debug_log_end` 时 flush 为一条日志。

## 不做事项

- 不新增通用 `format()`。
- 不做字符串拼接、分配器、转义花括号、格式选项或命名参数。
- 不实现 `String` 作为 struct/enum/Result payload 的完整布局。

## 输出产物

- shared `core.log` format parser。
- type checker 诊断 `MD0413` 和相关测试。
- WASM `String` handle lowering、formatted log lowering 和 host import 更新。
- CLI/IDE formatted log tests。
- core stdlib、WASM backend、CLI/IDE 模块文档更新。

## 验收标准

- `log("value is {}", value)` 在 CLI/IDE 中输出单行 `value is <n>`。
- `log("value is {} {} {}", i, b, s)` 支持 `i32`、`bool`、直接 `String`。
- 非字面量格式串报 `MD0413`。
- 占位符数量不匹配报 `MD0405`。
- 非支持插值类型报 `MD0401`。
- `cargo test --workspace`、`pnpm build`、`pnpm test` 成功。

## 完成后验收方式

复验者运行 Rust workspace tests、TS build/tests、CLI formatted log 示例和 IDE compiler client tests。人工确认 `core.log` 仍不是通用格式化 API，文档明确记录无 `format()`、无转义花括号、无复合类型 String payload 布局。

## 交接给下一任务

后续完整标准库任务可以在此基础上设计 `format()`、字符串分配/拼接、转义规则和复合类型 `String` payload 布局。后续任务不需要重新定义 `core.log` 的 debug chunk host flushing 语义。

## 风险与注意

`String` 的 packed `i64` 表示只覆盖直接值传递。若后续要支持 `Result.Err(String)` 或 struct/enum 字段，需要重新设计当前 enum/aggregate 的 `i32` payload 编码。

## 交接记录

状态：已完成。

实现记录：

- 新增共享格式串 helper，`core.log` type checker 特判支持字面量格式串和 `i32`/`bool`/`String` 插值。
- 新增 `MD0413` 诊断，用于非字面量格式串。
- WASM backend 将直接 `String` 降为 packed `i64`，并通过 debug chunk imports 输出格式化日志。
- CLI 和 IDE evaluation host 收集 chunks，并在 `debug_log_end` 时生成一条 logs 记录。
- Fibonacci IDE 示例展示 `log("fib({}) = {}", value, result)`。

验证：

- `cargo test -p maodie_compiler`：通过。
- `cargo fmt --all --check`：通过。
- `pnpm style:guard`：通过。
- `pnpm test`：通过。
- `pnpm build`：通过。
