# 04 Highlight Adapter Contracts

## 目标

为 Web IDE、VSCode 和 JetBrains 定义适配层契约，明确如何把通用 `HighlightToken` 映射到各平台高亮模型。

## 前置输入

- `03-highlight-fixtures-and-ranges-handoff.md` 中确认的 fixture 和 range 转换规则。
- VSCode 语法高亮和 semantic token 机制。
- JetBrains Lexer 与 SyntaxHighlighter 机制。

## 实现范围

- 编写 Web IDE 映射文档：通用 token kind 到 CodeMirror/Monaco token class 的映射。
- 编写 VSCode 映射文档：通用 token kind 到 semantic token type 或 TextMate scope 的映射策略。
- 编写 JetBrains 映射文档：通用 token kind 到 `TextAttributesKey` 的映射策略。
- 明确第一版外部 IDE 不打包插件，只保证协议可适配。

## 不做事项

- 不实现 VSCode extension。
- 不实现 JetBrains plugin。
- 不替代各 IDE 的主题系统。

## 输出产物

- 三类适配契约文档。
- token fallback 规则：未知 token kind 必须降级为 plain/default，不允许抛错破坏编辑。
- 后续插件任务的入口说明。

## 交接文档

任务完成后更新 `04-highlight-adapter-contracts-handoff.md`。

## 验收文档

复验者按 `04-highlight-adapter-contracts-acceptance.md` 执行。

