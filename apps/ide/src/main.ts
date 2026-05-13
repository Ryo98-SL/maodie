import type { CompileResponse } from "@maodie/compiler-wasm";
import {
  compileBrowserSource,
  compilerWasmDisplayUrl,
  evaluateMain,
  sourcePath
} from "./compilerClient";
import { type MaodieEditor, createMaodieEditor } from "./editor";
import { workbenchExamples } from "./examples";
import type { LiveLexerUpdate } from "./highlightAdapter";
import { createInitialSourceState } from "./initialSource";
import { diagnosticsSummaryLabel, renderDiagnostics } from "./panels";
import {
  type DumpKey,
  type EvaluationState,
  type IdeState,
  dumpKeys,
  renderWorkbench
} from "./view";

import "./tailwind.css";

const root = document.querySelector<HTMLDivElement>("#root");

if (!root) {
  throw new Error("Maodie IDE root element was not found.");
}

const appRoot = root;
let editor: MaodieEditor | undefined;

let state: IdeState = {
  ...createInitialSourceState(window.location.search),
  status: "idle",
  activeDump: "ast",
  result: undefined,
  liveLexer: { status: "loading", diagnostics: [], errorMessage: undefined },
  evalInput: "2",
  evaluation: { status: "idle", result: undefined, errorMessage: undefined },
  errorMessage: undefined,
  requestId: 0
};

render();

function render(): void {
  editor?.destroy();
  editor = undefined;
  appRoot.innerHTML = renderWorkbench(state);

  const editorMount = appRoot.querySelector<HTMLDivElement>("#source-editor");
  if (editorMount) {
    editor = createMaodieEditor({
      parent: editorMount,
      source: state.source,
      sourcePath,
      wasmUrl: compilerWasmDisplayUrl(),
      onLiveLexerUpdate: updateLiveLexer,
      onSourceChange: updateSourceFromEditor
    });
  }
  appRoot.querySelector<HTMLButtonElement>("#compile-button")?.addEventListener("click", () => {
    void runCurrentEditorSource();
  });
  appRoot.querySelector<HTMLInputElement>("#eval-input")?.addEventListener("input", (event) => {
    state = {
      ...state,
      evalInput: (event.currentTarget as HTMLInputElement).value
    };
  });
  appRoot.querySelector<HTMLButtonElement>("#evaluate-button")?.addEventListener("click", () => {
    void scheduleEvaluation(state.requestId);
  });
  appRoot.querySelectorAll<HTMLButtonElement>("[data-example]").forEach((button) => {
    button.addEventListener("click", () => {
      const example = workbenchExamples.find((candidate) => candidate.id === button.dataset.example);
      if (!example) {
        return;
      }

      const replacedInEditor = Boolean(editor);
      editor?.replaceSource(example.source);
      state = {
        ...state,
        source: example.source,
        activeExampleId: example.id,
        status: "idle",
        result: undefined,
        evaluation: { status: "idle", result: undefined, errorMessage: undefined },
        errorMessage: undefined,
        requestId: replacedInEditor ? state.requestId : state.requestId + 1
      };
      render();
    });
  });
  appRoot.querySelectorAll<HTMLButtonElement>("[data-dump]").forEach((button) => {
    button.addEventListener("click", () => {
      state = { ...state, activeDump: button.dataset.dump as DumpKey };
      render();
    });
  });
}

function updateLiveLexer(update: LiveLexerUpdate): void {
  state = {
    ...state,
    liveLexer: update
  };
  refreshDiagnosticsPanel();
}

function refreshDiagnosticsPanel(): void {
  const diagnosticsPanel = appRoot.querySelector<HTMLDivElement>("#diagnostics-panel");
  if (diagnosticsPanel) {
    diagnosticsPanel.innerHTML = renderDiagnostics(state);
  }

  const diagnosticsSummary = appRoot.querySelector<HTMLSpanElement>("#diagnostics-summary");
  if (diagnosticsSummary) {
    diagnosticsSummary.textContent = diagnosticsSummaryLabel(state);
  }
}

function updateSourceFromEditor(source: string): void {
  state = {
    ...state,
    source,
    activeExampleId: undefined,
    status: "idle",
    result: undefined,
    evaluation: { status: "idle", result: undefined, errorMessage: undefined },
    errorMessage: undefined,
    requestId: state.requestId + 1
  };
}

async function runCurrentEditorSource(): Promise<void> {
  const requestId = state.requestId + 1;
  const source = editor?.readSource() ?? state.source;
  state = {
    ...state,
    source,
    requestId,
    status: "compiling",
    result: undefined,
    errorMessage: undefined,
    evaluation: { status: "idle", result: undefined, errorMessage: undefined }
  };

  render();
  await runCompile(requestId, source);
}

async function runCompile(requestId: number, source: string): Promise<void> {
  try {
    const result = await compileBrowserSource(source);
    if (requestId !== state.requestId) {
      return;
    }

    state = {
      ...state,
      result,
      status: result.ok ? "ready" : "failed",
      errorMessage: undefined,
      activeDump: chooseActiveDump(state.activeDump, result),
      evaluation: result.ok
        ? { status: "running", result: undefined, errorMessage: undefined }
        : { status: "idle", result: undefined, errorMessage: undefined }
    };

    render();
    if (result.ok) {
      void runEvaluation(requestId, result, state.evalInput);
      return;
    }
  } catch (error) {
    if (requestId !== state.requestId) {
      return;
    }

    state = {
      ...state,
      result: undefined,
      status: "failed",
      evaluation: { status: "idle", result: undefined, errorMessage: undefined },
      errorMessage: error instanceof Error ? error.message : String(error)
    };
  }

  render();
}

async function scheduleEvaluation(requestId: number): Promise<void> {
  const result = state.result;
  if (!result?.ok) {
    state = {
      ...state,
      evaluation: {
        status: "failed",
        result: undefined,
        errorMessage: "需要先得到成功的编译结果。"
      }
    };
    render();
    return;
  }

  state = {
    ...state,
    evaluation: { status: "running", result: undefined, errorMessage: undefined }
  };
  render();
  await runEvaluation(requestId, result, state.evalInput);
}

async function runEvaluation(
  requestId: number,
  result: CompileResponse,
  inputText: string
): Promise<void> {
  const input = Number(inputText);
  if (!Number.isInteger(input)) {
    updateEvaluation(requestId, {
      status: "failed",
      result: undefined,
      errorMessage: "main 参数必须是整数 i32。"
    });
    return;
  }

  try {
    const evaluation = await evaluateMain(result, input);
    updateEvaluation(requestId, {
      status: "ready",
      result: evaluation,
      errorMessage: undefined
    });
  } catch (error) {
    updateEvaluation(requestId, {
      status: "failed",
      result: undefined,
      errorMessage: error instanceof Error ? error.message : String(error)
    });
  }
}

function updateEvaluation(requestId: number, evaluation: EvaluationState): void {
  if (requestId !== state.requestId) {
    return;
  }

  state = { ...state, evaluation };
  render();
}

function chooseActiveDump(activeDump: DumpKey, result: CompileResponse): DumpKey {
  if (result.dumps[activeDump]) {
    return activeDump;
  }

  return dumpKeys.find((key) => result.dumps[key]) ?? activeDump;
}
