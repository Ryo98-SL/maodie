# 05 Web IDE Highlight Acceptance

## 目标

建立 Web IDE 增量代码染色的最终验收闭环，确认 Rust 增量 session、WASM/TS/worker、CodeMirror shell、decorations 和实时 lexer diagnostics 可以稳定协作。

## 前置输入

- `01-incremental-highlight-session-handoff.md`
- `02-wasm-ts-worker-session-handoff.md`
- `03-codemirror-editor-shell-handoff.md`
- `04-decorations-live-lexer-diagnostics-handoff.md`

## 实现范围

- 汇总所有 Web IDE 增量高亮测试命令。
- 增加或记录浏览器 smoke：
  - 默认示例高亮。
  - 中文标识符和 emoji 附近编辑。
  - 未闭合字符串。
  - 块注释开闭。
  - 非法字符 error token 和 live lexer diagnostic。
  - 示例切换。
  - Run 编译当前 CodeMirror 文档。
- 记录性能基准：中等源码连续输入时不阻塞主线程，worker stale response 被丢弃。
- 更新本目录 README 状态和后续增强入口。

## 不做事项

- 不引入语义级 token。
- 不实现 VSCode 或 JetBrains 插件。
- 不把 parser/typechecker diagnostics 改为实时。

## 输出产物

- 最终验收记录。
- 浏览器 smoke 记录或自动化 smoke。
- Web IDE 增量染色后续任务入口。

## 交接文档

任务完成后更新 `05-web-ide-highlight-acceptance-handoff.md`。

## 验收文档

复验者按 `05-web-ide-highlight-acceptance-acceptance.md` 执行。

