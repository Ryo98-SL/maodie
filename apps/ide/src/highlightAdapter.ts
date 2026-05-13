import type { ByteRange, Diagnostic, HighlightToken } from "@maodie/compiler-wasm";
import type * as Monaco from "monaco-editor";

import {
  allHighlightKinds,
  clearMaodieSemanticTokens,
  diagnosticToMonacoMarkerData,
  isStaleMaodieHighlightResponse,
  maodieLiveLexerMarkerOwner,
  semanticTokenTypeForKind,
  setMaodieSemanticTokens
} from "./monacoLanguage";

export {
  allHighlightKinds,
  diagnosticToMonacoMarkerData,
  isStaleMaodieHighlightResponse,
  semanticTokenTypeForKind
} from "./monacoLanguage";

export interface LiveLexerUpdate {
  readonly status: "loading" | "ready" | "failed";
  readonly diagnostics: readonly Diagnostic[];
  readonly errorMessage: string | undefined;
}

export interface MaodieHighlightAdapterOptions {
  readonly monaco: typeof Monaco;
  readonly model: Monaco.editor.ITextModel;
  readonly sourcePath: string;
  readonly wasmUrl: string;
  readonly onLiveLexerUpdate?: (update: LiveLexerUpdate) => void;
}

interface HighlightWorkerSessionResponse {
  readonly type: "init" | "update" | "reset";
  readonly requestId: string;
  readonly ok: boolean;
  readonly editorVersion: number;
  readonly sessionVersion: number;
  readonly changedRange: ByteRange;
  readonly fullRehighlight: boolean;
  readonly tokens: readonly HighlightToken[];
  readonly diagnostics: readonly Diagnostic[];
}

interface HighlightWorkerDisposeResponse {
  readonly type: "dispose";
  readonly requestId: string;
  readonly ok: true;
  readonly editorVersion: number;
  readonly sessionVersion: number;
}

interface HighlightWorkerErrorResponse {
  readonly type: "init" | "update" | "reset" | "dispose";
  readonly requestId: string;
  readonly ok: false;
  readonly editorVersion: number;
  readonly sessionVersion: number;
  readonly diagnostics: readonly Diagnostic[];
}

type HighlightWorkerResponse =
  | HighlightWorkerSessionResponse
  | HighlightWorkerDisposeResponse
  | HighlightWorkerErrorResponse;

interface SingleEditorEdit {
  readonly range: ByteRange;
  readonly replacement: string;
}

export class MaodieHighlightAdapter {
  readonly #monaco: typeof Monaco;
  readonly #model: Monaco.editor.ITextModel;
  readonly #options: MaodieHighlightAdapterOptions;
  readonly #worker: Worker;
  #source: string;
  #editorVersion = 0;
  #sessionVersion: number | undefined;
  #requestId = 0;
  #pendingReset = false;
  #disposed = false;

  constructor(options: MaodieHighlightAdapterOptions) {
    this.#monaco = options.monaco;
    this.#model = options.model;
    this.#options = options;
    this.#source = options.model.getValue();
    this.#worker = new Worker(
      new URL("../../../packages/compiler-wasm/src/highlight.worker.ts", import.meta.url),
      { type: "module" }
    );
    this.#worker.addEventListener("message", this.#handleWorkerMessage);
    this.#worker.addEventListener("error", this.#handleWorkerError);
    this.#notify({ status: "loading", diagnostics: [], errorMessage: undefined });
    this.#postInit(this.#source);
  }

  handleModelChange(event: Monaco.editor.IModelContentChangedEvent): void {
    if (this.#disposed) {
      return;
    }

    const sourceBefore = this.#source;
    const sourceAfter = this.#model.getValue();
    this.#source = sourceAfter;
    this.#editorVersion += 1;

    const edit = singleEditorEditFromMonacoChange(sourceBefore, event);
    if (this.#sessionVersion === undefined) {
      this.#pendingReset = true;
      return;
    }

    if (edit) {
      this.#postUpdate(edit);
      return;
    }

    this.#postReset(sourceAfter);
  }

  destroy(): void {
    this.#disposed = true;
    clearMaodieSemanticTokens(this.#model);
    this.#monaco.editor.setModelMarkers(this.#model, maodieLiveLexerMarkerOwner, []);
    this.#worker.removeEventListener("message", this.#handleWorkerMessage);
    this.#worker.removeEventListener("error", this.#handleWorkerError);
    this.#worker.postMessage({
      type: "dispose",
      requestId: this.#nextRequestId(),
      editorVersion: this.#editorVersion
    });
    this.#worker.terminate();
  }

  readonly #handleWorkerMessage = (event: MessageEvent<HighlightWorkerResponse>): void => {
    if (this.#disposed) {
      return;
    }

    const response = event.data;
    if (response.type === "dispose") {
      return;
    }

    if (response.type === "init" && this.#sessionVersion === undefined) {
      this.#sessionVersion = response.sessionVersion;
    }

    if (
      isStaleMaodieHighlightResponse(response, this.#editorVersion, this.#sessionVersion) ||
      (response.type === "init" && response.editorVersion < this.#editorVersion)
    ) {
      this.#flushPendingReset();
      return;
    }

    this.#sessionVersion = response.sessionVersion;
    if (!response.ok && response.type === "update") {
      this.#notify({ status: "loading", diagnostics: [], errorMessage: undefined });
      this.#postReset(this.#source);
      return;
    }

    if ("tokens" in response) {
      this.#applyHighlight(response.tokens, response.diagnostics);
      this.#notify({
        status: response.ok ? "ready" : "failed",
        diagnostics: response.diagnostics,
        errorMessage: undefined
      });
    } else {
      this.#applyHighlight([], response.diagnostics);
      this.#notify({
        status: "failed",
        diagnostics: response.diagnostics,
        errorMessage: response.diagnostics[0]?.message ?? "Highlight worker failed."
      });
    }

    this.#flushPendingReset();
  };

  readonly #handleWorkerError = (event: ErrorEvent): void => {
    this.#notify({
      status: "failed",
      diagnostics: [],
      errorMessage: event.message || "Highlight worker failed."
    });
  };

  #applyHighlight(tokens: readonly HighlightToken[], diagnostics: readonly Diagnostic[]): void {
    setMaodieSemanticTokens(this.#model, this.#source, tokens);
    this.#monaco.editor.setModelMarkers(
      this.#model,
      maodieLiveLexerMarkerOwner,
      diagnostics.flatMap((diagnostic) => {
        const marker = diagnosticToMonacoMarkerData(this.#source, diagnostic, {
          error: this.#monaco.MarkerSeverity.Error,
          warning: this.#monaco.MarkerSeverity.Warning,
          info: this.#monaco.MarkerSeverity.Info
        });

        return marker ? [marker] : [];
      })
    );
  }

  #postInit(source: string): void {
    this.#worker.postMessage({
      type: "init",
      requestId: this.#nextRequestId(),
      editorVersion: this.#editorVersion,
      source,
      options: { sourcePath: this.#options.sourcePath },
      loaderOptions: { wasmUrl: this.#options.wasmUrl }
    });
  }

  #postUpdate(edit: SingleEditorEdit): void {
    this.#worker.postMessage({
      type: "update",
      requestId: this.#nextRequestId(),
      editorVersion: this.#editorVersion,
      sessionVersion: this.#sessionVersion ?? 0,
      edit
    });
  }

  #postReset(source: string): void {
    this.#pendingReset = false;
    this.#worker.postMessage({
      type: "reset",
      requestId: this.#nextRequestId(),
      editorVersion: this.#editorVersion,
      source,
      options: { sourcePath: this.#options.sourcePath }
    });
  }

  #flushPendingReset(): void {
    if (this.#pendingReset && this.#sessionVersion !== undefined) {
      this.#postReset(this.#source);
    }
  }

  #nextRequestId(): string {
    this.#requestId += 1;
    return `maodie-highlight-${this.#requestId}`;
  }

  #notify(update: LiveLexerUpdate): void {
    this.#options.onLiveLexerUpdate?.(update);
  }
}

export function singleEditorEditFromMonacoChange(
  sourceBefore: string,
  event: Monaco.editor.IModelContentChangedEvent
): SingleEditorEdit | undefined {
  if (event.changes.length !== 1) {
    return undefined;
  }

  const change = event.changes[0];
  if (!change) {
    return undefined;
  }

  try {
    return {
      range: {
        start: utf16OffsetToByteOffset(sourceBefore, change.rangeOffset),
        end: utf16OffsetToByteOffset(sourceBefore, change.rangeOffset + change.rangeLength)
      },
      replacement: change.text
    };
  } catch {
    return undefined;
  }
}

function utf16OffsetToByteOffset(source: string, targetOffset: number): number {
  if (!Number.isSafeInteger(targetOffset) || targetOffset < 0) {
    throw new RangeError(`UTF-16 offset ${targetOffset} must be a non-negative safe integer.`);
  }

  let utf16Offset = 0;
  let byteOffset = 0;
  for (const codePoint of source) {
    if (utf16Offset === targetOffset) {
      return byteOffset;
    }

    const nextUtf16Offset = utf16Offset + codePoint.length;
    if (targetOffset < nextUtf16Offset) {
      throw new RangeError(`UTF-16 offset ${targetOffset} splits a code point.`);
    }

    utf16Offset = nextUtf16Offset;
    byteOffset += new TextEncoder().encode(codePoint).byteLength;
  }

  if (utf16Offset === targetOffset) {
    return byteOffset;
  }

  throw new RangeError(`UTF-16 offset ${targetOffset} is outside the source length.`);
}
