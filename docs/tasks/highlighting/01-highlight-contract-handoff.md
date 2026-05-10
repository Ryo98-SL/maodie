# 01 Highlight Contract Handoff

状态：已完成。

## 完成摘要

`maodie_syntax` 已新增语法级 highlight API。实现基于现有 lexer token 流做稳定分类映射，不调用 parser/type checker，不输出颜色或 IDE 专属 token 名称。whitespace 和 EOF 不进入 highlight token 输出，lexer diagnostics 原样透传。

## 公共接口

Rust API：

- `highlight_source(source: &SourceFile) -> HighlightResult`

`HighlightResult` 字段：

- `tokens: Vec<HighlightToken>`
- `diagnostics: Vec<Diagnostic>`

`HighlightToken` 字段：

- `kind: HighlightKind`
- `range: TextRange`

`HighlightKind` 取值：

- `keyword`
- `identifier`
- `comment`
- `string`
- `number`
- `boolean`
- `operator`
- `punctuation`
- `error`

diagnostics 透传规则：

- `HighlightResult::diagnostics` 直接使用 `lex_source` 返回的 lexer diagnostics。
- highlight 层不新增、改写或吞掉 diagnostics。

映射规则：

- `Keyword(_)` -> `keyword`
- `Identifier` -> `identifier`
- `LineComment`、`BlockComment` -> `comment`
- `StringLiteral` -> `string`
- `IntegerLiteral` -> `number`
- `BoolLiteral` -> `boolean`
- `Arrow`、`FatArrow`、`Less`、`Greater`、`Question`、`Equal`、`Plus`、`Minus`、`Star`、`Slash` -> `operator`
- `LeftParen`、`RightParen`、`LeftBrace`、`RightBrace`、`LeftBracket`、`RightBracket`、`Comma`、`Colon`、`Semicolon`、`Dot` -> `punctuation`
- `Error` -> `error`
- `Whitespace`、`Eof` -> 不输出

## 测试结果

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_syntax`：通过，13 个单元测试全部通过。
- `cargo test --workspace`：通过，workspace Rust 单元测试和 doc-tests 全部通过。

## 已知限制

第一版只保证语法级分类，不保证语义分类。

`<` 和 `>` 只按 lexer token 映射为 `operator`，highlight 层不判断泛型或比较语义。

## 下一任务入口

任务 02 应只依赖本交接文档列出的 Rust highlight API，不直接复刻 token 映射逻辑。WASM/TS wrapper 可序列化 `HighlightResult { tokens, diagnostics }`，其中 `HighlightToken::range` 保持 Rust `TextRange { start, end }` byte range。
