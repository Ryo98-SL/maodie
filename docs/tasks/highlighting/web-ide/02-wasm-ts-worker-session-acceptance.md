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

状态：通过。

复验者：Codex。

日期：2026-05-11。

命令结果：

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_wasm_api`：通过，7 个 tests。
- `pnpm nx run compiler-wasm:typecheck`：通过。
- `pnpm nx run compiler-wasm:test`：通过，10 个 tests。
- `pnpm typecheck`：通过，6 个 projects。
- `pnpm test`：通过，7 个 projects 和 4 个依赖任务。
- `pnpm style:guard`：通过，33 个 documentation checkpoints。

人工检查结论：

- Worker 和 TS wrapper 只调用 WASM session，不重复实现 Maodie lexer。
- `MaodieHighlightSession` 释放 response buffer，并通过 `dispose` 释放 session handle；dispose 可重复调用。
- update request/response 携带 editor/session version；version mismatch 返回 `MD9000` 且不推进 wrapper current state。
- Worker response 携带 editor/session version，并提供 `isStaleHighlightWorkerResponse` 给主线程丢弃过期响应。
- dispose 后 update/reset 会抛出可读错误。
- 现有 `compileMaodieWasm`、`highlightMaodieSource` 和一次性 `MaodieCompilerWasm.highlight` 行为保持兼容，现有测试继续通过。
