# 02 Highlight WASM and TS Wrapper

## 目标

把 Rust highlight API 暴露给 `@maodie/compiler-wasm`，让 Node、Web IDE 和未来外部 IDE 适配层可以调用轻量染色入口。

## 前置输入

- `01-highlight-contract-handoff.md` 中确认的 Rust API。
- 现有 `crates/maodie_wasm_api` JSON ABI。
- 现有 `packages/compiler-wasm` 加载和内存管理方式。

## 实现范围

- WASM API 新增 highlight 入口，返回 JSON 响应。
- TS wrapper 新增 `highlightMaodieSource` 或等效函数。
- TS 类型导出 `HighlightToken`、`HighlightKind`、`HighlightResponse`。
- highlight 调用不触发完整 compile，不生成 artifacts 或 dumps。

## 不做事项

- 不改变现有 compile API 的 JSON shape。
- 不为某个 IDE 输出专用 token 名称。
- 不在 TS 侧重新实现 lexer。

## 输出产物

- WASM highlight ABI。
- TS wrapper highlight API。
- Node 集成测试，加载真实 wasm 并返回 highlight tokens。

## 交接文档

任务完成后更新 `02-highlight-wasm-and-ts-handoff.md`。

## 验收文档

复验者按 `02-highlight-wasm-and-ts-acceptance.md` 执行。

