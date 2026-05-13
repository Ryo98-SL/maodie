# 02 WASM TS Worker Session Handoff

状态：已完成。

## 完成摘要

`maodie_wasm_api` 已把 Rust `IncrementalHighlightSession` 暴露为低层 WASM session ABI。session 由 opaque handle 表示，create/reset/update 都返回 JSON response buffer，dispose 释放 Rust session handle。一次性 `maodie_highlight` 和 compile API 保持兼容。

`@maodie/compiler-wasm` 已新增 `MaodieHighlightSession` wrapper。wrapper 负责创建 session、发送 UTF-8 byte range edit、同步 editor/session version、在成功 update/reset 后推进本地 current response，并在 dispose 后阻止继续调用 WASM。

`packages/compiler-wasm/src/highlight.worker.ts` 已新增 Worker 协议类型、Node 可测 request handler 和 stale response helper。Worker 只转发 edit/reset 到 WASM session，不在 TS 侧重新扫描 Maodie 源码。

## 公共接口

WASM ABI：

- `maodie_highlight_session_create(source_ptr, source_len, options_ptr, options_len) -> ResponseBuffer`
- `maodie_highlight_session_update(session_handle, request_ptr, request_len) -> ResponseBuffer`
- `maodie_highlight_session_reset(session_handle, source_ptr, source_len, options_ptr, options_len) -> ResponseBuffer`
- `maodie_highlight_session_dispose(session_handle)`

Create response carries `sessionHandle`, `editorVersion`, `sessionVersion`, `changedRange`, `tokens`, `diagnostics`, and `fullRehighlight`. Update request carries `editorVersion`, `sessionVersion`, `range`, and `replacement`. Update rejects stale session versions with `ok: false`, `MD9000`, and the current session version without mutating the session.

TS wrapper:

- `MaodieCompilerWasm.createHighlightSession(source, options) -> MaodieHighlightSession`
- `MaodieHighlightSession.current`
- `MaodieHighlightSession.sessionVersion`
- `MaodieHighlightSession.editorVersion`
- `MaodieHighlightSession.update({ editorVersion, sessionVersion?, range, replacement })`
- `MaodieHighlightSession.reset(source, { sourcePath?, editorVersion })`
- `MaodieHighlightSession.dispose()`

Worker protocol:

- `init`: `{ type, requestId, editorVersion, source, options?, loaderOptions? }`
- `update`: `{ type, requestId, editorVersion, sessionVersion, edit }`
- `reset`: `{ type, requestId, editorVersion, source, options? }`
- `dispose`: `{ type, requestId, editorVersion }`
- session responses mirror `requestId`, `editorVersion`, `sessionVersion`, `changedRange`, `tokens`, `diagnostics`, and `fullRehighlight`.

`isStaleHighlightWorkerResponse(response, currentEditorVersion, currentSessionVersion?)` returns true when a response is older than the main-thread editor/session state. UI integration in task 03 should call this before applying tokens or diagnostics.

Version and fallback rules:

- Create starts at session version `0` and returns `fullRehighlight: true`.
- Successful update increments the Rust session version and may return `fullRehighlight: false` or `true` depending on Rust sync fallback.
- Reset increments the Rust session version and returns `fullRehighlight: true`.
- Version mismatch and invalid edit range responses are `ok: false` and do not advance TS wrapper `current`.

## 测试结果

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_wasm_api`：通过，7 个 tests。
- `pnpm nx run compiler-wasm:typecheck`：通过。
- `pnpm nx run compiler-wasm:test`：通过，10 个 tests。

## 已知限制

Worker 只服务语法级 highlighter，不运行 compile、evaluation、parser 或 typechecker。

Worker handler 当前维护一个活动 highlight session。多编辑器实例应创建多个 Worker，或在后续任务扩展协议以携带独立 session id。

## 下一任务入口

任务 03 应使用本任务确认的 TS/worker API 接入 CodeMirror，不直接调用 Rust session 或 WASM 内存。
