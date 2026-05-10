# 03 Highlight Fixtures and Ranges

## 目标

建立跨运行时 fixture 和 offset 转换验证，保证 Rust、WASM、TS 和未来编辑器适配看到同一组 highlight token。

## 前置输入

- `02-highlight-wasm-and-ts-handoff.md` 中确认的 TS highlight API。
- 现有 source model 的 byte offset、line、column 规则。

## 实现范围

- 新增典型 `.mao` highlight fixture：
  - 关键字、标识符、整数、bool、字符串。
  - 行注释和块注释。
  - 中文标识符。
  - 非法字符 error token。
- 新增 TS range 转换工具，把 Rust byte range 转成编辑器常用的 UTF-16 offset 或 line/column range。
- 添加 fixture 测试，比较 Rust/WASM/TS 输出一致性。

## 不做事项

- 不接入真实编辑器组件。
- 不决定主题颜色。
- 不引入语义 token fixture。

## 输出产物

- Highlight golden fixture。
- UTF-8 byte range 到 UTF-16 editor range 的转换工具和测试。
- 记录 fixture 更新规则。

## 交接文档

任务完成后更新 `03-highlight-fixtures-and-ranges-handoff.md`。

## 验收文档

复验者按 `03-highlight-fixtures-and-ranges-acceptance.md` 执行。

