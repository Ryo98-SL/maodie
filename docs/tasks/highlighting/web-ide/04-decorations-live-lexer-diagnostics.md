# 04 Decorations Live Lexer Diagnostics

## 目标

在 CodeMirror editor shell 上接入 highlight worker session，用 `HighlightKind -> mao-hl-*` decorations 渲染代码染色，并把实时 lexer diagnostics 显示到编辑器标记和 Diagnostics 面板。

## 前置输入

- `03-codemirror-editor-shell-handoff.md`。
- `02-wasm-ts-worker-session-handoff.md`。
- `docs/tasks/highlighting/adapters/web-ide.md`。

## 实现范围

- 新增 CodeMirror `ViewPlugin` 或等效 adapter，监听 changes 并发送 worker update。
- 输入后先 map 旧 decorations，worker 返回 patch 后精确更新 affected range。
- 覆盖全部 `HighlightKind` 到 `mao-hl-*` class，unknown kind 降级为 plain/default。
- error token 保留高亮样式。
- 实时 lexer diagnostics 通过 CodeMirror diagnostics/markers 展示。
- 右侧 Diagnostics 面板区分 `Live Lexer` 和 `Last Compile`，避免把词法诊断误认为完整编译诊断。
- stale worker response 不得覆盖新 decorations 或新 diagnostics。

## 不做事项

- 不显示 parser/typechecker 实时 diagnostics。
- 不把 live lexer diagnostics 写入 compile result。
- 不改变 VSCode 或 JetBrains adapter。

## 输出产物

- CodeMirror highlight adapter。
- Maodie highlight CSS class 或 theme rules。
- Live lexer diagnostics UI 状态和 panel 渲染。
- Fixture/smoke tests 覆盖全部 9 个 `HighlightKind`、中文 range、error token 和 unknown fallback。

## 交接文档

任务完成后更新 `04-decorations-live-lexer-diagnostics-handoff.md`。

## 验收文档

复验者按 `04-decorations-live-lexer-diagnostics-acceptance.md` 执行。

