//! High-level intermediate representation produced after name resolution.

use std::fmt;

use maodie_diagnostics::{SourceId, TextRange};

/// Stable module identifier inside one resolver result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(usize);

impl ModuleId {
    /// Creates a module id from a stable numeric index.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "m{}", self.0)
    }
}

/// Stable item identifier inside one resolver result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemId(usize);

impl ItemId {
    /// Creates an item id from a stable numeric index.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "i{}", self.0)
    }
}

/// Stable symbol identifier inside one resolver result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SymbolId(usize);

impl SymbolId {
    /// Creates a symbol id from a stable numeric index.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "s{}", self.0)
    }
}

/// Stable local binding identifier inside one function-like body.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalId(usize);

impl LocalId {
    /// Creates a local id from a stable numeric index.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for LocalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "l{}", self.0)
    }
}

/// Resolved HIR package containing all input modules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirPackage {
    /// Modules in resolver input order.
    pub modules: Vec<HirModule>,
    /// Symbols in deterministic allocation order.
    pub symbols: Vec<Symbol>,
}

impl HirPackage {
    /// Renders a stable dump for snapshot-style tests and handoff docs.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut dumper = HirDumper::default();
        dumper.line("Package");
        dumper.indented(1, "Symbols");
        for symbol in &self.symbols {
            dumper.indented(
                2,
                format!(
                    "{} {} {} kind={}",
                    symbol.id,
                    symbol.path.join("."),
                    symbol.owner,
                    symbol.kind.as_str()
                ),
            );
        }

        for module in &self.modules {
            module.dump(&mut dumper, 1);
        }

        dumper.finish()
    }
}

/// Resolved source module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirModule {
    /// Stable module id.
    pub id: ModuleId,
    /// Source file id.
    pub source_id: SourceId,
    /// Declared module path, or a generated root path for files without `module`.
    pub path: Vec<String>,
    /// Import declarations in source order.
    pub imports: Vec<HirImport>,
    /// Top-level items in source order.
    pub items: Vec<HirItem>,
    /// Source span.
    pub span: TextRange,
}

impl HirModule {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Module {} {} source={} @{}..{}",
                self.id,
                self.path.join("."),
                self.source_id.get(),
                self.span.start,
                self.span.end
            ),
        );

        for import in &self.imports {
            import.dump(dumper, indent + 1);
        }

        for item in &self.items {
            item.dump(dumper, indent + 1);
        }
    }
}

/// Resolved import declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirImport {
    /// Imported path segments.
    pub path: Vec<String>,
    /// Resolved target symbol, if the import was valid.
    pub resolved: Option<SymbolId>,
    /// Import span.
    pub span: TextRange,
}

impl HirImport {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Import {} -> {} @{}..{}",
                self.path.join("."),
                format_resolved_symbol(self.resolved),
                self.span.start,
                self.span.end
            ),
        );
    }
}

/// Resolved top-level item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirItem {
    /// Stable item id.
    pub id: ItemId,
    /// Stable item symbol.
    pub symbol: SymbolId,
    /// Item contents.
    pub kind: HirItemKind,
    /// Source span.
    pub span: TextRange,
}

impl HirItem {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        match &self.kind {
            HirItemKind::Function(function) => {
                dumper.indented(
                    indent,
                    format!(
                        "Item {} {} Fn {} @{}..{}",
                        self.id, self.symbol, function.name, self.span.start, self.span.end
                    ),
                );
                function.dump_contents(dumper, indent + 1);
            }
            HirItemKind::Struct(struct_) => {
                dumper.indented(
                    indent,
                    format!(
                        "Item {} {} Struct {} @{}..{}",
                        self.id, self.symbol, struct_.name, self.span.start, self.span.end
                    ),
                );
                if !struct_.generics.is_empty() {
                    dumper.indented(
                        indent + 1,
                        format!("Generics {}", struct_.generics.join(", ")),
                    );
                }
                for field in &struct_.fields {
                    field.dump(dumper, indent + 1);
                }
            }
            HirItemKind::Enum(enum_) => {
                dumper.indented(
                    indent,
                    format!(
                        "Item {} {} Enum {} @{}..{}",
                        self.id, self.symbol, enum_.name, self.span.start, self.span.end
                    ),
                );
                if !enum_.generics.is_empty() {
                    dumper.indented(
                        indent + 1,
                        format!("Generics {}", enum_.generics.join(", ")),
                    );
                }
                for variant in &enum_.variants {
                    variant.dump(dumper, indent + 1);
                }
            }
            HirItemKind::Trait(trait_) => {
                dumper.indented(
                    indent,
                    format!(
                        "Item {} {} Trait {} @{}..{}",
                        self.id, self.symbol, trait_.name, self.span.start, self.span.end
                    ),
                );
                if !trait_.generics.is_empty() {
                    dumper.indented(
                        indent + 1,
                        format!("Generics {}", trait_.generics.join(", ")),
                    );
                }
                for function in &trait_.functions {
                    function.dump(dumper, indent + 1);
                }
            }
            HirItemKind::Impl(impl_) => {
                dumper.indented(
                    indent,
                    format!(
                        "Item {} {} Impl @{}..{}",
                        self.id, self.symbol, self.span.start, self.span.end
                    ),
                );
                impl_.dump(dumper, indent + 1);
            }
        }
    }
}

/// Top-level item contents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HirItemKind {
    /// Function declaration.
    Function(HirFunction),
    /// Struct declaration.
    Struct(HirStruct),
    /// Enum declaration.
    Enum(HirEnum),
    /// Trait declaration.
    Trait(HirTrait),
    /// Impl block.
    Impl(HirImpl),
}

/// Resolved symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    /// Symbol id.
    pub id: SymbolId,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Fully qualified path.
    pub path: Vec<String>,
    /// Owning module.
    pub owner: ModuleId,
    /// Optional owning item for top-level items and child symbols.
    pub item: Option<ItemId>,
    /// Declaration span.
    pub span: TextRange,
}

/// Symbol kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    /// Module symbol.
    Module,
    /// Function symbol.
    Function,
    /// Struct symbol.
    Struct,
    /// Enum symbol.
    Enum,
    /// Enum variant symbol.
    Variant,
    /// Trait symbol.
    Trait,
    /// Impl block symbol.
    Impl,
}

impl SymbolKind {
    /// Returns the stable dump string for this symbol kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Variant => "variant",
            Self::Trait => "trait",
            Self::Impl => "impl",
        }
    }
}

/// Built-in type known before package resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    /// `Int`.
    Int,
    /// `i32`.
    I32,
    /// `Bool`.
    Bool,
    /// `bool`.
    BoolLower,
    /// `String`.
    String,
    /// `unit`.
    Unit,
}

impl BuiltinType {
    /// Resolves a built-in type by name.
    #[must_use]
    pub const fn from_name(name: &str) -> Option<Self> {
        match name.as_bytes() {
            b"Int" => Some(Self::Int),
            b"i32" => Some(Self::I32),
            b"Bool" => Some(Self::Bool),
            b"bool" => Some(Self::BoolLower),
            b"String" => Some(Self::String),
            b"unit" => Some(Self::Unit),
            _ => None,
        }
    }

    /// Returns the stable built-in type name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::I32 => "i32",
            Self::Bool => "Bool",
            Self::BoolLower => "bool",
            Self::String => "String",
            Self::Unit => "unit",
        }
    }
}

/// A resolved path target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedPath {
    /// Package symbol target.
    Symbol(SymbolId),
    /// Function-local binding target.
    Local(LocalId),
    /// Function generic parameter target.
    Generic(String),
    /// Built-in type target.
    Builtin(BuiltinType),
}

impl ResolvedPath {
    fn dump_text(&self) -> String {
        match self {
            Self::Symbol(symbol) => symbol.to_string(),
            Self::Local(local) => local.to_string(),
            Self::Generic(name) => format!("generic:{name}"),
            Self::Builtin(builtin) => format!("builtin:{}", builtin.as_str()),
        }
    }
}

/// Resolved type reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirTypeRef {
    /// Source path segments.
    pub path: Vec<String>,
    /// Resolved path target, when resolution succeeded.
    pub resolved: Option<ResolvedPath>,
    /// Generic argument types.
    pub generic_args: Vec<HirTypeRef>,
    /// Source span.
    pub span: TextRange,
}

impl HirTypeRef {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Type {} -> {} @{}..{}",
                self.display(),
                format_resolved_path(self.resolved.as_ref()),
                self.span.start,
                self.span.end
            ),
        );
    }

    /// Stable source-like type display.
    #[must_use]
    pub fn display(&self) -> String {
        let mut output = self.path.join(".");
        if !self.generic_args.is_empty() {
            let args = self
                .generic_args
                .iter()
                .map(Self::display)
                .collect::<Vec<_>>()
                .join(", ");
            output.push('<');
            output.push_str(&args);
            output.push('>');
        }
        output
    }
}

/// Resolved function declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirFunction {
    /// Function name.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Parameters in source order.
    pub params: Vec<HirParam>,
    /// Optional return type.
    pub return_type: Option<HirTypeRef>,
    /// Optional body.
    pub body: Option<HirBlock>,
    /// Locals allocated for parameters and let-bindings.
    pub locals: Vec<HirLocal>,
    /// Source span.
    pub span: TextRange,
}

impl HirFunction {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Fn {} @{}..{}", self.name, self.span.start, self.span.end),
        );
        self.dump_contents(dumper, indent + 1);
    }

    fn dump_contents(&self, dumper: &mut HirDumper, indent: usize) {
        if !self.generics.is_empty() {
            dumper.indented(indent, format!("Generics {}", self.generics.join(", ")));
        }
        if !self.locals.is_empty() {
            dumper.indented(indent, "Locals");
            for local in &self.locals {
                dumper.indented(
                    indent + 1,
                    format!(
                        "{} {} kind={} @{}..{}",
                        local.id,
                        local.name,
                        local.kind.as_str(),
                        local.span.start,
                        local.span.end
                    ),
                );
            }
        }
        for param in &self.params {
            param.dump(dumper, indent);
        }
        if let Some(return_type) = &self.return_type {
            dumper.indented(indent, "ReturnType");
            return_type.dump(dumper, indent + 1);
        }
        if let Some(body) = &self.body {
            body.dump(dumper, indent);
        }
    }
}

/// Function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirParam {
    /// Parameter local id.
    pub local: LocalId,
    /// Parameter name.
    pub name: String,
    /// Optional parameter type.
    pub ty: Option<HirTypeRef>,
    /// Source span.
    pub span: TextRange,
}

impl HirParam {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Param {} {} @{}..{}",
                self.local, self.name, self.span.start, self.span.end
            ),
        );
        if let Some(ty) = &self.ty {
            ty.dump(dumper, indent + 1);
        }
    }
}

/// Resolved function-local binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirLocal {
    /// Stable local id.
    pub id: LocalId,
    /// Binding name.
    pub name: String,
    /// Binding kind.
    pub kind: LocalKind,
    /// Source span.
    pub span: TextRange,
}

/// Local binding kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalKind {
    /// Function parameter.
    Param,
    /// `let` binding.
    Let,
    /// Pattern binding.
    Pattern,
}

impl LocalKind {
    /// Returns the stable dump string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Param => "param",
            Self::Let => "let",
            Self::Pattern => "pattern",
        }
    }
}

/// Struct declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirStruct {
    /// Struct name.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Fields in source order.
    pub fields: Vec<HirField>,
}

/// Struct field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirField {
    /// Field name.
    pub name: String,
    /// Optional field type.
    pub ty: Option<HirTypeRef>,
    /// Source span.
    pub span: TextRange,
}

impl HirField {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Field {} @{}..{}",
                self.name, self.span.start, self.span.end
            ),
        );
        if let Some(ty) = &self.ty {
            ty.dump(dumper, indent + 1);
        }
    }
}

/// Enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirEnum {
    /// Enum name.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Variants in source order.
    pub variants: Vec<HirVariant>,
}

/// Enum variant declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirVariant {
    /// Variant symbol.
    pub symbol: SymbolId,
    /// Variant name.
    pub name: String,
    /// Payload types.
    pub fields: Vec<HirTypeRef>,
    /// Source span.
    pub span: TextRange,
}

impl HirVariant {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Variant {} {} @{}..{}",
                self.symbol, self.name, self.span.start, self.span.end
            ),
        );
        for field in &self.fields {
            field.dump(dumper, indent + 1);
        }
    }
}

/// Trait declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirTrait {
    /// Trait name.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Function signatures in source order.
    pub functions: Vec<HirFunction>,
}

/// Impl block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirImpl {
    /// Optional trait path for `impl Trait for Type`.
    pub trait_path: Option<HirTypeRef>,
    /// Impl target type.
    pub target: Option<HirTypeRef>,
    /// Methods in source order.
    pub methods: Vec<HirFunction>,
}

impl HirImpl {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        if let Some(trait_path) = &self.trait_path {
            dumper.indented(indent, "Trait");
            trait_path.dump(dumper, indent + 1);
        }
        if let Some(target) = &self.target {
            dumper.indented(indent, "Target");
            target.dump(dumper, indent + 1);
        }
        for method in &self.methods {
            method.dump(dumper, indent);
        }
    }
}

/// Resolved block expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirBlock {
    /// Statements in source order.
    pub statements: Vec<HirStatement>,
    /// Source span.
    pub span: TextRange,
}

impl HirBlock {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Block @{}..{}", self.span.start, self.span.end),
        );
        for statement in &self.statements {
            statement.dump(dumper, indent + 1);
        }
    }
}

/// Resolved statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HirStatement {
    /// `let` statement.
    Let(HirLet),
    /// `return` statement.
    Return {
        /// Optional returned expression.
        expr: Option<HirExpr>,
        /// Source span.
        span: TextRange,
    },
    /// Expression statement.
    Expr(HirExpr),
}

impl HirStatement {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        match self {
            Self::Let(statement) => statement.dump(dumper, indent),
            Self::Return { expr, span } => {
                dumper.indented(indent, format!("Return @{}..{}", span.start, span.end));
                if let Some(expr) = expr {
                    expr.dump(dumper, indent + 1);
                }
            }
            Self::Expr(expr) => expr.dump(dumper, indent),
        }
    }
}

/// Resolved `let` statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirLet {
    /// Whether `mut` appears.
    pub mutable: bool,
    /// Let local, when a binding name was present.
    pub local: Option<LocalId>,
    /// Binding name.
    pub name: Option<String>,
    /// Optional type annotation.
    pub ty: Option<HirTypeRef>,
    /// Optional initializer.
    pub value: Option<HirExpr>,
    /// Source span.
    pub span: TextRange,
}

impl HirLet {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        let mutability = if self.mutable { " mut" } else { "" };
        dumper.indented(
            indent,
            format!(
                "Let{mutability} {} {} @{}..{}",
                self.local
                    .map_or_else(|| "_".to_owned(), |local| local.to_string()),
                self.name.as_deref().unwrap_or("<missing>"),
                self.span.start,
                self.span.end
            ),
        );
        if let Some(ty) = &self.ty {
            dumper.indented(indent + 1, "Type");
            ty.dump(dumper, indent + 2);
        }
        if let Some(value) = &self.value {
            dumper.indented(indent + 1, "Value");
            value.dump(dumper, indent + 2);
        }
    }
}

/// Resolved expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HirExpr {
    /// Missing expression placeholder after parse recovery.
    Missing { span: TextRange },
    /// Literal expression.
    Literal { text: String, span: TextRange },
    /// Path expression.
    Path {
        /// Source path segments.
        path: Vec<String>,
        /// Resolved target.
        resolved: Option<ResolvedPath>,
        /// Source span.
        span: TextRange,
    },
    /// Function call expression.
    Call {
        /// Callee expression.
        callee: Box<HirExpr>,
        /// Argument expressions.
        args: Vec<HirExpr>,
        /// Source span.
        span: TextRange,
    },
    /// Block expression.
    Block(HirBlock),
    /// `if` expression.
    If {
        /// Condition expression.
        condition: Box<HirExpr>,
        /// Then block.
        then_block: HirBlock,
        /// Optional else branch.
        else_branch: Option<Box<HirExpr>>,
        /// Source span.
        span: TextRange,
    },
    /// `match` expression.
    Match {
        /// Matched expression.
        scrutinee: Box<HirExpr>,
        /// Match arms.
        arms: Vec<HirMatchArm>,
        /// Source span.
        span: TextRange,
    },
    /// Binary expression.
    Binary {
        /// Operator text.
        op: &'static str,
        /// Left expression.
        left: Box<HirExpr>,
        /// Right expression.
        right: Box<HirExpr>,
        /// Source span.
        span: TextRange,
    },
    /// Postfix `?` expression.
    Try {
        /// Inner expression.
        expr: Box<HirExpr>,
        /// Source span.
        span: TextRange,
    },
}

impl HirExpr {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        match self {
            Self::Missing { span } => {
                dumper.indented(indent, format!("MissingExpr @{}..{}", span.start, span.end));
            }
            Self::Literal { text, span } => {
                dumper.indented(
                    indent,
                    format!("Literal {text} @{}..{}", span.start, span.end),
                );
            }
            Self::Path {
                path,
                resolved,
                span,
            } => {
                dumper.indented(
                    indent,
                    format!(
                        "Path {} -> {} @{}..{}",
                        path.join("."),
                        format_resolved_path(resolved.as_ref()),
                        span.start,
                        span.end
                    ),
                );
            }
            Self::Call { callee, args, span } => {
                dumper.indented(indent, format!("Call @{}..{}", span.start, span.end));
                dumper.indented(indent + 1, "Callee");
                callee.dump(dumper, indent + 2);
                for arg in args {
                    dumper.indented(indent + 1, "Arg");
                    arg.dump(dumper, indent + 2);
                }
            }
            Self::Block(block) => block.dump(dumper, indent),
            Self::If {
                condition,
                then_block,
                else_branch,
                span,
            } => {
                dumper.indented(indent, format!("If @{}..{}", span.start, span.end));
                dumper.indented(indent + 1, "Condition");
                condition.dump(dumper, indent + 2);
                dumper.indented(indent + 1, "Then");
                then_block.dump(dumper, indent + 2);
                if let Some(else_branch) = else_branch {
                    dumper.indented(indent + 1, "Else");
                    else_branch.dump(dumper, indent + 2);
                }
            }
            Self::Match {
                scrutinee,
                arms,
                span,
            } => {
                dumper.indented(indent, format!("Match @{}..{}", span.start, span.end));
                dumper.indented(indent + 1, "Scrutinee");
                scrutinee.dump(dumper, indent + 2);
                for arm in arms {
                    arm.dump(dumper, indent + 1);
                }
            }
            Self::Binary {
                op,
                left,
                right,
                span,
            } => {
                dumper.indented(indent, format!("Binary {op} @{}..{}", span.start, span.end));
                left.dump(dumper, indent + 1);
                right.dump(dumper, indent + 1);
            }
            Self::Try { expr, span } => {
                dumper.indented(indent, format!("Try @{}..{}", span.start, span.end));
                expr.dump(dumper, indent + 1);
            }
        }
    }
}

/// Resolved match arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirMatchArm {
    /// Arm pattern.
    pub pattern: HirPattern,
    /// Arm expression.
    pub expr: HirExpr,
    /// Source span.
    pub span: TextRange,
}

impl HirMatchArm {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Arm @{}..{}", self.span.start, self.span.end),
        );
        self.pattern.dump(dumper, indent + 1);
        self.expr.dump(dumper, indent + 1);
    }
}

/// Resolved pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HirPattern {
    /// `_`.
    Wildcard { span: TextRange },
    /// Binding pattern.
    Binding {
        /// Binding local id.
        local: LocalId,
        /// Binding name.
        name: String,
        /// Source span.
        span: TextRange,
    },
    /// Literal pattern.
    Literal { text: String, span: TextRange },
    /// Path pattern.
    Path {
        /// Source path segments.
        path: Vec<String>,
        /// Resolved target.
        resolved: Option<ResolvedPath>,
        /// Source span.
        span: TextRange,
    },
}

impl HirPattern {
    fn dump(&self, dumper: &mut HirDumper, indent: usize) {
        match self {
            Self::Wildcard { span } => {
                dumper.indented(indent, format!("Pattern _ @{}..{}", span.start, span.end));
            }
            Self::Binding { local, name, span } => {
                dumper.indented(
                    indent,
                    format!(
                        "Pattern Binding {local} {name} @{}..{}",
                        span.start, span.end
                    ),
                );
            }
            Self::Literal { text, span } => {
                dumper.indented(
                    indent,
                    format!("Pattern Literal {text} @{}..{}", span.start, span.end),
                );
            }
            Self::Path {
                path,
                resolved,
                span,
            } => {
                dumper.indented(
                    indent,
                    format!(
                        "Pattern Path {} -> {} @{}..{}",
                        path.join("."),
                        format_resolved_path(resolved.as_ref()),
                        span.start,
                        span.end
                    ),
                );
            }
        }
    }
}

fn format_resolved_symbol(resolved: Option<SymbolId>) -> String {
    resolved.map_or_else(|| "<unresolved>".to_owned(), |symbol| symbol.to_string())
}

fn format_resolved_path(resolved: Option<&ResolvedPath>) -> String {
    resolved.map_or_else(|| "<unresolved>".to_owned(), ResolvedPath::dump_text)
}

#[derive(Default)]
struct HirDumper {
    lines: Vec<String>,
}

impl HirDumper {
    fn line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    fn indented(&mut self, indent: usize, line: impl Into<String>) {
        self.line(format!("{}{}", "  ".repeat(indent), line.into()));
    }

    fn finish(self) -> String {
        self.lines.join("\n")
    }
}
