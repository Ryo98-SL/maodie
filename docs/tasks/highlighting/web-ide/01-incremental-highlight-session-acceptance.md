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

状态：未验收。

记录复验者、日期、命令结果和人工检查结论。

