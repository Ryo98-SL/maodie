# 04 Decorations Live Lexer Diagnostics Handoff

状态：已完成。

## 完成摘要

Web IDE 的 CodeMirror editor shell 已接入 `@maodie/compiler-wasm` highlight worker session。编辑器初始化时创建 worker session，普通单点编辑发送增量 `update`，多段变更或等待 init 期间的变更发送 `reset`。CodeMirror decoration state 会先随本地 transaction map 旧 decorations，worker response 到达后再用 Rust/WASM 返回的当前 token 和 lexer diagnostics 刷新 decorations；stale editor/session response 会被丢弃，不覆盖较新的编辑状态。

`HighlightKind` 已映射到稳定 `mao-hl-*` class，未知 kind 降级为 `mao-hl-plain`。error token 保留 `mao-hl-error` 样式，live lexer diagnostics 额外生成 CodeMirror marker underline。右侧 Diagnostics 面板现在分为 `Live Lexer` 和 `Last Compile`，实时词法诊断不写入 compile result，也不会被显示成完整编译诊断。

## 公共接口

- Highlight adapter module path：`apps/ide/src/highlightAdapter.ts`。
- Highlight decoration helper path：`apps/ide/src/highlightDecorations.ts`。
- Editor integration path：`apps/ide/src/editor.ts` 通过 `createMaodieHighlightAdapter` 挂载 worker/decorations extension。
- `HighlightKind -> mao-hl-*` mapping：
  - `keyword -> mao-hl-keyword`
  - `identifier -> mao-hl-identifier`
  - `comment -> mao-hl-comment`
  - `string -> mao-hl-string`
  - `number -> mao-hl-number`
  - `boolean -> mao-hl-boolean`
  - `operator -> mao-hl-operator`
  - `punctuation -> mao-hl-punctuation`
  - `error -> mao-hl-error`
  - unknown -> `mao-hl-plain`
- Live lexer diagnostics state shape：`LiveLexerState { status: "loading" | "ready" | "failed"; diagnostics: Diagnostic[]; errorMessage: string | undefined }` in `apps/ide/src/state.ts`。
- Diagnostics panel source labeling rules：`Live Lexer` only renders highlight worker lexer diagnostics; `Last Compile` only renders `CompileResponse.diagnostics` from the last Run.

## 测试结果

- `pnpm nx run ide:typecheck`：通过。
- `pnpm nx run ide:test`：通过，1 个 test file / 13 个 tests。
- `pnpm ide:build`：通过，Vite 成功产出 `highlight.worker-*.js` 和 WASM asset。
- `pnpm style:guard`：通过，33 个 documentation checkpoints。
- `pnpm test`：通过，7 个 projects 和依赖任务全部成功。
- Local Chrome CDP smoke：`?source=let x = @\n` 等待 worker 后显示 `1 live / not run`，DOM 中存在 `mao-hl-keyword`、`mao-hl-error` 和 `mao-live-diagnostic mao-live-diagnostic-error`。

## 已知限制

实时 diagnostics 只覆盖 lexer diagnostics；parser/typechecker diagnostics 仍由 Run/compile 更新。

CodeMirror marker 使用当前 diagnostic span 的 byte offsets 转 UTF-16 range。没有 span 或 range 无法转换的 diagnostic 仍会出现在 Diagnostics 面板，但不会生成编辑器 underline。

## 下一任务入口

任务 05 应使用本任务产物做浏览器 smoke、性能基准和最终验收记录。建议重点检查快速输入时 stale response 丢弃、中文/emoji 附近 range、未闭合字符串/块注释/非法字符的 live lexer diagnostic，以及 Run 后 `Last Compile` 不被 live lexer 状态覆盖。
