//! Shared source, span, and diagnostic types for the Maodie compiler.

mod diagnostic;
mod source;

pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticCodeError, DiagnosticSeverity, DiagnosticSpan,
};
pub use source::{SourceFile, SourceId, TextPosition, TextRange};
