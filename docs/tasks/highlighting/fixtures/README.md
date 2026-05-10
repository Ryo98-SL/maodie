# Highlight Fixtures

## Files

- `syntax-highlight.mao`: shared source fixture for syntax highlight token coverage.
- `syntax-highlight.tokens.json`: golden token and diagnostic ranges for the shared fixture.

## Update Rule

Update the `.mao` fixture first, then regenerate or review `syntax-highlight.tokens.json` from the Rust lexer/highlight output. The golden must keep byte ranges as UTF-8 byte offsets and must retain `error` tokens plus their diagnostics.

After changing either file, run:

```bash
cargo test -p maodie_syntax matches_shared_highlight_golden_fixture
pnpm nx run compiler-wasm:test
```

The fixture is syntax-level only. Do not add semantic token categories or editor theme names here.
