# 06 Type System V1

## 目标

实现 Maodie v1 静态类型系统：基础类型、局部推断、泛型类型/函数、struct/enum/trait/impl 的基础检查。

## 对应特性

语言可靠性核心。让 IDE 和后端能基于确定类型工作。

## 前置输入

- 任务 05 的 HIR、symbol id、resolver 结果。
- 任务 02 的诊断模型。

## 实现范围

- 支持基础类型：`i32`、`bool`、`String`、`unit`。
- 支持函数参数和返回类型显式声明。
- 支持局部 `let` 类型推断和 `mut` 可变性检查。
- 支持 struct field 类型、enum variant payload 类型。
- 支持泛型类型和泛型函数的 v1 表达与实例化准备。
- 支持 trait 方法签名和 `impl` 基础一致性检查。
- 输出 typed HIR 或类型表 dump。

## 不做事项

- 不做 borrow checker。
- 不做 trait object、动态分派或完整 trait bounds 求解。
- 不做高级数字类型、生命周期、宏。

## 输出产物

- Type checker API。
- 类型表、type id、substitution 或实例化数据结构。
- 类型错误中文诊断。
- typed HIR/type dump snapshot tests。

## 验收标准

- 错误返回类型、字段类型不匹配、不可变变量赋值都会报错。
- 泛型 `Option<T>` / `Result<T,E>` 的基础使用可通过类型检查。
- `impl` 缺失 trait 方法会报错。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 type checker 测试和 workspace 测试，抽查类型不匹配、不可变变量赋值、泛型 `Option`/`Result`、trait impl 缺失方法四类用例。人工检查 typed HIR 或类型表 dump 是否稳定，并确认没有引入 borrow checker、trait object 或完整 trait solver。检查任务 07 是否拿到 Result/Option 和 enum variant 的最终类型接口。

## 交接给下一任务

任务 07 可以依赖类型检查器识别 enum、variant、Result/Option 和表达式类型。match 和 `?` 的类型规则应在任务 07 中接入。

## 风险与注意

泛型 v1 使用单态化路线，类型系统需要保留足够实例化信息，但不要提前实现完整 trait solver。

## 交接记录

状态：已完成。

实现记录：

- `maodie_compiler::typeck` 提供 `TypeChecker`、`check_source`、`check_sources`、`TypeCheckResult`、`TypeTable`、`TypeId`、`TypeKind`。
- 基础类型在类型检查层统一为 `i32`、`bool`、`String`、`unit`；resolver 仍兼容历史 `Int`、`Bool` 拼写。
- parser/HIR 支持 `struct`、`enum`、`trait` 泛型参数；类型引用支持泛型实参并检查实参数量。
- 类型检查覆盖函数参数/返回、显式 `return`、局部 `let` 推断、类型标注一致性、`let mut` 赋值检查、基础算术/比较/赋值表达式、`if`/基础 `match` 表达式类型。
- enum variant 作为构造器参与类型检查，`Option<T>` / `Result<T,E>` 的普通泛型声明和基础构造调用可通过。
- trait 方法签名会被收集，`impl Trait for Type` 缺失方法会报 `MD0406`。
- `TypeTable::dump()` 输出稳定类型表、item/local/expression 类型和泛型 substitution，供 typed HIR/type dump snapshot 使用。

验证：

- `cargo test --workspace` 成功。
- `cargo clippy --workspace --all-targets` 无警告。

边界：

- 未实现 borrow checker、trait object、动态分派、完整 trait bounds 求解。
- `match` 穷尽性、`?` 的 Result 传播和 Option/Result 特殊语义仍留给任务 07。
