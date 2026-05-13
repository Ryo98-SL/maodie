import {
  createCompilerWasm,
  type CompilerWasmLoaderOptions,
  type Diagnostic,
  type HighlightOptions,
  type HighlightSessionResponse,
  type HighlightSessionUpdate,
  type MaodieCompilerWasm,
  type MaodieHighlightSession
} from "./index.js";

export type HighlightWorkerRequest =
  | HighlightWorkerInitRequest
  | HighlightWorkerUpdateRequest
  | HighlightWorkerResetRequest
  | HighlightWorkerDisposeRequest;

export type HighlightWorkerResponse =
  | HighlightWorkerSessionResponse
  | HighlightWorkerDisposeResponse
  | HighlightWorkerErrorResponse;

export interface HighlightWorkerInitRequest {
  readonly type: "init";
  readonly requestId: string;
  readonly editorVersion: number;
  readonly source: string;
  readonly options?: HighlightOptions;
  readonly loaderOptions?: CompilerWasmLoaderOptions;
}

export interface HighlightWorkerUpdateRequest {
  readonly type: "update";
  readonly requestId: string;
  readonly editorVersion: number;
  readonly sessionVersion: number;
  readonly edit: Omit<HighlightSessionUpdate, "editorVersion" | "sessionVersion">;
}

export interface HighlightWorkerResetRequest {
  readonly type: "reset";
  readonly requestId: string;
  readonly editorVersion: number;
  readonly source: string;
  readonly options?: HighlightOptions;
}

export interface HighlightWorkerDisposeRequest {
  readonly type: "dispose";
  readonly requestId: string;
  readonly editorVersion: number;
}

export interface HighlightWorkerSessionResponse extends HighlightSessionResponse {
  readonly type: "init" | "update" | "reset";
  readonly requestId: string;
}

export interface HighlightWorkerDisposeResponse {
  readonly type: "dispose";
  readonly requestId: string;
  readonly ok: true;
  readonly editorVersion: number;
  readonly sessionVersion: number;
}

export interface HighlightWorkerErrorResponse {
  readonly type: HighlightWorkerRequest["type"];
  readonly requestId: string;
  readonly ok: false;
  readonly editorVersion: number;
  readonly sessionVersion: number;
  readonly diagnostics: readonly Diagnostic[];
}

const protocolErrorCode = "MD9000";

export function isStaleHighlightWorkerResponse(
  response: Pick<HighlightWorkerResponse, "editorVersion" | "sessionVersion">,
  currentEditorVersion: number,
  currentSessionVersion?: number
): boolean {
  return (
    response.editorVersion < currentEditorVersion ||
    (currentSessionVersion !== undefined && response.sessionVersion < currentSessionVersion)
  );
}

export function createHighlightWorkerRequestHandler(
  defaultLoaderOptions: CompilerWasmLoaderOptions = {}
): (request: HighlightWorkerRequest) => Promise<HighlightWorkerResponse> {
  let compilerPromise: Promise<MaodieCompilerWasm> | undefined;
  let session: MaodieHighlightSession | undefined;

  return async (request) => {
    try {
      switch (request.type) {
        case "init": {
          session?.dispose();
          compilerPromise = createCompilerWasm({
            ...defaultLoaderOptions,
            ...request.loaderOptions
          });
          const compiler = await compilerPromise;
          session = compiler.createHighlightSession(request.source, {
            ...request.options,
            editorVersion: request.editorVersion
          });

          return withWorkerEnvelope("init", request.requestId, session.current);
        }
        case "update": {
          if (!session) {
            return workerErrorResponse(request, "Highlight worker session is not initialized.", 0);
          }

          return withWorkerEnvelope(
            "update",
            request.requestId,
            session.update({
              ...request.edit,
              editorVersion: request.editorVersion,
              sessionVersion: request.sessionVersion
            })
          );
        }
        case "reset": {
          if (!session) {
            return workerErrorResponse(request, "Highlight worker session is not initialized.", 0);
          }

          return withWorkerEnvelope(
            "reset",
            request.requestId,
            session.reset(request.source, {
              ...request.options,
              editorVersion: request.editorVersion
            })
          );
        }
        case "dispose": {
          const sessionVersion = session?.sessionVersion ?? 0;
          session?.dispose();
          session = undefined;

          return {
            type: "dispose",
            requestId: request.requestId,
            ok: true,
            editorVersion: request.editorVersion,
            sessionVersion
          };
        }
        default:
          return assertNever(request);
      }
    } catch (error) {
      return workerErrorResponse(
        request,
        error instanceof Error ? error.message : String(error),
        session?.sessionVersion ?? 0
      );
    }
  };
}

function withWorkerEnvelope(
  type: HighlightWorkerSessionResponse["type"],
  requestId: string,
  response: HighlightSessionResponse
): HighlightWorkerSessionResponse {
  return {
    type,
    requestId,
    ...response
  };
}

function workerErrorResponse(
  request: HighlightWorkerRequest,
  message: string,
  sessionVersion: number
): HighlightWorkerErrorResponse {
  return {
    type: request.type,
    requestId: request.requestId,
    ok: false,
    editorVersion: request.editorVersion,
    sessionVersion,
    diagnostics: [
      {
        code: protocolErrorCode,
        severity: "error",
        message,
        notes: []
      }
    ]
  };
}

function assertNever(value: never): never {
  throw new Error(`Unhandled highlight worker request: ${JSON.stringify(value)}`);
}

const workerScope = typeof self === "undefined" ? undefined : self;

if (workerScope && "addEventListener" in workerScope && "postMessage" in workerScope) {
  const handleRequest = createHighlightWorkerRequestHandler();

  workerScope.addEventListener("message", (event: MessageEvent<HighlightWorkerRequest>) => {
    void handleRequest(event.data).then((response) => workerScope.postMessage(response));
  });
}
