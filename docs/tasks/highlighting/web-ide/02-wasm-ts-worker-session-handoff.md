# 02 WASM TS Worker Session Handoff

状态：未开始。

## 完成摘要

待任务 02 完成后填写。

## 公共接口

待填写：

- WASM ABI 函数名。
- TS session wrapper 类型和方法。
- Worker request/response message shapes。
- version、stale response 和 fallback/full rehighlight 规则。

## 测试结果

待填写命令和结果。

## 已知限制

Worker 只服务语法级 highlighter，不运行 compile、evaluation、parser 或 typechecker。

## 下一任务入口

任务 03 应使用本任务确认的 TS/worker API 接入 CodeMirror，不直接调用 Rust session 或 WASM 内存。

