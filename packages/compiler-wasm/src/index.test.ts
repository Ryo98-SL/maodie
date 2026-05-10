import { describe, expect, it } from "vitest";

import { compileMaodieWasm, createCompilerWasm } from "./index.js";

const source = `module demo
import core.Result

fn parse(value: i32) -> Result<i32, String> {
  if value > 0 { Result.Ok(value) } else { Result.Ok(1) }
}

fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = parse(value)?
  return Result.Ok(parsed + 1)
}
`;

describe("compiler wasm wrapper", () => {
  it("loads the generated wasm module and compiles a source string", async () => {
    const compiler = await createCompilerWasm();
    const response = compiler.compile(source, {
      sourcePath: "smoke.mao",
      target: "wasm"
    });

    expect(response.ok).toBe(true);
    expect(response.diagnostics.length).toBeGreaterThan(0);
    expect(response.dumps.ast).toContain("File");
    expect(response.dumps.hir).toContain("Package");
    expect(response.dumps.mir).toContain("MIR");
    expect(response.dumps.wat).toContain("(module");
    expect(response.artifacts.map((artifact) => artifact.filename)).toEqual([
      "module.wat",
      "module.wasm"
    ]);

    const wasmArtifact = response.artifacts.find((artifact) => artifact.kind === "wasm");
    expect(wasmArtifact?.content).toBeInstanceOf(Uint8Array);
    expect(Array.from((wasmArtifact?.content as Uint8Array).slice(0, 4))).toEqual([0, 97, 115, 109]);
  });

  it("returns structured diagnostics instead of raw wasm pointers", async () => {
    const response = await compileMaodieWasm(`module demo

fn main() -> i32 {
  return @
}
`, {
      sourcePath: "empty.mao",
      target: "wasm"
    });

    expect(response.ok).toBe(false);
    expect(response.artifacts).toEqual([]);
    expect(response.diagnostics[0]).toMatchObject({
      severity: "error"
    });
    expect(response).toHaveProperty("dumps");
  });
});
