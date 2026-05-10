# 13 IDE Integration

## 目标

让浏览器 IDE 能编辑 `.mao` 源码，调用 WASM 编译器，并展示中文诊断和 AST/HIR/MIR/WASM 输出。

## 对应特性

Maodie v1 的交互式开发体验。

## 前置输入

- 任务 11 的 TS wrapper API。
- 当前 `apps/ide` Vite/Tailwind 项目结构。

## 实现范围

- IDE 使用 `.mao` 示例作为初始文档。
- 在浏览器中加载 compiler-wasm，优先放入 Web Worker 或明确记录单线程限制。
- 展示诊断列表、源码位置、编译状态。
- 展示 debug dump 选项：AST、HIR、MIR、WAT/WASM metadata。
- 对编译失败和 wasm 加载失败给出可读状态。

## 不做事项

- 不做 VSCode 级补全、跳转、悬浮和格式化。
- 不做多文件项目索引。
- 不做远程保存或账号系统。

## 输出产物

- IDE 编译面板和诊断视图。
- Browser-side compiler wrapper usage。
- IDE smoke test 或截图验证流程。

## 验收标准

- 浏览器打开 IDE 后能编译默认 `.mao` 示例。
- 修改源码为语法错误时能显示中文诊断。
- `pnpm ide:build` 成功。

## 完成后验收方式

复验者运行 `pnpm ide:build`，启动本地 IDE 后用默认 `.mao` 示例编译一次，再制造语法错误确认诊断刷新。人工检查 WASM 加载失败状态、编译中状态、诊断列表和 dump 面板。记录开发和生产构建下的 wasm 加载路径，并确认 IDE 只通过任务 11 的 TS wrapper 编译。

## 交接给下一任务

任务 14 可以把 IDE 作为可运行闭环的一部分验证。任务 14 需要记录 IDE URL、测试源码和预期诊断/输出。

## 风险与注意

浏览器 WASM 加载路径容易受 Vite build 输出影响。任务 13 必须记录开发和生产构建两种加载路径。

## 交接记录

状态：已完成。

实现记录：

- `apps/ide` 依赖切换为 `@maodie/compiler-wasm`，IDE 只通过任务 11 的 TS wrapper 调用编译器。
- 默认文档为 `workspace/main.mao`，初始源码使用 `.mao` 示例并支持浏览器内编辑；smoke test 可通过 `?source=` 注入临时源码。
- 新增 `src/compilerClient.ts`，通过 `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm?url` 加载 compiler-wasm；加载失败会在诊断区域显示中文状态。
- 新增 `src/view.ts`，展示编译中/加载中/诊断状态、诊断 code/severity/message/source position、artifact metadata，以及 AST/HIR/MIR/WAT/types dump tabs。
- 当前 IDE 编译仍在浏览器主线程运行；已在 `apps/ide/index.md`、`apps/ide/src/index.md` 和 `README.deep.md` 记录该限制。后续可把 `compilerClient.ts` 边界迁到 Web Worker。

WASM 加载路径：

- 开发构建：Vite 通过 `?url` import 服务 `target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm`。
- 生产构建：Vite 将该 `.wasm` 文件复制到 `dist/apps/ide/assets/maodie_wasm_api-*.wasm`，运行时代码使用重写后的 asset URL。

验证：

- `pnpm nx run ide:typecheck`：通过。
- `pnpm ide:build`：通过，并生成 production wasm asset。
- Headless Chrome 截图 smoke：默认源码显示 `Compiled`、`module.wat`/`module.wasm` metadata 和 AST dump；`?source=...return%20%40...` 显示中文 `MD0101`/`MD0201` 诊断和 `workspace/main.mao:4:10` 位置。

后续入口：

- 任务 14 可使用 `pnpm ide:dev` 打开 IDE，用默认源码确认编译成功，再把源码改为 `return @` 确认中文诊断和 dump 刷新。自动/截图 smoke 可打开 `/?source=module%20demo%0A%0Afn%20main(value%3A%20i32)%20-%3E%20i32%20%7B%0A%20%20return%20%40%0A%7D%0A` 验证语法错误诊断。
