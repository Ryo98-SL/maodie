# 02 Monaco Editor Shell Acceptance

## Validation Commands

- `pnpm nx run ide:test`
- `pnpm nx run ide:typecheck`

## Manual Checks

- Confirm `#source-editor[data-editor-mount="monaco"]` is rendered.
- Confirm `main.ts` only uses the `MaodieEditor` abstraction.
- Confirm example selection updates the current editor document without triggering a full compile.
- Confirm Run reads the current Monaco document.

## Acceptance Result

Status: Passed.

Reviewer: Codex
Date: 2026-05-13

Command Results:

- `pnpm nx run ide:test`: passed with 14 tests.
- `pnpm nx run ide:typecheck`: passed.

Manual Review:

- `#source-editor[data-editor-mount="monaco"]` is rendered by `apps/ide/src/view.ts`.
- `apps/ide/src/main.ts` uses only the `MaodieEditor` abstraction for source reads and replacement.
- Example selection replaces the current editor document and resets compile/evaluation state without calling compile.
- Run reads the current Monaco document through `MaodieEditor.readSource()`.
