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

状态：未验收。

记录复验者、日期、命令结果和人工检查结论。

