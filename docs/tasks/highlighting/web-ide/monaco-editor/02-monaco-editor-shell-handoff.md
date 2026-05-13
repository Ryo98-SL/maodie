# 02 Monaco Editor Shell Handoff

Status: Completed.

## Completion Summary

`createMaodieEditor` now creates a Monaco model and standalone editor using the `maodie` language and `maodie-dark` theme. The public source API remains stable for `main.ts`, example switching, and Run.

## Public Interfaces

- `MaodieEditor.readSource()` returns the Monaco model value.
- `MaodieEditor.replaceSource(source)` replaces the full Monaco model and moves the cursor to the end.
- `MaodieEditor.destroy()` disposes subscriptions, highlight adapter resources, editor, and model.
- `window.maodieIdeEditor` exposes a narrow smoke-test API for source replacement, text insertion, semantic token count, and live marker count.

## Test Results

- `pnpm nx run ide:test`: passed on 2026-05-13 with 14 tests.
- `pnpm nx run ide:typecheck`: passed on 2026-05-13.

## Known Limits

The shell intentionally keeps minimap disabled and does not add advanced language services.

## Next Task Entry

Task 03 can attach the existing highlight worker session to the Monaco model through `onDidChangeContent`.
