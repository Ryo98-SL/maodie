# 04 Highlight Adapter Contracts Acceptance

## 验收命令

- `pnpm typecheck`
- `pnpm test`

## 人工检查

- Web IDE、VSCode、JetBrains 三类映射都能覆盖全部 `HighlightKind`：`keyword`、`identifier`、`comment`、`string`、`number`、`boolean`、`operator`、`punctuation`、`error`。
- unknown token fallback 规则明确。
- 文档没有要求适配层重复实现 Maodie lexer。
- 文档明确第一版不包含外部插件打包。

## 验收结论

状态：已验收。

复验者：Codex。

日期：2026-05-10。

命令结果：以任务 05 最终验收套件复验，`pnpm typecheck` 和 `pnpm test` 均通过。

人工检查结论：Web IDE、VSCode、JetBrains 三类映射均覆盖全部 `HighlightKind`；unknown fallback 明确；文档未要求适配层复制 Maodie lexer；第一版边界明确不包含外部插件打包。
