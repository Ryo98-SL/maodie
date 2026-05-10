# 05 Name Resolution and HIR

## 目标

把 AST 降低为 HIR，完成模块、导入、符号表和名字解析，为类型检查提供语义结构。

## 对应特性

编译器语义前端。建立从源码语法到类型系统的桥梁。

## 前置输入

- 任务 04 的 `maodie_syntax` parser 与 AST。
- parser API：`parse_source(&SourceFile) -> ParseResult` 和 `Parser`。
- parse result：`ParseResult { ast, diagnostics }`，其中 diagnostics 已包含 lexer 与 parser 诊断。
- AST 入口：`AstFile { module, imports, items, span }`。
- 顶层 item：`Item::Function`、`Item::Struct`、`Item::Enum`、`Item::Trait`、`Item::Impl`。
- 可消费节点：`ModuleDecl`、`ImportDecl`、`FunctionDecl`、`StructDecl`、`EnumDecl`、`TraitDecl`、`ImplDecl`、`Statement`、`Expr`、`Pattern`、`TypeRef`。
- AST dump：`AstFile::dump()`。
- AST span 使用任务 02 的 `TextRange` byte range；有语法错误时 AST 仍可能包含 missing/partial 节点，任务 05 应先传递已有 diagnostics 再决定是否继续做可恢复的名字解析。
- 任务 02 的诊断模型：`Diagnostic`、`DiagnosticCode`、`DiagnosticSeverity`、`DiagnosticSpan`、`TextRange`。

## 实现范围

- 定义 HIR 节点、symbol id、module id、item id。
- 解析 `module` 与 `import`，建立模块路径和可见符号。
- 为函数、struct、enum、trait、impl 建立符号表。
- 将 AST paths 解析到符号或产生诊断。
- 生成 HIR dump，包含稳定 id 显示策略。

## 不做事项

- 不做完整包管理和外部依赖下载。
- 不做类型推断。
- 不做 trait 方法调用解析。

## 输出产物

- HIR crate 或模块。
- Resolver API。
- unresolved name、duplicate name、invalid import 的中文诊断。
- HIR snapshot tests。

## 验收标准

- 同一模块重复定义会报错。
- 导入不存在符号会报错。
- 合法示例能生成稳定 HIR dump。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 resolver/HIR snapshot 和 workspace 测试，检查重复定义、未知导入、未知路径三个错误用例。人工审阅 HIR dump，确认符号 id 稳定、按源码顺序或显式排序输出。检查任务 06 是否引用最终的 HIR、symbol id 和路径解析不变量。

## 交接给下一任务

任务 06 可以依赖 HIR 中的符号 id、声明关系和路径解析结果，不需要再从 AST 做名字解析。

## 风险与注意

稳定 id 不能依赖 HashMap 随机遍历顺序，否则 snapshot 会漂移。dump 输出应按源码顺序或显式排序。

## 交接记录

状态：已完成。

实现位置：

- `crates/maodie_compiler/src/hir.rs`
- `crates/maodie_compiler/src/resolver.rs`
- `crates/maodie_compiler/src/lib.rs`

对外 API：

- `maodie_compiler::resolver::resolve_source(&SourceFile) -> ResolveResult`
- `maodie_compiler::resolver::resolve_sources(&[&SourceFile]) -> ResolveResult`
- `maodie_compiler::resolver::Resolver`
- `ResolveResult { package: HirPackage, diagnostics: Vec<Diagnostic> }`
- `HirPackage::dump()` 输出稳定 HIR dump。

不变量：

- `ModuleId`、`ItemId`、`SymbolId`、`LocalId` 按输入文件顺序和源码顺序稳定分配。
- HIR dump 只按 Vec 顺序输出，不依赖 `HashMap` 遍历顺序。
- `Int`、`Bool`、`String` 在 resolver 中作为内建类型解析。
- 顶层 item、enum variant、module 会进入 symbol 表；函数参数、let 和 pattern binding 会进入函数 local 表。
- import 目前解析到当前包内已声明的 module 或 symbol；不做外部包管理。

诊断：

- `MD0301`：重复定义名称。
- `MD0302`：无法解析名称。
- `MD0303`：无效的 import。

验证：

- `cargo fmt --check` 成功。
- `cargo test --workspace` 成功。
