# 03 CodeMirror Editor Shell Acceptance

## 验收命令

- `pnpm nx run ide:typecheck`
- `pnpm nx run ide:test`
- `pnpm ide:build`
- `pnpm test`

## 人工检查

- Web IDE 不再渲染 `textarea#source-editor`，而是渲染 CodeMirror editor mount。
- 手动输入只更新 source state，不触发 compile。
- Run 按钮仍编译当前编辑器文本。
- 示例切换替换编辑器全文并清空旧 compile/evaluation 状态。
- `?source=` 注入仍可用于 smoke tests。
- 布局、滚动和右侧 panels 没有明显回退。

## 验收结论

状态：通过。

复验者：Codex。

日期：2026-05-11。

命令结果：

- `pnpm nx run ide:typecheck`：由 `pnpm typecheck` 覆盖通过。
- `pnpm nx run ide:test`：由 `pnpm test` 覆盖通过，`apps/ide/src/compilerClient.test.ts` 13 个 tests 通过。
- `pnpm ide:build`：通过。
- `pnpm test`：通过，7 个 projects 和 4 个依赖任务。

人工检查结论：

- Web IDE 渲染 `#source-editor[data-editor-mount="codemirror"]` 并挂载 CodeMirror，不再渲染 `<textarea>`。
- 手动输入只更新 source state、live lexer 和 stale compile invalidation，不触发完整 compile。
- Run 从当前 CodeMirror document 读取源码并编译。
- 示例切换通过 editor transaction 替换全文，清空旧 compile/evaluation 状态。
- `?source=` 注入仍可用于 smoke，最终浏览器 smoke 已覆盖。
- fixed viewport、CodeMirror scroller 和右侧 panels 滚动布局保持可用。
