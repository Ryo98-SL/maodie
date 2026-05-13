# 01 Monaco Language Contract Acceptance

## Validation Commands

- `pnpm nx run ide:test`
- `pnpm nx run ide:typecheck`

## Manual Checks

- Confirm `monacoLanguage.ts` contains no DOM or editor-instance dependency in its pure range/token/marker helpers.
- Confirm unknown highlight kinds have a deterministic fallback.
- Confirm diagnostic markers are derived from compiler byte offsets rather than diagnostic display line/column text.

## Acceptance Result

Status: Accepted.

Reviewer: Codex
Date: 2026-05-13

Command Results:

- `pnpm nx run ide:test`: passed. Vitest reported `apps/ide/src/compilerClient.test.ts` with 14 passing tests.
- `pnpm nx run ide:typecheck`: passed.
- `pnpm style:guard`: passed with 33 documentation checkpoints.

Manual Review:

- `apps/ide/src/monacoLanguage.ts` keeps token, range, and marker helpers pure; DOM/editor APIs are limited to registration and model-token storage helpers.
- Unknown highlight kinds deterministically fall back to Monaco `variable` semantic tokens.
- Diagnostic markers are derived from compiler byte offsets. The focused emoji diagnostic test intentionally supplies incorrect display line/column values and still resolves the marker from byte offsets.
