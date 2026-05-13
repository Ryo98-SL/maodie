# 01 Incremental Highlight Session Acceptance

## 验收命令

- `cargo fmt --all --check`
- `cargo test -p maodie_syntax`
- `cargo test --workspace`

## 人工检查

- update 输入使用旧 source 的 UTF-8 byte range，且校验 char boundary。
- 多轮编辑后，session tokens 和 diagnostics 与 full `highlight_source` 一致。
- 增量 relex 对块注释、字符串、error token、合并/拆分 token 有明确回退策略。
- 同步失败能安全降级为 full rehighlight。
- 现有 `highlight_source` API 仍兼容。

## 验收结论

状态：通过。

复验者：Codex。

日期：2026-05-11。

命令结果：

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_syntax`：由 `cargo test --workspace` 覆盖通过，`maodie_syntax` 23 个 tests 通过。
- `cargo test --workspace`：通过。

人工检查结论：

- `HighlightEdit` 使用旧 source UTF-8 byte range，并拒绝越界或拆分 UTF-8 code point 的 range。
- 多轮编辑、全文替换、中文标识符/emoji、块注释、未闭合字符串、非法字符、`->`/`=>` 边界均有测试覆盖，并与 full highlight 结果保持一致。
- relex 起点会回退到安全 token 边界；同步失败时可降级为 full rehighlight。
- 现有 `highlight_source` API 保持兼容。
