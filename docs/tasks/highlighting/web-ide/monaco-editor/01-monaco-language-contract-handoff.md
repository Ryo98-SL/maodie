# 01 Monaco Language Contract Handoff

Status: Completed.

## Completion Summary

`apps/ide/src/monacoLanguage.ts` now owns the Monaco `maodie` language id, dark theme, semantic token legend, byte-range conversion, diagnostic marker conversion, semantic token storage, and stale-response helper. Focused IDE tests cover Chinese identifiers, emoji-safe UTF-8 byte offsets, unknown token fallback, and diagnostic marker spans derived from compiler offsets.

## Public Interfaces

- `registerMaodieMonacoLanguage(monaco)` registers language configuration, theme, and the semantic token provider.
- `semanticTokenTypeForKind(kind)` maps Maodie highlight kinds to Monaco token types, with unknown kinds falling back to `variable`.
- `byteRangeToMonacoRange(source, range)` converts compiler byte ranges into Monaco ranges.
- `diagnosticToMonacoMarkerData(source, diagnostic, severities)` creates marker data for live lexer diagnostics.
- `setMaodieSemanticTokens`, `clearMaodieSemanticTokens`, and `maodieSemanticTokenCount` manage model-specific semantic token data.

## Test Results

- `pnpm nx run ide:test`: passed on 2026-05-13 with 14 tests.
- `pnpm nx run ide:typecheck`: passed on 2026-05-13.
- `pnpm style:guard`: passed on 2026-05-13 with 33 documentation checkpoints.

## Known Limits

This task only defines editor-side language contracts. Completion, hover, formatting, and LSP-style services remain deferred.

## Next Task Entry

Task 02 can create the Monaco editor shell by calling `registerMaodieMonacoLanguage` before editor creation and using the exported language/theme ids.
