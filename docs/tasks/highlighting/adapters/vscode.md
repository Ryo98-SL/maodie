# VSCode Highlight Adapter Contract

## 适配目标

VSCode 适配层负责把 Maodie `HighlightToken` 映射到 VSCode semantic tokens，必要时提供 TextMate scope fallback。第一版只定义映射策略，不交付 VSCode extension、不发布语法包。

## 输入和 range

VSCode 扩展应调用 `@maodie/compiler-wasm` 或等价 WASM bridge 获取 `HighlightResponse`。每个 token 的 byte range 必须转换为 VSCode 使用的 0-based UTF-16 `line` / `character` 范围。TS 扩展可直接复用 `byteRangeToUtf16LineColumnRange(source, token.range)`。

`HighlightResponse.ok === false` 时仍应发布已有 tokens。lexer diagnostics 应通过 VSCode diagnostics collection 呈现，不应由 semantic token provider 内联处理。

## Semantic token 映射

| `HighlightKind` | VSCode semantic token type | Modifier | TextMate fallback scope | 说明 |
| --- | --- | --- | --- | --- |
| `keyword` | `keyword` | none | `keyword.control.maodie` | 语言关键字。 |
| `identifier` | `variable` | none | `variable.other.maodie` | 无语义区分的标识符。 |
| `comment` | `comment` | none | `comment.line.double-slash.maodie` / `comment.block.maodie` | Maodie 通用 token 不区分注释形态；TextMate fallback 可由正则细分。 |
| `string` | `string` | none | `string.quoted.double.maodie` | 字符串字面量。 |
| `number` | `number` | none | `constant.numeric.integer.maodie` | 整数字面量。 |
| `boolean` | `keyword` | `readonly` | `constant.language.boolean.maodie` | VSCode 默认 semantic token 没有稳定 `boolean` type，第一版用 keyword + modifier 表达语言常量。 |
| `operator` | `operator` | none | `keyword.operator.maodie` | 箭头、比较、算术和 `?`。 |
| `punctuation` | no semantic token | none | `punctuation.separator.maodie` / `punctuation.definition.maodie` | VSCode semantic token 默认不强调标点，TextMate 或默认样式即可。 |
| `error` | no semantic token | none | `invalid.illegal.maodie` | 不伪装成合法语义；诊断层是主要错误呈现。 |
| unknown | no semantic token | none | `source.maodie` | 必须降级为默认文本样式。 |

扩展注册 `SemanticTokensLegend` 时应至少包含上表用到的 token type 和 modifier。如果 VSCode 或主题忽略某个 semantic token，文本仍必须保持可编辑和可读。

## TextMate fallback 策略

TextMate grammar 只能作为 fallback 或冷启动体验，不能成为 Maodie 语法事实来源。若第一版插件先提供 TextMate scope，应满足：

- scope 命名与上表一致。
- 不复制 Rust lexer 的完整状态机，只做轻量正则级 fallback。
- WASM semantic token provider 可用时，以 WASM 输出覆盖 TextMate 粗略结果。

## Fallback 和容错

- 未知 kind：不 push semantic token。
- 无效 range：跳过该 token，保留后续 token。
- 空 tokens：返回空 `SemanticTokens`，不报错。
- `punctuation` 和 `error` token：可以只依赖 TextMate fallback 或默认样式；具体红线、hover 或问题面板由 diagnostics collection 负责。

## 后续 VSCode 插件入口

后续 VSCode 任务应从一个 `DocumentSemanticTokensProvider` 开始：

1. 读取当前 document 文本。
2. 调用 Maodie highlight API。
3. 转换 byte range 到 UTF-16 position。
4. 按本文映射填充 `SemanticTokensBuilder`。
5. 用共享 fixture 验证全部 `HighlightKind`、中文标识符 range 和 unknown fallback。

插件打包、marketplace metadata、语言服务能力、增量 token delta 都不属于本任务。
