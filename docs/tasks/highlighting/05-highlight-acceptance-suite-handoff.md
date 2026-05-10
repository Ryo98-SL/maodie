# 05 Highlight Acceptance Suite Handoff

状态：已完成。

## 完成摘要

语法级染色第一阶段已形成最终验收闭环。`packages/compiler-wasm/src/index.test.ts` 新增最终 smoke 场景，直接通过 TS wrapper `highlightMaodieSource` 读取共享 fixture，确认：

- fixture 源码能输出稳定 highlight tokens。
- lexer diagnostics 和 `error` highlight token 在同一响应中同时保留。
- UTF-8 byte range 转 UTF-16 editor range 时，中文标识符 `名称` 没有发生偏移。

`docs/tasks/highlighting/README.md` 已更新任务链完成状态、highlight 相关验收命令汇总、adapter 覆盖核对和后续插件任务入口。

## 公共接口

最终稳定的 Rust highlight API：

- `highlight_source(source: &SourceFile) -> HighlightResult`
- `HighlightResult { tokens: Vec<HighlightToken>, diagnostics: Vec<Diagnostic> }`
- `HighlightToken { kind: HighlightKind, range: TextRange }`

最终稳定的 TS highlight API：

- `MaodieCompilerWasm.highlight(source, options?)`
- `highlightMaodieSource(source, options?)`
- `HighlightResponse { ok, tokens, diagnostics }`
- `HighlightToken { kind, range: { start, end } }`
- range helpers：`byteOffsetToUtf16Offset`、`byteOffsetToUtf16Position`、`byteRangeToUtf16Range`、`byteRangeToUtf16LineColumnRange`

`HighlightKind` 取值：

- `keyword`
- `identifier`
- `comment`
- `string`
- `number`
- `boolean`
- `operator`
- `punctuation`
- `error`

fixture 路径：

- `docs/tasks/highlighting/fixtures/syntax-highlight.mao`
- `docs/tasks/highlighting/fixtures/syntax-highlight.tokens.json`
- `docs/tasks/highlighting/fixtures/README.md`

adapter contract 路径：

- `docs/tasks/highlighting/adapters/index.md`
- `docs/tasks/highlighting/adapters/web-ide.md`
- `docs/tasks/highlighting/adapters/vscode.md`
- `docs/tasks/highlighting/adapters/jetbrains.md`
- `docs/tasks/highlighting/adapters/README.md`

## 测试结果

- `pnpm nx run compiler-wasm:test`：通过，7 个 Vitest 测试全部通过，包含最终 highlight acceptance smoke。
- `cargo fmt --all --check`：通过。
- `cargo test --workspace`：通过。
- `pnpm typecheck`：通过。
- `pnpm test`：通过。
- `pnpm style:guard`：通过。

adapter 契约核对：

- Web IDE、VSCode、JetBrains 文档均覆盖全部 9 个 `HighlightKind`。
- 三类文档均规定 unknown kind 降级为 plain/default，不抛错、不中断后续 token 渲染。
- 三类文档均要求先做 UTF-8 byte range 到 UTF-16 editor range 转换。
- 三类文档均明确 diagnostics 由诊断层消费，不混入 token kind 映射。

## 已知限制

第一阶段结束后，真实 Web IDE UI 染色和外部 IDE 插件仍是后续任务。

第一阶段只保证语法级 token；函数名、类型名、字段、局部变量等语义分类仍需后续语言服务或语义 token 能力。

## 下一任务入口

后续可以拆成三个独立任务，三者都应先复用共享 fixture 做 adapter smoke test：

- Web IDE 编辑器接入：从 `docs/tasks/highlighting/adapters/web-ide.md` 开始，接入 CodeMirror 或 Monaco token adapter。
- VSCode extension 最小可用版：从 `docs/tasks/highlighting/adapters/vscode.md` 开始，实现 `DocumentSemanticTokensProvider` 和 diagnostics collection。
- JetBrains plugin 最小可用版：从 `docs/tasks/highlighting/adapters/jetbrains.md` 开始，实现 highlight 驱动的 lexer adapter 和 `SyntaxHighlighter` 查表。
