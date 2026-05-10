use maodie_diagnostics::TextRange;

/// Common AST node behavior.
pub trait AstNode {
    /// Returns the byte range covered by this AST node.
    fn span(&self) -> TextRange;
}

/// Parsed Maodie source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstFile {
    /// Full file span.
    pub span: TextRange,
    /// Optional module declaration.
    pub module: Option<ModuleDecl>,
    /// Import declarations in source order.
    pub imports: Vec<ImportDecl>,
    /// Item declarations in source order.
    pub items: Vec<Item>,
}

impl AstFile {
    /// Renders a stable debug dump for snapshot tests and later tasks.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut dumper = AstDumper::default();
        dumper.line(format!("File @{}..{}", self.span.start, self.span.end));

        if let Some(module) = &self.module {
            module.dump(&mut dumper, 1);
        }

        for import in &self.imports {
            import.dump(&mut dumper, 1);
        }

        for item in &self.items {
            item.dump(&mut dumper, 1);
        }

        dumper.finish()
    }
}

impl AstNode for AstFile {
    fn span(&self) -> TextRange {
        self.span
    }
}

/// `module` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDecl {
    /// Module path.
    pub path: Vec<String>,
    /// Declaration span.
    pub span: TextRange,
}

/// `import` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDecl {
    /// Imported path.
    pub path: Vec<String>,
    /// Declaration span.
    pub span: TextRange,
}

/// Top-level item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    /// Function declaration.
    Function(FunctionDecl),
    /// Struct declaration.
    Struct(StructDecl),
    /// Enum declaration.
    Enum(EnumDecl),
    /// Trait declaration.
    Trait(TraitDecl),
    /// Impl declaration.
    Impl(ImplDecl),
}

/// Function declaration or method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionDecl {
    /// Function name.
    pub name: Option<String>,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Parameter list.
    pub params: ParamList,
    /// Optional return type.
    pub return_type: Option<TypeRef>,
    /// Optional function body.
    pub body: Option<BlockExpr>,
    /// Function span.
    pub span: TextRange,
}

/// Function parameter list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamList {
    /// Parameters in source order.
    pub params: Vec<FunctionParam>,
    /// Parameter list span.
    pub span: TextRange,
}

/// Function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParam {
    /// Parameter name.
    pub name: Option<String>,
    /// Parameter type.
    pub ty: Option<TypeRef>,
    /// Parameter span.
    pub span: TextRange,
}

/// Struct declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDecl {
    /// Struct name.
    pub name: Option<String>,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Field declarations.
    pub fields: Vec<FieldDecl>,
    /// Struct span.
    pub span: TextRange,
}

/// Struct field declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDecl {
    /// Field name.
    pub name: Option<String>,
    /// Field type.
    pub ty: Option<TypeRef>,
    /// Field span.
    pub span: TextRange,
}

/// Enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDecl {
    /// Enum name.
    pub name: Option<String>,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Variants in source order.
    pub variants: Vec<EnumVariant>,
    /// Enum span.
    pub span: TextRange,
}

/// Enum variant declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    /// Variant name.
    pub name: Option<String>,
    /// Variant payload types.
    pub fields: Vec<TypeRef>,
    /// Variant span.
    pub span: TextRange,
}

/// Trait declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitDecl {
    /// Trait name.
    pub name: Option<String>,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Function signatures in source order.
    pub functions: Vec<FunctionDecl>,
    /// Trait span.
    pub span: TextRange,
}

/// Impl declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplDecl {
    /// Optional trait path in `impl Trait for Type`.
    pub trait_path: Option<TypeRef>,
    /// Impl target type.
    pub target: Option<TypeRef>,
    /// Methods in source order.
    pub methods: Vec<FunctionDecl>,
    /// Impl span.
    pub span: TextRange,
}

/// Statement inside a block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    /// `let` statement.
    Let(LetStmt),
    /// `return` statement.
    Return {
        /// Optional returned expression.
        expr: Option<Expr>,
        /// Statement span.
        span: TextRange,
    },
    /// Expression statement.
    Expr(Expr),
}

/// `let` statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LetStmt {
    /// Whether `mut` appears.
    pub mutable: bool,
    /// Binding name.
    pub name: Option<String>,
    /// Optional type annotation.
    pub ty: Option<TypeRef>,
    /// Optional initializer.
    pub value: Option<Expr>,
    /// Statement span.
    pub span: TextRange,
}

/// Expression node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// Missing expression placeholder after a parse error.
    Missing { span: TextRange },
    /// Literal expression.
    Literal { literal: Literal, span: TextRange },
    /// Path expression.
    Path { path: Vec<String>, span: TextRange },
    /// Function call.
    Call {
        /// Callee expression.
        callee: Box<Expr>,
        /// Argument expressions.
        args: Vec<Expr>,
        /// Expression span.
        span: TextRange,
    },
    /// Block expression.
    Block(BlockExpr),
    /// `if` expression.
    If {
        /// Condition expression.
        condition: Box<Expr>,
        /// Then block.
        then_block: BlockExpr,
        /// Optional else branch.
        else_branch: Option<Box<Expr>>,
        /// Expression span.
        span: TextRange,
    },
    /// `match` expression.
    Match {
        /// Matched expression.
        scrutinee: Box<Expr>,
        /// Match arms.
        arms: Vec<MatchArm>,
        /// Expression span.
        span: TextRange,
    },
    /// Binary expression.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left expression.
        left: Box<Expr>,
        /// Right expression.
        right: Box<Expr>,
        /// Expression span.
        span: TextRange,
    },
    /// Postfix `?` expression.
    Try {
        /// Inner expression.
        expr: Box<Expr>,
        /// Expression span.
        span: TextRange,
    },
}

/// Block expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockExpr {
    /// Statements in source order.
    pub statements: Vec<Statement>,
    /// Block span.
    pub span: TextRange,
}

/// Match arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArm {
    /// Arm pattern.
    pub pattern: Pattern,
    /// Arm expression.
    pub expr: Expr,
    /// Arm span.
    pub span: TextRange,
}

/// Pattern node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    /// `_`.
    Wildcard { span: TextRange },
    /// Binding name.
    Binding { name: String, span: TextRange },
    /// Literal pattern.
    Literal { literal: Literal, span: TextRange },
    /// Enum variant or path pattern.
    Path { path: Vec<String>, span: TextRange },
}

/// Literal value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Literal {
    /// Integer source text.
    Integer(String),
    /// Bool literal.
    Bool(bool),
    /// String source text, including quotes.
    String(String),
}

/// Binary operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    /// `=`
    Assign,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
}

/// Type reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeRef {
    /// Path segments.
    pub path: Vec<String>,
    /// Generic argument types.
    pub generic_args: Vec<TypeRef>,
    /// Type span.
    pub span: TextRange,
}

impl AstNode for ModuleDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for ImportDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for Item {
    fn span(&self) -> TextRange {
        match self {
            Self::Function(node) => node.span,
            Self::Struct(node) => node.span,
            Self::Enum(node) => node.span,
            Self::Trait(node) => node.span,
            Self::Impl(node) => node.span,
        }
    }
}

impl Expr {
    /// Returns this expression span.
    #[must_use]
    pub fn span(&self) -> TextRange {
        match self {
            Self::Missing { span }
            | Self::Literal { span, .. }
            | Self::Path { span, .. }
            | Self::Call { span, .. }
            | Self::If { span, .. }
            | Self::Match { span, .. }
            | Self::Binary { span, .. }
            | Self::Try { span, .. } => *span,
            Self::Block(block) => block.span,
        }
    }
}

impl Pattern {
    /// Returns this pattern span.
    #[must_use]
    pub fn span(&self) -> TextRange {
        match self {
            Self::Wildcard { span }
            | Self::Binding { span, .. }
            | Self::Literal { span, .. }
            | Self::Path { span, .. } => *span,
        }
    }
}

impl Statement {
    /// Returns this statement span.
    #[must_use]
    pub fn span(&self) -> TextRange {
        match self {
            Self::Let(statement) => statement.span,
            Self::Return { span, .. } => *span,
            Self::Expr(expr) => expr.span(),
        }
    }
}

impl AstNode for FunctionDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for StructDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for EnumDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for TraitDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

impl AstNode for ImplDecl {
    fn span(&self) -> TextRange {
        self.span
    }
}

#[derive(Default)]
struct AstDumper {
    lines: Vec<String>,
}

impl AstDumper {
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

trait Dump {
    fn dump(&self, dumper: &mut AstDumper, indent: usize);
}

impl Dump for ModuleDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Module {} @{}..{}",
                self.path.join("."),
                self.span.start,
                self.span.end
            ),
        );
    }
}

impl Dump for ImportDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Import {} @{}..{}",
                self.path.join("."),
                self.span.start,
                self.span.end
            ),
        );
    }
}

impl Dump for Item {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        match self {
            Self::Function(node) => node.dump(dumper, indent),
            Self::Struct(node) => node.dump(dumper, indent),
            Self::Enum(node) => node.dump(dumper, indent),
            Self::Trait(node) => node.dump(dumper, indent),
            Self::Impl(node) => node.dump(dumper, indent),
        }
    }
}

impl Dump for FunctionDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        let name = self.name.as_deref().unwrap_or("<missing>");
        dumper.indented(
            indent,
            format!("Fn {name} @{}..{}", self.span.start, self.span.end),
        );

        if !self.generics.is_empty() {
            dumper.indented(indent + 1, format!("Generics {}", self.generics.join(", ")));
        }

        for param in &self.params.params {
            let name = param.name.as_deref().unwrap_or("<missing>");
            dumper.indented(
                indent + 1,
                format!("Param {name} @{}..{}", param.span.start, param.span.end),
            );
            if let Some(ty) = &param.ty {
                ty.dump(dumper, indent + 2);
            }
        }

        if let Some(ty) = &self.return_type {
            dumper.indented(indent + 1, "ReturnType");
            ty.dump(dumper, indent + 2);
        }

        if let Some(body) = &self.body {
            body.dump(dumper, indent + 1);
        }
    }
}

impl Dump for StructDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        let name = self.name.as_deref().unwrap_or("<missing>");
        dumper.indented(
            indent,
            format!("Struct {name} @{}..{}", self.span.start, self.span.end),
        );
        if !self.generics.is_empty() {
            dumper.indented(indent + 1, format!("Generics {}", self.generics.join(", ")));
        }
        for field in &self.fields {
            let name = field.name.as_deref().unwrap_or("<missing>");
            dumper.indented(
                indent + 1,
                format!("Field {name} @{}..{}", field.span.start, field.span.end),
            );
            if let Some(ty) = &field.ty {
                ty.dump(dumper, indent + 2);
            }
        }
    }
}

impl Dump for EnumDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        let name = self.name.as_deref().unwrap_or("<missing>");
        dumper.indented(
            indent,
            format!("Enum {name} @{}..{}", self.span.start, self.span.end),
        );
        if !self.generics.is_empty() {
            dumper.indented(indent + 1, format!("Generics {}", self.generics.join(", ")));
        }
        for variant in &self.variants {
            let name = variant.name.as_deref().unwrap_or("<missing>");
            dumper.indented(
                indent + 1,
                format!(
                    "Variant {name} @{}..{}",
                    variant.span.start, variant.span.end
                ),
            );
            for field in &variant.fields {
                field.dump(dumper, indent + 2);
            }
        }
    }
}

impl Dump for TraitDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        let name = self.name.as_deref().unwrap_or("<missing>");
        dumper.indented(
            indent,
            format!("Trait {name} @{}..{}", self.span.start, self.span.end),
        );
        if !self.generics.is_empty() {
            dumper.indented(indent + 1, format!("Generics {}", self.generics.join(", ")));
        }
        for function in &self.functions {
            function.dump(dumper, indent + 1);
        }
    }
}

impl Dump for ImplDecl {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Impl @{}..{}", self.span.start, self.span.end),
        );
        if let Some(trait_path) = &self.trait_path {
            dumper.indented(indent + 1, "Trait");
            trait_path.dump(dumper, indent + 2);
        }
        if let Some(target) = &self.target {
            dumper.indented(indent + 1, "Target");
            target.dump(dumper, indent + 2);
        }
        for method in &self.methods {
            method.dump(dumper, indent + 1);
        }
    }
}

impl Dump for BlockExpr {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Block @{}..{}", self.span.start, self.span.end),
        );
        for statement in &self.statements {
            statement.dump(dumper, indent + 1);
        }
    }
}

impl Dump for Statement {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
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

impl Dump for LetStmt {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        let name = self.name.as_deref().unwrap_or("<missing>");
        let mutability = if self.mutable { " mut" } else { "" };
        dumper.indented(
            indent,
            format!(
                "Let{mutability} {name} @{}..{}",
                self.span.start, self.span.end
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

impl Dump for Expr {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        match self {
            Self::Missing { span } => {
                dumper.indented(indent, format!("MissingExpr @{}..{}", span.start, span.end));
            }
            Self::Literal { literal, span } => {
                dumper.indented(
                    indent,
                    format!(
                        "Literal {} @{}..{}",
                        literal.dump_text(),
                        span.start,
                        span.end
                    ),
                );
            }
            Self::Path { path, span } => {
                dumper.indented(
                    indent,
                    format!("Path {} @{}..{}", path.join("."), span.start, span.end),
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
                dumper.indented(
                    indent,
                    format!("Binary {} @{}..{}", op.dump_text(), span.start, span.end),
                );
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

impl Dump for MatchArm {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("Arm @{}..{}", self.span.start, self.span.end),
        );
        self.pattern.dump(dumper, indent + 1);
        self.expr.dump(dumper, indent + 1);
    }
}

impl Dump for Pattern {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        match self {
            Self::Wildcard { span } => {
                dumper.indented(indent, format!("Pattern _ @{}..{}", span.start, span.end));
            }
            Self::Binding { name, span } => {
                dumper.indented(
                    indent,
                    format!("Pattern Binding {name} @{}..{}", span.start, span.end),
                );
            }
            Self::Literal { literal, span } => {
                dumper.indented(
                    indent,
                    format!(
                        "Pattern Literal {} @{}..{}",
                        literal.dump_text(),
                        span.start,
                        span.end
                    ),
                );
            }
            Self::Path { path, span } => {
                dumper.indented(
                    indent,
                    format!(
                        "Pattern Path {} @{}..{}",
                        path.join("."),
                        span.start,
                        span.end
                    ),
                );
            }
        }
    }
}

impl Dump for TypeRef {
    fn dump(&self, dumper: &mut AstDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Type {} @{}..{}",
                self.display(),
                self.span.start,
                self.span.end
            ),
        );
    }
}

impl TypeRef {
    /// Stable display form.
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

impl Literal {
    fn dump_text(&self) -> String {
        match self {
            Self::Integer(value) => format!("int({value})"),
            Self::Bool(value) => format!("bool({value})"),
            Self::String(value) => format!("string({value})"),
        }
    }
}

impl BinaryOp {
    fn dump_text(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Less => "<",
            Self::Greater => ">",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
        }
    }
}
