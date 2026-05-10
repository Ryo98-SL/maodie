# 08 MIR Lowering

## 目标

把 typed HIR 降低为 MIR，形成适合 WASM 后端生成代码的控制流和局部变量表示。

## 对应特性

编译器中端。把表达式导向语法转成明确的基本块、局部变量和控制流。

## 前置输入

- 任务 07 完成后的 typed HIR、match 和 `?` 语义。
- 任务 06 的类型信息和泛型实例化准备数据。

## 实现范围

- 定义 MIR 函数、基本块、局部变量、statement、terminator。
- 降低 `if`、`match`、块表达式、调用、返回、`?`。
- 保留 source span，用于后端错误和 debug dump。
- 生成 MIR dump，稳定可 snapshot。
- 为单态化后端保留实例化入口。

## 不做事项

- 不做复杂优化。
- 不做 SSA 完整转换。
- 不做寄存器分配或 native backend。

## 输出产物

- MIR 数据结构和 lowering API。
- MIR dump。
- lowering 测试覆盖控制流、match、Result 传播。

## 验收标准

- 表达式函数能降为返回值明确的 MIR。
- `match` 能降为分支控制流。
- `?` 能降为错误分支提前返回。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 MIR lowering snapshot 和 workspace 测试，人工比较同一示例的 typed HIR 与 MIR dump，确认表达式导向语法已被降成基本块、局部变量和 terminator。检查 `if`、`match`、`?` 三个控制流用例。确认 MIR 没有保留不必要 AST 细节，并把后端所需不变量写入任务 10。

## 交接给下一任务

任务 09 可以基于 MIR 需要补齐 core 标准库运行时契约。任务 10 可以直接消费 MIR 生成 WASM。

## 风险与注意

MIR 不应保留过多 AST 形状，否则后端会被语法细节污染。MIR dump 需要稳定排序和稳定 id。

## 交接记录

状态：已完成。

实现记录：

- 新增 `maodie_compiler::mir` 模块，提供 `MirLowerer::lower` 和 `lower_package` API。
- MIR 包含函数、基本块、局部变量、statement、terminator、operand、rvalue、branch pattern，并保留 source span。
- `if` 降为 bool `branch` terminator，`match` 降为 ordered variant/literal/wildcard targets，块表达式和调用降为临时 local + assignment。
- `?` 降为 Result `Ok`/`Err` variant match；`Ok` 分支 project payload 后继续，`Err` 分支 aggregate `Result.Err` 并提前 return。
- MIR dump 由稳定分配顺序生成；泛型 substitutions 保留为 `MirInstantiation`，供后续单态化入口使用。
- 已添加 lowering 测试覆盖 `if`、`match`、Result `?` 传播。

验证：

- `cargo test --workspace` 通过。
