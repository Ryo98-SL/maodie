# 01 Incremental Highlight Session Handoff

状态：已完成。

## 完成摘要

`maodie_syntax` 已新增 Rust 侧增量语法高亮 session。session 维护当前 `SourceFile`、完整 lexer token cache、highlight tokens、lexer diagnostics 和递增 `version`。

`update` 接收基于旧 source UTF-8 byte range 的 replacement edit，先校验 range 是否在旧 source 内且位于 UTF-8 char boundary；随后从编辑点前的安全 token 边界重新词法分析。若编辑触碰块注释、字符串或 error token，relex 起点会回退到能覆盖该 token 的边界。增量扫描逐 token 尝试与旧 token suffix 在应用 byte delta 后重新同步；同步成功时拼接 prefix、新扫描 patch 和 shifted suffix，同步失败时安全降级为 full rehighlight。

现有 `highlight_source(&SourceFile) -> HighlightResult` 行为保持兼容。

## 公共接口

Rust API 由 `maodie_syntax` re-export：

- `IncrementalHighlightSession::new(source: SourceFile) -> IncrementalHighlightSession`
- `IncrementalHighlightSession::version(&self) -> u64`
- `IncrementalHighlightSession::source(&self) -> &SourceFile`
- `IncrementalHighlightSession::lex_tokens(&self) -> &[Token]`
- `IncrementalHighlightSession::tokens(&self) -> &[HighlightToken]`
- `IncrementalHighlightSession::diagnostics(&self) -> &[Diagnostic]`
- `IncrementalHighlightSession::reset(&mut self, source: SourceFile) -> IncrementalHighlightUpdate`
- `IncrementalHighlightSession::update(&mut self, edit: HighlightEdit) -> Result<IncrementalHighlightUpdate, IncrementalHighlightError>`

Edit delta：

- `HighlightEdit { range: TextRange, replacement: String }`
- `range` 是旧 source 的半开 UTF-8 byte range。
- invalid range 返回 `IncrementalHighlightError::InvalidEditRange { range, source_len }`，session 不变且 version 不递增。

Update response：

- `IncrementalHighlightUpdate { version, changed_range, tokens, diagnostics, full_rehighlight }`
- `changed_range` 是新 source byte range，表示本次重算或失效的高亮范围。
- `tokens` 是 patch 后的当前完整 highlight token 列表。
- `diagnostics` 是 patch 后的当前完整 lexer diagnostics。
- `full_rehighlight` 为 `true` 表示本次 reset 或同步失败触发了完整重建。

Version 规则：

- `new` 初始 version 为 `0`。
- `reset` 和成功 `update` 每次递增 `1`。
- invalid edit 不修改 source/cache/version。

## 测试结果

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_syntax`：通过，23 个测试全部通过。
- `cargo test --workspace`：通过，全部 workspace 单元测试和 doc tests 通过。

新增覆盖包含：

- 中文标识符和 emoji 多轮编辑。
- `->` / `=>` 合并边界。
- 块注释闭合与破坏。
- 未闭合字符串恢复。
- 非法字符恢复。
- 全文替换。
- UTF-8 非 char boundary edit 拒绝。

## 已知限制

第一版只增量维护 lexer/highlight 结果，不维护 parser、typechecker 或 compile artifacts。

`changed_range` 使用 byte range 表达；WASM/TS 层仍需按 adapter contract 转换到 UTF-16 editor range。

## 下一任务入口

任务 02 应只依赖本交接文档列出的 Rust session API，不在 WASM 或 TS 层复刻增量 lexer 逻辑。WASM ABI 应包装 `IncrementalHighlightSession` 的 `new/reset/update`、`HighlightEdit` 和 `IncrementalHighlightUpdate`，并把 `changed_range`、`tokens`、`diagnostics` 统一序列化给 TS worker session。
