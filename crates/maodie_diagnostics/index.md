# Maodie Diagnostics Crate

## Purpose

`maodie_diagnostics` owns the shared source file, byte span, diagnostic severity, stable error code, Chinese CLI rendering, and JSON serialization model used by compiler stages.

## Integration Notes

Compiler phases should store spans as byte ranges. Display and JSON boundary code can use `SourceFile` to convert byte offsets into 1-based line and column positions.
