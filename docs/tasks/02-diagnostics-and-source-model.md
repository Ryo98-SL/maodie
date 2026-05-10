# 02 Diagnostics and Source Model

## 目标

定义 Maodie 编译器统一的源码、位置、span、诊断、错误码和中文诊断输出模型。

## 对应特性

编译器基础模型和 IDE/CLI 诊断契约。所有后续阶段都通过这个模型报告错误和警告。

## 前置输入

- 任务 01 的 Rust workspace：根 `Cargo.toml`，`crates/maodie_compiler` facade crate，`maodie_*` crate 命名约定，Nx 项目 `rust` 的 `build`、`test`、`check`、`fmt`、`lint`、`wasm-build` targets。
- `docs/tasks/README.md` 中“中文优先、错误码稳定可测试”的要求。

## 实现范围

- 新增 diagnostics/source model crate。
- 定义 `SourceFile`、`SourceId`、`TextRange`、`TextPosition`、`Diagnostic`、`DiagnosticCode` 等核心类型。
- 支持从 byte offset 映射到行列位置。
- 支持 error/warning/info 三类 severity。
- 提供 CLI 友好的中文格式化输出和机器可读 JSON 结构。
- 建立错误码命名规则，例如 `MD0001`。

## 不做事项

- 不实现具体 lexer/parser 诊断。
- 不做多语言诊断翻译系统。
- 不做 LSP 协议适配。

## 输出产物

- Rust diagnostics/source model crate。
- 单元测试覆盖 offset、line/column、range、diagnostic serialization。
- 文档列出诊断字段稳定契约。

## 验收标准

- `cargo test --workspace` 成功。
- 测试包含中文诊断文案和稳定错误码。
- JSON 输出能被 TypeScript wrapper 直接消费，无需猜字段含义。

## 完成后验收方式

复验者运行 workspace 测试，并额外查看 diagnostics/source model 的单元测试是否覆盖 UTF-8 中文、byte range 到行列映射、JSON serialization 和 severity。人工抽查一个中文诊断示例，确认它包含错误码、severity、message、span。检查任务 03 的 `前置输入` 是否引用了最终的 span 与 diagnostic 字段名。

## 交接给下一任务

任务 03 可以直接使用本任务的源码与诊断类型来报告词法错误。任务 03 需要信任 span 使用 byte range，显示层负责行列换算。

## 风险与注意

Rust 字符串是 UTF-8，中文标识符和字符串字面量必须避免把 byte offset 误当字符索引。span 内部建议保持 byte range，展示时再换算行列。

## 交接记录

状态：已完成。

完成摘要：

- 新增 Rust crate `crates/maodie_diagnostics`，并加入 workspace。
- `crates/maodie_compiler` facade 通过 `diagnostics` 模块重新导出诊断与 source model 类型。
- 定义 `SourceId`、`SourceFile`、`TextRange`、`TextPosition`、`DiagnosticCode`、`DiagnosticSeverity`、`DiagnosticSpan`、`Diagnostic`。
- `SourceFile` 以 UTF-8 byte offset 为内部事实，支持 byte offset 到 1-based `line`/`column`/`byte_offset` 的换算，并拒绝落在 UTF-8 code point 中间的 offset/range。
- `DiagnosticCode` 使用稳定规则 `MD` + 四位数字，例如 `MD0001`。
- `DiagnosticSeverity` 的 JSON 字符串为 `error`、`warning`、`info`。
- `Diagnostic::render_chinese` 提供中文 CLI 输出，包含错误码、severity 中文标签、message、文件位置和源码摘录。

稳定 JSON 字段契约：

- `Diagnostic`：`code`、`severity`、`message`、`span`、`notes`。
- `DiagnosticSpan`：`source_id`、`file_name`、`range`、`start`、`end`。
- `TextRange`：`start`、`end`，均为 byte offset，半开区间 `[start, end)`。
- `TextPosition`：`line`、`column`、`byte_offset`，其中 `line` 和 `column` 为 1-based 显示位置。

测试命令结果：

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：通过。

已知限制：

- 本任务只提供统一模型和格式化能力，不包含 lexer/parser 的具体诊断生产逻辑。
- CLI source excerpt 当前标记 primary span 的起始行；多行 span 后续可由显示层扩展。

下一任务入口：

- 任务 03 lexer 应依赖 `maodie_diagnostics`，token 和词法错误 span 使用 `TextRange` byte range。
- 词法诊断应构造 `DiagnosticCode`、`DiagnosticSeverity::Error` 和 `DiagnosticSpan::from_source`，并保持 JSON 字段名不变。
