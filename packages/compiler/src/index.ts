import {
  type CompileArtifact,
  type Diagnostic,
  type SourceFile,
  createSourceFile
} from "@maodie/language-core";

export type CompileTarget = "maodie-ir" | "native" | "wasm";

export interface CompileOptions {
  readonly sourcePath?: string;
  readonly target?: CompileTarget;
  readonly moduleName?: string;
}

export interface CompileResult {
  readonly source: SourceFile;
  readonly diagnostics: readonly Diagnostic[];
  readonly artifacts: readonly CompileArtifact[];
  readonly ok: boolean;
}

export function compileMaodie(text: string, options: CompileOptions = {}): CompileResult {
  const source = createSourceFile(text, options.sourcePath);
  const diagnostics = validateInitialSource(source);

  if (diagnostics.some((diagnostic) => diagnostic.severity === "error")) {
    return {
      source,
      diagnostics,
      artifacts: [],
      ok: false
    };
  }

  const moduleName = options.moduleName ?? inferModuleName(source.path);
  const artifact: CompileArtifact = {
    kind: "ir",
    filename: `${moduleName}.mdir`,
    content: [
      `module ${moduleName}`,
      `target ${options.target ?? "maodie-ir"}`,
      "entry main",
      "  ; compiler pipeline placeholder"
    ].join("\n")
  };

  return {
    source,
    diagnostics,
    artifacts: [artifact],
    ok: true
  };
}

function validateInitialSource(source: SourceFile): Diagnostic[] {
  if (source.text.trim().length > 0) {
    return [];
  }

  return [
    {
      code: "MD0001",
      severity: "error",
      message: "Maodie source is empty."
    }
  ];
}

function inferModuleName(sourcePath: string): string {
  const filename = sourcePath.split(/[\\/]/).pop() ?? "main.md";
  return filename.replace(/\.[^.]+$/, "") || "main";
}
