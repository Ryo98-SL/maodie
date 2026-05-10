import { describe, expect, it } from "vitest";
import { readFile } from "node:fs/promises";

import {
  byteOffsetToUtf16Position,
  byteRangeToUtf16LineColumnRange,
  byteRangeToUtf16Range,
  compileMaodieWasm,
  createCompilerWasm,
  highlightMaodieSource,
  type HighlightKind,
  type HighlightToken
} from "./index.js";

interface HighlightGolden {
  readonly sourcePath: string;
  readonly tokens: readonly HighlightToken[];
  readonly diagnostics: readonly {
    readonly code: string;
    readonly range: {
      readonly start: number;
      readonly end: number;
    };
  }[];
}

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

  it("highlights source through the generated wasm module without compile artifacts", async () => {
    const compiler = await createCompilerWasm();
    const response = compiler.highlight(`module demo

fn main() -> bool {
  let flag = true
  return flag
}
`, {
      sourcePath: "highlight.mao"
    });

    expect(response.ok).toBe(true);
    expect(response.diagnostics).toEqual([]);
    expect(response.tokens.map((token) => token.kind)).toContain("keyword");
    expect(response.tokens).toContainEqual({
      kind: "boolean",
      range: {
        start: 46,
        end: 50
      }
    });
    expect(response).not.toHaveProperty("artifacts");
    expect(response).not.toHaveProperty("dumps");
  });

  it("exposes a singleton highlight helper that returns lexer diagnostics", async () => {
    const response = await highlightMaodieSource("let x = @", {
      sourcePath: "bad-highlight.mao"
    });

    expect(response.ok).toBe(false);
    expect(response.tokens).toContainEqual({
      kind: "error",
      range: {
        start: 8,
        end: 9
      }
    });
    expect(response.diagnostics[0]).toMatchObject({
      code: "MD0101",
      severity: "error"
    });
  });

  it("keeps the shared highlight fixture aligned with the generated wasm module", async () => {
    const fixture = await readHighlightFixture();
    const compiler = await createCompilerWasm();
    const response = compiler.highlight(fixture.source, {
      sourcePath: fixture.golden.sourcePath
    });

    expect(response.ok).toBe(false);
    expect(response.tokens).toEqual(fixture.golden.tokens);
    expect(response.tokens.map((token) => token.kind)).toEqual(
      expect.arrayContaining<HighlightKind>([
        "keyword",
        "identifier",
        "number",
        "boolean",
        "string",
        "comment",
        "operator",
        "punctuation",
        "error"
      ])
    );
    expect(response.diagnostics.map(toDiagnosticGolden)).toEqual(fixture.golden.diagnostics);
  });

  it("converts Rust byte ranges to UTF-16 editor ranges", async () => {
    const fixture = await readHighlightFixture();
    const chineseName = fixture.golden.tokens.find(
      (token) => token.kind === "identifier" && token.range.start === 66
    );
    const stringLiteral = fixture.golden.tokens.find((token) => token.kind === "string");
    const errorToken = fixture.golden.tokens.find((token) => token.kind === "error");

    expect(chineseName).toBeDefined();
    expect(stringLiteral).toBeDefined();
    expect(errorToken).toBeDefined();

    expect(byteRangeToUtf16Range(fixture.source, chineseName!.range)).toEqual({
      start: 58,
      end: 60
    });
    expect(byteRangeToUtf16LineColumnRange(fixture.source, chineseName!.range)).toEqual({
      start: {
        line: 4,
        character: 6,
        offset: 58
      },
      end: {
        line: 4,
        character: 8,
        offset: 60
      }
    });
    expect(byteRangeToUtf16Range(fixture.source, stringLiteral!.range)).toEqual({
      start: 79,
      end: 83
    });
    expect(byteOffsetToUtf16Position(fixture.source, errorToken!.range.start)).toEqual({
      line: 8,
      character: 14,
      offset: 120
    });
    expect(() => byteOffsetToUtf16Position(fixture.source, chineseName!.range.start + 1)).toThrow(
      RangeError
    );
  });

  it("runs the final highlighting acceptance smoke through the TS wrapper", async () => {
    const fixture = await readHighlightFixture();
    const response = await highlightMaodieSource(fixture.source, {
      sourcePath: fixture.golden.sourcePath
    });
    const chineseName = response.tokens.find(
      (token) => token.kind === "identifier" && token.range.start === 66
    );
    const errorToken = response.tokens.find((token) => token.kind === "error");

    expect(response.ok).toBe(false);
    expect(response.tokens).toEqual(fixture.golden.tokens);
    expect(response.diagnostics.map(toDiagnosticGolden)).toEqual(fixture.golden.diagnostics);
    expect(response.diagnostics.length).toBeGreaterThan(0);
    expect(chineseName).toBeDefined();
    expect(errorToken).toBeDefined();
    expect(byteRangeToUtf16Range(fixture.source, chineseName!.range)).toEqual({
      start: 58,
      end: 60
    });
  });
});

async function readHighlightFixture(): Promise<{ source: string; golden: HighlightGolden }> {
  const sourceUrl = new URL(
    "../../../docs/tasks/highlighting/fixtures/syntax-highlight.mao",
    import.meta.url
  );
  const goldenUrl = new URL(
    "../../../docs/tasks/highlighting/fixtures/syntax-highlight.tokens.json",
    import.meta.url
  );

  return {
    source: await readFile(sourceUrl, "utf8"),
    golden: JSON.parse(await readFile(goldenUrl, "utf8")) as HighlightGolden
  };
}

function toDiagnosticGolden(diagnostic: {
  readonly code: string;
  readonly span?: {
    readonly start: {
      readonly offset: number;
    };
    readonly end: {
      readonly offset: number;
    };
  };
}): HighlightGolden["diagnostics"][number] {
  expect(diagnostic.span).toBeDefined();

  return {
    code: diagnostic.code,
    range: {
      start: diagnostic.span!.start.offset,
      end: diagnostic.span!.end.offset
    }
  };
}
