import type { CompileResponse } from "@maodie/compiler-wasm";
import {
  compileBrowserSource,
  defaultSource,
  evaluateMain
} from "./compilerClient";
import { defaultExampleId, workbenchExamples } from "./examples";
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

let compileTimer: number | undefined;
let state: IdeState = {
  ...initialSourceState(),
  status: "loading",
  activeDump: "ast",
  result: undefined,
  evalInput: "2",
  evaluation: { status: "idle", result: undefined, errorMessage: undefined },
  errorMessage: undefined,
  requestId: 0
};

render();
void scheduleCompile(0);

function render(): void {
  appRoot.innerHTML = renderWorkbench(state);

  appRoot.querySelector<HTMLTextAreaElement>("#source-editor")?.addEventListener("input", (event) => {
    const source = (event.currentTarget as HTMLTextAreaElement).value;
    state = { ...state, source, activeExampleId: undefined, status: "compiling" };
    render();
    void scheduleCompile(350);
  });
  appRoot.querySelector<HTMLButtonElement>("#compile-button")?.addEventListener("click", () => {
    state = { ...state, status: "compiling" };
    render();
    void scheduleCompile(0);
  });
  appRoot.querySelector<HTMLInputElement>("#eval-input")?.addEventListener("input", (event) => {
    state = {
      ...state,
      evalInput: (event.currentTarget as HTMLInputElement).value,
      evaluation: { status: "idle", result: undefined, errorMessage: undefined }
    };
    render();
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

      state = {
        ...state,
        source: example.source,
        activeExampleId: example.id,
        status: "compiling"
      };
      render();
      void scheduleCompile(0);
    });
  });
  appRoot.querySelectorAll<HTMLButtonElement>("[data-dump]").forEach((button) => {
    button.addEventListener("click", () => {
      state = { ...state, activeDump: button.dataset.dump as DumpKey };
      render();
    });
  });
}

async function scheduleCompile(delayMs: number): Promise<void> {
  if (compileTimer !== undefined) {
    window.clearTimeout(compileTimer);
  }

  const requestId = state.requestId + 1;
  state = {
    ...state,
    requestId,
    status: requestId === 1 ? "loading" : "compiling",
    evaluation: { status: "idle", result: undefined, errorMessage: undefined }
  };

  compileTimer = window.setTimeout(() => {
    void runCompile(requestId, state.source);
  }, delayMs);
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

function initialSourceState(): Pick<IdeState, "source" | "activeExampleId"> {
  const sourceParam = new URLSearchParams(window.location.search).get("source");
  if (sourceParam) {
    return { source: sourceParam, activeExampleId: undefined };
  }

  return { source: defaultSource, activeExampleId: defaultExampleId };
}
