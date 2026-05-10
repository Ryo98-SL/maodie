# 04 Parser and AST

## 目标

实现手写 parser，将 token 流解析为 AST，并在语法错误时尽量恢复，供 IDE 显示 partial AST 和诊断。

## 对应特性

语言语法结构。覆盖模块、导入、函数、struct、enum、trait、impl、表达式和基础模式语法。

## 前置输入

- 任务 03 的 `maodie_syntax` crate。
- lexer API：`lex_source(&SourceFile) -> LexResult` 和 `Lexer`。
- token 类型：`Token { kind, range, text }`、`TokenKind`、`Keyword`。
- `LexResult` 字段：`tokens`、`diagnostics`。
- token stream 包含 trivia：`Whitespace`、`LineComment`、`BlockComment`，并以 `Eof` 结束；parser 应跳过 trivia。
- token span 使用 `TextRange` byte range；AST span 应从 token range 合并得到。
- 词法诊断错误码：`MD0101` 非法字符、`MD0102` 未闭合字符串、`MD0103` 未闭合块注释。
- 任务 02 的诊断模型：`Diagnostic`、`DiagnosticCode`、`DiagnosticSeverity`、`DiagnosticSpan`、`TextRange`。

## 实现范围

- 定义 AST 节点和 dump 格式。
- 解析 `module`、`import`、`fn`、`let`、`struct`、`enum`、`trait`、`impl`。
- 解析表达式：字面量、路径、调用、块、`if`、`match`、二元运算、`?`。
- 解析类型语法：路径类型、泛型实参、函数返回类型。
- 解析模式：enum variant、字面量、绑定名、`_`。
- 实现错误恢复同步点，避免一个错误吞掉整个文件。

## 不做事项

- 不做名字解析、类型检查或 trait impl 验证。
- 不做格式化器。
- 不做完整宏系统。

## 输出产物

- AST 类型和 parser API。
- AST debug dump。
- parser recovery 测试和语法 snapshot。

## 验收标准

- 典型 `.mao` 文件能生成稳定 AST dump。
- 缺少括号、缺少类型、非法表达式能报告中文诊断并继续解析后续声明。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 parser snapshot 和 workspace 测试，至少查看一个完整文件 AST dump 和一个带语法错误的 partial AST dump。人工确认 parser 只负责结构解析，没有做名字存在性、类型正确性或 trait 检查。检查任务 05 是否明确消费 AST 节点、span 和 parser recovery 约定。

## 交接给下一任务

任务 05 可以从 AST 读取模块声明、导入、声明列表、表达式树和 span。AST 不保证语义正确，只保证结构可遍历。

## 风险与注意

parser 不要把语义规则写死。例如类型是否存在、trait 是否实现，都属于后续 HIR/type checker。

## 交接记录

状态：已完成。

完成摘要：

- 在 `crates/maodie_syntax` 中新增 AST 与 parser 模块。
- `crates/maodie_compiler` facade 通过既有 `syntax` 模块重新导出 parser 与 AST API。
- 实现 `parse_source(&SourceFile) -> ParseResult` 和 `Parser`。
- 定义 `ParseResult { ast, diagnostics }`，其中 diagnostics 合并 lexer diagnostics 与 parser diagnostics。
- 定义 `AstFile`、`ModuleDecl`、`ImportDecl`、`Item`、`FunctionDecl`、`StructDecl`、`EnumDecl`、`TraitDecl`、`ImplDecl`、`Statement`、`Expr`、`Pattern`、`TypeRef` 等 AST 类型。
- AST 节点 span 使用任务 02 的 `TextRange` byte range，由 token range 合并得到。
- `AstFile::dump()` 提供稳定 AST debug dump，用于 snapshot 和后续任务人工复验。
- parser 跳过 lexer trivia：`Whitespace`、`LineComment`、`BlockComment`。
- parser 保留 lexer 的 `<`/`>` 词法中立性：类型上下文解析泛型，表达式上下文解析比较/二元运算。

parser 诊断错误码：

- `MD0201`：遇到意外 token。
- `MD0202`：缺少预期语法元素。

解析范围：

- 顶层：`module`、`import`、`fn`、`struct`、`enum`、`trait`、`impl`。
- 类型：路径类型与泛型实参，例如 `Option<Int>`。
- 函数：泛型参数、参数列表、返回类型、函数体或签名分号。
- 表达式：整数、bool、字符串、路径、调用、块、`if`、`match`、二元运算、postfix `?`。
- 模式：`_`、绑定名、字面量、路径形式。
- 恢复：参数列表缺少右括号、缺少类型、非法表达式或非法顶层 token 后尽量继续解析后续声明。

测试命令结果：

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：通过。

测试覆盖：

- 完整 `.mao` 文件生成稳定 AST dump。
- 缺少 `)` 时报告中文诊断并继续解析后续函数。
- 缺少类型时报告中文诊断并继续解析后续函数。
- 非法顶层表达式报告中文诊断并继续解析后续声明。
- 词法测试仍覆盖 lexer token dump 和 lexer diagnostics。

已知限制：

- AST 只表达语法结构，不保证名字存在、类型正确或 trait impl 合法。
- parser 当前支持显式分号；没有实现自动分号插入或格式化器。
- 二元运算优先级为 parser 内部基础规则，后续语义阶段仍需决定具体类型规则。

下一任务入口：

- 任务 05 应依赖 `maodie_syntax`。
- 任务 05 可调用 `parse_source(&SourceFile)` 得到 `ParseResult`。
- 任务 05 应从 `ParseResult.ast` 读取模块声明、导入、声明列表、表达式树和 `TextRange` span。
- `ParseResult.diagnostics` 应原样向后传递或合并；AST 在有诊断时仍可能包含 missing/partial 节点。
