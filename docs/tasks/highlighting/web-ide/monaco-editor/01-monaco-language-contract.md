# 01 Monaco Language Contract

## Goal

Define the Monaco-side Maodie language contract so downstream tasks can rely on stable token, range, theme, and marker behavior.

## Preconditions

- Existing `packages/compiler-wasm/src/highlight.worker.ts` protocol.
- Existing `HighlightKind`, `HighlightToken`, `Diagnostic`, and byte-range helpers from `@maodie/compiler-wasm`.

## Scope

- Add `monaco-editor` to `@maodie/ide`.
- Register the `maodie` language id, file extension, bracket/comment behavior, and dark workbench theme.
- Define `HighlightKind -> Monaco semantic token type` mapping.
- Convert UTF-8 byte ranges to Monaco 1-based line/column ranges.
- Convert lexer diagnostics into Monaco marker data.
- Add focused unit coverage for Chinese identifiers, emoji-safe offsets, unknown token fallback, and diagnostic spans.

## Non-Goals

- Do not replace the runtime editor shell.
- Do not connect the highlight worker.
- Do not add completion, hover, formatting, or LSP behavior.

## Outputs

- Monaco language contract helpers in `apps/ide/src/monacoLanguage.ts`.
- Updated `apps/ide/package.json` and lockfile.
- Updated IDE tests for the contract helpers.

## Handoff Document

Update `01-monaco-language-contract-handoff.md` when complete.

## Acceptance Document

Reviewer follows `01-monaco-language-contract-acceptance.md`.

