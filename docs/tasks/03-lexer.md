# 03 Lexer

## 目标

实现 `.mao` 源码词法分析，把源码转换为 token 流，并用统一诊断模型报告词法错误。

## 对应特性

语言表层语法入口。覆盖关键字、标识符、字面量、注释、操作符、分隔符和错误 token。

## 前置输入

- 任务 02 的 `maodie_diagnostics` crate。
- source model 类型：`SourceId`、`SourceFile`、`TextRange`、`TextPosition`。
- diagnostic 类型：`DiagnosticCode`、`DiagnosticSeverity`、`DiagnosticSpan`、`Diagnostic`。
- span 使用 `TextRange { start, end }` byte range，半开区间 `[start, end)`；显示层通过 `SourceFile` 换算 1-based `line`、`column` 和 `byte_offset`。
- JSON 诊断字段稳定为 `code`、`severity`、`message`、`span`、`notes`，其中 `span` 包含 `source_id`、`file_name`、`range`、`start`、`end`。
- `.mao` 源码扩展名约定。

## 实现范围

- 支持英文关键字：`module`、`import`、`fn`、`let`、`mut`、`struct`、`enum`、`trait`、`impl`、`if`、`else`、`match`、`return`。
- 支持 Unicode 标识符，保留关键字 token。
- 支持整数、布尔、字符串字面量。
- 支持行注释和块注释。
- 支持常用符号：括号、花括号、逗号、冒号、分号、点、箭头、fat arrow、泛型尖括号、`?`。
- 对非法字符、未闭合字符串、未闭合块注释产生错误诊断。

## 不做事项

- 不做语法解析。
- 不做缩进敏感语法。
- 不做宏、原始字符串或复杂数字后缀。

## 输出产物

- `maodie_syntax` 或等效 crate 中的 lexer 模块。
- Token kind、token text/range、lexing API。
- 词法 snapshot 或 golden tests。

## 验收标准

- `.mao` 示例能输出稳定 token 序列。
- 非法字符和未闭合字符串有中文诊断和正确 span。
- `cargo test --workspace` 成功。

## 完成后验收方式

复验者运行 lexer 测试和 workspace 测试，抽查合法 `.mao` 示例、非法字符、未闭合字符串、未闭合块注释四类输入。确认 token dump 稳定，且 token range 使用任务 02 的 span 规则。人工检查 lexer 没有实现 parser 语义，泛型尖括号和比较运算仍保留给 parser 判断。

## 交接给下一任务

任务 04 可以假设 token 流保留所有必要分隔符和 `TextRange` byte span。parser 不需要重新扫描源码，只消费 lexer API。

## 风险与注意

泛型尖括号和比较运算在词法层不应过度解释，交给 parser 根据上下文判断。

## 交接记录

状态：已完成。

完成摘要：

- 新增 Rust crate `crates/maodie_syntax`，并加入 workspace。
- `crates/maodie_compiler` facade 通过 `syntax` 模块重新导出 lexer API。
- 实现 `lex_source(&SourceFile) -> LexResult` 和 `Lexer`。
- 定义 `LexResult`、`Token`、`TokenKind`、`Keyword`。
- token 保留 trivia：`Whitespace`、`LineComment`、`BlockComment`，并在末尾追加 `Eof`。
- token range 使用任务 02 的 `TextRange` byte range，token text 保留原始源码片段。
- Unicode 标识符使用 `unicode-ident` 的 XID 规则，额外允许 `_` 作为标识符起始和继续字符。
- 关键字固定为 `module`、`import`、`fn`、`let`、`mut`、`struct`、`enum`、`trait`、`impl`、`if`、`else`、`match`、`return`。
- `true` 和 `false` 产出 `BoolLiteral`。
- 字符串字面量支持双引号扫描和基础反斜杠转义跳过；未闭合字符串产生错误诊断。
- 行注释扫描到换行前；块注释扫描到 `*/`，不做嵌套块注释。
- 泛型尖括号和比较符号都只产出 `Less`/`Greater` token，不在词法层解释。

词法诊断错误码：

- `MD0101`：非法字符。
- `MD0102`：字符串字面量没有闭合。
- `MD0103`：块注释没有闭合。

测试命令结果：

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：通过。

测试覆盖：

- 合法 `.mao` 示例的稳定 token dump。
- Unicode 中文标识符 byte range。
- 关键字、bool、整数、字符串、行注释、块注释、常用符号。
- 非法字符中文诊断和 span。
- 未闭合字符串中文诊断和 span。
- 未闭合块注释中文诊断和 span。
- `<` 和 `>` 保持为普通 token，留给 parser 按上下文解释。

已知限制：

- lexer 不实现 parser 语义，不判断泛型、比较表达式或类型合法性。
- 不支持嵌套块注释、原始字符串、复杂数字后缀或字符串转义合法性校验。
- 目前没有把分号自动插入到换行处；后续 parser 应直接消费显式 token。

下一任务入口：

- 任务 04 parser 应依赖 `maodie_syntax`。
- parser 可通过 `lex_source(&SourceFile)` 获取 `LexResult { tokens, diagnostics }`。
- parser 应跳过 `Whitespace`、`LineComment`、`BlockComment` trivia，消费其余 token kind。
- parser 应沿用 `Token.range: TextRange` 作为 AST span 的来源，并把 lexer diagnostics 原样并入最终诊断列表。
