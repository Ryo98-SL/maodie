import { defaultSource } from "./compilerClient";
import { defaultExampleId } from "./examples";
import type { IdeState } from "./state";

export function createInitialSourceState(
  search: string
): Pick<IdeState, "source" | "activeExampleId"> {
  const sourceParam = new URLSearchParams(search).get("source");
  if (sourceParam) {
    return { source: sourceParam, activeExampleId: undefined };
  }

  return { source: defaultSource, activeExampleId: defaultExampleId };
}
