# 01 Highlight Contract

## 目标

定义 Maodie 语法级染色的公共 token 契约，并在 Rust 侧基于现有 lexer 暴露 highlight API。

## 前置输入

- `crates/maodie_syntax` 的 `lex_source`、`Token`、`TokenKind`、`Keyword`。
- `maodie_diagnostics` 的 `SourceFile`、`TextRange`、位置换算模型。

## 实现范围

- 新增 `HighlightToken` 和 `HighlightKind`。
- 基于 lexer token 映射语法级分类：
  - `keyword`
  - `identifier`
  - `comment`
  - `string`
  - `number`
  - `boolean`
  - `operator`
  - `punctuation`
  - `error`
- EOF 和纯 whitespace 不输出 highlight token。
- 保留 byte range，不在本任务决定具体主题颜色。

## 不做事项

- 不做语义级 token。
- 不实现 Web IDE、VSCode、JetBrains 适配。
- 不调用 parser/type checker。

## 输出产物

- Rust highlight API，例如 `highlight_source(&SourceFile) -> HighlightResult`。
- `HighlightResult { tokens, diagnostics }`，diagnostics 透传 lexer diagnostics。
- Rust 单元测试覆盖 token kind 映射。

## 交接文档

任务完成后更新 `01-highlight-contract-handoff.md`。

## 验收文档

复验者按 `01-highlight-contract-acceptance.md` 执行。

