//! WebAssembly ABI for the Maodie Rust compiler facade.
//!
//! The exported functions form a low-level memory contract consumed by the
//! TypeScript wrapper. Public callers should use `@maodie/compiler-wasm`.

use std::collections::BTreeMap;

use maodie_compiler::{
    core::check_source_with_core,
    diagnostics::{
        Diagnostic as CompilerDiagnostic, DiagnosticSeverity, DiagnosticSpan, SourceFile, SourceId,
    },
    mir::lower_package,
    syntax::parse_source,
    wasm::compile_mir_to_wasm,
};
use serde::{Deserialize, Serialize};

const API_ERROR_CODE: &str = "MD9000";
const WASM_NOTE_CODE: &str = "MD9001";

/// Compile options accepted by the WASM JSON API.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompileOptions {
    /// Display path for diagnostics and downstream artifact naming.
    pub source_path: Option<String>,
    /// Optional caller-owned module name for future package-level APIs.
    pub module_name: Option<String>,
    /// Requested target. Task 11 supports `wasm`.
    pub target: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompileResponse {
    ok: bool,
    diagnostics: Vec<ApiDiagnostic>,
    artifacts: Vec<ApiArtifact>,
    dumps: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiArtifact {
    kind: &'static str,
    filename: String,
    content: ArtifactContent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum ArtifactContent {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiDiagnostic {
    code: String,
    severity: &'static str,
    message: String,
    span: Option<ApiDiagnosticSpan>,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiDiagnosticSpan {
    source_id: usize,
    file_name: String,
    start: ApiPosition,
    end: ApiPosition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiPosition {
    offset: usize,
    line: usize,
    column: usize,
}

pub struct ResponseBuffer {
    bytes: Vec<u8>,
}

/// Allocates a byte buffer inside WASM memory.
///
/// The caller must eventually pass the same pointer and length to
/// [`maodie_dealloc`] unless ownership is transferred to another API function.
#[no_mangle]
pub extern "C" fn maodie_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer
}

/// Frees a byte buffer allocated by [`maodie_alloc`].
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_dealloc(pointer: *mut u8, len: usize) {
    if pointer.is_null() || len == 0 {
        return;
    }

    unsafe {
        drop(Vec::from_raw_parts(pointer, len, len));
    }
}

/// Compiles a UTF-8 source buffer using options JSON and returns a response handle.
///
/// The returned pointer is a response handle, not a byte pointer. Use
/// [`maodie_response_len`] to read its byte length, then read that many bytes
/// from the pointer address before calling [`maodie_free_response`].
#[no_mangle]
pub extern "C" fn maodie_compile(
    source_pointer: *const u8,
    source_len: usize,
    options_pointer: *const u8,
    options_len: usize,
) -> *mut ResponseBuffer {
    let source = read_utf8(source_pointer, source_len);
    let options = read_options(options_pointer, options_len);
    let response = match (source, options) {
        (Ok(source), Ok(options)) => compile_source(&source, options),
        (Err(message), _) | (_, Err(message)) => error_response(message),
    };
    let bytes = serialize_response(&response);

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Returns the JSON byte length stored in a response handle.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_response_len(response: *const ResponseBuffer) -> usize {
    if response.is_null() {
        return 0;
    }

    unsafe { (*response).bytes.len() }
}

/// Returns a pointer to JSON bytes stored in a response handle.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_response_bytes(response: *const ResponseBuffer) -> *const u8 {
    if response.is_null() {
        return std::ptr::null();
    }

    unsafe { (*response).bytes.as_ptr() }
}

/// Frees a response handle returned by [`maodie_compile`].
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_free_response(response: *mut ResponseBuffer) {
    if response.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(response));
    }
}

fn compile_source(source_text: &str, options: CompileOptions) -> CompileResponse {
    let source_path = options.source_path.as_deref().unwrap_or("<memory>");
    if source_text.trim().is_empty() {
        return CompileResponse {
            ok: false,
            diagnostics: vec![ApiDiagnostic {
                code: "MD0001".to_owned(),
                severity: "error",
                message: "Maodie 源文件为空。".to_owned(),
                span: None,
                notes: Vec::new(),
            }],
            artifacts: Vec::new(),
            dumps: BTreeMap::new(),
        };
    }

    let source = SourceFile::new(SourceId::new(1), source_path, source_text);
    let parsed = parse_source(&source);
    let typed = check_source_with_core(&source);
    let mut diagnostics = typed
        .diagnostics
        .iter()
        .map(ApiDiagnostic::from)
        .collect::<Vec<_>>();
    let has_error = typed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    let mut dumps = BTreeMap::from([
        ("ast".to_owned(), parsed.ast.dump()),
        ("hir".to_owned(), typed.package.dump()),
        ("types".to_owned(), typed.dump()),
    ]);

    if has_error {
        return CompileResponse {
            ok: false,
            diagnostics,
            artifacts: Vec::new(),
            dumps,
        };
    }

    let mir = lower_package(&typed);
    dumps.insert("mir".to_owned(), mir.dump());
    let wasm = compile_mir_to_wasm(&mir);
    dumps.insert("wat".to_owned(), wasm.wat.clone());
    diagnostics.extend(wasm.diagnostics.iter().map(|diagnostic| ApiDiagnostic {
        code: WASM_NOTE_CODE.to_owned(),
        severity: "warning",
        message: diagnostic.message.clone(),
        span: None,
        notes: Vec::new(),
    }));

    let artifacts = vec![
        ApiArtifact {
            kind: "wat",
            filename: wasm.artifact_names.wat_dump.to_owned(),
            content: ArtifactContent::Text(wasm.wat),
        },
        ApiArtifact {
            kind: "wasm",
            filename: wasm.artifact_names.wasm_binary.to_owned(),
            content: ArtifactContent::Bytes(wasm.wasm),
        },
    ];

    CompileResponse {
        ok: true,
        diagnostics,
        artifacts,
        dumps,
    }
}

fn read_utf8(pointer: *const u8, len: usize) -> Result<String, String> {
    let bytes = read_bytes(pointer, len)?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| format!("source buffer is not valid UTF-8: {error}"))
}

fn read_options(pointer: *const u8, len: usize) -> Result<CompileOptions, String> {
    if len == 0 {
        return Ok(CompileOptions::default());
    }

    let bytes = read_bytes(pointer, len)?;
    serde_json::from_slice(bytes)
        .map_err(|error| format!("compile options are invalid JSON: {error}"))
}

fn read_bytes<'bytes>(pointer: *const u8, len: usize) -> Result<&'bytes [u8], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err("received a null pointer with non-zero length".to_owned());
    }

    unsafe { Ok(std::slice::from_raw_parts(pointer, len)) }
}

fn serialize_response(response: &CompileResponse) -> Vec<u8> {
    serde_json::to_vec(response).unwrap_or_else(|error| {
        let fallback = error_response(format!("failed to serialize compile response: {error}"));
        serde_json::to_vec(&fallback).expect("fallback response must serialize")
    })
}

fn error_response(message: impl Into<String>) -> CompileResponse {
    CompileResponse {
        ok: false,
        diagnostics: vec![ApiDiagnostic {
            code: API_ERROR_CODE.to_owned(),
            severity: "error",
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }],
        artifacts: Vec::new(),
        dumps: BTreeMap::new(),
    }
}

impl From<&CompilerDiagnostic> for ApiDiagnostic {
    fn from(diagnostic: &CompilerDiagnostic) -> Self {
        Self {
            code: diagnostic.code.to_string(),
            severity: diagnostic.severity.as_str(),
            message: diagnostic.message.clone(),
            span: diagnostic.span.as_ref().map(ApiDiagnosticSpan::from),
            notes: diagnostic.notes.clone(),
        }
    }
}

impl From<&DiagnosticSpan> for ApiDiagnosticSpan {
    fn from(span: &DiagnosticSpan) -> Self {
        Self {
            source_id: span.source_id.get(),
            file_name: span.file_name.clone(),
            start: ApiPosition {
                offset: span.start.byte_offset,
                line: span.start.line,
                column: span.start.column,
            },
            end: ApiPosition {
                offset: span.end.byte_offset,
                line: span.end.line,
                column: span.end.column,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compile_source, CompileOptions};

    #[test]
    fn compiles_source_to_json_contract() {
        let response = compile_source(
            "\
module demo

fn main(value: i32) -> i32 {
  return value + 1
}
",
            CompileOptions {
                source_path: Some("smoke.mao".to_owned()),
                module_name: None,
                target: Some("wasm".to_owned()),
            },
        );

        assert!(response.ok, "{:#?}", response.diagnostics);
        assert!(response
            .artifacts
            .iter()
            .any(|artifact| artifact.filename == "module.wat"));
        assert!(response
            .artifacts
            .iter()
            .any(|artifact| artifact.filename == "module.wasm"));
        assert!(response.dumps.contains_key("hir"));
        assert!(response.dumps.contains_key("mir"));
        assert!(response.dumps.contains_key("wat"));
        assert!(response.dumps.contains_key("ast"));
    }

    #[test]
    fn reports_source_diagnostics_without_artifacts() {
        let response = compile_source(
            "\
module demo

fn main() -> i32 {
  return @
}
",
            CompileOptions::default(),
        );

        assert!(!response.ok);
        assert!(!response.diagnostics.is_empty());
        assert!(response.artifacts.is_empty());
        assert!(response.dumps.contains_key("ast"));
        assert!(response.dumps.contains_key("hir"));
    }

    #[test]
    fn reports_empty_source_as_error() {
        let response = compile_source("", CompileOptions::default());

        assert!(!response.ok);
        assert_eq!(response.diagnostics[0].code, "MD0001");
        assert!(response.artifacts.is_empty());
        assert!(response.dumps.is_empty());
    }
}
