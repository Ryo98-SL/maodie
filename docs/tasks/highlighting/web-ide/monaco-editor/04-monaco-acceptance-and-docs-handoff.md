# 04 Monaco Acceptance And Docs Handoff

Status: Completed.

## Completion Summary

The browser smoke script now drives the Monaco editor through `window.maodieIdeEditor`, and IDE docs describe the Monaco editor shell, semantic token flow, live markers, and smoke-test hook.

## Public Interfaces

- `tools/ide-highlight-smoke.mjs <ide-url> <chrome-devtools-url>` keeps the previous command shape.
- The smoke API is `window.maodieIdeEditor`, owned by `apps/ide/src/editor.ts`.
- The Monaco task chain is discoverable from `docs/tasks/highlighting/web-ide/README.md`.

## Test Results

- `pnpm nx run ide:test`: passed.
- `pnpm nx run ide:typecheck`: passed.
- `pnpm ide:build`: passed; Vite produced the Monaco editor bundle, editor worker, highlight worker, and WASM asset.
- `node tools/ide-highlight-smoke.mjs http://127.0.0.1:5174/ http://127.0.0.1:9226`: passed against a local Chrome DevTools endpoint.
- `pnpm style:guard`: passed.

## Known Limits

The final smoke still requires a running Vite IDE and a Chrome DevTools endpoint, as before.

## Next Task Entry

Future editor work can start from this chain for completions, hover, semantic compiler diagnostics, or a compile worker.
