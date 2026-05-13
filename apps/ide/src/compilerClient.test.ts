import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import { compileMaodieWasm, type Diagnostic, type HighlightToken } from "@maodie/compiler-wasm";
import { describe, expect, it } from "vitest";

import { defaultSource, evaluateMain, sourcePath } from "./compilerClient";
import { defaultExampleId, workbenchExamples } from "./examples";
import {
  isStaleMaodieHighlightResponse,
  singleEditorEditFromMonacoChange
} from "./highlightAdapter";
import {
  allHighlightKinds,
  byteRangeToMonacoRange,
  createMaodieSemanticTokenData,
  diagnosticToMonacoMarkerData,
  semanticTokenTypeForKind
} from "./monacoLanguage";
import { createInitialSourceState } from "./initialSource";
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
    expect(html).toContain("Live Lexer");
    expect(html).toContain("Last Compile");
    expect(html).toContain("正在加载实时词法检查");
    expect(html).toContain("flex h-screen flex-col overflow-hidden");
    expect(html).toContain("id=\"source-editor\"");
    expect(html).toContain("data-editor-mount=\"monaco\"");
    expect(html).toContain("role=\"textbox\"");
    expect(html).not.toContain("<textarea");
    expect(html).toContain("min-[600px]:grid-cols-[minmax(0,1fr)_minmax(320px,440px)]");
    expect(html).toContain("min-[600px]:grid-rows-1");
    expect(html).toContain("min-h-0 flex-1 overflow-hidden bg-neutral-950");
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

  it("captures formatted core.log output as one evaluation log", async () => {
    const response = await compileMaodieWasm(
      `module demo
import core.Result
import core.log

fn label() -> String { return "ok" }

fn main(value: i32) -> Result<i32, String> {
  let enabled: bool = true
  let message: String = label()
  log("value is {} {} {}", value, enabled, message)
  return Result.Ok(value)
}
`,
      {
        sourcePath,
        target: "wasm"
      }
    );
    const evaluation = await evaluateMain(response, 5);

    expect(response.ok).toBe(true);
    expect(response.dumps.wat).toContain("call $__maodie_debug_i32");
    expect(response.dumps.wat).toContain("call $__maodie_debug_bool");
    expect(evaluation.logs).toEqual(["value is 5 true ok"]);
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
    expect(html).toContain("Last Compile");
  });

  it("keeps smoke-test query source as a custom editor document", () => {
    const customSource = "module smoke\nfn main(value: i32) -> i32 { value }\n";

    expect(createInitialSourceState(`?source=${encodeURIComponent(customSource)}`)).toEqual({
      source: customSource,
      activeExampleId: undefined
    });
  });

  it("maps every highlight kind to a stable Maodie Monaco semantic token type", () => {
    expect(allHighlightKinds.map((kind) => [kind, semanticTokenTypeForKind(kind)])).toEqual([
      ["keyword", "keyword"],
      ["identifier", "variable"],
      ["comment", "comment"],
      ["string", "string"],
      ["number", "number"],
      ["boolean", "enumMember"],
      ["operator", "operator"],
      ["punctuation", "delimiter"],
      ["error", "invalid"]
    ]);
    expect(semanticTokenTypeForKind("future-kind")).toBe("variable");
  });

  it("creates Monaco semantic tokens and markers for Chinese ranges, error tokens, diagnostics, and unknown fallback", () => {
    const source = "let 名字 = \"ok\" @\n// 尾巴\n";
    const tokens: HighlightToken[] = [
      { kind: "keyword", range: byteRangeFor(source, "let") },
      { kind: "identifier", range: byteRangeFor(source, "名字") },
      { kind: "string", range: byteRangeFor(source, "\"ok\"") },
      { kind: "error", range: byteRangeFor(source, "@") },
      { kind: "comment", range: byteRangeFor(source, "// 尾巴") },
      { kind: "identifier", range: byteRangeFor(source, "不存在") }
    ];
    const unknownToken = {
      kind: "unknown-kind",
      range: byteRangeFor(source, "=")
    } as unknown as HighlightToken;
    const diagnostic: Diagnostic = {
      code: "MD0101",
      severity: "error",
      message: "无法识别的字符。",
      span: {
        sourceId: 0,
        fileName: sourcePath,
        start: { offset: byteRangeFor(source, "@").start, line: 1, column: 17 },
        end: { offset: byteRangeFor(source, "@").end, line: 1, column: 18 }
      },
      notes: []
    };

    expect(byteRangeToMonacoRange(source, byteRangeFor(source, "名字"))).toEqual({
      startLineNumber: 1,
      startColumn: 5,
      endLineNumber: 1,
      endColumn: 7
    });
    expect(Array.from(createMaodieSemanticTokenData(source, [...tokens, unknownToken]))).toEqual(
      [
        0, 0, 3, 0, 0,
        0, 4, 2, 1, 0,
        0, 3, 1, 1, 0,
        0, 2, 4, 3, 0,
        0, 5, 1, 8, 0,
        1, 0, 5, 2, 0
      ]
    );
    expect(
      diagnosticToMonacoMarkerData(source, diagnostic, {
        error: 8,
        warning: 4,
        info: 2
      })
    ).toEqual({
      startLineNumber: 1,
      startColumn: 15,
      endLineNumber: 1,
      endColumn: 16,
      severity: 8,
      code: "MD0101",
      message: "无法识别的字符。"
    });
  });

  it("converts emoji-safe byte offsets into Monaco ranges and diagnostic marker spans", () => {
    const source = "a🧪b\n名字🙂 = 1\n";
    const warningDiagnostic: Diagnostic = {
      code: "MD0202",
      severity: "warning",
      message: "测试诊断。",
      span: {
        sourceId: 0,
        fileName: sourcePath,
        start: { offset: byteRangeFor(source, "🙂").start, line: 99, column: 99 },
        end: { offset: byteRangeFor(source, "🙂").end, line: 99, column: 99 }
      },
      notes: []
    };

    expect(byteRangeToMonacoRange(source, byteRangeFor(source, "🧪"))).toEqual({
      startLineNumber: 1,
      startColumn: 2,
      endLineNumber: 1,
      endColumn: 4
    });
    expect(byteRangeToMonacoRange(source, byteRangeFor(source, "b"))).toEqual({
      startLineNumber: 1,
      startColumn: 4,
      endLineNumber: 1,
      endColumn: 5
    });
    expect(
      diagnosticToMonacoMarkerData(source, warningDiagnostic, {
        error: 8,
        warning: 4,
        info: 2
      })
    ).toEqual({
      startLineNumber: 2,
      startColumn: 3,
      endLineNumber: 2,
      endColumn: 5,
      severity: 4,
      code: "MD0202",
      message: "测试诊断。"
    });
  });

  it("detects stale highlight worker responses by editor and session version", () => {
    expect(
      isStaleMaodieHighlightResponse({ editorVersion: 1, sessionVersion: 4 }, 2, 4)
    ).toBe(true);
    expect(
      isStaleMaodieHighlightResponse({ editorVersion: 2, sessionVersion: 3 }, 2, 4)
    ).toBe(true);
    expect(
      isStaleMaodieHighlightResponse({ editorVersion: 2, sessionVersion: 4 }, 2, 4)
    ).toBe(false);
  });

  it("converts Monaco single changes into UTF-8 byte edits and rejects complex edits", () => {
    const source = "let 名字 = 1\nlet emoji = 🧪\n";
    const chineseRange = byteRangeFor(source, "名字");
    const emojiRange = byteRangeFor(source, "🧪");

    expect(
      singleEditorEditFromMonacoChange(
        source,
        monacoChangeEvent([
          {
            rangeOffset: source.indexOf("名字"),
            rangeLength: "名字".length,
            text: "名称"
          }
        ])
      )
    ).toEqual({
      range: chineseRange,
      replacement: "名称"
    });
    expect(
      singleEditorEditFromMonacoChange(
        source,
        monacoChangeEvent([
          {
            rangeOffset: source.indexOf("🧪") + "🧪".length,
            rangeLength: 0,
            text: "值"
          }
        ])
      )
    ).toEqual({
      range: {
        start: emojiRange.end,
        end: emojiRange.end
      },
      replacement: "值"
    });
    expect(
      singleEditorEditFromMonacoChange(
        source,
        monacoChangeEvent([
          {
            rangeOffset: 0,
            rangeLength: 0,
            text: "a"
          },
          {
            rangeOffset: 1,
            rangeLength: 0,
            text: "b"
          }
        ])
      )
    ).toBeUndefined();
    expect(
      singleEditorEditFromMonacoChange(
        source,
        monacoChangeEvent([
          {
            rangeOffset: source.indexOf("🧪") + 1,
            rangeLength: 0,
            text: "x"
          }
        ])
      )
    ).toBeUndefined();
  });
});

const baseState: IdeState = {
  source: "",
  status: "ready",
  activeDump: "ast",
  activeExampleId: undefined,
  result: undefined,
  liveLexer: { status: "loading", diagnostics: [], errorMessage: undefined },
  evalInput: "2",
  evaluation: { status: "idle", result: undefined, errorMessage: undefined },
  errorMessage: undefined,
  requestId: 1
};

async function readExample(filename: string): Promise<string> {
  return await readFile(join(examplesRoot, filename), "utf8");
}

function byteRangeFor(source: string, needle: string): { start: number; end: number } {
  const start = source.indexOf(needle);
  if (start < 0) {
    return { start: 0, end: 0 };
  }

  return {
    start: new TextEncoder().encode(source.slice(0, start)).byteLength,
    end: new TextEncoder().encode(source.slice(0, start + needle.length)).byteLength
  };
}

function monacoChangeEvent(
  changes: Array<{ rangeOffset: number; rangeLength: number; text: string }>
): Parameters<typeof singleEditorEditFromMonacoChange>[1] {
  return { changes } as Parameters<typeof singleEditorEditFromMonacoChange>[1];
}
