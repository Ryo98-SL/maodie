//! WebAssembly ABI for the Maodie Rust compiler facade.
//!
//! The exported functions form a low-level memory contract consumed by the
//! TypeScript wrapper. Public callers should use `@maodie/compiler-wasm`.

use std::collections::BTreeMap;

use maodie_compiler::{
    core::check_source_with_core,
    diagnostics::{
        Diagnostic as CompilerDiagnostic, DiagnosticSeverity, DiagnosticSpan, SourceFile, SourceId,
        TextRange,
    },
    mir::lower_package,
    syntax::{
        highlight_source, parse_source, HighlightEdit, HighlightKind as SyntaxHighlightKind,
        IncrementalHighlightError, IncrementalHighlightSession, IncrementalHighlightUpdate,
    },
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

/// Highlight options accepted by the WASM JSON API.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HighlightOptions {
    /// Display path for diagnostics.
    pub source_path: Option<String>,
}

/// Initial options accepted when creating an incremental highlight session.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HighlightSessionOptions {
    /// Display path for diagnostics.
    pub source_path: Option<String>,
    /// Caller-owned editor version mirrored back in the response.
    pub editor_version: Option<u64>,
}

/// Reset options accepted by an existing incremental highlight session.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HighlightSessionResetOptions {
    /// Optional new display path for diagnostics. Omitted values preserve the session path.
    pub source_path: Option<String>,
    /// Caller-owned editor version mirrored back in the response.
    pub editor_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct HighlightSessionUpdateRequest {
    editor_version: u64,
    session_version: u64,
    range: ApiTextRange,
    replacement: String,
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
struct HighlightResponse {
    ok: bool,
    tokens: Vec<ApiHighlightToken>,
    diagnostics: Vec<ApiDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct HighlightSessionResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_handle: Option<usize>,
    editor_version: u64,
    session_version: u64,
    changed_range: ApiTextRange,
    tokens: Vec<ApiHighlightToken>,
    diagnostics: Vec<ApiDiagnostic>,
    full_rehighlight: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiHighlightToken {
    kind: ApiHighlightKind,
    range: ApiTextRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ApiHighlightKind {
    Keyword,
    Identifier,
    Comment,
    String,
    Number,
    Boolean,
    Operator,
    Punctuation,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiTextRange {
    start: usize,
    end: usize,
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

pub struct HighlightSessionHandle {
    source_path: String,
    session: IncrementalHighlightSession,
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
    let options = read_json_options(options_pointer, options_len, "compile options");
    let response = match (source, options) {
        (Ok(source), Ok(options)) => compile_source(&source, options),
        (Err(message), _) | (_, Err(message)) => compile_error_response(message),
    };
    let bytes = serialize_response(&response, "compile response", compile_error_response);

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Highlights a UTF-8 source buffer using options JSON and returns a response handle.
///
/// The returned pointer uses the same response-buffer contract as [`maodie_compile`].
#[no_mangle]
pub extern "C" fn maodie_highlight(
    source_pointer: *const u8,
    source_len: usize,
    options_pointer: *const u8,
    options_len: usize,
) -> *mut ResponseBuffer {
    let source = read_utf8(source_pointer, source_len);
    let options = read_json_options(options_pointer, options_len, "highlight options");
    let response = match (source, options) {
        (Ok(source), Ok(options)) => highlight_source_for_json(&source, options),
        (Err(message), _) | (_, Err(message)) => highlight_error_response(message),
    };
    let bytes = serialize_response(&response, "highlight response", highlight_error_response);

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Creates an incremental highlight session and returns a response containing its handle.
///
/// The handle is owned by the caller until passed to [`maodie_highlight_session_dispose`].
#[no_mangle]
pub extern "C" fn maodie_highlight_session_create(
    source_pointer: *const u8,
    source_len: usize,
    options_pointer: *const u8,
    options_len: usize,
) -> *mut ResponseBuffer {
    let source = read_utf8(source_pointer, source_len);
    let options = read_json_options(options_pointer, options_len, "highlight session options");
    let response = match (source, options) {
        (Ok(source), Ok(options)) => create_highlight_session_response(&source, options),
        (Err(message), _) | (_, Err(message)) => {
            highlight_session_error_response(message, None, 0, 0)
        }
    };
    let bytes = serialize_response(&response, "highlight session create response", |message| {
        highlight_session_error_response(message, None, 0, 0)
    });

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Applies a source edit to an incremental highlight session.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_highlight_session_update(
    session_handle: *mut HighlightSessionHandle,
    request_pointer: *const u8,
    request_len: usize,
) -> *mut ResponseBuffer {
    let request = read_json_request::<HighlightSessionUpdateRequest>(
        request_pointer,
        request_len,
        "highlight session update request",
    );
    let response = match (session_handle.is_null(), request) {
        (true, Ok(request)) => highlight_session_error_response(
            "highlight session handle is null",
            None,
            request.editor_version,
            0,
        ),
        (true, Err(message)) => highlight_session_error_response(message, None, 0, 0),
        (false, Ok(request)) => unsafe {
            update_highlight_session_for_json(&mut *session_handle, request)
        },
        (false, Err(message)) => unsafe {
            let session = &*session_handle;
            highlight_session_error_response(message, None, 0, session.session.version())
        },
    };
    let bytes = serialize_response(&response, "highlight session update response", |message| {
        highlight_session_error_response(message, None, 0, 0)
    });

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Replaces the source snapshot for an incremental highlight session.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_highlight_session_reset(
    session_handle: *mut HighlightSessionHandle,
    source_pointer: *const u8,
    source_len: usize,
    options_pointer: *const u8,
    options_len: usize,
) -> *mut ResponseBuffer {
    let source = read_utf8(source_pointer, source_len);
    let options = read_json_options::<HighlightSessionResetOptions>(
        options_pointer,
        options_len,
        "highlight session reset options",
    );
    let response = match (session_handle.is_null(), source, options) {
        (true, Ok(_), Ok(options)) => highlight_session_error_response(
            "highlight session handle is null",
            None,
            options.editor_version.unwrap_or(0),
            0,
        ),
        (true, Err(message), _) | (true, _, Err(message)) => {
            highlight_session_error_response(message, None, 0, 0)
        }
        (false, Ok(source), Ok(options)) => unsafe {
            reset_highlight_session_for_json(&mut *session_handle, &source, options)
        },
        (false, Err(message), _) | (false, _, Err(message)) => unsafe {
            let session = &*session_handle;
            highlight_session_error_response(message, None, 0, session.session.version())
        },
    };
    let bytes = serialize_response(&response, "highlight session reset response", |message| {
        highlight_session_error_response(message, None, 0, 0)
    });

    Box::into_raw(Box::new(ResponseBuffer { bytes }))
}

/// Disposes an incremental highlight session handle.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn maodie_highlight_session_dispose(session_handle: *mut HighlightSessionHandle) {
    if session_handle.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(session_handle));
    }
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

fn highlight_source_for_json(source_text: &str, options: HighlightOptions) -> HighlightResponse {
    let source_path = options.source_path.as_deref().unwrap_or("<memory>");
    let source = SourceFile::new(SourceId::new(1), source_path, source_text);
    let highlighted = highlight_source(&source);
    let has_error = highlighted
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

    HighlightResponse {
        ok: !has_error,
        tokens: highlighted
            .tokens
            .into_iter()
            .map(ApiHighlightToken::from)
            .collect(),
        diagnostics: highlighted
            .diagnostics
            .iter()
            .map(ApiDiagnostic::from)
            .collect(),
    }
}

fn create_highlight_session_response(
    source_text: &str,
    options: HighlightSessionOptions,
) -> HighlightSessionResponse {
    let source_path = options.source_path.unwrap_or_else(|| "<memory>".to_owned());
    let source = SourceFile::new(SourceId::new(1), &source_path, source_text);
    let session = IncrementalHighlightSession::new(source);
    let response = highlight_session_snapshot_response(
        None,
        options.editor_version.unwrap_or(0),
        session.version(),
        ApiTextRange {
            start: 0,
            end: session.source().len_bytes(),
        },
        session.tokens(),
        session.diagnostics(),
        true,
    );
    let handle = Box::into_raw(Box::new(HighlightSessionHandle {
        source_path,
        session,
    })) as usize;

    HighlightSessionResponse {
        session_handle: Some(handle),
        ..response
    }
}

fn update_highlight_session_for_json(
    handle: &mut HighlightSessionHandle,
    request: HighlightSessionUpdateRequest,
) -> HighlightSessionResponse {
    let current_version = handle.session.version();
    if request.session_version != current_version {
        return highlight_session_error_response(
            format!(
                "highlight session version mismatch: request version {}, current version {}",
                request.session_version, current_version
            ),
            None,
            request.editor_version,
            current_version,
        );
    }

    let edit = HighlightEdit {
        range: TextRange::new(request.range.start, request.range.end),
        replacement: request.replacement,
    };

    match handle.session.update(edit) {
        Ok(update) => highlight_session_update_response(None, request.editor_version, update),
        Err(IncrementalHighlightError::InvalidEditRange { range, source_len }) => {
            highlight_session_error_response(
                format!(
                    "invalid highlight edit range {}..{} for source length {}",
                    range.start, range.end, source_len
                ),
                None,
                request.editor_version,
                handle.session.version(),
            )
        }
    }
}

fn reset_highlight_session_for_json(
    handle: &mut HighlightSessionHandle,
    source_text: &str,
    options: HighlightSessionResetOptions,
) -> HighlightSessionResponse {
    if let Some(source_path) = options.source_path {
        handle.source_path = source_path;
    }

    let source = SourceFile::new(SourceId::new(1), &handle.source_path, source_text);
    let update = handle.session.reset(source);
    highlight_session_update_response(None, options.editor_version.unwrap_or(0), update)
}

fn highlight_session_update_response(
    session_handle: Option<usize>,
    editor_version: u64,
    update: IncrementalHighlightUpdate,
) -> HighlightSessionResponse {
    highlight_session_snapshot_response(
        session_handle,
        editor_version,
        update.version,
        ApiTextRange::from(update.changed_range),
        &update.tokens,
        &update.diagnostics,
        update.full_rehighlight,
    )
}

fn highlight_session_snapshot_response(
    session_handle: Option<usize>,
    editor_version: u64,
    session_version: u64,
    changed_range: ApiTextRange,
    tokens: &[maodie_compiler::syntax::HighlightToken],
    diagnostics: &[CompilerDiagnostic],
    full_rehighlight: bool,
) -> HighlightSessionResponse {
    let has_error = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

    HighlightSessionResponse {
        ok: !has_error,
        session_handle,
        editor_version,
        session_version,
        changed_range,
        tokens: tokens
            .iter()
            .cloned()
            .map(ApiHighlightToken::from)
            .collect(),
        diagnostics: diagnostics.iter().map(ApiDiagnostic::from).collect(),
        full_rehighlight,
    }
}

fn read_utf8(pointer: *const u8, len: usize) -> Result<String, String> {
    let bytes = read_bytes(pointer, len)?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| format!("source buffer is not valid UTF-8: {error}"))
}

fn read_json_options<T>(pointer: *const u8, len: usize, label: &str) -> Result<T, String>
where
    T: Default + for<'de> Deserialize<'de>,
{
    if len == 0 {
        return Ok(T::default());
    }

    let bytes = read_bytes(pointer, len)?;
    serde_json::from_slice(bytes).map_err(|error| format!("{label} are invalid JSON: {error}"))
}

fn read_json_request<T>(pointer: *const u8, len: usize, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = read_bytes(pointer, len)?;
    serde_json::from_slice(bytes).map_err(|error| format!("{label} is invalid JSON: {error}"))
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

fn serialize_response<T, F>(response: &T, label: &str, error_builder: F) -> Vec<u8>
where
    T: Serialize,
    F: FnOnce(String) -> T,
{
    serde_json::to_vec(response).unwrap_or_else(|error| {
        let fallback = error_builder(format!("failed to serialize {label}: {error}"));
        serde_json::to_vec(&fallback).expect("fallback response must serialize")
    })
}

fn compile_error_response(message: impl Into<String>) -> CompileResponse {
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

fn highlight_error_response(message: impl Into<String>) -> HighlightResponse {
    HighlightResponse {
        ok: false,
        tokens: Vec::new(),
        diagnostics: vec![ApiDiagnostic {
            code: API_ERROR_CODE.to_owned(),
            severity: "error",
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }],
    }
}

fn highlight_session_error_response(
    message: impl Into<String>,
    session_handle: Option<usize>,
    editor_version: u64,
    session_version: u64,
) -> HighlightSessionResponse {
    HighlightSessionResponse {
        ok: false,
        session_handle,
        editor_version,
        session_version,
        changed_range: ApiTextRange { start: 0, end: 0 },
        tokens: Vec::new(),
        diagnostics: vec![ApiDiagnostic {
            code: API_ERROR_CODE.to_owned(),
            severity: "error",
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }],
        full_rehighlight: false,
    }
}

impl From<maodie_compiler::syntax::HighlightToken> for ApiHighlightToken {
    fn from(token: maodie_compiler::syntax::HighlightToken) -> Self {
        Self {
            kind: ApiHighlightKind::from(token.kind),
            range: ApiTextRange {
                start: token.range.start,
                end: token.range.end,
            },
        }
    }
}

impl From<TextRange> for ApiTextRange {
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<SyntaxHighlightKind> for ApiHighlightKind {
    fn from(kind: SyntaxHighlightKind) -> Self {
        match kind {
            SyntaxHighlightKind::Keyword => Self::Keyword,
            SyntaxHighlightKind::Identifier => Self::Identifier,
            SyntaxHighlightKind::Comment => Self::Comment,
            SyntaxHighlightKind::String => Self::String,
            SyntaxHighlightKind::Number => Self::Number,
            SyntaxHighlightKind::Boolean => Self::Boolean,
            SyntaxHighlightKind::Operator => Self::Operator,
            SyntaxHighlightKind::Punctuation => Self::Punctuation,
            SyntaxHighlightKind::Error => Self::Error,
        }
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
    use super::{
        compile_source, create_highlight_session_response, highlight_source_for_json,
        reset_highlight_session_for_json, update_highlight_session_for_json, ApiHighlightKind,
        ApiTextRange, CompileOptions, HighlightOptions, HighlightSessionHandle,
        HighlightSessionOptions, HighlightSessionResetOptions, HighlightSessionUpdateRequest,
    };

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

    #[test]
    fn highlights_source_to_json_contract() {
        let response = highlight_source_for_json(
            "\
module demo

fn main() -> bool {
  return true
}
",
            HighlightOptions {
                source_path: Some("highlight.mao".to_owned()),
            },
        );

        assert!(response.ok, "{:#?}", response.diagnostics);
        assert!(response.diagnostics.is_empty());
        assert!(response
            .tokens
            .iter()
            .any(|token| token.kind == ApiHighlightKind::Keyword));
        assert!(response
            .tokens
            .iter()
            .any(|token| token.kind == ApiHighlightKind::Boolean));
    }

    #[test]
    fn reports_highlight_lexer_diagnostics_without_compile_outputs() {
        let response = highlight_source_for_json("let x = @", HighlightOptions::default());

        assert!(!response.ok);
        assert!(response
            .tokens
            .iter()
            .any(|token| token.kind == ApiHighlightKind::Error));
        assert_eq!(response.diagnostics[0].severity, "error");
        assert_eq!(response.diagnostics[0].code, "MD0101");
    }

    #[test]
    fn creates_updates_and_resets_highlight_session_contract() {
        let create = create_highlight_session_response(
            "let x = 1\n",
            HighlightSessionOptions {
                source_path: Some("session.mao".to_owned()),
                editor_version: Some(7),
            },
        );

        assert!(create.ok, "{:#?}", create.diagnostics);
        assert_eq!(create.editor_version, 7);
        assert_eq!(create.session_version, 0);
        assert_eq!(create.changed_range, ApiTextRange { start: 0, end: 10 });
        assert!(create.full_rehighlight);
        assert!(create.session_handle.is_some());

        let mut handle = unsafe {
            Box::from_raw(
                create.session_handle.expect("session handle") as *mut HighlightSessionHandle
            )
        };
        let update = update_highlight_session_for_json(
            &mut handle,
            HighlightSessionUpdateRequest {
                editor_version: 8,
                session_version: 0,
                range: ApiTextRange { start: 8, end: 9 },
                replacement: "2".to_owned(),
            },
        );

        assert!(update.ok, "{:#?}", update.diagnostics);
        assert_eq!(update.editor_version, 8);
        assert_eq!(update.session_version, 1);
        assert!(!update.full_rehighlight);
        assert!(update
            .tokens
            .iter()
            .any(|token| token.kind == ApiHighlightKind::Number));

        let reset = reset_highlight_session_for_json(
            &mut handle,
            "let y = @\n",
            HighlightSessionResetOptions {
                source_path: Some("reset.mao".to_owned()),
                editor_version: Some(9),
            },
        );

        assert!(!reset.ok);
        assert_eq!(reset.editor_version, 9);
        assert_eq!(reset.session_version, 2);
        assert!(reset.full_rehighlight);
        assert_eq!(reset.diagnostics[0].code, "MD0101");
    }

    #[test]
    fn rejects_stale_highlight_session_update_versions() {
        let create = create_highlight_session_response(
            "let x = 1\n",
            HighlightSessionOptions {
                source_path: None,
                editor_version: Some(1),
            },
        );
        let mut handle = unsafe {
            Box::from_raw(
                create.session_handle.expect("session handle") as *mut HighlightSessionHandle
            )
        };
        let first = update_highlight_session_for_json(
            &mut handle,
            HighlightSessionUpdateRequest {
                editor_version: 2,
                session_version: 0,
                range: ApiTextRange { start: 8, end: 9 },
                replacement: "2".to_owned(),
            },
        );
        let stale = update_highlight_session_for_json(
            &mut handle,
            HighlightSessionUpdateRequest {
                editor_version: 3,
                session_version: 0,
                range: ApiTextRange { start: 8, end: 9 },
                replacement: "3".to_owned(),
            },
        );

        assert!(first.ok);
        assert!(!stale.ok);
        assert_eq!(stale.session_version, 1);
        assert_eq!(stale.diagnostics[0].code, "MD9000");
        assert!(stale.diagnostics[0].message.contains("version mismatch"));
    }
}
