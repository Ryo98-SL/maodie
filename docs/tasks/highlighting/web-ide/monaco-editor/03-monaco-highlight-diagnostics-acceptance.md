# 03 Monaco Highlight Diagnostics Acceptance

## Validation Commands

- `pnpm nx run ide:test`
- `pnpm nx run ide:typecheck`

## Manual Checks

- Confirm single edits send worker `update` requests and complex edits can reset.
- Confirm stale worker responses cannot overwrite newer editor state.
- Confirm illegal characters create Monaco markers and still appear in the Diagnostics panel.
- Confirm Chinese identifiers and emoji-adjacent edits keep valid token and marker ranges.

## Acceptance Result

Status: Not reviewed.

Record reviewer, date, command results, and manual review conclusion here.

