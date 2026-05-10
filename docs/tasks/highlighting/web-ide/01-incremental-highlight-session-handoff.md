# 01 Incremental Highlight Session Handoff

状态：未开始。

## 完成摘要

待任务 01 完成后填写。

## 公共接口

待填写：

- Rust session 类型名。
- create/reset/update 方法签名。
- edit delta 字段。
- update response 字段。
- version 和 fallback/full rehighlight 规则。

## 测试结果

待填写命令和结果。

## 已知限制

第一版只增量维护 lexer/highlight 结果，不维护 parser、typechecker 或 compile artifacts。

## 下一任务入口

任务 02 应只依赖本交接文档列出的 Rust session API，不在 WASM 或 TS 层复刻增量 lexer 逻辑。

