# 14 V1 Acceptance Suite

## 目标

建立 Maodie v1 端到端验收套件，确认语言、编译器、CLI 和 IDE 达到可运行闭环。

## 对应特性

v1 完成定义和回归测试体系。

## 前置输入

- 任务 10 的 WASM backend。
- 任务 11 的 TS wrapper。
- 任务 12 的 CLI。
- 任务 13 的 IDE。

## 实现范围

- 编写代表性 `.mao` 示例，覆盖函数、let/mut、if、match、struct、enum、trait/impl、泛型、Option/Result、`?`。
- 建立端到端 CLI 测试：源码到 WAT/WASM、诊断、exit code。
- 建立 IDE smoke 验证：默认示例编译成功，错误示例显示诊断。
- 汇总 v1 支持/暂缓能力清单。
- 记录下一阶段候选：native backend、async、包管理、完整标准库。

## 不做事项

- 不做发布、官网、版本分发。
- 不新增 v1 范围外语言特性。
- 不把验收测试变成大型 benchmark。

## 输出产物

- `examples/` 中的 v1 示例。
- CLI/IDE/编译器端到端测试。
- v1 acceptance report。
- 更新 README 和任务手册完成状态。

## 验收标准

- 代表性 `.mao` 示例能通过 CLI 编译到 WASM。
- IDE 能加载同一示例并展示诊断与 dumps。
- 错误示例有稳定中文错误码。
- `pnpm build`、`pnpm test`、Rust workspace tests 全部成功。

## 完成后验收方式

复验者从仓库根目录执行完整验收命令组：Rust workspace tests、`pnpm build`、`pnpm test`、CLI 端到端示例、IDE build 和浏览器 smoke test。人工检查 v1 支持/暂缓能力清单是否与实际行为一致，确保没有把发布、官网、包管理、native backend 或 async 混入 v1 完成定义。最终验收记录应汇总所有失败过但已修复的问题。

## 交接给下一任务

任务 14 是 v1 手册终点。完成后应创建下一阶段路线手册，而不是继续在本任务中追加 native、async 或包管理实现。

## 风险与注意

验收套件应保护 v1 行为稳定，不应在最后阶段引入新语言设计。发现缺口时优先回到对应上游任务补齐。

## 交接记录

状态：已完成。

实现记录：

- 新增并对齐 `examples/main.mao`、`examples/v1_acceptance.mao`、`examples/v1_surface.mao` 和 `examples/v1_error.mao`。
- `examples/v1_acceptance.mao` 是 v1 主闭环样例，覆盖函数、`let mut`、赋值、`if`、`match`、`struct`、`enum`、`trait`/`impl`、泛型函数、`Option`、`Result` 和 `?`。
- CLI smoke tests 现在直接编译 public examples，验证 WAT stdout、WASM artifact、AST/HIR/MIR dumps、错误 exit code 和中文诊断。
- IDE smoke tests 现在验证默认源码与 v1 acceptance example 对齐，默认示例可编译并渲染 dumps，错误示例可渲染稳定中文诊断。
- 新增 `docs/v1-acceptance-report.md`，汇总 v1 支持能力、暂缓能力、验收 fixture 和修复过的问题。
- README、README.deep、CLI/IDE module docs 和 examples module docs 已同步 v1 acceptance 入口。

验证：

- `cargo fmt --all --check`：通过。
- `pnpm style:guard`：通过。
- `pnpm rust:test`：通过。
- `pnpm build`：通过。
- `pnpm test`：通过。
- `pnpm ide:build`：通过。
- `pnpm nx run cli:test`：通过。
- `pnpm nx run ide:test`：通过。
- `node packages/cli/dist/main.js compile examples/v1_acceptance.mao --emit wasm --out /private/tmp/maodie-v1-acceptance.wasm`：通过。
- `node packages/cli/dist/main.js compile examples/v1_error.mao --emit wat`：按预期返回 exit code 1，并输出 `MD0101`、`MD0201` 中文诊断。

修复记录：

- 第一版验收示例使用了 `Enum.Variant(payload)` match 语法，发现当前 v1 parser/type checker 支持的是 `Enum.Variant` 路径模式；示例已改为当前实现支持的形式。
- 第一版 IDE 默认示例沿用了同一 payload pattern，已改为与 `examples/v1_acceptance.mao` 完全对齐。
- 曾将 `pnpm build` 和 `pnpm test` 并行执行，触发 Cargo `target/` 临时文件竞争；按任务要求顺序复验后全部通过。

已知限制：

- v1 `match` 变体模式使用 `Enum.Variant` 路径，不支持 `Enum.Variant(payload)` 绑定语法；payload projection 目前通过 `Result` 的 `?` lowering 路径覆盖。
- 浏览器 IDE 仍在主线程调用 WASM compiler；任务 13 已记录后续可迁移到 Worker。
- 未在本任务实现 native backend、async、包管理、完整标准库、发布或官网能力。

下一阶段入口：

- 从 `docs/v1-acceptance-report.md` 的 `Deferred After V1` 开始拆分下一阶段路线手册。
- 候选下一阶段任务包括 native backend、async、包管理和完整标准库；这些应作为新路线任务创建，不继续追加到任务 14。
