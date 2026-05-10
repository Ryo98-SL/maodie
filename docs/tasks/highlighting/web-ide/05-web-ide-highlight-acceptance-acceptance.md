# 05 Web IDE Highlight Acceptance Acceptance

## 验收命令

- `cargo fmt --all --check`
- `cargo test --workspace`
- `pnpm typecheck`
- `pnpm test`
- `pnpm ide:build`
- `pnpm style:guard`

## 人工检查

- 所有上游 handoff 文档状态都是 `已完成`。
- 所有上游 acceptance 文档都有验收结论。
- Web IDE README 的任务链和实际产物一致。
- 默认示例、中文标识符、emoji、块注释、未闭合字符串、非法字符、示例切换和 Run 编译当前文档均通过 smoke。
- 输入期间没有触发完整 compile。
- 主线程输入体验没有明显阻塞；worker 过期响应不会覆盖新状态。

## 验收结论

状态：未验收。

记录复验者、日期、命令结果和人工检查结论。

