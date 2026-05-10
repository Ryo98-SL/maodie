import { existsSync } from "node:fs";
import { relative, resolve } from "node:path";

const root = process.cwd();
const requiredPaths = [
  "README.md",
  "README.deep.md",
  "index.md",
  "apps/index.md",
  "apps/ide/index.md",
  "apps/ide/src/index.md",
  "docs/index.md",
  "docs/tasks/index.md",
  "docs/tasks/README.md",
  "docs/tasks/01-rust-workspace-and-nx-bridge.md",
  "docs/tasks/02-diagnostics-and-source-model.md",
  "docs/tasks/03-lexer.md",
  "docs/tasks/04-parser-and-ast.md",
  "docs/tasks/05-name-resolution-and-hir.md",
  "docs/tasks/06-type-system-v1.md",
  "docs/tasks/07-pattern-match-and-errors.md",
  "docs/tasks/08-mir-lowering.md",
  "docs/tasks/09-core-stdlib.md",
  "docs/tasks/10-wasm-backend.md",
  "docs/tasks/11-wasm-api-and-ts-wrapper.md",
  "docs/tasks/12-cli-integration.md",
  "docs/tasks/13-ide-integration.md",
  "docs/tasks/14-v1-acceptance-suite.md",
  "packages/index.md",
  "packages/language-core/index.md",
  "packages/language-core/src/index.md",
  "packages/compiler/index.md",
  "packages/compiler/src/index.md",
  "packages/cli/index.md",
  "packages/cli/src/index.md",
  "packages/ide-protocol/index.md",
  "packages/ide-protocol/src/index.md",
  "tools/index.md"
];

const missing = requiredPaths.filter((filePath) => !existsSync(resolve(root, filePath)));

if (missing.length > 0) {
  console.error("style guard failed: missing required documentation files");
  for (const filePath of missing) {
    console.error(`- ${relative(root, resolve(root, filePath))}`);
  }
  process.exit(1);
}

console.log(`style guard passed (${requiredPaths.length} documentation checkpoints)`);
