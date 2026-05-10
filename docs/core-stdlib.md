# Maodie Core Stdlib v1

## Scope

The v1 core standard library is loaded as a compiler-provided source module named `core`.
It is deliberately small and only defines contracts required by the type checker, MIR, and the first WASM backend.

```mao
module core

enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }

struct Slice<T> {
  ptr: i32,
  len: i32
}

fn log(message: String) -> unit;
```

Callers can import the declarations as ordinary source symbols:

```mao
import core.Option
import core.Result
import core.Slice
import core.log
```

## Compiler Recognition

`Option<T>` is a normal generic enum in v1. Its canonical shape is exactly:

- `Option.Some(T)`
- `Option.None`

`Result<T, E>` is also a normal generic enum, but the compiler has stable name-based special cases for `?` and MIR lowering. The canonical shape is exactly:

- `Result.Ok(T)`
- `Result.Err(E)`

The special recognition points are:

- Type checking treats `?` as valid only when the expression type is a nominal enum whose final path segment is `Result` and whose generic arity is two.
- Type checking requires the containing function to return a `Result<_, E>` with an assignable error type.
- MIR lowering looks up variants named `Ok` and `Err` on that same `Result` enum and lowers `?` to a variant `match`, payload projection, and early `Err` return.

These names and shapes must stay aligned with `crates/maodie_compiler/src/core.rs`.

## String Representation

`String` remains a compiler built-in type in v1 rather than a source-level core declaration. String literals type-check as `String` and remain literal constants in MIR.

For the WASM backend, a `String` value at the host boundary is represented as a UTF-8 slice in linear memory:

- pointer: `i32`
- length in bytes: `i32`
- encoding: UTF-8
- ownership: borrowed unless a later API explicitly states otherwise

Task 10 should lower string literals by placing UTF-8 bytes in module data or another deterministic readonly allocation strategy, then pass pointer and byte length through the host glue. v1 does not require mutation, concatenation, formatting, or allocation APIs for `String`.

## Slice And Array Minimum

The v1 collection contract is `core.Slice<T>`, represented as:

- `ptr: i32`
- `len: i32`

`ptr` points into WASM linear memory and `len` is an element count, not a byte count. Element layout is defined by the WASM backend's type mapping. Slices are borrowed views; v1 does not include owned arrays, indexing syntax, resizing, iteration traits, or bounds-checking helpers.

If a later parser task adds array syntax, it should lower array parameters or borrowed views to this same `Slice<T>` contract unless the task explicitly introduces an owned array type.

## WASM Runtime Glue

The stable v1 host module is:

- module: `maodie`
- memory export: `memory`

Minimum imports reserved for task 10:

- `maodie.panic(ptr: i32, len: i32) -> unit`
- `maodie.debug_string(ptr: i32, len: i32) -> unit`

The source-level logging API is:

- `core.log(message: String) -> unit`

`core.log("Hello world")` lowers to `maodie.debug_string(ptr, len)` in the WASM backend. In v1, string literal logs preserve UTF-8 byte length. Non-literal `String` values are accepted by the type checker, but the WASM backend records a limitation diagnostic and passes length `0` until owned string metadata exists.

The runtime glue intentionally excludes filesystem, network, time, path, package management, and general IO. These names are exposed as constants from `maodie_compiler::core`.

## Test Examples

Core tests live in `maodie_compiler::core` and cover:

- resolving `core.Option`, `core.Result`, and `core.Slice` imports
- type-checking `Option`, `Result`, `String`, and `Slice`
- lowering `core.log("Hello world")` to the `maodie.debug_string` host import
- lowering a `Result` `?` expression from core-loaded source into MIR
- checking the stable WASM glue names
