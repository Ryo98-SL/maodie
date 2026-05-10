# Web IDE Highlight Adapter Contract

## 适配目标

Web IDE 适配层负责把通用 `HighlightToken` 映射到浏览器编辑器可消费的 token class 或 highlight tag。第一版同时给出 CodeMirror 和 Monaco 的命名契约；实际 Web IDE 可以先接入其中一种编辑器，但映射表必须保持一致。

## 输入和 range

输入使用 `@maodie/compiler-wasm`：

- 调用 `highlightMaodieSource(source, { sourcePath })` 或已创建实例的 `MaodieCompilerWasm.highlight(source, options)`。
- 对每个 token 调用 `byteRangeToUtf16LineColumnRange(source, token.range)`。CodeMirror 和 Monaco 都应使用 0-based UTF-16 line/character 坐标。
- 如果 `HighlightResponse.ok === false`，仍渲染 `tokens`，并把 `diagnostics` 交给诊断面板或 marker 层。

Web 适配层不得重新扫描 `.mao` 源码来推断 token kind。重跑 highlight 可以由编辑器变更事件触发，但分类来源仍是 WASM/Rust lexer。

## Token class 映射

| `HighlightKind` | CodeMirror tag/class | Monaco token class | 说明 |
| --- | --- | --- | --- |
| `keyword` | `tags.keyword` / `mao-hl-keyword` | `keyword.maodie` | 语言关键字。 |
| `identifier` | `tags.variableName` / `mao-hl-identifier` | `identifier.maodie` | 无语义区分的普通标识符。 |
| `comment` | `tags.comment` / `mao-hl-comment` | `comment.maodie` | 行注释和块注释。 |
| `string` | `tags.string` / `mao-hl-string` | `string.maodie` | 字符串字面量。 |
| `number` | `tags.number` / `mao-hl-number` | `number.maodie` | 整数字面量。 |
| `boolean` | `tags.bool` / `mao-hl-boolean` | `constant.language.boolean.maodie` | `true` / `false`。 |
| `operator` | `tags.operator` / `mao-hl-operator` | `operator.maodie` | 箭头、比较、算术和 `?`。 |
| `punctuation` | `tags.punctuation` / `mao-hl-punctuation` | `delimiter.maodie` | 括号、逗号、冒号、分号和点。 |
| `error` | `tags.invalid` / `mao-hl-error` | `invalid.maodie` | lexer error token，通常由主题或 marker 额外强调。 |
| unknown | no tag / `mao-hl-plain` | `source.maodie` | 必须降级为默认文本样式。 |

`mao-hl-*` 是 Maodie 自有、跨 Web 编辑器稳定的 CSS class 命名。CodeMirror 可以通过 `HighlightStyle.define`、decorations 或等价机制把 token 映射到这些 class；Monaco 可以在 tokens provider 中返回 `type.maodie` token class，再由主题决定颜色。

## CodeMirror 约定

CodeMirror 适配器应把每个 token 转换为 UTF-16 line/character range，并用 Maodie kind 查表得到 tag 或 class。若使用 decorations，class 应使用上表的 `mao-hl-*` 名称；若使用 `HighlightStyle`，应优先使用对应 `tags.*`，并把 Maodie class 作为调试或自定义主题挂钩。

CodeMirror fallback 行为：

- 未知 kind：不加 highlight tag，或使用 `mao-hl-plain`。
- 无效 range：跳过该 token，不影响后续 token。
- error token：保留为 `tags.invalid`，diagnostic underline 由 diagnostics 层单独处理。

## Monaco 约定

Monaco 适配器应返回上表中的 token class，例如 `keyword.maodie`。这些 class 只表达语法类别，不携带颜色。主题可以把它们映射到 Monaco 内置 token 规则或自定义规则。

Monaco fallback 行为：

- 未知 kind：返回 `source.maodie` 或空 token type。
- 无效 range：跳过该 token，不影响后续 token。
- error token：返回 `invalid.maodie`，diagnostic marker 由 diagnostics 层单独处理。

## 后续 Web IDE 入口

后续 Web IDE 任务应在编辑器客户端边界新增一个小型 adapter：

1. 调用 `@maodie/compiler-wasm` highlight API。
2. 通过 `ranges.ts` helper 转换 UTF-16 range。
3. 按本文映射 token class。
4. 用共享 fixture 做 smoke test，确认中文标识符后的 token 不偏移，unknown kind 不破坏渲染。
