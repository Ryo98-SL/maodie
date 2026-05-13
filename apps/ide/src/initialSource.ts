import { defaultSource } from "./compilerClient";
import { defaultExampleId, workbenchExamples } from "./examples";
import type { IdeState } from "./state";

export function createInitialSourceState(
  search: string
): Pick<IdeState, "source" | "activeExampleId"> {
  const sourceParam = new URLSearchParams(search).get("source");
  if (sourceParam) {
    return { source: sourceParam, activeExampleId: undefined };
  }

  const defaultExample = workbenchExamples.find((example) => example.id === defaultExampleId);
  return {
    source: defaultExample?.source ?? defaultSource,
    activeExampleId: defaultExampleId
  };
}
