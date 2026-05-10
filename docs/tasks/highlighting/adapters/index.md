# Highlight Adapter Contracts

## 目的

本目录定义 Maodie 第一版语法级染色如何从通用 `HighlightToken` 适配到不同编辑器平台。适配层只消费 `@maodie/compiler-wasm` 或等价 WASM API 输出，不重新实现 lexer、parser 或主题系统。

## 输入契约

所有适配器都以同一份输入为事实来源：

- `HighlightResponse.ok`：当 lexer 产生错误诊断时为 `false`，但 `tokens` 仍可用于尽力染色。
- `HighlightResponse.tokens`：按源码顺序排列的 `HighlightToken[]`。
- `HighlightToken.kind`：`keyword`、`identifier`、`comment`、`string`、`number`、`boolean`、`operator`、`punctuation` 或 `error`。
- `HighlightToken.range`：Rust 侧 UTF-8 half-open byte range，字段为 `{ start, end }`。
- `HighlightResponse.diagnostics`：lexer diagnostics，由 IDE 诊断通道消费，不参与 token kind 映射。

适配器进入编辑器 API 前必须先把 byte range 转换为对应平台需要的编辑器范围。TypeScript/Web 侧优先使用 `@maodie/compiler-wasm` 导出的 `byteRangeToUtf16Range` 或 `byteRangeToUtf16LineColumnRange`。非 TS 插件应复刻任务 03 确认的 UTF-8 byte offset 到 UTF-16 line/character 规则，并用共享 fixture 校验。

## 目录结构

- `web-ide.md`：Web IDE 到 CodeMirror/Monaco 的 token class 映射契约。
- `vscode.md`：VSCode semantic token 与 TextMate scope 映射策略。
- `jetbrains.md`：JetBrains Lexer/SyntaxHighlighter 到 `TextAttributesKey` 的映射策略。

## 通用 fallback 规则

未知 `HighlightToken.kind`、空 token、无法识别的平台 token type，必须降级为 plain/default 文本样式。适配层不得因为未知 kind 抛错、停止渲染或丢弃后续合法 token。

推荐处理顺序：

1. 转换 range。无效 range 只跳过该 token，并把异常留给开发日志或测试断言。
2. 查表映射 `kind`。
3. 未命中时返回平台 plain/default，不注册额外颜色。
4. 继续处理后续 token。

## 第一版边界

第一版只定义协议和映射表，不交付 VSCode extension、JetBrains plugin，也不替代各 IDE 的主题系统。所有颜色由宿主主题决定；Maodie 只提供稳定语法分类。

后续插件任务应从本目录开始，先实现平台适配和 fixture smoke test，再考虑增量更新、缓存、diagnostics UI、语言服务能力或插件打包。
