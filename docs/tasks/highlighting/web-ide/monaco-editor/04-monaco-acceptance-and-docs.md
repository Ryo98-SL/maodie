# 04 Monaco Acceptance And Docs

## Goal

Close the Monaco refactor with browser smoke coverage, module documentation, and discoverable task-chain documentation.

## Preconditions

- `01-monaco-language-contract-handoff.md`.
- `02-monaco-editor-shell-handoff.md`.
- `03-monaco-highlight-diagnostics-handoff.md`.

## Scope

- Update `tools/ide-highlight-smoke.mjs` to use the Monaco smoke-test API.
- Preserve existing smoke scenarios for default highlight, Chinese/emoji edits, live lexer errors, example switching, Run, and rapid input settling.
- Update IDE module docs to describe Monaco instead of CodeMirror.
- Link this task chain from the Web IDE highlighting README.
- Run the final validation commands.

## Non-Goals

- Do not redesign the workbench UI.
- Do not add advanced Monaco language services.

## Outputs

- Updated browser smoke script.
- Updated `apps/ide/index.md`, `apps/ide/src/index.md`, and Web IDE highlighting README.
- Completed Monaco task-chain docs.

## Handoff Document

Update `04-monaco-acceptance-and-docs-handoff.md` when complete.

## Acceptance Document

Reviewer follows `04-monaco-acceptance-and-docs-acceptance.md`.

