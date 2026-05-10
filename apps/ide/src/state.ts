import type { CompileResponse } from "@maodie/compiler-wasm";

import type { EvaluationResult } from "./compilerClient";
import type { WorkbenchExampleId } from "./examples";

export type CompileStatus = "loading" | "compiling" | "ready" | "failed";
export type DumpKey = "ast" | "hir" | "mir" | "wat" | "types";
export type EvaluationStatus = "idle" | "running" | "ready" | "failed";

export interface EvaluationState {
  readonly status: EvaluationStatus;
  readonly result: EvaluationResult | undefined;
  readonly errorMessage: string | undefined;
}

export interface IdeState {
  readonly source: string;
  readonly status: CompileStatus;
  readonly activeDump: DumpKey;
  readonly activeExampleId: WorkbenchExampleId | undefined;
  readonly result: CompileResponse | undefined;
  readonly evalInput: string;
  readonly evaluation: EvaluationState;
  readonly errorMessage: string | undefined;
  readonly requestId: number;
}

export const dumpKeys: readonly DumpKey[] = ["ast", "hir", "mir", "wat", "types"];
