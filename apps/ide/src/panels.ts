import type { Diagnostic } from "@maodie/compiler-wasm";

import type { IdeState } from "./state";

export function renderEvaluation(state: IdeState): string {
  if (state.errorMessage) {
    return diagnosticPanel("info", "等待编译器", "WASM 编译器加载成功后才能执行。");
  }
  if (!state.result) {
    return diagnosticPanel("info", "等待运行", "点击 Run 编译并执行当前源码。");
  }
  if (!state.result.ok) {
    return diagnosticPanel("error", "未执行", "当前源码有诊断错误。");
  }
  if (state.evaluation.status === "running") {
    return diagnosticPanel("info", "正在执行", `调用 main(${state.evalInput})。`);
  }
  if (state.evaluation.status === "failed") {
    return diagnosticPanel("error", "执行失败", state.evaluation.errorMessage ?? "未知错误。");
  }
  if (state.evaluation.status !== "ready" || !state.evaluation.result) {
    return diagnosticPanel("info", "未执行", "点击 Evaluate 运行 main。");
  }

  const { result } = state.evaluation;

  return `
    <div class="grid grid-cols-2 gap-3 text-sm">
      <div class="col-span-2 rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">logs</p>
        <pre class="mt-2 whitespace-pre-wrap font-mono text-cyan-100">${escapeHtml(renderLogs(result.logs))}</pre>
      </div>
      <div class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">call</p>
        <p class="mt-2 font-mono text-cyan-100">${escapeHtml(result.exportName)}(${result.input})</p>
      </div>
      <div class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">raw i32</p>
        <p class="mt-2 font-mono text-cyan-100">${result.raw}</p>
      </div>
      <div class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">tag</p>
        <p class="mt-2 font-mono text-cyan-100">${result.decoded.tag}</p>
      </div>
      <div class="rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">payload</p>
        <p class="mt-2 font-mono text-cyan-100">${result.decoded.payload}</p>
      </div>
      <div class="col-span-2 rounded border border-neutral-800 bg-neutral-900/40 p-3">
        <p class="text-xs text-neutral-500">variant</p>
        <p class="mt-2 font-mono text-cyan-100">${escapeHtml(result.decoded.display)}</p>
        <pre class="mt-3 whitespace-pre-wrap rounded border border-neutral-800 bg-neutral-950 p-3 font-mono text-xs leading-5 text-cyan-50">${escapeHtml(result.decoded.serialized)}</pre>
      </div>
    </div>
  `;
}

export function renderDiagnostics(state: IdeState): string {
  if (state.errorMessage) {
    return diagnosticPanel("error", "WASM 加载失败", state.errorMessage);
  }
  if (state.status === "loading") {
    return diagnosticPanel("info", "正在加载编译器", "首次编译会获取 Rust WASM 产物。");
  }
  if (state.status === "compiling") {
    return diagnosticPanel("info", "正在运行", "当前源码正在编译，成功后会执行 main。");
  }

  const diagnostics = state.result?.diagnostics ?? [];
  if (diagnostics.length === 0) {
    return diagnosticPanel("success", "通过", "没有诊断。");
  }

  return diagnostics.map(renderDiagnostic).join("");
}

function renderDiagnostic(diagnostic: Diagnostic): string {
  const location = diagnostic.span
    ? `${diagnostic.span.fileName}:${diagnostic.span.start.line}:${diagnostic.span.start.column}`
    : "无源码位置";
  const notes =
    diagnostic.notes.length > 0
      ? `<ul class="mt-2 list-disc space-y-1 pl-5 text-xs text-neutral-300">${diagnostic.notes
          .map((note) => `<li>${escapeHtml(note)}</li>`)
          .join("")}</ul>`
      : "";

  return `
    <article class="${severityClass(diagnostic.severity)} rounded border p-3">
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <span class="text-xs font-semibold uppercase tracking-normal">${diagnostic.severity}</span>
        <span class="text-xs text-neutral-400">${diagnostic.code}</span>
      </div>
      <p class="text-sm leading-6">${escapeHtml(diagnostic.message)}</p>
      <p class="mt-2 font-mono text-xs text-neutral-400">${escapeHtml(location)}</p>
      ${notes}
    </article>
  `;
}

function diagnosticPanel(kind: "success" | "info" | "error", title: string, message: string): string {
  const classes = {
    success: "border-emerald-500/35 bg-emerald-500/10 text-emerald-100",
    info: "border-cyan-500/35 bg-cyan-500/10 text-cyan-100",
    error: "border-rose-500/35 bg-rose-500/10 text-rose-100"
  };

  return `
    <div class="${classes[kind]} rounded border p-3">
      <p class="text-sm font-semibold">${escapeHtml(title)}</p>
      <p class="mt-2 text-sm leading-6">${escapeHtml(message)}</p>
    </div>
  `;
}

function severityClass(severity: Diagnostic["severity"]): string {
  if (severity === "error") {
    return "border-rose-500/35 bg-rose-500/10 text-rose-100";
  }
  if (severity === "warning") {
    return "border-amber-500/35 bg-amber-500/10 text-amber-100";
  }

  return "border-cyan-500/35 bg-cyan-500/10 text-cyan-100";
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function renderLogs(logs: readonly string[]): string {
  return logs.length > 0 ? logs.join("\n") : "No logs";
}
