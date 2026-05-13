# Maodie Syntax Crate

## Purpose

`maodie_syntax` owns source-level syntax utilities. It provides a lexer, parser, AST model, and a syntax-level highlight contract built on the lexer token stream.

## Integration Notes

The lexer stores token spans as `TextRange` byte ranges. Parser stages should consume the token stream and avoid rescanning source text.

The highlight API is exposed as `highlight_source(&SourceFile) -> HighlightResult`. It maps lexer tokens into stable `HighlightKind` values, omits whitespace and EOF, preserves token byte ranges, and forwards lexer diagnostics unchanged. It does not perform semantic analysis or assign editor/theme colors.

The incremental highlight API is exposed as `IncrementalHighlightSession`. The session owns a `SourceFile`, the full lexer token cache, highlight tokens, lexer diagnostics, and a monotonically increasing version. `HighlightEdit` replacement ranges are expressed in old-source UTF-8 byte offsets. `update` relexes from a safe token boundary, patches caches when it can synchronize with an unchanged old token suffix, and falls back to a full rebuild when synchronization is unsafe.

The shared highlight fixture under `docs/tasks/highlighting/fixtures/` is checked from this crate against Rust output, then reused by the WASM/TS wrapper tests. Keep fixture ranges as UTF-8 byte offsets.

## Current Structure

- `src/lexer.rs`: tokenizes source text into `Token` values and lexical diagnostics.
- `src/parser.rs`: parses the lexer stream into AST declarations, statements, expressions, and diagnostics.
- `src/ast.rs`: defines syntax tree data structures.
- `src/highlight.rs`: maps lexer tokens to the public syntax highlight contract.
- `src/incremental.rs`: maintains an incremental lexer/highlight session for editor integrations.
- `src/lib.rs`: re-exports the public syntax, lexer, parser, and highlight APIs.
