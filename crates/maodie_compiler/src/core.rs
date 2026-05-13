//! Core standard library contract for Maodie v1.
//!
//! The v1 core library is intentionally tiny: it provides the nominal
//! declarations that the compiler already treats as well-known by name, plus
//! stable WASM boundary constants for the backend task.

use maodie_diagnostics::{SourceFile, SourceId};

use crate::resolver::{resolve_sources, ResolveResult};
use crate::typeck::{check_sources, TypeCheckResult};

/// Stable source id used by [`core_source`] when callers do not provide one.
pub const CORE_SOURCE_ID: SourceId = SourceId::new(0);

/// Display path for the built-in core source module.
pub const CORE_SOURCE_NAME: &str = "core.mao";

/// Source text for the Maodie v1 core library.
///
/// `String` remains a compiler built-in type in v1. `Option`, `Result`, and
/// `Slice` are ordinary source declarations loaded before user modules. `log`
/// is declared as a host-backed function. The type checker and WASM backend
/// recognize `core.log` specially to support minimal `{}` interpolation.
pub const CORE_SOURCE: &str = "\
module core

enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }

struct Slice<T> {
  ptr: i32,
  len: i32
}

fn log(message: String) -> unit;
";

/// WASM module name used for Maodie host imports in v1.
pub const WASM_HOST_MODULE: &str = "maodie";

/// Host import name for reporting a panic/trap message by string slice.
pub const WASM_IMPORT_PANIC: &str = "panic";

/// Host import name for writing a debug string slice.
pub const WASM_IMPORT_DEBUG_STRING: &str = "debug_string";

/// Host import name for writing a debug i32 chunk.
pub const WASM_IMPORT_DEBUG_I32: &str = "debug_i32";

/// Host import name for writing a debug bool chunk.
pub const WASM_IMPORT_DEBUG_BOOL: &str = "debug_bool";

/// Host import name for ending one formatted debug log line.
pub const WASM_IMPORT_DEBUG_LOG_END: &str = "debug_log_end";

/// Linear-memory export name expected by host glue.
pub const WASM_MEMORY_EXPORT: &str = "memory";

/// Creates a [`SourceFile`] for the built-in core library.
#[must_use]
pub fn core_source() -> SourceFile {
    core_source_with_id(CORE_SOURCE_ID)
}

/// Creates a [`SourceFile`] for the built-in core library with a caller-owned id.
#[must_use]
pub fn core_source_with_id(id: SourceId) -> SourceFile {
    SourceFile::new(id, CORE_SOURCE_NAME, CORE_SOURCE)
}

/// Resolves user sources with the core library preloaded.
#[must_use]
pub fn resolve_sources_with_core(sources: &[&SourceFile]) -> ResolveResult {
    let core = core_source();
    let mut all_sources = Vec::with_capacity(sources.len() + 1);
    all_sources.push(&core);
    all_sources.extend_from_slice(sources);
    resolve_sources(&all_sources)
}

/// Type-checks user sources with the core library preloaded.
#[must_use]
pub fn check_sources_with_core(sources: &[&SourceFile]) -> TypeCheckResult {
    let core = core_source();
    let mut all_sources = Vec::with_capacity(sources.len() + 1);
    all_sources.push(&core);
    all_sources.extend_from_slice(sources);
    check_sources(&all_sources)
}

/// Resolves one user source with the core library preloaded.
#[must_use]
pub fn resolve_source_with_core(source: &SourceFile) -> ResolveResult {
    resolve_sources_with_core(&[source])
}

/// Type-checks one user source with the core library preloaded.
#[must_use]
pub fn check_source_with_core(source: &SourceFile) -> TypeCheckResult {
    check_sources_with_core(&[source])
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId};

    use super::{
        check_source_with_core, core_source, resolve_source_with_core, WASM_HOST_MODULE,
        WASM_IMPORT_DEBUG_BOOL, WASM_IMPORT_DEBUG_I32, WASM_IMPORT_DEBUG_LOG_END,
        WASM_IMPORT_DEBUG_STRING, WASM_IMPORT_PANIC, WASM_MEMORY_EXPORT,
    };
    use crate::mir::lower_package;

    #[test]
    fn exposes_core_source_contract() {
        let core = core_source();

        assert_eq!(core.name(), "core.mao");
        assert!(core.text().contains("enum Option<T> { Some(T), None }"));
        assert!(core.text().contains("enum Result<T, E> { Ok(T), Err(E) }"));
        assert!(core.text().contains("struct Slice<T>"));
        assert!(core.text().contains("fn log(message: String) -> unit;"));
    }

    #[test]
    fn resolves_core_imports_as_ordinary_symbols() {
        let source = SourceFile::new(
            SourceId::new(1),
            "uses_core.mao",
            "\
module demo
import core.Option
import core.Result
import core.Slice
import core.log

fn main(value: i32, text: String, items: Slice<i32>) -> Result<Option<i32>, String> {
  log(\"core ready\")
  let wrapped: Option<i32> = Option.Some(value)
  return Result.Ok(wrapped)
}
",
        );

        let resolved = resolve_source_with_core(&source);

        assert!(
            resolved.diagnostics.is_empty(),
            "{:#?}",
            resolved.diagnostics
        );
        let dump = resolved.package.dump();
        assert!(dump.contains("core.Option"));
        assert!(dump.contains("core.Result"));
        assert!(dump.contains("core.Slice"));
        assert!(dump.contains("core.log"));
        assert!(dump.contains("Import core.Option ->"));
    }

    #[test]
    fn typechecks_option_result_string_and_slice_with_core_loaded() {
        let source = SourceFile::new(
            SourceId::new(1),
            "core_types.mao",
            "\
module demo
import core.Option
import core.Result
import core.Slice

fn parse(value: i32, text: String, items: Slice<i32>) -> Result<Option<i32>, String> {
  let none: Option<i32> = Option.None
  let wrapped: Option<i32> = Option.Some(value)
  return Result.Ok(wrapped)
}
",
        );

        let typed = check_source_with_core(&source);

        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        let dump = typed.dump();
        assert!(dump.contains("String"));
        assert!(dump.contains("Substitutions"));
        assert!(dump.contains("T="));
    }

    #[test]
    fn lowers_result_try_from_core_to_mir() {
        let source = SourceFile::new(
            SourceId::new(1),
            "core_try.mao",
            "\
module demo
import core.Result

fn parse(value: i32) -> Result<i32, String> { return Result.Ok(value) }
fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = parse(value)?
  return Result.Ok(parsed)
}
",
        );

        let typed = check_source_with_core(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);

        let dump = lower_package(&typed).dump();

        assert!(dump.contains("match copy"));
        assert!(dump.contains("project copy"));
        assert!(dump.contains("aggregate variant("));
    }

    #[test]
    fn documents_wasm_glue_names_as_stable_constants() {
        assert_eq!(WASM_HOST_MODULE, "maodie");
        assert_eq!(WASM_IMPORT_PANIC, "panic");
        assert_eq!(WASM_IMPORT_DEBUG_STRING, "debug_string");
        assert_eq!(WASM_IMPORT_DEBUG_I32, "debug_i32");
        assert_eq!(WASM_IMPORT_DEBUG_BOOL, "debug_bool");
        assert_eq!(WASM_IMPORT_DEBUG_LOG_END, "debug_log_end");
        assert_eq!(WASM_MEMORY_EXPORT, "memory");
    }
}
