//! Public facade for the Maodie Rust compiler core.
//!
//! Language stages and WASM APIs are introduced by later tasks.

/// Core standard library source and runtime boundary contract.
pub mod core;
/// High-level intermediate representation after name resolution.
pub mod hir;
mod log_format;
/// Mid-level intermediate representation for backend lowering.
pub mod mir;
/// Name resolution from syntax AST to HIR.
pub mod resolver;
/// Static type checking for resolved HIR.
pub mod typeck;
/// MIR to WAT/WASM v1 backend.
pub mod wasm;

/// Shared source, span, and diagnostic model.
pub mod diagnostics {
    pub use maodie_diagnostics::*;
}

/// Source-level syntax utilities, including the lexer.
pub mod syntax {
    pub use maodie_syntax::*;
}

/// Stable metadata for the compiler facade crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompilerFacade {
    crate_name: &'static str,
    version: &'static str,
}

impl CompilerFacade {
    /// Creates a facade describing the current Rust compiler crate.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            crate_name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Returns the Rust crate name that owns the compiler facade.
    #[must_use]
    pub const fn crate_name(self) -> &'static str {
        self.crate_name
    }

    /// Returns the Cargo package version for the facade crate.
    #[must_use]
    pub const fn version(self) -> &'static str {
        self.version
    }
}

impl Default for CompilerFacade {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostics::DiagnosticSeverity, resolver::Resolver, syntax::TokenKind, CompilerFacade,
    };

    #[test]
    fn exposes_workspace_facade_metadata() {
        let facade = CompilerFacade::new();

        assert_eq!(facade.crate_name(), "maodie_compiler");
        assert_eq!(facade.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn re_exports_diagnostic_model() {
        assert_eq!(DiagnosticSeverity::Error.as_str(), "error");
    }

    #[test]
    fn re_exports_syntax_model() {
        assert_eq!(TokenKind::Eof, TokenKind::Eof);
    }

    #[test]
    fn exposes_resolver_api() {
        let resolver = Resolver::new();
        assert_eq!(resolver, Resolver::new());
    }
}
