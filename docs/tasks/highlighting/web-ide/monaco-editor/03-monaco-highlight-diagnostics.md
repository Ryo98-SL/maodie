# 03 Monaco Highlight Diagnostics

## Goal

Connect the existing WASM highlight worker session to Monaco so live semantic tokens and lexer diagnostics continue to work after the editor replacement.

## Preconditions

- `01-monaco-language-contract-handoff.md`.
- `02-monaco-editor-shell-handoff.md`.
- Existing `packages/compiler-wasm/src/highlight.worker.ts` request/response protocol.

## Scope

- Replace the CodeMirror `ViewPlugin` adapter with a Monaco model adapter.
- Convert single Monaco content changes into UTF-8 byte edits against the previous source.
- Fall back to worker reset for complex edits or pending init changes.
- Preserve editor/session stale-response filtering and update-protocol reset recovery.
- Write worker tokens into Monaco semantic token storage.
- Write worker diagnostics into Monaco model markers and the existing right-side Diagnostics panel.
- Dispose worker, markers, and token storage with the editor.

## Non-Goals

- Do not change the Rust/WASM worker protocol.
- Do not move full compile to a worker.
- Do not add parser/typechecker live diagnostics.

## Outputs

- Monaco-backed `apps/ide/src/highlightAdapter.ts`.
- Updated semantic token and marker helper tests.

## Handoff Document

Update `03-monaco-highlight-diagnostics-handoff.md` when complete.

## Acceptance Document

Reviewer follows `03-monaco-highlight-diagnostics-acceptance.md`.

