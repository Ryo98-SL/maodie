# 03 Monaco Highlight Diagnostics Handoff

Status: Completed.

## Completion Summary

`MaodieHighlightAdapter` now owns a Monaco model highlight session. It posts init/update/reset messages to the existing WASM worker, filters stale responses, applies semantic token data, writes Monaco markers for live lexer diagnostics, and notifies the Diagnostics panel.

## Public Interfaces

- `new MaodieHighlightAdapter({ monaco, model, sourcePath, wasmUrl, onLiveLexerUpdate })`.
- `handleModelChange(event)` accepts Monaco model content changes.
- `destroy()` clears semantic tokens, model markers, and worker resources.
- `singleEditorEditFromMonacoChange(sourceBefore, event)` converts single Monaco edits into compiler byte edits.

## Test Results

- `pnpm nx run ide:test`: passed on 2026-05-13 with 15 IDE tests, including direct Monaco change to UTF-8 byte edit coverage.
- `pnpm nx run ide:typecheck`: passed on 2026-05-13.
- `pnpm style:guard`: passed on 2026-05-13.
- `node tools/ide-highlight-smoke.mjs http://127.0.0.1:5173/ http://127.0.0.1:9226`: passed on 2026-05-13 against a local `pnpm nx run ide:dev -- --host 127.0.0.1 --port 5173` server.

## Known Limits

Only lexer-level highlight diagnostics are live. Compile diagnostics remain tied to the manual Run path.

## Next Task Entry

Task 04 should update browser smoke automation to use `window.maodieIdeEditor` instead of CodeMirror DOM internals.
