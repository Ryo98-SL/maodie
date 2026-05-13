# 04 Decorations Live Lexer Diagnostics Acceptance

## 验收命令

- `pnpm nx run ide:typecheck`
- `pnpm nx run ide:test`
- `pnpm ide:build`
- `pnpm test`

## 人工检查

- 全部 9 个 `HighlightKind` 都有稳定 class 或 fallback。
- unknown kind 降级为 plain/default，不破坏后续 token 渲染。
- 中文标识符和 emoji 附近 decorations 不偏移。
- 未闭合字符串、块注释和非法字符能实时显示 lexer diagnostics。
- Diagnostics 面板能区分 live lexer 与 last compile。
- 输入不会触发完整 compile。

## 验收结论

状态：通过。

复验者：Codex。

日期：2026-05-11。

命令结果：

- `pnpm nx run ide:typecheck`：由 `pnpm typecheck` 覆盖通过。
- `pnpm nx run ide:test`：由 `pnpm test` 覆盖通过，`apps/ide/src/compilerClient.test.ts` 13 个 tests 通过。
- `pnpm ide:build`：通过。
- `pnpm test`：通过，7 个 projects 和 4 个依赖任务。

人工检查结论：

- 全部 9 个 `HighlightKind` 都映射到稳定 `mao-hl-*` class，unknown kind 降级为 `mao-hl-plain`。
- 中文标识符、emoji 附近编辑、error token 和 live diagnostic marker 由单元测试和浏览器 smoke 覆盖。
- 未闭合字符串、块注释开闭和非法字符都能实时显示或清除 lexer diagnostics。
- Diagnostics 面板区分 `Live Lexer` 和 `Last Compile`。
- 输入不会触发完整 compile；Run 才更新 Last Compile。
