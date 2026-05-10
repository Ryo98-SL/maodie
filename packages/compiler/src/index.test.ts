import { describe, expect, it } from "vitest";

import { compileMaodie } from "./index.js";

describe("compileMaodie", () => {
  it("emits placeholder IR for non-empty input", () => {
    const result = compileMaodie("fn main() -> i32 { 0 }", {
      sourcePath: "examples/main.md"
    });

    expect(result.ok).toBe(true);
    expect(result.diagnostics).toHaveLength(0);
    expect(result.artifacts[0]?.filename).toBe("main.mdir");
  });

  it("returns a diagnostic for empty source", () => {
    const result = compileMaodie("");

    expect(result.ok).toBe(false);
    expect(result.diagnostics[0]?.code).toBe("MD0001");
    expect(result.artifacts).toHaveLength(0);
  });
});
