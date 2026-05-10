# 02 Highlight WASM and TS Handoff

状态：已完成。

## 完成摘要

`crates/maodie_wasm_api` 已新增轻量 highlight JSON ABI，`packages/compiler-wasm` 已新增 TS wrapper 和类型导出。Highlight 路径直接调用任务 01 的 Rust `highlight_source`，只返回 tokens 和 lexer diagnostics，不触发 parse/typecheck/MIR/WAT/WASM 生成，也不返回 artifacts 或 dumps。

## 公共接口

- WASM ABI 函数：
  - `maodie_highlight(source_pointer, source_len, options_pointer, options_len) -> *mut ResponseBuffer`
  - response buffer 生命周期沿用 `maodie_response_len`、`maodie_response_bytes`、`maodie_free_response`
- TS wrapper：
  - `MaodieCompilerWasm.highlight(source, options?)`
  - `highlightMaodieSource(source, options?)`
- TS 类型：
  - `HighlightKind`
  - `HighlightToken`
  - `HighlightOptions`
  - `HighlightResponse`
- `HighlightResponse` 字段：
  - `ok: boolean`
  - `tokens: readonly HighlightToken[]`
  - `diagnostics: readonly Diagnostic[]`
- `HighlightToken` 字段：
  - `kind: HighlightKind`
  - `range: { start: number; end: number }`
- `HighlightKind` 取值：
  - `keyword`
  - `identifier`
  - `comment`
  - `string`
  - `number`
  - `boolean`
  - `operator`
  - `punctuation`
  - `error`
- 错误响应规则：
  - UTF-8 输入或 options JSON 无法读取时返回 `ok: false`、空 `tokens`、`MD9000` diagnostic。
  - lexer 产生 error diagnostic 时返回 `ok: false`，同时保留已产生的 highlight tokens。
  - 正常 lexing 包括空源码时返回 `ok: true`。

Compile API 的 JSON shape 未变，现有调用方不需要修改。

## 测试结果

- `cargo fmt --all --check`：通过。
- `cargo test -p maodie_wasm_api`：通过，5 个单元测试全部通过。
- `pnpm nx run compiler-wasm:typecheck`：通过。
- `pnpm nx run compiler-wasm:test`：通过，4 个 Vitest 测试全部通过，并覆盖真实 `.wasm` highlight 调用。

## 已知限制

highlight API 只保证语法级 token，外部 IDE 增量调用策略由适配层决定。

`HighlightKind` 不包含语义分类；泛型、比较符号等语境差异仍按 lexer token 分类。

## 下一任务入口

任务 03 应使用 TS wrapper 的 highlight API 生成 fixtures，不直接访问 WASM 内存或 Rust crate。
