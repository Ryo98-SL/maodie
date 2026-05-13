import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import MonacoEditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import "monaco-editor/min/vs/editor/editor.main.css";

import { MaodieHighlightAdapter, type LiveLexerUpdate } from "./highlightAdapter";
import {
  maodieLanguageId,
  maodieLiveLexerMarkerOwner,
  maodieSemanticTokenCount,
  maodieThemeId,
  registerMaodieMonacoLanguage
} from "./monacoLanguage";

interface MonacoEnvironment {
  getWorker(workerId: string, label: string): Worker;
}

interface MaodieEditorTestApi {
  getSource(): string;
  replaceSource(source: string): string;
  insertText(text: string): string;
  getLiveMarkerCount(): number;
  getSemanticTokenCount(): number;
}

interface WindowWithMaodieEditor extends Window {
  maodieIdeEditor?: MaodieEditorTestApi;
}

export interface MaodieEditor {
  readSource(): string;
  replaceSource(source: string): void;
  destroy(): void;
}

export interface CreateMaodieEditorOptions {
  readonly parent: HTMLElement;
  readonly source: string;
  readonly sourcePath: string;
  readonly wasmUrl: string;
  readonly onSourceChange: (source: string) => void;
  readonly onLiveLexerUpdate: (update: LiveLexerUpdate) => void;
}

const globalWithMonaco = globalThis as typeof globalThis & {
  MonacoEnvironment?: MonacoEnvironment;
};

globalWithMonaco.MonacoEnvironment = {
  getWorker() {
    return new MonacoEditorWorker();
  }
};

registerMaodieMonacoLanguage(monaco);

export function createMaodieEditor(options: CreateMaodieEditorOptions): MaodieEditor {
  const model = monaco.editor.createModel(
    options.source,
    maodieLanguageId,
    monaco.Uri.from({
      scheme: "inmemory",
      path: `/${options.sourcePath}`
    })
  );
  const editor = monaco.editor.create(options.parent, {
    model,
    theme: maodieThemeId,
    automaticLayout: true,
    lineNumbers: "on",
    lineDecorationsWidth: 12,
    glyphMargin: false,
    folding: false,
    minimap: { enabled: false },
    renderLineHighlight: "line",
    roundedSelection: false,
    scrollBeyondLastLine: false,
    wordWrap: "on",
    bracketPairColorization: { enabled: true },
    guides: {
      bracketPairs: true,
      indentation: false
    },
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    fontSize: 14,
    lineHeight: 28,
    padding: {
      top: 20,
      bottom: 20
    },
    scrollbar: {
      alwaysConsumeMouseWheel: false,
      verticalScrollbarSize: 10,
      horizontalScrollbarSize: 10
    },
    overviewRulerLanes: 0,
    hideCursorInOverviewRuler: true
  });
  const highlightAdapter = new MaodieHighlightAdapter({
    monaco,
    model,
    sourcePath: options.sourcePath,
    wasmUrl: options.wasmUrl,
    onLiveLexerUpdate: options.onLiveLexerUpdate
  });
  const changeSubscription = model.onDidChangeContent((event) => {
    highlightAdapter.handleModelChange(event);
    options.onSourceChange(model.getValue());
  });
  const testApi = createTestApi(editor, model);
  const browserWindow = window as WindowWithMaodieEditor;
  browserWindow.maodieIdeEditor = testApi;

  return {
    readSource(): string {
      return model.getValue();
    },
    replaceSource(source: string): void {
      replaceModelSource(editor, model, source);
    },
    destroy(): void {
      if (browserWindow.maodieIdeEditor === testApi) {
        delete browserWindow.maodieIdeEditor;
      }
      changeSubscription.dispose();
      highlightAdapter.destroy();
      editor.dispose();
      model.dispose();
    }
  };
}

function createTestApi(
  editor: monaco.editor.IStandaloneCodeEditor,
  model: monaco.editor.ITextModel
): MaodieEditorTestApi {
  return {
    getSource(): string {
      return model.getValue();
    },
    replaceSource(source: string): string {
      replaceModelSource(editor, model, source);
      return model.getValue();
    },
    insertText(text: string): string {
      insertTextAtSelection(editor, model, text);
      return model.getValue();
    },
    getLiveMarkerCount(): number {
      return monaco.editor.getModelMarkers({
        owner: maodieLiveLexerMarkerOwner,
        resource: model.uri
      }).length;
    },
    getSemanticTokenCount(): number {
      return maodieSemanticTokenCount(model);
    }
  };
}

function replaceModelSource(
  editor: monaco.editor.IStandaloneCodeEditor,
  model: monaco.editor.ITextModel,
  source: string
): void {
  editor.executeEdits("maodie-source-replace", [
    {
      range: model.getFullModelRange(),
      text: source,
      forceMoveMarkers: true
    }
  ]);
  const lastLine = model.getLineCount();
  editor.setPosition({
    lineNumber: lastLine,
    column: model.getLineMaxColumn(lastLine)
  });
}

function insertTextAtSelection(
  editor: monaco.editor.IStandaloneCodeEditor,
  model: monaco.editor.ITextModel,
  text: string
): void {
  const selection = editor.getSelection();
  const range =
    selection ??
    new monaco.Range(
      model.getLineCount(),
      model.getLineMaxColumn(model.getLineCount()),
      model.getLineCount(),
      model.getLineMaxColumn(model.getLineCount())
    );
  editor.executeEdits("maodie-smoke-input", [
    {
      range,
      text,
      forceMoveMarkers: true
    }
  ]);
}
