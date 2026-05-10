import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import { compileMaodieWasm } from "@maodie/compiler-wasm";
import { describe, expect, it } from "vitest";

import { defaultSource, evaluateMain, sourcePath } from "./compilerClient";
import { defaultExampleId, workbenchExamples } from "./examples";
import { renderWorkbench, type IdeState } from "./view";

const examplesRoot = resolve(process.cwd(), "examples");

describe("IDE compiler smoke", () => {
  it("keeps the browser default source aligned with the v1 acceptance example", async () => {
    expect(defaultSource.trim()).toBe((await readExample("v1_acceptance.mao")).trim());
  });

  it("renders an idle workbench that waits for a manual Run", () => {
    const html = renderWorkbench({
      ...baseState,
      status: "idle",
      source: defaultSource,
      activeExampleId: defaultExampleId
    });

    expect(html).toContain("Run");
    expect(html).toContain("Ready to run");
    expect(html).toContain("点击 Run 编译并执行当前源码。");
    expect(html).toContain("flex h-screen flex-col overflow-hidden");
    expect(html).toContain("id=\"source-editor\"");
    expect(html).toContain("min-[600px]:grid-cols-[minmax(0,1fr)_minmax(320px,440px)]");
    expect(html).toContain("min-[600px]:grid-rows-1");
    expect(html).toContain("min-h-0 flex-1 resize-none overflow-auto");
    expect(html).toContain("grid min-h-0 grid-rows");
    expect(html).toContain("min-h-0 flex-1 overflow-auto p-4 pb-8 text-sm");
    expect(html).toContain("min-h-0 flex-1 overflow-auto p-4 pb-8 text-xs");
    expect(html).not.toContain("Compile\n");
    expect(html).not.toContain("max-h-[28vh]");
    expect(html).not.toContain("max-h-[52vh]");
  });

  it("renders the default example as a successful compile with dumps", async () => {
    const response = await compileMaodieWasm(defaultSource, {
      sourcePath,
      target: "wasm"
    });
    const evaluation = await evaluateMain(response, 2);

    const html = renderWorkbench({
      ...baseState,
      source: defaultSource,
      activeExampleId: defaultExampleId,
      status: response.ok ? "ready" : "failed",
      result: response,
      evaluation: { status: "ready", result: evaluation, errorMessage: undefined }
    });

    expect(response.ok).toBe(true);
    expect(response.dumps.ast).toContain("File");
    expect(response.dumps.hir).toContain("Package");
    expect(response.dumps.mir).toContain("MIR");
    expect(response.dumps.wat).toContain("(module");
    expect(html).toContain("Compiled");
    expect(html).toContain("Evaluation");
    expect(html).toContain("main(2)");
    expect(html).toContain("raw i32");
    expect(evaluation.raw).toBe(1024);
    expect(evaluation.decoded).toEqual({
      tag: 0,
      payload: 4,
      variant: "Ok",
      display: "Result.Ok(4)",
      serialized: JSON.stringify(
        {
          type: "Result",
          variant: "Ok",
          value: 4,
          raw: 1024
        },
        null,
        2
      )
    });
    expect(html.indexOf("Evaluation")).toBeLessThan(html.indexOf("Diagnostics"));
    expect(html.indexOf("logs")).toBeLessThan(html.indexOf("call"));
    expect(html).toContain("Result.Ok(4)");
    expect(html).toContain("&quot;type&quot;: &quot;Result&quot;");
    expect(html).toContain("&quot;value&quot;: 4");
    expect(html).toContain("module.wat");
    expect(html).toContain("AST");
    expect(html).toContain("Hello World");
    expect(html).toContain("函数调用");
    expect(html).toContain("斐波那契");
  });

  it.each(workbenchExamples)("compiles the $label workbench example", async (example) => {
    const response = await compileMaodieWasm(example.source, {
      sourcePath,
      target: "wasm"
    });

    expect(response.ok).toBe(true);
    expect(response.artifacts.map((artifact) => artifact.filename)).toEqual([
      "module.wat",
      "module.wasm"
    ]);
  });

  it("captures core.log output from the Hello World example", async () => {
    const hello = workbenchExamples.find((example) => example.id === "hello");
    expect(hello).toBeDefined();

    const response = await compileMaodieWasm(hello?.source ?? "", {
      sourcePath,
      target: "wasm"
    });
    const evaluation = await evaluateMain(response, 2);

    expect(response.ok).toBe(true);
    expect(response.dumps.wat).toContain("call $__maodie_debug_string");
    expect(evaluation.logs).toEqual(["Hello world"]);
  });

  it("renders stable Chinese diagnostics for the error example", async () => {
    const source = await readExample("v1_error.mao");
    const response = await compileMaodieWasm(source, {
      sourcePath,
      target: "wasm"
    });

    const html = renderWorkbench({
      ...baseState,
      source,
      status: "failed",
      result: response
    });

    expect(response.ok).toBe(false);
    expect(response.diagnostics.map((diagnostic) => diagnostic.code)).toContain("MD0201");
    expect(html).toContain("MD0201");
    expect(html).toContain("workspace/main.mao:4:10");
    expect(html).toContain("Diagnostics available");
  });
});

const baseState: IdeState = {
  source: "",
  status: "ready",
  activeDump: "ast",
  activeExampleId: undefined,
  result: undefined,
  evalInput: "2",
  evaluation: { status: "idle", result: undefined, errorMessage: undefined },
  errorMessage: undefined,
  requestId: 1
};

async function readExample(filename: string): Promise<string> {
  return await readFile(join(examplesRoot, filename), "utf8");
}
