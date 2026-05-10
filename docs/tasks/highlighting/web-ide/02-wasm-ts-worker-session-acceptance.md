# 02 WASM TS Worker Session Acceptance

## 验收命令

- `cargo fmt --all --check`
- `cargo test -p maodie_wasm_api`
- `pnpm nx run compiler-wasm:typecheck`
- `pnpm nx run compiler-wasm:test`
- `pnpm typecheck`
- `pnpm test`

## 人工检查

- Worker 和 TS wrapper 没有重复实现 Maodie lexer。
- session lifecycle 不泄漏 WASM response/session handles。
- version mismatch 或 stale response 不会覆盖较新的 editor state。
- dispose 后调用有可读错误或安全 no-op。
- 现有 full highlight 和 compile helper 保持兼容。

## 验收结论

状态：未验收。

记录复验者、日期、命令结果和人工检查结论。

