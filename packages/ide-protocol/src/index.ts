import type { CompileArtifact, Diagnostic } from "@maodie/language-core";

export interface IdeDocument {
  readonly uri: string;
  readonly languageId: "maodie";
  readonly version: number;
  readonly text: string;
}

export interface CompileRequest {
  readonly document: IdeDocument;
  readonly target?: "maodie-ir" | "native" | "wasm";
}

export interface CompileResponse {
  readonly uri: string;
  readonly diagnostics: readonly Diagnostic[];
  readonly artifacts: readonly CompileArtifact[];
  readonly ok: boolean;
}
