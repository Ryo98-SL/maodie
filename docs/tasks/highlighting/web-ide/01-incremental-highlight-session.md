# 01 Incremental Highlight Session

## 目标

在 Rust 侧实现真正增量的 Maodie 语法级高亮 session。session 维护 source、完整 lexer token cache、highlight tokens、lexer diagnostics 和 version，编辑发生后只重新词法分析受影响范围，并保证最终结果与 full `highlight_source` 等价。

## 前置输入

- `docs/tasks/highlighting/05-highlight-acceptance-suite-handoff.md`。
- `crates/maodie_syntax` 的 `lex_source`、`highlight_source`、`Token`、`HighlightToken`。
- `maodie_diagnostics` 的 `SourceFile`、`TextRange`、diagnostic span 模型。

## 实现范围

- 新增 `IncrementalHighlightSession`。
- 定义 edit delta：基于旧 source UTF-8 byte range 的 replacement edits。
- `create/reset/update` 维护 session version。
- 增量 relex 从编辑点前的安全 token 边界开始，必要时回退到 block comment、string 或 error token 起点。
- relex 直到新 token 流与旧 token suffix 重新同步；同步失败时允许降级为 full rehighlight。
- session 输出 changed range、patched highlight tokens 和当前 lexer diagnostics。
- property 或表格测试验证多轮编辑后与 full highlight 完全一致。

## 不做事项

- 不做 parser/typechecker 增量分析。
- 不改变现有 `highlight_source` 公共行为。
- 不实现 WASM ABI、TS wrapper、worker 或 Web UI。

## 输出产物

- Rust incremental highlight session API。
- edit delta 和 update response 类型。
- Rust 测试覆盖中文标识符、emoji、arrow/fat arrow、块注释开闭、未闭合字符串、非法字符、全文替换。

## 交接文档

任务完成后更新 `01-incremental-highlight-session-handoff.md`。

## 验收文档

复验者按 `01-incremental-highlight-session-acceptance.md` 执行。

