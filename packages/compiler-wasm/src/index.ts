export type CompileTarget = "wasm";
export type DiagnosticSeverity = "error" | "warning" | "info";
export type ArtifactKind = "wat" | "wasm";

export interface CompileOptions {
  readonly sourcePath?: string;
  readonly moduleName?: string;
  readonly target?: CompileTarget;
}

export interface DiagnosticPosition {
  readonly offset: number;
  readonly line: number;
  readonly column: number;
}

export interface DiagnosticSpan {
  readonly sourceId: number;
  readonly fileName: string;
  readonly start: DiagnosticPosition;
  readonly end: DiagnosticPosition;
}

export interface Diagnostic {
  readonly code: string;
  readonly severity: DiagnosticSeverity;
  readonly message: string;
  readonly span?: DiagnosticSpan;
  readonly notes: readonly string[];
}

export interface Artifact {
  readonly kind: ArtifactKind;
  readonly filename: string;
  readonly content: string | Uint8Array;
}

export interface CompileResponse {
  readonly ok: boolean;
  readonly diagnostics: readonly Diagnostic[];
  readonly artifacts: readonly Artifact[];
  readonly dumps: Readonly<Record<string, string>>;
}

export interface CompilerWasmLoaderOptions {
  readonly wasmUrl?: string | URL;
  readonly wasmBytes?: ArrayBuffer | Uint8Array;
  readonly wasmModule?: WebAssembly.Module;
  readonly instance?: WebAssembly.Instance;
  readonly imports?: WebAssembly.Imports;
}

interface RawArtifact {
  readonly kind: ArtifactKind;
  readonly filename: string;
  readonly content: string | readonly number[];
}

interface RawCompileResponse {
  readonly ok: boolean;
  readonly diagnostics: readonly Diagnostic[];
  readonly artifacts: readonly RawArtifact[];
  readonly dumps: Readonly<Record<string, string>>;
}

interface CompilerWasmExports extends WebAssembly.Exports {
  readonly memory: WebAssembly.Memory;
  readonly maodie_alloc: (len: number) => number;
  readonly maodie_dealloc: (pointer: number, len: number) => void;
  readonly maodie_compile: (
    sourcePointer: number,
    sourceLen: number,
    optionsPointer: number,
    optionsLen: number
  ) => number;
  readonly maodie_response_len: (responsePointer: number) => number;
  readonly maodie_response_bytes: (responsePointer: number) => number;
  readonly maodie_free_response: (responsePointer: number) => void;
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const defaultWasmUrl = new URL(
  "../../../target/wasm32-unknown-unknown/debug/maodie_wasm_api.wasm",
  import.meta.url
);

let defaultCompiler: Promise<MaodieCompilerWasm> | undefined;

export class MaodieCompilerWasm {
  readonly #exports: CompilerWasmExports;

  private constructor(instance: WebAssembly.Instance) {
    this.#exports = assertCompilerExports(instance.exports);
  }

  static async create(options: CompilerWasmLoaderOptions = {}): Promise<MaodieCompilerWasm> {
    if (options.instance) {
      return new MaodieCompilerWasm(options.instance);
    }

    const imports = createImports(options.imports);
    if (options.wasmModule) {
      return new MaodieCompilerWasm(await WebAssembly.instantiate(options.wasmModule, imports));
    }

    const bytes = await loadWasmBytes(options);
    const { instance } = await WebAssembly.instantiate(bytes, imports);
    return new MaodieCompilerWasm(instance);
  }

  compile(source: string, options: CompileOptions = {}): CompileResponse {
    const sourceBytes = textEncoder.encode(source);
    const optionsBytes = textEncoder.encode(JSON.stringify(options));
    const sourcePointer = this.#copyIntoWasm(sourceBytes);
    const optionsPointer = this.#copyIntoWasm(optionsBytes);
    let responsePointer = 0;

    try {
      responsePointer = this.#exports.maodie_compile(
        sourcePointer,
        sourceBytes.byteLength,
        optionsPointer,
        optionsBytes.byteLength
      );
      return this.#readResponse(responsePointer);
    } finally {
      this.#exports.maodie_dealloc(sourcePointer, sourceBytes.byteLength);
      this.#exports.maodie_dealloc(optionsPointer, optionsBytes.byteLength);
      if (responsePointer !== 0) {
        this.#exports.maodie_free_response(responsePointer);
      }
    }
  }

  #copyIntoWasm(bytes: Uint8Array): number {
    const pointer = this.#exports.maodie_alloc(bytes.byteLength);
    new Uint8Array(this.#exports.memory.buffer, pointer, bytes.byteLength).set(bytes);
    return pointer;
  }

  #readResponse(responsePointer: number): CompileResponse {
    const responseLen = this.#exports.maodie_response_len(responsePointer);
    const responseBytesPointer = this.#exports.maodie_response_bytes(responsePointer);
    const responseBytes = new Uint8Array(
      this.#exports.memory.buffer,
      responseBytesPointer,
      responseLen
    );
    const raw = JSON.parse(textDecoder.decode(responseBytes)) as RawCompileResponse;

    return {
      ok: raw.ok,
      diagnostics: raw.diagnostics,
      artifacts: raw.artifacts.map((artifact) => ({
        kind: artifact.kind,
        filename: artifact.filename,
        content:
          typeof artifact.content === "string"
            ? artifact.content
            : Uint8Array.from(artifact.content)
      })),
      dumps: raw.dumps
    };
  }
}

export async function createCompilerWasm(
  options: CompilerWasmLoaderOptions = {}
): Promise<MaodieCompilerWasm> {
  return MaodieCompilerWasm.create(options);
}

export async function compileMaodieWasm(
  source: string,
  options: CompileOptions & CompilerWasmLoaderOptions = {}
): Promise<CompileResponse> {
  const { compilerOptions, loaderOptions } = splitOptions(options);
  defaultCompiler ??= createCompilerWasm(loaderOptions);
  const compiler = await defaultCompiler;
  return compiler.compile(source, compilerOptions);
}

function splitOptions(
  options: CompileOptions & CompilerWasmLoaderOptions
): {
  compilerOptions: CompileOptions;
  loaderOptions: CompilerWasmLoaderOptions;
} {
  const compilerOptions: CompileOptions = {
    ...(options.sourcePath ? { sourcePath: options.sourcePath } : {}),
    ...(options.moduleName ? { moduleName: options.moduleName } : {}),
    ...(options.target ? { target: options.target } : {})
  };
  const loaderOptions: CompilerWasmLoaderOptions = {
    ...(options.wasmUrl ? { wasmUrl: options.wasmUrl } : {}),
    ...(options.wasmBytes ? { wasmBytes: options.wasmBytes } : {}),
    ...(options.wasmModule ? { wasmModule: options.wasmModule } : {}),
    ...(options.instance ? { instance: options.instance } : {}),
    ...(options.imports ? { imports: options.imports } : {})
  };

  return { compilerOptions, loaderOptions };
}

function assertCompilerExports(exports: WebAssembly.Exports): CompilerWasmExports {
  const required = [
    "memory",
    "maodie_alloc",
    "maodie_dealloc",
    "maodie_compile",
    "maodie_response_len",
    "maodie_response_bytes",
    "maodie_free_response"
  ] as const;

  for (const name of required) {
    if (!(name in exports)) {
      throw new Error(`Maodie compiler WASM export \`${name}\` is missing.`);
    }
  }

  return exports as CompilerWasmExports;
}

function createImports(imports: WebAssembly.Imports = {}): WebAssembly.Imports {
  return {
    ...imports,
    maodie: {
      panic: () => undefined,
      debug_string: () => undefined,
      ...imports.maodie
    }
  };
}

async function loadWasmBytes(options: CompilerWasmLoaderOptions): Promise<ArrayBuffer> {
  if (options.wasmBytes instanceof Uint8Array) {
    return toArrayBuffer(options.wasmBytes);
  }
  if (options.wasmBytes) {
    return options.wasmBytes;
  }

  const wasmUrl = options.wasmUrl ? new URL(options.wasmUrl, import.meta.url) : defaultWasmUrl;
  if (isNodeRuntime() && wasmUrl.protocol === "file:") {
    const { readFile } = await import("node:fs/promises");
    return toArrayBuffer(await readFile(wasmUrl));
  }

  const response = await fetch(wasmUrl);
  if (!response.ok) {
    throw new Error(`Failed to load Maodie compiler WASM from ${wasmUrl.href}: ${response.status}`);
  }
  return await response.arrayBuffer();
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

function isNodeRuntime(): boolean {
  return typeof process !== "undefined" && process.versions?.node !== undefined;
}
