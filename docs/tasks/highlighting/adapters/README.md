# Highlight Adapters Module

## Purpose

This module owns editor-adapter contracts for Maodie syntax highlighting. It translates the shared `HighlightToken` protocol into Web IDE, VSCode, and JetBrains platform concepts without implementing those plugins.

## Current Directory Structure

- `index.md`: shared adapter input contract, fallback rules, and first-version boundaries.
- `web-ide.md`: CodeMirror and Monaco token class mapping for browser editors.
- `vscode.md`: VSCode semantic token and TextMate fallback mapping strategy.
- `jetbrains.md`: JetBrains `Lexer`, `SyntaxHighlighter`, and `TextAttributesKey` mapping strategy.

## Key Behaviors

All adapter contracts consume the same lexer-backed `HighlightResponse`. Token ranges remain UTF-8 byte ranges until the adapter converts them to the host editor's UTF-16 offsets or line/character positions. Unknown token kinds must become plain/default text rather than throwing or interrupting rendering.

## Integration Notes

Future IDE plugin tasks should begin from these contracts, then add platform tests around the shared highlighting fixture. The contracts intentionally avoid colors, semantic analysis, external plugin packaging, and duplicate lexer implementations.
