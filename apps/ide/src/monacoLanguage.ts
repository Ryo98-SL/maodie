import type { ByteRange, Diagnostic, HighlightKind, HighlightToken } from "@maodie/compiler-wasm";
import { byteRangeToUtf16Range } from "@maodie/compiler-wasm";
import type * as Monaco from "monaco-editor";

export interface HighlightWorkerVersion {
  readonly editorVersion: number;
  readonly sessionVersion: number;
}

export interface MaodieMonacoRange {
  readonly startLineNumber: number;
  readonly startColumn: number;
  readonly endLineNumber: number;
  readonly endColumn: number;
}

export interface MaodieMarkerSeverityMap {
  readonly error: number;
  readonly warning: number;
  readonly info: number;
}

export interface MaodieMarkerData extends MaodieMonacoRange {
  readonly severity: number;
  readonly message: string;
  readonly code: string;
}

export const maodieLanguageId = "maodie";
export const maodieThemeId = "maodie-dark";
export const maodieLiveLexerMarkerOwner = "maodie-live-lexer";

export const allHighlightKinds: readonly HighlightKind[] = [
  "keyword",
  "identifier",
  "comment",
  "string",
  "number",
  "boolean",
  "operator",
  "punctuation",
  "error"
];

export const maodieSemanticTokenLegend = {
  tokenTypes: [
    "keyword",
    "variable",
    "comment",
    "string",
    "number",
    "enumMember",
    "operator",
    "delimiter",
    "invalid"
  ],
  tokenModifiers: []
} satisfies Monaco.languages.SemanticTokensLegend;

const semanticTokenTypeByKind = {
  keyword: "keyword",
  identifier: "variable",
  comment: "comment",
  string: "string",
  number: "number",
  boolean: "enumMember",
  operator: "operator",
  punctuation: "delimiter",
  error: "invalid"
} satisfies Record<HighlightKind, string>;

const semanticTokenTypeIndexByType = new Map(
  maodieSemanticTokenLegend.tokenTypes.map((type, index) => [type, index])
);

let registeredMonacoApi: typeof Monaco | undefined;
let registeredSemanticTokenProvider: Monaco.IDisposable | undefined;
const semanticTokensByModel = new Map<string, Uint32Array>();
const semanticTokenListeners = new Set<() => void>();

export function semanticTokenTypeForKind(kind: string): string {
  return Object.prototype.hasOwnProperty.call(semanticTokenTypeByKind, kind)
    ? semanticTokenTypeByKind[kind as HighlightKind]
    : "variable";
}

export function byteRangeToMonacoRange(
  source: string,
  range: ByteRange
): MaodieMonacoRange | undefined {
  try {
    const utf16Range = byteRangeToUtf16Range(source, range);
    return utf16RangeToMonacoRange(source, utf16Range.start, utf16Range.end);
  } catch {
    return undefined;
  }
}

export function diagnosticToMonacoMarkerData(
  source: string,
  diagnostic: Diagnostic,
  severities: MaodieMarkerSeverityMap
): MaodieMarkerData | undefined {
  if (!diagnostic.span) {
    return undefined;
  }

  const range = byteRangeToMonacoRange(source, {
    start: diagnostic.span.start.offset,
    end: diagnostic.span.end.offset
  });
  if (!range || isEmptyMonacoRange(range)) {
    return undefined;
  }

  return {
    ...range,
    severity: severities[diagnostic.severity],
    code: diagnostic.code,
    message: diagnostic.message
  };
}

export function createMaodieSemanticTokenData(
  source: string,
  tokens: readonly HighlightToken[]
): Uint32Array {
  const segments = tokens
    .flatMap((token) => semanticTokenSegments(source, token))
    .sort(
      (left, right) =>
        left.line - right.line ||
        left.startCharacter - right.startCharacter ||
        left.length - right.length
    );

  const data: number[] = [];
  let previousLine = 0;
  let previousStartCharacter = 0;
  for (const segment of segments) {
    const deltaLine = segment.line - previousLine;
    const deltaStartCharacter =
      deltaLine === 0
        ? segment.startCharacter - previousStartCharacter
        : segment.startCharacter;

    data.push(deltaLine, deltaStartCharacter, segment.length, segment.tokenType, 0);
    previousLine = segment.line;
    previousStartCharacter = segment.startCharacter;
  }

  return new Uint32Array(data);
}

export function isStaleMaodieHighlightResponse(
  response: HighlightWorkerVersion,
  currentEditorVersion: number,
  currentSessionVersion?: number
): boolean {
  return (
    response.editorVersion < currentEditorVersion ||
    (currentSessionVersion !== undefined && response.sessionVersion < currentSessionVersion)
  );
}

export function registerMaodieMonacoLanguage(monaco: typeof Monaco): void {
  registeredMonacoApi = monaco;

  if (!monaco.languages.getLanguages().some((language) => language.id === maodieLanguageId)) {
    monaco.languages.register({
      id: maodieLanguageId,
      extensions: [".mao"],
      aliases: ["Maodie", "maodie"]
    });
  }

  monaco.languages.setLanguageConfiguration(maodieLanguageId, {
    comments: {
      lineComment: "//",
      blockComment: ["/*", "*/"]
    },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"]
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' }
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' }
    ]
  });

  monaco.editor.defineTheme(maodieThemeId, {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "67e8f9", fontStyle: "bold" },
      { token: "variable", foreground: "f5f5f5" },
      { token: "comment", foreground: "86efac", fontStyle: "italic" },
      { token: "string", foreground: "fbbf24" },
      { token: "number", foreground: "c4b5fd" },
      { token: "enumMember", foreground: "fdba74" },
      { token: "operator", foreground: "f0abfc" },
      { token: "delimiter", foreground: "a3a3a3" },
      { token: "invalid", foreground: "fda4af", fontStyle: "underline" }
    ],
    colors: {
      "editor.background": "#0a0a0a",
      "editor.foreground": "#f5f5f5",
      "editorLineNumber.foreground": "#737373",
      "editorLineNumber.activeForeground": "#d4d4d4",
      "editorCursor.foreground": "#67e8f9",
      "editor.selectionBackground": "#0891b24d",
      "editor.lineHighlightBackground": "#171717",
      "editorGutter.background": "#0a0a0a"
    }
  });

  registeredSemanticTokenProvider?.dispose();
  registeredSemanticTokenProvider =
    monaco.languages.registerDocumentSemanticTokensProvider(maodieLanguageId, {
      getLegend() {
        return maodieSemanticTokenLegend;
      },
      provideDocumentSemanticTokens(model) {
        return {
          data: semanticTokensByModel.get(model.uri.toString()) ?? new Uint32Array()
        };
      },
      releaseDocumentSemanticTokens() {},
      onDidChange(listener) {
        semanticTokenListeners.add(listener);
        return {
          dispose() {
            semanticTokenListeners.delete(listener);
          }
        };
      }
    });
}

export function setMaodieSemanticTokens(
  model: Monaco.editor.ITextModel,
  source: string,
  tokens: readonly HighlightToken[]
): void {
  semanticTokensByModel.set(model.uri.toString(), createMaodieSemanticTokenData(source, tokens));
  for (const listener of semanticTokenListeners) {
    listener();
  }
}

export function clearMaodieSemanticTokens(model: Monaco.editor.ITextModel): void {
  semanticTokensByModel.delete(model.uri.toString());
  registeredMonacoApi?.editor.setModelMarkers(model, maodieLiveLexerMarkerOwner, []);
  for (const listener of semanticTokenListeners) {
    listener();
  }
}

export function maodieSemanticTokenCount(model: Monaco.editor.ITextModel): number {
  return (semanticTokensByModel.get(model.uri.toString())?.length ?? 0) / 5;
}

function semanticTokenSegments(
  source: string,
  token: HighlightToken
): Array<{ line: number; startCharacter: number; length: number; tokenType: number }> {
  const range = byteRangeToMonacoRange(source, token.range);
  if (!range || isEmptyMonacoRange(range)) {
    return [];
  }

  const tokenType = semanticTokenTypeIndexByType.get(semanticTokenTypeForKind(token.kind));
  if (tokenType === undefined) {
    return [];
  }

  const lines = source.split("\n");
  const segments: Array<{
    line: number;
    startCharacter: number;
    length: number;
    tokenType: number;
  }> = [];
  for (let lineNumber = range.startLineNumber; lineNumber <= range.endLineNumber; lineNumber += 1) {
    const lineText = lines[lineNumber - 1] ?? "";
    const startColumn = lineNumber === range.startLineNumber ? range.startColumn : 1;
    const endColumn =
      lineNumber === range.endLineNumber ? range.endColumn : lineText.length + 1;
    const length = endColumn - startColumn;
    if (length <= 0) {
      continue;
    }

    segments.push({
      line: lineNumber - 1,
      startCharacter: startColumn - 1,
      length,
      tokenType
    });
  }

  return segments;
}

function utf16RangeToMonacoRange(
  source: string,
  start: number,
  end: number
): MaodieMonacoRange {
  const startPosition = utf16OffsetToMonacoPosition(source, start);
  const endPosition = utf16OffsetToMonacoPosition(source, end);

  return {
    startLineNumber: startPosition.lineNumber,
    startColumn: startPosition.column,
    endLineNumber: endPosition.lineNumber,
    endColumn: endPosition.column
  };
}

function utf16OffsetToMonacoPosition(
  source: string,
  offset: number
): { lineNumber: number; column: number } {
  if (!Number.isSafeInteger(offset) || offset < 0 || offset > source.length) {
    throw new RangeError(`UTF-16 offset ${offset} is outside the source length.`);
  }

  let lineNumber = 1;
  let lineStartOffset = 0;
  for (let index = 0; index < offset; index += 1) {
    if (source.charCodeAt(index) === 10) {
      lineNumber += 1;
      lineStartOffset = index + 1;
    }
  }

  return {
    lineNumber,
    column: offset - lineStartOffset + 1
  };
}

function isEmptyMonacoRange(range: MaodieMonacoRange): boolean {
  return range.startLineNumber === range.endLineNumber && range.startColumn >= range.endColumn;
}
