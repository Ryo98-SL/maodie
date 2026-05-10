# 03 CodeMirror Editor Shell Acceptance

## 验收命令

- `pnpm nx run ide:typecheck`
- `pnpm nx run ide:test`
- `pnpm ide:build`
- `pnpm test`

## 人工检查

- Web IDE 不再渲染 `textarea#source-editor`，而是渲染 CodeMirror editor mount。
- 手动输入只更新 source state，不触发 compile。
- Run 按钮仍编译当前编辑器文本。
- 示例切换替换编辑器全文并清空旧 compile/evaluation 状态。
- `?source=` 注入仍可用于 smoke tests。
- 布局、滚动和右侧 panels 没有明显回退。

## 验收结论

状态：未验收。

记录复验者、日期、命令结果和人工检查结论。

