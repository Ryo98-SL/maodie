#!/usr/bin/env node

import { realpathSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  type Artifact,
  type CompileResponse,
  type Diagnostic,
  compileMaodieWasm
} from "@maodie/compiler-wasm";

export type EmitKind = "wasm" | "wat" | "ast" | "hir" | "mir";

export interface CliIo {
  readonly cwd?: string;
  readonly stdout?: WritableStreamLike;
  readonly stderr?: WritableStreamLike;
}

interface WritableStreamLike {
  write(chunk: string | Uint8Array): boolean | void;
}

interface CompileCommand {
  readonly kind: "compile";
  readonly sourcePath: string;
  readonly emit: EmitKind;
  readonly outPath?: string;
}

interface RunCommand {
  readonly kind: "run";
  readonly sourcePath: string;
  readonly input: number;
}

type CliCommand = CompileCommand | RunCommand;

const emitKinds = new Set<EmitKind>(["wasm", "wat", "ast", "hir", "mir"]);
const usage = [
  "用法:",
  "  maodie compile <source.mao> --emit <wasm|wat|ast|hir|mir> [--out <path>]",
  "  maodie run <source.mao> [--input <i32>]",
  "",
  "示例:",
  "  maodie compile examples/main.mao --emit wat",
  "  maodie compile examples/main.mao --emit wasm --out dist/main.wasm",
  "  maodie run examples/hello_world.mao --input 0"
].join("\n");

export async function runCli(argv: readonly string[], io: CliIo = {}): Promise<number> {
  const stderr = io.stderr ?? process.stderr;

  try {
    const command = parseArgs(argv);
    if (!command.ok) {
      writeLine(stderr, command.message);
      writeLine(stderr, "");
      writeLine(stderr, usage);
      return 1;
    }

    return command.value.kind === "compile"
      ? await runCompile(command.value, io)
      : await runProgram(command.value, io);
  } catch (error) {
    writeLine(stderr, `错误[MD9003]: ${error instanceof Error ? error.message : String(error)}`);
    return 1;
  }
}

async function runCompile(command: CompileCommand, io: CliIo): Promise<number> {
  const cwd = io.cwd ?? process.cwd();
  const stdout = io.stdout ?? process.stdout;
  const stderr = io.stderr ?? process.stderr;
  const sourcePath = resolve(cwd, command.sourcePath);
  const source = await readFile(sourcePath, "utf8");
  const response = await compileMaodieWasm(source, {
    sourcePath,
    moduleName: basename(sourcePath).replace(/\.[^.]+$/, ""),
    target: "wasm"
  });

  printDiagnostics(response.diagnostics, stderr);

  if (!response.ok) {
    return 1;
  }

  const selected = selectOutput(command.emit, response);
  if (!selected.ok) {
    writeLine(stderr, selected.message);
    return 1;
  }

  if (command.outPath) {
    const outPath = resolve(cwd, command.outPath);
    await mkdir(dirname(outPath), { recursive: true });
    await writeFile(outPath, selected.content);
    return 0;
  }

  if (command.emit === "wasm") {
    await writeFile(resolve(cwd, selected.filename), selected.content);
    return 0;
  }

  stdout.write(selected.content);
  if (typeof selected.content === "string" && !selected.content.endsWith("\n")) {
    stdout.write("\n");
  }
  return 0;
}

async function runProgram(command: RunCommand, io: CliIo): Promise<number> {
  const cwd = io.cwd ?? process.cwd();
  const stdout = io.stdout ?? process.stdout;
  const stderr = io.stderr ?? process.stderr;
  const sourcePath = resolve(cwd, command.sourcePath);
  const source = await readFile(sourcePath, "utf8");
  const response = await compileMaodieWasm(source, {
    sourcePath,
    moduleName: basename(sourcePath).replace(/\.[^.]+$/, ""),
    target: "wasm"
  });

  printDiagnostics(response.diagnostics, stderr);

  if (!response.ok) {
    return 1;
  }

  const run = await executeMain(response, command.input);
  if (!run.ok) {
    writeLine(stderr, run.message);
    return 1;
  }

  if (run.logs.length > 0) {
    for (const log of run.logs) {
      writeLine(stdout, log);
    }
  } else {
    writeLine(stdout, `main(${command.input}) => ${run.raw}`);
  }
  return 0;
}

function parseArgs(argv: readonly string[]):
  | { readonly ok: true; readonly value: CliCommand }
  | { readonly ok: false; readonly message: string } {
  const [command, sourcePath, ...rest] = argv;
  if (command !== "compile" && command !== "run") {
    return { ok: false, message: "错误[MD9004]: 缺少 compile 或 run 子命令。" };
  }
  if (!sourcePath || sourcePath.startsWith("-")) {
    return { ok: false, message: "错误[MD9004]: 缺少要编译的 .mao 源文件路径。" };
  }

  if (command === "run") {
    return parseRunArgs(sourcePath, rest);
  }

  let emit: EmitKind = "wasm";
  let outPath: string | undefined;

  for (let index = 0; index < rest.length; index += 1) {
    const arg = rest[index];
    if (arg === "--emit") {
      const value = rest[index + 1];
      if (!value || value.startsWith("-")) {
        return { ok: false, message: "错误[MD9004]: --emit 需要一个输出类型。" };
      }
      if (!isEmitKind(value)) {
        return { ok: false, message: `错误[MD9004]: 不支持的输出类型 \`${value}\`。` };
      }
      emit = value;
      index += 1;
      continue;
    }
    if (arg === "--out" || arg === "-o") {
      const value = rest[index + 1];
      if (!value || value.startsWith("-")) {
        return { ok: false, message: `错误[MD9004]: ${arg} 需要一个输出路径。` };
      }
      outPath = value;
      index += 1;
      continue;
    }

    return { ok: false, message: `错误[MD9004]: 未识别的参数 \`${arg ?? ""}\`。` };
  }

  return {
    ok: true,
    value: outPath
      ? { kind: "compile", sourcePath, emit, outPath }
      : { kind: "compile", sourcePath, emit }
  };
}

function parseRunArgs(
  sourcePath: string,
  rest: readonly string[]
): { readonly ok: true; readonly value: RunCommand } | { readonly ok: false; readonly message: string } {
  let input = 0;

  for (let index = 0; index < rest.length; index += 1) {
    const arg = rest[index];
    if (arg === "--input") {
      const value = rest[index + 1];
      if (!value || value.startsWith("-")) {
        return { ok: false, message: "错误[MD9004]: --input 需要一个 i32 整数。" };
      }
      input = Number(value);
      if (!Number.isInteger(input)) {
        return { ok: false, message: `错误[MD9004]: --input 值 \`${value}\` 不是整数。` };
      }
      index += 1;
      continue;
    }

    return { ok: false, message: `错误[MD9004]: 未识别的参数 \`${arg ?? ""}\`。` };
  }

  return { ok: true, value: { kind: "run", sourcePath, input } };
}

function selectOutput(
  emit: EmitKind,
  response: CompileResponse
):
  | { readonly ok: true; readonly filename: string; readonly content: string | Uint8Array }
  | { readonly ok: false; readonly message: string } {
  if (emit === "wasm" || emit === "wat") {
    const artifact = response.artifacts.find((candidate) => candidate.kind === emit);
    if (!artifact) {
      return { ok: false, message: `错误[MD9002]: 编译结果中没有 ${emit} artifact。` };
    }
    return { ok: true, filename: artifact.filename, content: artifact.content };
  }

  const dump = response.dumps[emit];
  if (dump === undefined) {
    return { ok: false, message: `错误[MD9002]: 编译结果中没有 ${emit} dump。` };
  }
  return { ok: true, filename: `${emit}.txt`, content: dump };
}

function printDiagnostics(diagnostics: readonly Diagnostic[], stderr: WritableStreamLike): void {
  for (const diagnostic of diagnostics) {
    const label = formatSeverity(diagnostic.severity);
    writeLine(stderr, `${label}[${diagnostic.code}]: ${diagnostic.message}`);
    if (diagnostic.span) {
      writeLine(
        stderr,
        `  位置: ${diagnostic.span.fileName}:${diagnostic.span.start.line}:${diagnostic.span.start.column}`
      );
    }
    for (const note of diagnostic.notes) {
      writeLine(stderr, `  提示: ${note}`);
    }
  }
}

async function executeMain(
  response: CompileResponse,
  input: number
): Promise<
  | { readonly ok: true; readonly raw: number; readonly logs: readonly string[] }
  | { readonly ok: false; readonly message: string }
> {
  const wasm = response.artifacts.find((artifact) => artifact.kind === "wasm");
  if (!wasm) {
    return { ok: false, message: "错误[MD9002]: 编译结果中没有 wasm artifact。" };
  }

  const logs: string[] = [];
  let logChunks: string[] = [];
  let instance: WebAssembly.Instance | undefined;
  const module = await WebAssembly.compile(artifactBytes(wasm));
  instance = await WebAssembly.instantiate(module, {
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
    return { ok: false, message: "错误[MD9002]: WASM module 没有导出 main 函数。" };
  }

  const raw = main(input);
  if (typeof raw !== "number") {
    return { ok: false, message: "错误[MD9002]: main 返回值不是 i32 number。" };
  }
  if (logChunks.length > 0) {
    logs.push(logChunks.join(""));
  }

  return { ok: true, raw, logs };
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

  return new TextDecoder().decode(new Uint8Array(memory.buffer, pointer, length));
}

function formatSeverity(severity: Diagnostic["severity"]): string {
  switch (severity) {
    case "error":
      return "错误";
    case "warning":
      return "警告";
    case "info":
      return "信息";
  }
}

function isEmitKind(value: string): value is EmitKind {
  return emitKinds.has(value as EmitKind);
}

function writeLine(stream: WritableStreamLike, line: string): void {
  stream.write(`${line}\n`);
}

const entryPath = process.argv[1] ? realpathSync(resolve(process.argv[1])) : "";
const currentPath = realpathSync(fileURLToPath(import.meta.url));

if (entryPath === currentPath) {
  process.exitCode = await runCli(process.argv.slice(2));
}
