# 02 Highlight WASM and TS Acceptance

## 验收命令

- `cargo fmt --all --check`
- `cargo test -p maodie_wasm_api`
- `pnpm nx run compiler-wasm:typecheck`
- `pnpm nx run compiler-wasm:test`

## 人工检查

- highlight 入口没有触发完整 compile pipeline。
- TS wrapper 没有复制 Rust lexer 逻辑。
- API 返回值包含 tokens 和 diagnostics。
- compile API 现有调用方不需要改代码。
- Node 加载真实 `.wasm` 的测试覆盖 highlight 入口。

## 验收结论

状态：已验收。

复验者：Codex。

日期：2026-05-10。

命令结果：以任务 05 最终验收套件复验，`cargo test --workspace`、`pnpm typecheck`、`pnpm test` 均通过；`pnpm nx run compiler-wasm:test` 通过且覆盖真实 `.wasm` highlight 调用。

人工检查结论：highlight 入口只返回 tokens 和 diagnostics，不触发完整 compile pipeline；TS wrapper 未复制 Rust lexer 逻辑；compile API 现有调用方保持兼容。
