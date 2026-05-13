import type { Artifact } from "@maodie/compiler-wasm";
import {
  compilerWasmDisplayUrl,
  sourcePath,
  wasmAssetNotes
} from "./compilerClient";
import { workbenchExamples, type WorkbenchExample } from "./examples";
import { diagnosticsSummaryLabel, renderDiagnostics, renderEvaluation } from "./panels";
import { dumpKeys, type DumpKey, type IdeState } from "./state";

export { dumpKeys } from "./state";
export type { DumpKey, EvaluationState, IdeState } from "./state";

export function renderWorkbench(state: IdeState): string {
  return `
    <main class="flex h-screen flex-col overflow-hidden bg-neutral-950 text-neutral-100">
      <header class="shrink-0 border-b border-neutral-800 bg-neutral-950 px-5 py-4">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <div>
            <p class="text-xs font-semibold uppercase tracking-normal text-cyan-300">Maodie IDE</p>
            <h1 class="text-xl font-semibold tracking-normal text-white">Browser Workbench</h1>
          </div>
          <div class="${statusClass(state.status)} rounded border px-3 py-2 text-sm">
            ${statusLabel(state)}
          </div>
        </div>
      </header>
      <section class="grid min-h-0 flex-1 grid-rows-[minmax(0,1fr)_minmax(0,1fr)] overflow-hidden min-[600px]:grid-cols-[minmax(0,1fr)_minmax(320px,440px)] min-[600px]:grid-rows-1">
        <section class="flex min-h-0 flex-col overflow-hidden border-b border-neutral-800 min-[600px]:border-b-0 min-[600px]:border-r">
          <div class="shrink-0 space-y-3 border-b border-neutral-800 px-4 py-3">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p class="text-sm font-medium text-neutral-100">${sourcePath}</p>
                <p class="mt-1 text-xs text-neutral-500">${escapeHtml(compilerWasmDisplayUrl())}</p>
              </div>
              <button id="compile-button" class="rounded border border-cyan-500/50 px-3 py-2 text-sm font-medium text-cyan-100 hover:bg-cyan-500/10 focus:outline-none focus:ring-2 focus:ring-cyan-400">
                Run
              </button>
            </div>
            <div class="flex flex-wrap items-center gap-2" role="tablist" aria-label="Workbench examples">
              ${workbenchExamples.map((example) => renderExampleButton(example, state)).join("")}
            </div>
            ${renderSelectedExampleDescription(state)}
          </div>
          <div id="source-editor" data-editor-mount="monaco" class="min-h-0 flex-1 overflow-hidden bg-neutral-950" role="textbox" aria-label="${sourcePath} editor"></div>
        </section>
        <aside class="grid min-h-0 grid-rows-[minmax(130px,0.7fr)_minmax(160px,0.75fr)_minmax(220px,1fr)] overflow-hidden min-[600px]:grid-rows-[minmax(180px,0.75fr)_minmax(170px,0.6fr)_minmax(260px,1fr)]">
          <section class="flex min-h-0 flex-col overflow-hidden border-b border-neutral-800">
            <div class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-neutral-800 px-4 py-3">
              <h2 class="text-sm font-semibold text-neutral-100">Evaluation</h2>
              <div class="flex items-center gap-2">
                <label class="text-xs text-neutral-500" for="eval-input">main(i32)</label>
                <input id="eval-input" class="h-8 w-20 rounded border border-neutral-700 bg-neutral-950 px-2 font-mono text-xs text-neutral-100 outline-none focus:border-cyan-400" inputmode="numeric" value="${escapeHtml(state.evalInput)}" />
                <button id="evaluate-button" class="h-8 rounded border border-cyan-500/50 px-3 text-xs font-medium text-cyan-100 hover:bg-cyan-500/10 focus:outline-none focus:ring-2 focus:ring-cyan-400" ${state.result?.ok ? "" : "disabled"}>
                  Evaluate
                </button>
              </div>
            </div>
            <div class="min-h-0 flex-1 overflow-auto p-4 pb-8 text-sm">
              ${renderEvaluation(state)}
            </div>
          </section>
          <section class="flex min-h-0 flex-col overflow-hidden border-b border-neutral-800">
            <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
              <h2 class="text-sm font-semibold text-neutral-100">Diagnostics</h2>
              <span id="diagnostics-summary" class="text-xs text-neutral-500">${diagnosticsSummaryLabel(state)}</span>
            </div>
            <div id="diagnostics-panel" class="min-h-0 flex-1 space-y-5 overflow-auto p-4 pb-8 text-sm">
              ${renderDiagnostics(state)}
            </div>
          </section>
          <section class="flex min-h-0 flex-col overflow-hidden">
            <div class="shrink-0 border-b border-neutral-800 px-4 py-3">
              <div class="mb-3 flex items-center justify-between gap-3">
                <h2 class="text-sm font-semibold text-neutral-100">Compiler Output</h2>
                <span class="text-xs text-neutral-500">${renderArtifactSummary(state.result?.artifacts ?? [])}</span>
              </div>
              <div class="flex flex-wrap gap-2">
                ${dumpKeys.map((key) => renderDumpButton(key, state)).join("")}
              </div>
            </div>
            <pre class="min-h-0 flex-1 overflow-auto p-4 pb-8 text-xs leading-6 text-cyan-50"><code>${escapeHtml(renderActiveDump(state))}</code></pre>
          </section>
        </aside>
      </section>
    </main>
  `;
}

function renderExampleButton(example: WorkbenchExample, state: IdeState): string {
  const selected = state.activeExampleId === example.id;
  const classes = selected
    ? "border-cyan-400 bg-cyan-500/15 text-cyan-50"
    : "border-neutral-700 text-neutral-200 hover:bg-neutral-800";

  return `<button data-example="${example.id}" class="${classes} rounded border px-3 py-1.5 text-xs font-medium" role="tab" aria-selected="${selected}" title="${escapeHtml(example.description)}">${escapeHtml(example.label)}</button>`;
}

function renderSelectedExampleDescription(state: IdeState): string {
  const selectedExample = workbenchExamples.find((example) => example.id === state.activeExampleId);
  if (!selectedExample) {
    return `<p class="text-xs text-neutral-500">Custom source</p>`;
  }

  return `<p class="text-xs text-neutral-500">${escapeHtml(selectedExample.description)}</p>`;
}

function renderDumpButton(key: DumpKey, state: IdeState): string {
  const available = Boolean(state.result?.dumps[key]);
  const selected = state.activeDump === key;
  const classes = selected
    ? "border-cyan-400 bg-cyan-500/15 text-cyan-50"
    : available
      ? "border-neutral-700 text-neutral-200 hover:bg-neutral-800"
      : "border-neutral-800 text-neutral-500";

  return `<button data-dump="${key}" class="${classes} rounded border px-3 py-1.5 text-xs font-medium" ${available ? "" : "disabled"}>${key.toUpperCase()}</button>`;
}

function renderActiveDump(state: IdeState): string {
  if (state.errorMessage) {
    return [
      state.errorMessage,
      "",
      `dev: ${wasmAssetNotes.development}`,
      `prod: ${wasmAssetNotes.production}`,
      `source: ${wasmAssetNotes.source}`
    ].join("\n");
  }
  if (!state.result) {
    return "点击 Run 编译并执行当前源码。";
  }

  const dump = state.result.dumps[state.activeDump];
  return dump || `当前编译阶段没有生成 ${state.activeDump.toUpperCase()} dump。`;
}

function renderArtifactSummary(artifacts: readonly Artifact[]): string {
  if (artifacts.length === 0) {
    return "no artifacts";
  }

  return artifacts
    .map((artifact) =>
      typeof artifact.content === "string"
        ? `${artifact.filename} ${artifact.content.length} chars`
        : `${artifact.filename} ${artifact.content.byteLength} bytes`
    )
    .join(" | ");
}

function statusLabel(state: IdeState): string {
  if (state.status === "idle") {
    return "Ready to run";
  }
  if (state.status === "loading") {
    return "Loading WASM compiler";
  }
  if (state.status === "compiling") {
    return "Compiling";
  }
  if (state.status === "failed") {
    return state.errorMessage ? "WASM load failed" : "Diagnostics available";
  }

  return "Compiled";
}

function statusClass(status: IdeState["status"]): string {
  if (status === "idle") {
    return "border-neutral-700 bg-neutral-900/60 text-neutral-200";
  }
  if (status === "ready") {
    return "border-emerald-500/40 bg-emerald-500/10 text-emerald-100";
  }
  if (status === "failed") {
    return "border-rose-500/40 bg-rose-500/10 text-rose-100";
  }

  return "border-cyan-500/40 bg-cyan-500/10 text-cyan-100";
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
