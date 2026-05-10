# CLI Module

## Purpose

`cli` owns the `maodie` command line interface around the compiler WASM wrapper.

## Current Directory Structure

- `src/`: command implementation and public exports.
- `project.json`: Nx tasks and dependency metadata.
- `tsconfig.json`: TypeScript project configuration with a reference to `compiler`.

## Key Files

- `src/index.ts`: package API export surface.
- `src/main.ts`: executable command entry point and testable command runner.
- `src/main.test.ts`: CLI smoke tests for v1 success examples, dumps, WASM output, and diagnostics.

## Runtime Behaviors

The CLI reads a source file, compiles it through `@maodie/compiler-wasm`, prints diagnostics to stderr, and writes the requested output to stdout or a file. The `run` command instantiates the emitted WASM, calls `main(i32)`, and writes `core.log` messages captured from `maodie.debug_string` to stdout.

The acceptance suite reads `examples/v1_acceptance.mao` and `examples/v1_error.mao` directly, keeping CLI behavior tied to the public examples instead of embedded test-only source strings.

## Command Reference

```bash
maodie compile <source.mao> --emit <wasm|wat|ast|hir|mir> [--out <path>]
maodie run <source.mao> [--input <i32>]
```

- `--emit wasm` selects the binary WASM artifact. Without `--out`, it writes the wrapper-provided artifact filename, currently `module.wasm`, in the current working directory.
- `--emit wat` selects the text WAT artifact. Without `--out`, it prints WAT to stdout.
- `--emit ast`, `--emit hir`, and `--emit mir` select debug dumps from `CompileResponse.dumps`. Without `--out`, they print text to stdout.
- `--out <path>` or `-o <path>` writes the selected output to that path. Parent directories are created when needed.
- `run` defaults `--input` to `0`; stdout is program log output when `core.log` is called, otherwise it prints the raw `main(i32)` result.

Diagnostics are printed to stderr in Chinese:

```text
错误[MD0201]: <message>
  位置: path/to/file.mao:line:column
```

Warnings and informational diagnostics use `警告[...]` and `信息[...]`. Empty source files return `错误[MD0001]`. Source or argument failures return a non-zero exit code and use CLI-level `MD9003` or `MD9004` diagnostics.

## Integration Notes

Keep process and filesystem concerns in this package. Compiler phases and WASM memory handling should stay behind `packages/compiler-wasm`.
