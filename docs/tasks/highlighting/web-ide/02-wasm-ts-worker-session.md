# 02 WASM TS Worker Session

## 目标

把 Rust incremental highlight session 暴露到 WASM 和 TypeScript，并新增 Web Worker 协议，让 Web IDE 主线程可以发送编辑 delta，异步获取增量高亮 patch 和实时 lexer diagnostics。

## 前置输入

- `01-incremental-highlight-session-handoff.md`。
- 现有 `crates/maodie_wasm_api` JSON ABI 和 `packages/compiler-wasm` loader。
- `packages/compiler-wasm/src/ranges.ts` 的 byte/UTF-16 range helper。

## 实现范围

- WASM API 新增 session lifecycle：create、update、reset、dispose。
- TS wrapper 新增 `MaodieHighlightSession` 或等效类。
- Worker 协议包含 `init`、`update`、`reset`、`dispose`。
- 每个 request/response 都携带 editor/session version；主线程必须能丢弃过期响应。
- update response 返回 changed range、tokens、diagnostics 和 fallback/full rehighlight 标记。
- TS tests 覆盖 session lifecycle、version mismatch、dispose 后调用、worker stale response。

## 不做事项

- 不接入 CodeMirror UI。
- 不在 TS 或 worker 里重新扫描 Maodie 源码。
- 不改变现有 `compileMaodieWasm` 和 `highlightMaodieSource` 行为。

## 输出产物

- WASM session ABI。
- TS session wrapper 类型和实现。
- `highlight.worker.ts` 协议文档或类型。
- Node/worker 可测的集成测试。

## 交接文档

任务完成后更新 `02-wasm-ts-worker-session-handoff.md`。

## 验收文档

复验者按 `02-wasm-ts-worker-session-acceptance.md` 执行。

