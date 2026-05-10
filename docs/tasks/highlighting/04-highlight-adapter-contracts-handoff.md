# 04 Highlight Adapter Contracts Handoff

状态：已完成。

## 完成摘要

已新增 Web IDE、VSCode、JetBrains 三类 highlight adapter 契约文档。文档统一规定适配层只消费 `HighlightToken` / `HighlightResponse`，进入平台 API 前先把 Rust UTF-8 byte range 转换为编辑器 UTF-16 range，并且不在适配层重复实现 Maodie lexer。

三类文档都覆盖当前全部 `HighlightKind`：`keyword`、`identifier`、`comment`、`string`、`number`、`boolean`、`operator`、`punctuation`、`error`。未知 kind 必须降级为 plain/default，不能抛错破坏编辑器渲染。

## 公共接口

- 共享适配契约：`docs/tasks/highlighting/adapters/index.md`
- Web IDE 映射文档：`docs/tasks/highlighting/adapters/web-ide.md`
- VSCode 映射文档：`docs/tasks/highlighting/adapters/vscode.md`
- JetBrains 映射文档：`docs/tasks/highlighting/adapters/jetbrains.md`
- adapters 模块索引：`docs/tasks/highlighting/adapters/README.md`

unknown token fallback 规则：

- 未知 `HighlightToken.kind`、空 token 或平台无法识别的 token type 必须降级为 plain/default 文本样式。
- 无效 range 只跳过该 token，不影响后续 token。
- error token 仍映射为平台错误/invalid 分类，但诊断 underline、marker、inspection 由 diagnostics 层负责。
- 适配层不得因未知 kind 或无法映射的 token 中断编辑器渲染。

## 测试结果

- `pnpm style:guard`：通过，当前文档和源码结构检查通过。
- `pnpm typecheck`：通过，6 个 Nx project 的 typecheck 目标全部通过。
- `pnpm test`：通过，7 个 Nx project 的 test 目标全部通过。

## 已知限制

本任务只定义适配契约，不交付外部 IDE 插件。

Web IDE 文档同时给出 CodeMirror 和 Monaco 命名契约，但本任务没有接入真实浏览器编辑器。

VSCode 文档定义 semantic token / TextMate fallback 策略，但本任务没有创建 extension、language configuration 或 marketplace metadata。

JetBrains 文档定义 Lexer / SyntaxHighlighter / `TextAttributesKey` 策略，但本任务没有创建 IntelliJ Platform plugin。

## 下一任务入口

任务 05 应读取 `docs/tasks/highlighting/adapters/index.md` 及三类平台文档，验证契约文档、fixture、range 转换和 TS/Rust API 能形成完整交付闭环。后续 Web IDE、VSCode、JetBrains 插件任务应以本目录为入口，先实现平台 adapter 和 fixture smoke test，再考虑插件打包或语言服务能力。
