# 02 Monaco Editor Shell

## Goal

Replace the CodeMirror editor shell with Monaco while preserving the existing `MaodieEditor` API and workbench behavior.

## Preconditions

- `01-monaco-language-contract-handoff.md`.
- Existing `main.ts` lifecycle and example switching flow.

## Scope

- Create a Monaco model/editor in `createMaodieEditor`.
- Keep `readSource()`, `replaceSource(source)`, and `destroy()` as the only main-entry editor operations.
- Configure a restrained IDE profile: line numbers, current line, bracket matching, word wrap, automatic layout, and disabled minimap.
- Keep `main.ts` independent from Monaco-specific APIs.
- Change the workbench mount marker to `data-editor-mount="monaco"`.

## Non-Goals

- Do not connect live highlight worker results.
- Do not change compile or evaluation behavior.

## Outputs

- Monaco-backed `apps/ide/src/editor.ts`.
- Updated `apps/ide/src/view.ts` and render tests.

## Handoff Document

Update `02-monaco-editor-shell-handoff.md` when complete.

## Acceptance Document

Reviewer follows `02-monaco-editor-shell-acceptance.md`.

