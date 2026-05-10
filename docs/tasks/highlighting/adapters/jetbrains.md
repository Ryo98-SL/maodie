# JetBrains Highlight Adapter Contract

## 适配目标

JetBrains 适配层负责把 Maodie `HighlightToken` 映射到 IntelliJ Platform 的 `Lexer`、`SyntaxHighlighter` 和 `TextAttributesKey`。第一版只定义契约，不交付 JetBrains plugin、不替代 IDE 主题。

## 输入和 range

JetBrains 插件通常通过 `Lexer` 逐 token 输出 `IElementType`，再由 `SyntaxHighlighter.getTokenHighlights` 映射到 `TextAttributesKey`。Maodie 插件可以用 Rust/WASM highlight 输出驱动 lexer，但仍必须满足 IntelliJ 对 token 顺序和 range 的要求：

- `HighlightToken.range` 是 UTF-8 byte range，插件进入 IntelliJ API 前必须转换为 UTF-16 document offset。
- Token offset 必须是 half-open `[startOffset, endOffset)`。
- `HighlightResponse.ok === false` 时仍输出已有 tokens；diagnostics 由 annotator、inspection 或 external annotator 层消费。
- 不得在 JetBrains 插件内复制 Maodie lexer 作为事实来源。

Kotlin/Java 插件若不能直接复用 TS helper，应按任务 03 的规则实现等价转换，并用共享 fixture 校验中文标识符和 error token。

## TextAttributesKey 映射

| `HighlightKind` | Suggested `IElementType` | `TextAttributesKey` | Default fallback key | 说明 |
| --- | --- | --- | --- | --- |
| `keyword` | `MAODIE_KEYWORD` | `MAODIE_KEYWORD` | `DefaultLanguageHighlighterColors.KEYWORD` | 语言关键字。 |
| `identifier` | `MAODIE_IDENTIFIER` | `MAODIE_IDENTIFIER` | `DefaultLanguageHighlighterColors.IDENTIFIER` | 无语义区分的标识符。 |
| `comment` | `MAODIE_COMMENT` | `MAODIE_COMMENT` | `DefaultLanguageHighlighterColors.LINE_COMMENT` | 行注释和块注释都可先共用。 |
| `string` | `MAODIE_STRING` | `MAODIE_STRING` | `DefaultLanguageHighlighterColors.STRING` | 字符串字面量。 |
| `number` | `MAODIE_NUMBER` | `MAODIE_NUMBER` | `DefaultLanguageHighlighterColors.NUMBER` | 整数字面量。 |
| `boolean` | `MAODIE_BOOLEAN` | `MAODIE_BOOLEAN` | `DefaultLanguageHighlighterColors.KEYWORD` | 布尔字面量，主题可选择与关键字同色。 |
| `operator` | `MAODIE_OPERATOR` | `MAODIE_OPERATOR` | `DefaultLanguageHighlighterColors.OPERATION_SIGN` | 箭头、比较、算术和 `?`。 |
| `punctuation` | `MAODIE_PUNCTUATION` | `MAODIE_PUNCTUATION` | `DefaultLanguageHighlighterColors.COMMA` | 括号、分隔符和点。 |
| `error` | `MAODIE_BAD_CHARACTER` | `MAODIE_BAD_CHARACTER` | `HighlighterColors.BAD_CHARACTER` | lexer error token。 |
| unknown | `MAODIE_PLAIN` | none | none | 必须降级为默认文本样式。 |

`TextAttributesKey.createTextAttributesKey("MAODIE_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD)` 这类命名应保持稳定。主题作者可以覆盖 `MAODIE_*` key，但插件不能硬编码颜色值。

## Lexer 和 SyntaxHighlighter 约定

JetBrains Lexer 适配器应缓存一次 highlight 调用的 token 列表，并按 UTF-16 offset 顺序向 IDE 暴露 token：

- `getTokenStart()` / `getTokenEnd()` 返回 UTF-16 document offsets。
- `getTokenType()` 返回上表建议的 `IElementType`。
- unknown kind 可返回 `TokenType.WHITE_SPACE` 之外的 plain token type，或让 `SyntaxHighlighter` 返回空 attributes。不要把未知 kind 标成 bad character。
- error kind 才映射到 `MAODIE_BAD_CHARACTER`。

`SyntaxHighlighter.getTokenHighlights(type)` 只做 `IElementType -> TextAttributesKey[]` 查表。诊断 underline、quick fix、inspection 不属于 syntax highlighter 责任。

## Fallback 和容错

- 未知 kind：返回 plain/default，不抛异常。
- 无效 range：跳过该 token，保证 lexer 能继续推进。
- range 转换失败：开发构建可记录日志，发布构建必须优雅降级。
- 空 tokens：lexer 到达 EOF，不创建 synthetic error token。

## 后续 JetBrains 插件入口

后续 JetBrains 任务应先实现最小插件骨架：

1. 注册 Maodie file type 和 language。
2. 实现由 Maodie highlight API 驱动的 lexer adapter。
3. 实现 `SyntaxHighlighter` 的 `IElementType -> TextAttributesKey` 查表。
4. 用共享 fixture 验证全部 `HighlightKind`、中文标识符 UTF-16 offset、error token 和 unknown fallback。

PSI、completion、inspection、formatter、插件打包和 Marketplace 发布均不属于本任务。
