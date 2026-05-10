import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { runCli } from "./main.js";

const examplesRoot = resolve(process.cwd(), "examples");

class MemoryStream {
  readonly chunks: Array<string | Uint8Array> = [];

  write(chunk: string | Uint8Array): void {
    this.chunks.push(chunk);
  }

  text(): string {
    return this.chunks
      .map((chunk) => (typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk)))
      .join("");
  }
}

describe("maodie compile", () => {
  it("prints WAT to stdout for the v1 acceptance example", async () => {
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["compile", "examples/v1_acceptance.mao", "--emit", "wat"], {
      stdout,
      stderr
    });

    expect(exitCode).toBe(0);
    expect(stdout.text()).toContain("(module");
    expect(stdout.text()).toContain("(export \"main\"");
    expect(stderr.text()).toContain("警告[MD9001]");
  });

  it("writes WASM for the v1 acceptance example", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "maodie-cli-wasm-"));
    await writeFile(join(cwd, "main.mao"), await readExample("v1_acceptance.mao"));
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["compile", "main.mao", "--emit", "wasm"], {
      cwd,
      stdout,
      stderr
    });

    const wasm = await readFile(join(cwd, "module.wasm"));
    expect(exitCode).toBe(0);
    expect(stdout.text()).toBe("");
    expect(Array.from(wasm.subarray(0, 4))).toEqual([0, 97, 115, 109]);
  });

  it("returns non-zero and prints Chinese diagnostics for the stable error example", async () => {
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["compile", "examples/v1_error.mao", "--emit", "wat"], {
      stdout,
      stderr
    });

    expect(exitCode).toBe(1);
    expect(stdout.text()).toBe("");
    expect(stderr.text()).toContain("错误[MD0201]");
    expect(stderr.text()).toContain("位置:");
  });

  it("returns non-zero and prints MD0001 for empty source files", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "maodie-cli-empty-"));
    await writeFile(join(cwd, "empty.mao"), "");
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["compile", "empty.mao", "--emit", "wat"], {
      cwd,
      stdout,
      stderr
    });

    expect(exitCode).toBe(1);
    expect(stdout.text()).toBe("");
    expect(stderr.text()).toContain("错误[MD0001]");
  });

  it("prints AST, HIR, and MIR dumps through the wrapper response", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "maodie-cli-dumps-"));
    await writeFile(join(cwd, "main.mao"), await readExample("v1_acceptance.mao"));

    for (const [emit, expected] of [
      ["ast", "File"],
      ["hir", "Package"],
      ["mir", "MIR"]
    ] as const) {
      const stdout = new MemoryStream();
      const stderr = new MemoryStream();
      const exitCode = await runCli(["compile", "main.mao", "--emit", emit], {
        cwd,
        stdout,
        stderr
      });

      expect(exitCode).toBe(0);
      expect(stdout.text()).toContain(expected);
    }
  });

  it("typechecks the v1 surface example through CLI dumps", async () => {
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["compile", "examples/v1_surface.mao", "--emit", "hir"], {
      stdout,
      stderr
    });

    expect(exitCode).toBe(0);
    expect(stdout.text()).toContain("Struct");
    expect(stdout.text()).toContain("Trait");
    expect(stdout.text()).toContain("Impl");
    expect(stderr.text()).toContain("警告[MD9001]");
  });

  it("runs a Hello world program through core.log", async () => {
    const cwd = await mkdtemp(join(tmpdir(), "maodie-cli-run-"));
    await writeFile(
      join(cwd, "hello.mao"),
      `module demo
import core.Result
import core.log

fn main(value: i32) -> Result<i32, String> {
  log("Hello world")
  return Result.Ok(value)
}
`
    );
    const stdout = new MemoryStream();
    const stderr = new MemoryStream();

    const exitCode = await runCli(["run", "hello.mao", "--input", "0"], {
      cwd,
      stdout,
      stderr
    });

    expect(exitCode).toBe(0);
    expect(stdout.text()).toBe("Hello world\n");
    expect(stderr.text()).toContain("警告[MD9001]");
  });
});

async function readExample(filename: string): Promise<string> {
  return await readFile(join(examplesRoot, filename), "utf8");
}
