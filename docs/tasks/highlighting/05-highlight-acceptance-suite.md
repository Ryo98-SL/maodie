# 05 Highlight Acceptance Suite

## 目标

建立语法级染色第一阶段的最终验收闭环，确认通用 API、fixture、range 转换和适配契约可以支撑后续 Web IDE、VSCode、JetBrains 工作。

## 前置输入

- `01-highlight-contract-handoff.md`
- `02-highlight-wasm-and-ts-handoff.md`
- `03-highlight-fixtures-and-ranges-handoff.md`
- `04-highlight-adapter-contracts-handoff.md`
- `adapters/index.md`
- `adapters/web-ide.md`
- `adapters/vscode.md`
- `adapters/jetbrains.md`

## 实现范围

- 汇总所有 highlight 相关测试命令。
- 增加最终 smoke 场景：
  - 使用 TS wrapper 对 fixture 源码输出 highlight tokens。
  - 验证 diagnostics 和 error token 同时存在。
  - 验证 range 转换不偏移中文标识符。
- 更新 highlighting README 的完成状态。

## 不做事项

- 不新增语言语法。
- 不接入真实 Web IDE 编辑器。
- 不实现 VSCode 或 JetBrains 插件。

## 输出产物

- 最终验收记录。
- 后续 Web IDE、VSCode、JetBrains 插件任务入口。
- 核对 adapters 契约覆盖全部 `HighlightKind` 和 unknown fallback。

## 交接文档

任务完成后更新 `05-highlight-acceptance-suite-handoff.md`。

## 验收文档

复验者按 `05-highlight-acceptance-suite-acceptance.md` 执行。
