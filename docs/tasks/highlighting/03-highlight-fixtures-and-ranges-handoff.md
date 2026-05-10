# 03 Highlight Fixtures and Ranges Handoff

状态：已完成。

## 完成摘要

已新增共享 highlight fixture 和 golden token 文件，并让 Rust `maodie_syntax` 与 TS/WASM wrapper 测试读取同一份 fixture。fixture 覆盖关键字、标识符、整数、bool、字符串、行注释、块注释、中文标识符和非法字符 `error` token。

`packages/compiler-wasm` 已新增 UTF-8 byte range 到 UTF-16 editor range 的转换工具。转换结果使用编辑器常见的 0-based UTF-16 absolute offset 与 0-based line/character position；无效 byte offset、拆分 UTF-8 code point 的 offset、逆序 range 会抛 `RangeError`。

## 公共接口

fixture 文件路径：

- `docs/tasks/highlighting/fixtures/syntax-highlight.mao`
- `docs/tasks/highlighting/fixtures/syntax-highlight.tokens.json`
- `docs/tasks/highlighting/fixtures/README.md`

TS range 转换工具：

- `byteOffsetToUtf16Offset(source, byteOffset)`
- `byteOffsetToUtf16Position(source, byteOffset)`
- `byteRangeToUtf16Range(source, range)`
- `byteRangeToUtf16LineColumnRange(source, range)`

导出类型：

- `ByteRange`
- `Utf16OffsetRange`
- `Utf16Position`
- `Utf16LineColumnRange`

fixture 更新流程：

1. 先修改 `syntax-highlight.mao`。
2. 按 Rust lexer/highlight 输出更新 `syntax-highlight.tokens.json`，保留 UTF-8 byte range、`error` token 和 diagnostic range。
3. 执行 `cargo test -p maodie_syntax matches_shared_highlight_golden_fixture`。
4. 执行 `pnpm nx run compiler-wasm:test`。

## 测试结果

- `cargo fmt --all --check`：通过。
- `pnpm nx run compiler-wasm:typecheck`：通过。
- `cargo test -p maodie_syntax matches_shared_highlight_golden_fixture`：通过。
- `pnpm nx run compiler-wasm:test`：通过，6 个 Vitest 测试全部通过。
- `pnpm style:guard`：通过，33 个文档检查点通过。
- `pnpm test`：通过，7 个 Nx project 的测试目标全部通过。
- `cargo test --workspace`：通过，Rust workspace 单元测试和 doc-tests 全部通过。

## 已知限制

fixture 只覆盖语法级分类，不覆盖 IDE 主题渲染。

range 转换工具不做编辑器 API 绑定，只提供通用 UTF-16 offset 和 line/character range。VSCode、JetBrains、Web IDE 的具体 token 映射仍由任务 04 定义。

## 下一任务入口

任务 04 应使用本任务确认的 fixture 和 range 转换规则设计 Web IDE、VSCode、JetBrains 映射。适配层应继续把 Rust byte range 视为事实来源，并在进入编辑器 API 前调用 `packages/compiler-wasm` 导出的 UTF-16 range helper。
