# Maodie Syntax Crate

## Purpose

`maodie_syntax` owns source-level syntax utilities. Its first module is the lexer, which turns `.mao` source text into a stable token stream and reports lexical errors through `maodie_diagnostics`.

## Integration Notes

The lexer stores token spans as `TextRange` byte ranges. Parser stages should consume the token stream and avoid rescanning source text.
