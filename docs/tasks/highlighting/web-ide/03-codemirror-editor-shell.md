# 03 CodeMirror Editor Shell

## 目标

把 Web IDE 左侧源码编辑器从 `textarea` 替换为 CodeMirror 6 editor shell，同时保持现有示例切换、Run 编译、Evaluation、source state 和布局行为。

## 前置输入

- `02-wasm-ts-worker-session-handoff.md`。
- `docs/tasks/highlighting/adapters/web-ide.md`。
- 当前 `apps/ide/src/main.ts`、`view.ts`、`state.ts` 和 `compilerClient.ts`。

## 实现范围

- 为 `@maodie/ide` 添加 CodeMirror 6 依赖。
- `renderWorkbench` 渲染 editor mount，不再渲染 `textarea`。
- 新增 editor setup 模块，初始化 CodeMirror document、theme、basic keymap 和 change listener。
- CodeMirror change listener 同步 `state.source`，但不触发 full compile。
- 示例按钮通过 CodeMirror transaction 替换全文，并触发 worker reset。
- Run 按钮读取 CodeMirror 当前文档文本，沿用现有 compile/evaluate 流程。
- 保持现有 fixed viewport、右侧 panels 和 smoke test query `?source=` 行为。

## 不做事项

- 不实现 decorations 或实时 diagnostics。
- 不改变 compile/evaluate 数据流。
- 不引入 Monaco。

## 输出产物

- CodeMirror editor shell。
- 编辑器生命周期清理和示例切换逻辑。
- IDE tests 更新，确认 editor mount、Run、示例切换和 query source 可用。

## 交接文档

任务完成后更新 `03-codemirror-editor-shell-handoff.md`。

## 验收文档

复验者按 `03-codemirror-editor-shell-acceptance.md` 执行。

