import {
  type Artifact,
  type CompileResponse,
  type MaodieCompilerWasm,
  createCompilerWasm
} from "@maodie/compiler-wasm";

import compilerWasmUrl from "../../../target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm?url";

export { defaultSource } from "./examples";

export const sourcePath = "workspace/main.mao";

export interface EvaluationResult {
  readonly exportName: string;
  readonly input: number;
  readonly logs: readonly string[];
  readonly raw: number;
  readonly decoded: {
    readonly tag: number;
    readonly payload: number;
    readonly variant: string;
    readonly display: string;
    readonly serialized: string;
  };
}

export const wasmAssetNotes = {
  source: "target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm",
  development:
    "Vite serves the Rust build output through the imported ?url asset during dev.",
  production:
    "Vite copies the imported wasm file into dist/apps/ide/assets and rewrites the runtime URL."
} as const;

let compilerPromise: Promise<MaodieCompilerWasm> | undefined;

export async function compileBrowserSource(source: string): Promise<CompileResponse> {
  const compiler = await loadCompiler();

  return compiler.compile(source, {
    sourcePath,
    target: "wasm"
  });
}

export async function evaluateMain(
  response: CompileResponse,
  input: number
): Promise<EvaluationResult> {
  if (!response.ok) {
    throw new Error("编译失败时无法执行 main。");
  }

  const wasm = response.artifacts.find((artifact) => artifact.kind === "wasm");
  if (!wasm) {
    throw new Error("编译结果中没有 WASM artifact。");
  }

  const wasmModule = await WebAssembly.compile(artifactBytes(wasm));
  let instance: WebAssembly.Instance | undefined;
  const logs: string[] = [];
  let logChunks: string[] = [];
  instance = await WebAssembly.instantiate(wasmModule, {
    maodie: {
      panic: (pointer: number, length: number) => {
        throw new Error(readGuestString(instance, pointer, length) || "Maodie panic");
      },
      debug_string: (pointer: number, length: number) => {
        logChunks.push(readGuestString(instance, pointer, length));
      },
      debug_i32: (value: number) => {
        logChunks.push(String(value | 0));
      },
      debug_bool: (value: number) => {
        logChunks.push(value === 0 ? "false" : "true");
      },
      debug_log_end: () => {
        logs.push(logChunks.join(""));
        logChunks = [];
      }
    }
  });
  const main = instance.exports.main;

  if (typeof main !== "function") {
    throw new Error("WASM module 没有导出可执行的 main 函数。");
  }

  const raw = main(input);
  if (typeof raw !== "number") {
    throw new Error("main 返回值不是 i32 number。");
  }
  if (logChunks.length > 0) {
    logs.push(logChunks.join(""));
  }

  return {
    exportName: "main",
    input,
    logs,
    raw,
    decoded: decodeV1EnumResult(raw)
  };
}

export function compilerWasmDisplayUrl(): string {
  return compilerWasmUrl;
}

async function loadCompiler(): Promise<MaodieCompilerWasm> {
  compilerPromise ??= createCompilerWasm({
    wasmUrl: compilerWasmUrl
  });

  try {
    return await compilerPromise;
  } catch (error) {
    compilerPromise = undefined;
    throw new Error(`无法加载 Maodie WASM 编译器：${describeError(error)}`);
  }
}

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function artifactBytes(artifact: Artifact): ArrayBuffer {
  if (artifact.content instanceof Uint8Array) {
    const copy = new Uint8Array(artifact.content.byteLength);
    copy.set(artifact.content);
    return copy.buffer;
  }

  throw new Error(`${artifact.filename} 不是二进制 WASM artifact。`);
}

function readGuestString(
  instance: WebAssembly.Instance | undefined,
  pointer: number,
  length: number
): string {
  const memory = instance?.exports.memory;
  if (!(memory instanceof WebAssembly.Memory) || length <= 0) {
    return "";
  }

  const bytes = new Uint8Array(memory.buffer, pointer, length);
  return new TextDecoder().decode(bytes);
}

function decodeV1EnumResult(raw: number): EvaluationResult["decoded"] {
  const tag = raw & 0xff;
  const payload = raw >> 8;
  const variant = tag === 0 ? "Ok" : tag === 1 ? "Err" : `tag ${tag}`;
  const decodedValue = {
    type: "Result",
    variant,
    value: payload,
    raw
  };

  return {
    tag,
    payload,
    variant,
    display: `Result.${variant}(${payload})`,
    serialized: JSON.stringify(decodedValue, null, 2)
  };
}
