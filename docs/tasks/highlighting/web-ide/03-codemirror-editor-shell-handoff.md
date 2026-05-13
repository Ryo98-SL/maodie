# 03 CodeMirror Editor Shell Handoff

状态：已完成。

## 完成摘要

Web IDE 左侧源码编辑器已从 `textarea` 换成 CodeMirror 6 editor shell。`renderWorkbench` 现在只渲染 `div#source-editor[data-editor-mount="codemirror"]`，真实 editor 由入口在 mount 上初始化。现有示例切换、手动 Run、Evaluation、source state、fixed viewport layout、右侧 panels 和 `?source=` smoke path 保持可用。

本任务只建立 editor shell，没有接入 highlight decorations 或实时 lexer diagnostics。

## 公共接口

- Editor setup module path：`apps/ide/src/editor.ts`。
- `createMaodieEditor({ parent, source, onSourceChange })` 创建 CodeMirror `EditorView`，初始化 document、line numbers、history/default keymap、line wrapping、dark theme 和 change listener。
- `MaodieEditor.readSource()` 返回当前 CodeMirror document 文本；Run 按钮通过它读取源码，再沿用现有 compile/evaluate 流程。
- `MaodieEditor.replaceSource(source)` 通过 CodeMirror transaction 替换全文；示例按钮调用该 API，然后清空旧 compile/evaluation state。
- `MaodieEditor.destroy()` 在每次 workbench rerender 前清理旧 `EditorView`。
- `apps/ide/src/initialSource.ts` 暴露 `createInitialSourceState(search)`，保持默认 v1 示例和 `?source=` 自定义源码路径。

## 测试结果

- `pnpm nx run ide:typecheck`：通过。
- `pnpm nx run ide:test`：通过，1 个 test file / 10 个 tests。
- `pnpm ide:build`：通过。
- `pnpm style:guard`：通过，33 个 documentation checkpoints。
- `pnpm test`：通过，7 个 projects 和依赖任务全部成功。
- `curl -I http://127.0.0.1:5176/`：dev server 返回 `200 OK`。
- Headless Chrome `--dump-dom http://127.0.0.1:5176/`：页面渲染 `.cm-editor`，`#source-editor` 为 CodeMirror mount。
- Headless Chrome `--dump-dom 'http://127.0.0.1:5176/?source=...'`：自定义 source 进入 CodeMirror 文档并显示 `Custom source`。

内置浏览器连接在本机 smoke 时两次超时，因此本轮浏览器验证使用 headless Chrome 代替。

## 已知限制

本任务只替换编辑器 shell，不渲染 highlight decorations，不显示实时 lexer diagnostics。

## 下一任务入口

任务 04 应从 `apps/ide/src/editor.ts` 接入 worker session、decorations 和 diagnostics。建议扩展 `createMaodieEditor` 的 options 或 extensions 组装点，复用现有 `readSource`、`replaceSource` 和 `destroy` 生命周期，不重写 `main.ts` 的 editor mount 流程。
