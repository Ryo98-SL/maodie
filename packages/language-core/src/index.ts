export type MaodieSeverity = "info" | "warning" | "error";

export interface SourcePosition {
  readonly offset: number;
  readonly line: number;
  readonly column: number;
}

export interface SourceSpan {
  readonly start: SourcePosition;
  readonly end: SourcePosition;
}

export interface Diagnostic {
  readonly code: string;
  readonly severity: MaodieSeverity;
  readonly message: string;
  readonly span?: SourceSpan;
}

export interface SourceFile {
  readonly path: string;
  readonly text: string;
  readonly languageId: "maodie";
}

export type CompileArtifactKind = "ir" | "assembly" | "object" | "executable";

export interface CompileArtifact {
  readonly kind: CompileArtifactKind;
  readonly filename: string;
  readonly content: string | Uint8Array;
}

export function createSourceFile(text: string, path = "<memory>"): SourceFile {
  return {
    path,
    text,
    languageId: "maodie"
  };
}
