# 07 Pattern Match and Errors

## 目标

实现 `match` 语义、基础穷尽性检查、`Option`/`Result` 类型规则和 `?` 错误传播。

## 对应特性

Rust-like 错误处理和代数数据类型体验。

## 前置输入

- 任务 06 的类型检查器、enum/variant 类型信息。
- 任务 09 尚未完成前，可先使用内建 core 类型占位契约。

## 实现范围

- 支持 enum variant、字面量和 `_` 通配模式。
- 支持 pattern binding 的类型绑定。
- 检查 `match` 分支结果类型一致。
- 对 enum 做基础穷尽性检查。
- 定义 `Option<T>`、`Result<T,E>` 的编译器已知语义。
- 支持 `?` 在返回 `Result` 的函数中传播错误。

## 不做事项

- 不做完整结构体解构模式。
- 不做 guard pattern。
- 不做异常机制。

## 输出产物

- Pattern checker。
- `?` lowering 前的语义标记或 typed HIR 表达。
- match/Result/Option 诊断。
- 相关 snapshot 和单元测试。

## 验收标准

- `match` 缺少 enum variant 且无 `_` 时报告中文诊断。
- `?` 用在非 Result 返回函数中时报错。
- `Result<i32, String>` 示例可通过类型检查。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 pattern/error 测试和 workspace 测试，检查 enum 穷尽性、`_` 通配、分支结果类型不一致、`?` 非法位置和合法 `Result` 传播。人工确认 `Option`/`Result` 的特殊语义与任务 09 计划的 core 标准库名称一致。检查任务 08 是否明确 typed HIR 中 `match` 和 `?` 的降低输入形态。

## 交接给下一任务

任务 08 可以假设 `match` 和 `?` 已在 typed HIR 中有明确语义，不需要重新做穷尽性和错误传播合法性检查。

## 风险与注意

任务 09 会把 Option/Result 放入 core 标准库。任务 07 的内建语义必须与任务 09 的库定义对齐，避免出现两个不兼容的 Result。

## 交接记录

状态：已完成。

实现记录：

- `match` 已在类型检查阶段校验模式类型、分支结果类型和 enum 基础穷尽性。
- pattern binding 会绑定为对应 scrutinee 类型，作用域限制在当前 match arm。
- `Result<T,E>` 的 `?` 会产出 `T`，并要求所在函数返回 `Result<_,E>`。
- 新增中文诊断覆盖缺失 enum 变体、非法 pattern 和非法 `?` 使用。

验收记录：

- `cargo test --workspace` 通过。
