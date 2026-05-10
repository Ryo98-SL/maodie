# 01 Highlight Contract Acceptance

## 验收命令

- `cargo fmt --all --check`
- `cargo test -p maodie_syntax`
- `cargo test --workspace`

## 人工检查

- `HighlightKind` 不包含颜色值或 IDE 专属名称。
- whitespace 和 EOF 不进入 highlight token 输出。
- lexer diagnostics 没有被吞掉。
- token range 沿用 byte range，未改写 source model。
- `<` 和 `>` 仍按 lexer token 映射，不在 highlight 层判断泛型或比较语义。

## 验收结论

状态：已验收。

复验者：Codex。

日期：2026-05-10。

命令结果：以任务 05 最终验收套件复验，`cargo fmt --all --check` 和 `cargo test --workspace` 均通过，覆盖本任务要求的 `maodie_syntax` 测试。

人工检查结论：`HighlightKind` 保持语法级分类且不包含颜色值或 IDE 专属名称；whitespace 和 EOF 不输出 token；lexer diagnostics 保留；token range 沿用 UTF-8 byte range；`<` 和 `>` 仍按 lexer token 映射。
