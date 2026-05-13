//! Static type checking for resolved Maodie HIR.

use std::collections::{HashMap, HashSet};
use std::fmt;

use maodie_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticSpan, SourceFile, SourceId, TextRange,
};

use crate::hir::{
    BuiltinType, HirBlock, HirEnum, HirExpr, HirFunction, HirImpl, HirItem, HirItemKind, HirLet,
    HirMatchArm, HirPackage, HirPattern, HirStatement, HirStruct, HirTrait, HirTypeRef, HirVariant,
    ItemId, LocalId, ResolvedPath, SymbolId, SymbolKind,
};
use crate::log_format::parse_log_format;
use crate::resolver::{resolve_source, resolve_sources};

/// Type mismatch diagnostic.
pub const MD_TYPE_MISMATCH: &str = "MD0401";
/// Assignment to immutable binding diagnostic.
pub const MD_IMMUTABLE_ASSIGNMENT: &str = "MD0402";
/// Invalid generic argument count diagnostic.
pub const MD_INVALID_TYPE_ARITY: &str = "MD0403";
/// Calling a non-function value diagnostic.
pub const MD_NOT_CALLABLE: &str = "MD0404";
/// Invalid call argument count diagnostic.
pub const MD_CALL_ARITY: &str = "MD0405";
/// Missing trait method diagnostic.
pub const MD_MISSING_TRAIT_METHOD: &str = "MD0406";
/// Invalid trait impl diagnostic.
pub const MD_INVALID_IMPL: &str = "MD0407";
/// Invalid operator operand diagnostic.
pub const MD_INVALID_OPERATOR: &str = "MD0408";
/// Invalid assignment target diagnostic.
pub const MD_INVALID_ASSIGNMENT_TARGET: &str = "MD0409";
/// Non-exhaustive match diagnostic.
pub const MD_NON_EXHAUSTIVE_MATCH: &str = "MD0410";
/// Invalid pattern diagnostic.
pub const MD_INVALID_PATTERN: &str = "MD0411";
/// Invalid `?` usage diagnostic.
pub const MD_INVALID_TRY: &str = "MD0412";
/// Invalid `core.log` format string diagnostic.
pub const MD_INVALID_LOG_FORMAT: &str = "MD0413";

/// Stable type identifier inside one type-check result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeId(usize);

impl TypeId {
    /// Creates a type id from a stable numeric index.
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

impl fmt::Display for TypeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "t{}", self.0)
    }
}

/// Interned type shape.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TypeKind {
    /// Unknown type after an earlier error.
    Error,
    /// `unit`.
    Unit,
    /// `i32`.
    I32,
    /// `bool`.
    Bool,
    /// `String`.
    String,
    /// Rigid generic parameter inside a declaration.
    Generic(String),
    /// Flexible generic variable for call/variant instantiation.
    Infer(String),
    /// Nominal user type with generic arguments.
    Nominal {
        /// Defining symbol.
        symbol: SymbolId,
        /// Generic argument types.
        args: Vec<TypeId>,
    },
    /// Function-like value.
    Function {
        /// Generic parameter names.
        generics: Vec<String>,
        /// Parameter types.
        params: Vec<TypeId>,
        /// Return type.
        return_type: TypeId,
    },
}

/// Type table entry for a local binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalType {
    /// Function or method owning the local.
    pub owner: String,
    /// Local id inside the owner.
    pub local: LocalId,
    /// Source binding name.
    pub name: String,
    /// Whether assignment is allowed.
    pub mutable: bool,
    /// Inferred or declared type.
    pub ty: TypeId,
}

/// Type table entry for an expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExprType {
    /// Function or method owning the expression.
    pub owner: String,
    /// Expression span.
    pub span: TextRange,
    /// Expression type.
    pub ty: TypeId,
}

/// Type table entry for a top-level item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemType {
    /// Item id.
    pub item: ItemId,
    /// Item symbol.
    pub symbol: SymbolId,
    /// Item type.
    pub ty: TypeId,
}

/// Generic substitution observed while checking a call or variant use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeSubstitution {
    /// Source expression span that caused the substitution.
    pub span: TextRange,
    /// Generic parameter name.
    pub generic: String,
    /// Concrete replacement type.
    pub replacement: TypeId,
}

/// Stable type information produced by the checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeTable {
    /// Interned types in deterministic allocation order.
    pub types: Vec<TypeKind>,
    /// Top-level item types.
    pub items: Vec<ItemType>,
    /// Function-local binding types.
    pub locals: Vec<LocalType>,
    /// Expression types.
    pub expressions: Vec<ExprType>,
    /// Generic substitutions prepared for later instantiation.
    pub substitutions: Vec<TypeSubstitution>,
}

impl TypeTable {
    /// Renders a stable dump for snapshot tests and later compiler stages.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut dumper = TypeDumper::default();
        dumper.line("Types");
        for (index, kind) in self.types.iter().enumerate() {
            dumper.indented(1, format!("t{index} {}", self.display_kind(kind)));
        }

        if !self.items.is_empty() {
            dumper.line("Items");
            for item in &self.items {
                dumper.indented(1, format!("{} {} {}", item.item, item.symbol, item.ty));
            }
        }

        if !self.locals.is_empty() {
            dumper.line("Locals");
            for local in &self.locals {
                let mutability = if local.mutable { " mut" } else { "" };
                dumper.indented(
                    1,
                    format!(
                        "{}{} {} {} owner={}",
                        local.local, mutability, local.name, local.ty, local.owner
                    ),
                );
            }
        }

        if !self.expressions.is_empty() {
            dumper.line("Expressions");
            for expr in &self.expressions {
                dumper.indented(
                    1,
                    format!(
                        "@{}..{} {} owner={}",
                        expr.span.start, expr.span.end, expr.ty, expr.owner
                    ),
                );
            }
        }

        if !self.substitutions.is_empty() {
            dumper.line("Substitutions");
            for substitution in &self.substitutions {
                dumper.indented(
                    1,
                    format!(
                        "@{}..{} {}={}",
                        substitution.span.start,
                        substitution.span.end,
                        substitution.generic,
                        substitution.replacement
                    ),
                );
            }
        }

        dumper.finish()
    }

    fn display_type(&self, ty: TypeId) -> String {
        self.types
            .get(ty.get())
            .map_or_else(|| "<invalid>".to_owned(), |kind| self.display_kind(kind))
    }

    fn display_kind(&self, kind: &TypeKind) -> String {
        match kind {
            TypeKind::Error => "<error>".to_owned(),
            TypeKind::Unit => "unit".to_owned(),
            TypeKind::I32 => "i32".to_owned(),
            TypeKind::Bool => "bool".to_owned(),
            TypeKind::String => "String".to_owned(),
            TypeKind::Generic(name) => name.clone(),
            TypeKind::Infer(name) => format!("?{name}"),
            TypeKind::Nominal { symbol, args } => {
                if args.is_empty() {
                    symbol.to_string()
                } else {
                    let args = args
                        .iter()
                        .map(|arg| self.display_type(*arg))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{symbol}<{args}>")
                }
            }
            TypeKind::Function {
                generics,
                params,
                return_type,
            } => {
                let generics = if generics.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", generics.join(", "))
                };
                let params = params
                    .iter()
                    .map(|param| self.display_type(*param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "fn{generics}({params}) -> {}",
                    self.display_type(*return_type)
                )
            }
        }
    }
}

/// Type checker output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeCheckResult {
    /// HIR package that was checked.
    pub package: HirPackage,
    /// Type table and typed expression/local data.
    pub type_table: TypeTable,
    /// Parser, resolver, and type diagnostics in stable order.
    pub diagnostics: Vec<Diagnostic>,
}

impl TypeCheckResult {
    /// Renders a stable typed dump.
    #[must_use]
    pub fn dump(&self) -> String {
        self.type_table.dump()
    }
}

/// Type checker facade.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeChecker;

impl TypeChecker {
    /// Creates a type checker.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Resolves and type-checks one source file.
    #[must_use]
    pub fn check_source(&self, source: &SourceFile) -> TypeCheckResult {
        let resolved = resolve_source(source);
        let mut result = self.check_package(&resolved.package, &[source]);
        let mut diagnostics = resolved.diagnostics;
        diagnostics.extend(result.diagnostics);
        result.diagnostics = diagnostics;
        result
    }

    /// Resolves and type-checks a set of source files as one package.
    #[must_use]
    pub fn check_sources(&self, sources: &[&SourceFile]) -> TypeCheckResult {
        let resolved = resolve_sources(sources);
        let mut result = self.check_package(&resolved.package, sources);
        let mut diagnostics = resolved.diagnostics;
        diagnostics.extend(result.diagnostics);
        result.diagnostics = diagnostics;
        result
    }

    /// Type-checks an already resolved package.
    #[must_use]
    pub fn check_package(&self, package: &HirPackage, sources: &[&SourceFile]) -> TypeCheckResult {
        TypeSession::new(package, sources).check()
    }
}

/// Resolves and type-checks one source file.
#[must_use]
pub fn check_source(source: &SourceFile) -> TypeCheckResult {
    TypeChecker::new().check_source(source)
}

/// Resolves and type-checks a set of source files as one package.
#[must_use]
pub fn check_sources(sources: &[&SourceFile]) -> TypeCheckResult {
    TypeChecker::new().check_sources(sources)
}

#[derive(Clone, Debug)]
struct FunctionSig {
    name: String,
    generics: Vec<String>,
    params: Vec<TypeId>,
    return_type: TypeId,
    span: TextRange,
}

#[derive(Clone, Debug)]
struct TraitInfo {
    name: String,
    generics: Vec<String>,
    methods: Vec<FunctionSig>,
}

#[derive(Clone, Debug)]
struct VariantInfo {
    enum_symbol: SymbolId,
    enum_generics: Vec<String>,
    name: String,
    fields: Vec<TypeId>,
}

#[derive(Clone, Debug)]
struct BindingInfo {
    name: String,
    mutable: bool,
    ty: TypeId,
}

#[derive(Clone, Debug)]
struct TypeSession<'package, 'source> {
    package: &'package HirPackage,
    sources: HashMap<SourceId, &'source SourceFile>,
    diagnostics: Vec<Diagnostic>,
    types: Vec<TypeKind>,
    type_ids: HashMap<TypeKind, TypeId>,
    items: Vec<ItemType>,
    locals: Vec<LocalType>,
    expressions: Vec<ExprType>,
    substitutions: Vec<TypeSubstitution>,
    item_types: HashMap<SymbolId, TypeId>,
    functions: HashMap<SymbolId, FunctionSig>,
    traits: HashMap<SymbolId, TraitInfo>,
    variants: HashMap<SymbolId, VariantInfo>,
    enum_variants: HashMap<SymbolId, Vec<SymbolId>>,
    generic_counts: HashMap<SymbolId, usize>,
}

impl<'package, 'source> TypeSession<'package, 'source> {
    fn new(package: &'package HirPackage, sources: &[&'source SourceFile]) -> Self {
        let sources = sources
            .iter()
            .map(|source| (source.id(), *source))
            .collect::<HashMap<_, _>>();

        Self {
            package,
            sources,
            diagnostics: Vec::new(),
            types: Vec::new(),
            type_ids: HashMap::new(),
            items: Vec::new(),
            locals: Vec::new(),
            expressions: Vec::new(),
            substitutions: Vec::new(),
            item_types: HashMap::new(),
            functions: HashMap::new(),
            traits: HashMap::new(),
            variants: HashMap::new(),
            enum_variants: HashMap::new(),
            generic_counts: HashMap::new(),
        }
    }

    fn check(mut self) -> TypeCheckResult {
        self.collect_declarations();
        self.check_items();

        TypeCheckResult {
            package: self.package.clone(),
            type_table: TypeTable {
                types: self.types,
                items: self.items,
                locals: self.locals,
                expressions: self.expressions,
                substitutions: self.substitutions,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn collect_declarations(&mut self) {
        for module in &self.package.modules {
            for item in &module.items {
                match &item.kind {
                    HirItemKind::Function(function) => {
                        let sig = self.function_sig(module.source_id, function, &[]);
                        let ty = self.function_type(&sig);
                        self.record_item_type(item, ty);
                        self.functions.insert(item.symbol, sig);
                    }
                    HirItemKind::Struct(struct_) => {
                        self.collect_struct(module.source_id, item, struct_);
                    }
                    HirItemKind::Enum(enum_) => {
                        self.collect_enum(module.source_id, item, enum_);
                    }
                    HirItemKind::Trait(trait_) => {
                        self.collect_trait(module.source_id, item, trait_);
                    }
                    HirItemKind::Impl(_) => {
                        let ty = self.intern(TypeKind::Unit);
                        self.record_item_type(item, ty);
                    }
                }
            }
        }
    }

    fn collect_struct(&mut self, source_id: SourceId, item: &HirItem, struct_: &HirStruct) {
        self.generic_counts
            .insert(item.symbol, struct_.generics.len());
        let args = struct_
            .generics
            .iter()
            .map(|name| self.intern(TypeKind::Generic(name.clone())))
            .collect::<Vec<_>>();
        let ty = self.intern(TypeKind::Nominal {
            symbol: item.symbol,
            args,
        });
        self.record_item_type(item, ty);

        let generics = generic_scope(&struct_.generics, &[]);
        for field in &struct_.fields {
            if let Some(ty) = &field.ty {
                self.resolve_type_ref(source_id, ty, &generics);
            }
        }
    }

    fn collect_enum(&mut self, source_id: SourceId, item: &HirItem, enum_: &HirEnum) {
        self.generic_counts
            .insert(item.symbol, enum_.generics.len());
        let args = enum_
            .generics
            .iter()
            .map(|name| self.intern(TypeKind::Generic(name.clone())))
            .collect::<Vec<_>>();
        let ty = self.intern(TypeKind::Nominal {
            symbol: item.symbol,
            args,
        });
        self.record_item_type(item, ty);

        let generics = generic_scope(&enum_.generics, &[]);
        for variant in &enum_.variants {
            self.collect_variant(source_id, item.symbol, &enum_.generics, variant, &generics);
            self.enum_variants
                .entry(item.symbol)
                .or_default()
                .push(variant.symbol);
        }
    }

    fn collect_variant(
        &mut self,
        source_id: SourceId,
        enum_symbol: SymbolId,
        enum_generics: &[String],
        variant: &HirVariant,
        generics: &HashSet<String>,
    ) {
        let fields = variant
            .fields
            .iter()
            .map(|field| self.resolve_type_ref(source_id, field, generics))
            .collect::<Vec<_>>();
        self.variants.insert(
            variant.symbol,
            VariantInfo {
                enum_symbol,
                enum_generics: enum_generics.to_vec(),
                name: variant.name.clone(),
                fields,
            },
        );
    }

    fn collect_trait(&mut self, source_id: SourceId, item: &HirItem, trait_: &HirTrait) {
        self.generic_counts
            .insert(item.symbol, trait_.generics.len());
        let args = trait_
            .generics
            .iter()
            .map(|name| self.intern(TypeKind::Generic(name.clone())))
            .collect::<Vec<_>>();
        let ty = self.intern(TypeKind::Nominal {
            symbol: item.symbol,
            args,
        });
        self.record_item_type(item, ty);

        let methods = trait_
            .functions
            .iter()
            .map(|function| self.function_sig(source_id, function, &trait_.generics))
            .collect::<Vec<_>>();
        self.traits.insert(
            item.symbol,
            TraitInfo {
                name: trait_.name.clone(),
                generics: trait_.generics.clone(),
                methods,
            },
        );
    }

    fn check_items(&mut self) {
        for module in &self.package.modules {
            for item in &module.items {
                match &item.kind {
                    HirItemKind::Function(function) => {
                        if let Some(sig) = self.functions.get(&item.symbol).cloned() {
                            self.check_function(module.source_id, function, &sig);
                        }
                    }
                    HirItemKind::Impl(impl_) => self.check_impl(module.source_id, impl_),
                    HirItemKind::Struct(_) | HirItemKind::Enum(_) | HirItemKind::Trait(_) => {}
                }
            }
        }
    }

    fn check_impl(&mut self, source_id: SourceId, impl_: &HirImpl) {
        let target_ty = match &impl_.target {
            Some(ty) => self.resolve_type_ref(source_id, ty, &HashSet::new()),
            None => self.intern(TypeKind::Error),
        };

        if let Some(trait_path) = &impl_.trait_path {
            let trait_ty = self.resolve_type_ref(source_id, trait_path, &HashSet::new());
            let Some(trait_symbol) = nominal_symbol(self.kind(trait_ty)) else {
                self.push(
                    source_id,
                    MD_INVALID_IMPL,
                    trait_path.span,
                    "impl 的 trait 路径必须指向 trait 类型",
                    "请确认 `impl Trait for Type` 中的 Trait 是 trait 声明",
                );
                return;
            };
            let Some(trait_info) = self.traits.get(&trait_symbol).cloned() else {
                self.push(
                    source_id,
                    MD_INVALID_IMPL,
                    trait_path.span,
                    "impl 的 trait 路径必须指向 trait 类型",
                    "请确认 `impl Trait for Type` 中的 Trait 是 trait 声明",
                );
                return;
            };
            self.check_trait_impl(source_id, impl_, target_ty, &trait_info);
        }

        for method in &impl_.methods {
            let sig = self.function_sig(source_id, method, &[]);
            self.check_function(source_id, method, &sig);
        }
    }

    fn check_trait_impl(
        &mut self,
        source_id: SourceId,
        impl_: &HirImpl,
        _target_ty: TypeId,
        trait_info: &TraitInfo,
    ) {
        let methods = impl_
            .methods
            .iter()
            .map(|method| (method.name.as_str(), method))
            .collect::<HashMap<_, _>>();

        for required in &trait_info.methods {
            let Some(actual) = methods.get(required.name.as_str()) else {
                let span = impl_.target.as_ref().map_or(
                    impl_
                        .trait_path
                        .as_ref()
                        .map_or(TextRange::at(0), |ty| ty.span),
                    |ty| ty.span,
                );
                self.push(
                    source_id,
                    MD_MISSING_TRAIT_METHOD,
                    span,
                    format!(
                        "impl 缺少 trait `{}` 要求的方法 `{}`",
                        trait_info.name, required.name
                    ),
                    "请在 impl 块中补齐 trait 签名要求的所有方法",
                );
                continue;
            };

            let actual_sig = self.function_sig(source_id, actual, &trait_info.generics);
            self.compare_method_sig(source_id, &actual_sig, required);
        }
    }

    fn compare_method_sig(
        &mut self,
        source_id: SourceId,
        actual: &FunctionSig,
        required: &FunctionSig,
    ) {
        if actual.params.len() != required.params.len() {
            self.push(
                source_id,
                MD_TYPE_MISMATCH,
                actual.span,
                format!("方法 `{}` 的参数数量与 trait 签名不一致", actual.name),
                "impl 方法必须与 trait 方法签名保持一致",
            );
            return;
        }

        for (actual_ty, required_ty) in actual.params.iter().zip(&required.params) {
            self.expect_exact(source_id, *required_ty, *actual_ty, actual.span);
        }
        self.expect_exact(
            source_id,
            required.return_type,
            actual.return_type,
            actual.span,
        );
    }

    fn check_function(&mut self, source_id: SourceId, function: &HirFunction, sig: &FunctionSig) {
        let owner = function.name.clone();
        let mut context = BodyContext {
            source_id,
            owner: owner.clone(),
            return_type: sig.return_type,
            bindings: HashMap::new(),
        };

        for (param, ty) in function.params.iter().zip(&sig.params) {
            context.bindings.insert(
                param.local,
                BindingInfo {
                    name: param.name.clone(),
                    mutable: false,
                    ty: *ty,
                },
            );
            self.locals.push(LocalType {
                owner: owner.clone(),
                local: param.local,
                name: param.name.clone(),
                mutable: false,
                ty: *ty,
            });
        }

        if let Some(body) = &function.body {
            let body_ty = self.check_block(body, &mut context);
            if should_check_body_tail(body) {
                self.expect_assignable(source_id, sig.return_type, body_ty, body.span);
            }
        }
    }

    fn check_block(&mut self, block: &HirBlock, context: &mut BodyContext) -> TypeId {
        let mut result = self.intern(TypeKind::Unit);
        for statement in &block.statements {
            result = match statement {
                HirStatement::Let(statement) => {
                    self.check_let(statement, context);
                    self.intern(TypeKind::Unit)
                }
                HirStatement::Return { expr, span } => {
                    let actual = match expr {
                        Some(expr) => self.check_expr(expr, context),
                        None => self.intern(TypeKind::Unit),
                    };
                    self.expect_assignable(context.source_id, context.return_type, actual, *span);
                    self.intern(TypeKind::Unit)
                }
                HirStatement::Expr(expr) => self.check_expr(expr, context),
            };
        }
        result
    }

    fn check_let(&mut self, statement: &HirLet, context: &mut BodyContext) {
        let annotation = statement
            .ty
            .as_ref()
            .map(|ty| self.resolve_type_ref(context.source_id, ty, &HashSet::new()));
        let value = statement
            .value
            .as_ref()
            .map(|expr| self.check_expr(expr, context));

        if let (Some(expected), Some(actual)) = (annotation, value) {
            self.expect_assignable(context.source_id, expected, actual, statement.span);
        }

        let ty = annotation
            .or(value)
            .unwrap_or_else(|| self.intern(TypeKind::Error));

        if let Some(local) = statement.local {
            let name = statement
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned());
            context.bindings.insert(
                local,
                BindingInfo {
                    name: name.clone(),
                    mutable: statement.mutable,
                    ty,
                },
            );
            self.locals.push(LocalType {
                owner: context.owner.clone(),
                local,
                name,
                mutable: statement.mutable,
                ty,
            });
        }
    }

    fn check_expr(&mut self, expr: &HirExpr, context: &mut BodyContext) -> TypeId {
        let ty = match expr {
            HirExpr::Missing { .. } => self.intern(TypeKind::Error),
            HirExpr::Literal { text, .. } => self.literal_type(text),
            HirExpr::Path { resolved, .. } => self.path_type(resolved.as_ref(), context),
            HirExpr::Call { callee, args, span } => self.check_call(callee, args, *span, context),
            HirExpr::Block(block) => self.check_block(block, context),
            HirExpr::If {
                condition,
                then_block,
                else_branch,
                span,
            } => self.check_if(
                condition,
                then_block,
                else_branch.as_deref(),
                *span,
                context,
            ),
            HirExpr::Match {
                scrutinee,
                arms,
                span,
            } => self.check_match(scrutinee, arms, *span, context),
            HirExpr::Binary {
                op,
                left,
                right,
                span,
            } => self.check_binary(op, left, right, *span, context),
            HirExpr::Try { expr, span } => self.check_try(expr, *span, context),
        };

        self.expressions.push(ExprType {
            owner: context.owner.clone(),
            span: expr_span(expr),
            ty,
        });
        ty
    }

    fn check_if(
        &mut self,
        condition: &HirExpr,
        then_block: &HirBlock,
        else_branch: Option<&HirExpr>,
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        let condition_ty = self.check_expr(condition, context);
        let bool_ty = self.intern(TypeKind::Bool);
        self.expect_assignable(
            context.source_id,
            bool_ty,
            condition_ty,
            expr_span(condition),
        );

        let then_ty = self.check_block(then_block, context);
        if let Some(else_branch) = else_branch {
            let else_ty = self.check_expr(else_branch, context);
            self.expect_assignable(context.source_id, then_ty, else_ty, span);
            then_ty
        } else {
            self.intern(TypeKind::Unit)
        }
    }

    fn check_match(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        let scrutinee_ty = self.check_expr(scrutinee, context);
        let mut arm_ty = None;
        let mut covers_all = false;
        let mut covered_variants = HashSet::new();

        for arm in arms {
            let saved_bindings = context.bindings.clone();
            let coverage = self.check_pattern(&arm.pattern, scrutinee_ty, context);
            match coverage {
                PatternCoverage::All => covers_all = true,
                PatternCoverage::Variant(symbol) => {
                    covered_variants.insert(symbol);
                }
                PatternCoverage::Partial => {}
            }

            let ty = self.check_expr(&arm.expr, context);
            if let Some(expected) = arm_ty {
                self.expect_assignable(context.source_id, expected, ty, arm.span);
            } else {
                arm_ty = Some(ty);
            }
            context.bindings = saved_bindings;
        }

        self.check_match_exhaustive(
            context.source_id,
            scrutinee_ty,
            &covered_variants,
            covers_all,
            span,
        );
        arm_ty.unwrap_or_else(|| self.intern(TypeKind::Unit))
    }

    fn check_pattern(
        &mut self,
        pattern: &HirPattern,
        scrutinee_ty: TypeId,
        context: &mut BodyContext,
    ) -> PatternCoverage {
        match pattern {
            HirPattern::Wildcard { .. } => PatternCoverage::All,
            HirPattern::Binding { local, name, .. } => {
                context.bindings.insert(
                    *local,
                    BindingInfo {
                        name: name.clone(),
                        mutable: false,
                        ty: scrutinee_ty,
                    },
                );
                self.locals.push(LocalType {
                    owner: context.owner.clone(),
                    local: *local,
                    name: name.clone(),
                    mutable: false,
                    ty: scrutinee_ty,
                });
                PatternCoverage::All
            }
            HirPattern::Literal { text, span } => {
                let literal_ty = self.literal_type(text);
                self.expect_assignable(context.source_id, scrutinee_ty, literal_ty, *span);
                PatternCoverage::Partial
            }
            HirPattern::Path { resolved, span, .. } => {
                self.check_variant_pattern(resolved.as_ref(), scrutinee_ty, *span, context)
            }
        }
    }

    fn check_variant_pattern(
        &mut self,
        resolved: Option<&ResolvedPath>,
        scrutinee_ty: TypeId,
        span: TextRange,
        context: &BodyContext,
    ) -> PatternCoverage {
        let Some(ResolvedPath::Symbol(variant_symbol)) = resolved else {
            self.push(
                context.source_id,
                MD_INVALID_PATTERN,
                span,
                "路径模式必须指向 enum 变体",
                "当前版本仅支持 `_`、绑定、字面量和 enum 变体模式",
            );
            return PatternCoverage::Partial;
        };

        let Some(variant) = self.variants.get(variant_symbol).cloned() else {
            self.push(
                context.source_id,
                MD_INVALID_PATTERN,
                span,
                "路径模式必须指向 enum 变体",
                "请使用形如 `Option.Some` 或 `Result.Err` 的变体路径",
            );
            return PatternCoverage::Partial;
        };

        match self.kind(scrutinee_ty).clone() {
            TypeKind::Nominal {
                symbol: scrutinee_symbol,
                ..
            } if scrutinee_symbol == variant.enum_symbol => {
                PatternCoverage::Variant(*variant_symbol)
            }
            _ => {
                let expected =
                    self.enum_instance_type(variant.enum_symbol, &variant.enum_generics, false);
                self.push_mismatch(context.source_id, expected, scrutinee_ty, span);
                PatternCoverage::Variant(*variant_symbol)
            }
        }
    }

    fn check_match_exhaustive(
        &mut self,
        source_id: SourceId,
        scrutinee_ty: TypeId,
        covered_variants: &HashSet<SymbolId>,
        covers_all: bool,
        span: TextRange,
    ) {
        if covers_all || self.is_error(scrutinee_ty) {
            return;
        }

        let TypeKind::Nominal { symbol, .. } = self.kind(scrutinee_ty).clone() else {
            return;
        };
        let Some(variants) = self.enum_variants.get(&symbol).cloned() else {
            return;
        };

        let missing = variants
            .into_iter()
            .filter(|variant| !covered_variants.contains(variant))
            .filter_map(|variant| self.variants.get(&variant).map(|info| info.name.clone()))
            .collect::<Vec<_>>();

        if missing.is_empty() {
            return;
        }

        self.push(
            source_id,
            MD_NON_EXHAUSTIVE_MATCH,
            span,
            format!("match 缺少 enum 变体：{}", missing.join(", ")),
            "请补齐所有 enum 变体分支，或添加 `_` 通配分支",
        );
    }

    fn check_try(&mut self, expr: &HirExpr, span: TextRange, context: &mut BodyContext) -> TypeId {
        let expr_ty = self.check_expr(expr, context);
        let Some((ok_ty, err_ty)) = self.result_parts(expr_ty) else {
            self.push(
                context.source_id,
                MD_INVALID_TRY,
                span,
                "`?` 只能用于 Result 表达式",
                "请确认 `?` 左侧表达式的类型是 `Result<T, E>`",
            );
            return self.intern(TypeKind::Error);
        };

        let Some((_, return_err_ty)) = self.result_parts(context.return_type) else {
            self.push(
                context.source_id,
                MD_INVALID_TRY,
                span,
                "`?` 只能出现在返回 Result 的函数中",
                "请把函数返回类型改为 `Result<T, E>`，或移除 `?`",
            );
            return ok_ty;
        };

        self.expect_assignable(context.source_id, return_err_ty, err_ty, span);
        ok_ty
    }

    fn check_binary(
        &mut self,
        op: &str,
        left: &HirExpr,
        right: &HirExpr,
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        if op == "=" {
            return self.check_assignment(left, right, span, context);
        }

        let left_ty = self.check_expr(left, context);
        let right_ty = self.check_expr(right, context);
        let i32_ty = self.intern(TypeKind::I32);
        self.expect_assignable(context.source_id, i32_ty, left_ty, expr_span(left));
        self.expect_assignable(context.source_id, i32_ty, right_ty, expr_span(right));

        match op {
            "+" | "-" | "*" | "/" => i32_ty,
            "<" | ">" => self.intern(TypeKind::Bool),
            _ => {
                self.push(
                    context.source_id,
                    MD_INVALID_OPERATOR,
                    span,
                    format!("不支持的二元运算符 `{op}`"),
                    "v1 只支持 i32 算术和 i32 比较",
                );
                self.intern(TypeKind::Error)
            }
        }
    }

    fn check_assignment(
        &mut self,
        left: &HirExpr,
        right: &HirExpr,
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        let right_ty = self.check_expr(right, context);
        let HirExpr::Path {
            resolved: Some(ResolvedPath::Local(local)),
            ..
        } = left
        else {
            self.push(
                context.source_id,
                MD_INVALID_ASSIGNMENT_TARGET,
                span,
                "赋值左侧必须是局部变量",
                "只有 `let mut` 声明的局部变量可以被重新赋值",
            );
            return self.intern(TypeKind::Unit);
        };

        let Some(binding) = context.bindings.get(local).cloned() else {
            return self.intern(TypeKind::Unit);
        };

        if !binding.mutable {
            self.push(
                context.source_id,
                MD_IMMUTABLE_ASSIGNMENT,
                span,
                format!("不能给不可变变量 `{}` 赋值", binding.name),
                "请使用 `let mut` 声明可变绑定",
            );
        }
        self.expect_assignable(context.source_id, binding.ty, right_ty, span);
        self.intern(TypeKind::Unit)
    }

    fn check_call(
        &mut self,
        callee: &HirExpr,
        args: &[HirExpr],
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        let callee_ty = self.check_expr(callee, context);
        if self.is_core_log_callee(callee) {
            return self.check_core_log_call(args, span, context);
        }
        let TypeKind::Function {
            generics,
            params,
            return_type,
        } = self.kind(callee_ty).clone()
        else {
            self.push(
                context.source_id,
                MD_NOT_CALLABLE,
                span,
                "只能调用函数或 enum 变体构造器",
                "请确认调用目标是函数名或带载荷的 enum 变体",
            );
            return self.intern(TypeKind::Error);
        };

        if args.len() != params.len() {
            self.push(
                context.source_id,
                MD_CALL_ARITY,
                span,
                format!(
                    "调用需要 {} 个参数，但传入了 {} 个",
                    params.len(),
                    args.len()
                ),
                "请调整实参数量以匹配函数签名",
            );
            return return_type;
        }

        let mut substitutions = HashMap::<String, TypeId>::new();
        let params = params
            .into_iter()
            .map(|param| self.instantiate_flexible(param, &generics))
            .collect::<Vec<_>>();
        let return_type = self.instantiate_flexible(return_type, &generics);

        for (arg, expected) in args.iter().zip(params) {
            let actual = self.check_expr(arg, context);
            self.unify(
                context.source_id,
                expected,
                actual,
                expr_span(arg),
                &mut substitutions,
            );
        }

        for generic in generics {
            if let Some(replacement) = substitutions.get(&generic) {
                self.substitutions.push(TypeSubstitution {
                    span,
                    generic,
                    replacement: *replacement,
                });
            }
        }

        self.apply_substitution(return_type, &substitutions)
    }

    fn check_core_log_call(
        &mut self,
        args: &[HirExpr],
        span: TextRange,
        context: &mut BodyContext,
    ) -> TypeId {
        let unit = self.intern(TypeKind::Unit);
        let Some(message) = args.first() else {
            self.push(
                context.source_id,
                MD_CALL_ARITY,
                span,
                "core.log 至少需要 1 个参数",
                "请传入日志消息或格式字符串",
            );
            return unit;
        };

        let string = self.intern(TypeKind::String);
        let message_ty = self.check_expr(message, context);
        self.expect_exact(context.source_id, string, message_ty, expr_span(message));

        if args.len() == 1 {
            return unit;
        }

        let format = match message {
            HirExpr::Literal { text, .. } => parse_log_format(text),
            _ => None,
        };
        let Some(format) = format else {
            self.push(
                context.source_id,
                MD_INVALID_LOG_FORMAT,
                expr_span(message),
                "core.log 格式串必须是字符串字面量",
                "请把第一个参数写成包含 `{}` 占位符的字符串字面量",
            );
            for arg in &args[1..] {
                self.check_log_argument(arg, context);
            }
            return unit;
        };

        let expected = format.placeholder_count();
        let actual = args.len() - 1;
        if expected != actual {
            self.push(
                context.source_id,
                MD_CALL_ARITY,
                span,
                format!("core.log 格式串需要 {expected} 个插值参数，但传入了 {actual} 个"),
                "请调整 `{}` 占位符数量或实参数量",
            );
        }

        for arg in &args[1..] {
            self.check_log_argument(arg, context);
        }

        unit
    }

    fn check_log_argument(&mut self, arg: &HirExpr, context: &mut BodyContext) {
        let ty = self.check_expr(arg, context);
        if self.is_error(ty) {
            return;
        }
        if matches!(
            self.kind(ty),
            TypeKind::I32 | TypeKind::Bool | TypeKind::String
        ) {
            return;
        }

        let actual = self.display_type(ty);
        self.push(
            context.source_id,
            MD_TYPE_MISMATCH,
            expr_span(arg),
            format!("core.log 插值参数只支持 `i32`、`bool` 或 `String`，实际是 `{actual}`"),
            "请传入可直接打印的标量值",
        );
    }

    fn path_type(&mut self, resolved: Option<&ResolvedPath>, context: &BodyContext) -> TypeId {
        match resolved {
            Some(ResolvedPath::Local(local)) => context
                .bindings
                .get(local)
                .map_or_else(|| self.intern(TypeKind::Error), |binding| binding.ty),
            Some(ResolvedPath::Symbol(symbol)) => self.symbol_value_type(*symbol),
            Some(ResolvedPath::Builtin(builtin)) => self.builtin_type(*builtin),
            Some(ResolvedPath::Generic(name)) => self.intern(TypeKind::Generic(name.clone())),
            None => self.intern(TypeKind::Error),
        }
    }

    fn symbol_value_type(&mut self, symbol: SymbolId) -> TypeId {
        let Some(symbol_info) = self.package.symbols.get(symbol.get()) else {
            return self.intern(TypeKind::Error);
        };

        match symbol_info.kind {
            SymbolKind::Function => match self.functions.get(&symbol).cloned() {
                Some(sig) => self.function_type(&sig),
                None => self.intern(TypeKind::Error),
            },
            SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait => self
                .item_types
                .get(&symbol)
                .copied()
                .unwrap_or_else(|| self.intern(TypeKind::Error)),
            SymbolKind::Variant => self.variant_value_type(symbol),
            SymbolKind::Module | SymbolKind::Impl => self.intern(TypeKind::Error),
        }
    }

    fn variant_value_type(&mut self, symbol: SymbolId) -> TypeId {
        let Some(variant) = self.variants.get(&symbol).cloned() else {
            return self.intern(TypeKind::Error);
        };
        let return_type =
            self.enum_instance_type(variant.enum_symbol, &variant.enum_generics, true);

        if variant.fields.is_empty() {
            return return_type;
        }

        self.intern(TypeKind::Function {
            generics: variant.enum_generics,
            params: variant.fields,
            return_type,
        })
    }

    fn enum_instance_type(
        &mut self,
        enum_symbol: SymbolId,
        generics: &[String],
        flexible: bool,
    ) -> TypeId {
        let args = generics
            .iter()
            .map(|generic| {
                if flexible {
                    self.intern(TypeKind::Infer(generic.clone()))
                } else {
                    self.intern(TypeKind::Generic(generic.clone()))
                }
            })
            .collect::<Vec<_>>();
        self.intern(TypeKind::Nominal {
            symbol: enum_symbol,
            args,
        })
    }

    fn literal_type(&mut self, text: &str) -> TypeId {
        if text.starts_with("int(") {
            self.intern(TypeKind::I32)
        } else if text.starts_with("bool(") {
            self.intern(TypeKind::Bool)
        } else if text.starts_with("string(") {
            self.intern(TypeKind::String)
        } else {
            self.intern(TypeKind::Error)
        }
    }

    fn function_sig(
        &mut self,
        source_id: SourceId,
        function: &HirFunction,
        outer_generics: &[String],
    ) -> FunctionSig {
        let generics = generic_scope(outer_generics, &function.generics);
        let params = function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) => self.resolve_type_ref(source_id, ty, &generics),
                None => self.intern(TypeKind::Error),
            })
            .collect::<Vec<_>>();
        let return_type = match &function.return_type {
            Some(ty) => self.resolve_type_ref(source_id, ty, &generics),
            None => self.intern(TypeKind::Unit),
        };

        FunctionSig {
            name: function.name.clone(),
            generics: function.generics.clone(),
            params,
            return_type,
            span: function.span,
        }
    }

    fn function_type(&mut self, sig: &FunctionSig) -> TypeId {
        self.intern(TypeKind::Function {
            generics: sig.generics.clone(),
            params: sig.params.clone(),
            return_type: sig.return_type,
        })
    }

    fn resolve_type_ref(
        &mut self,
        source_id: SourceId,
        ty: &HirTypeRef,
        generics: &HashSet<String>,
    ) -> TypeId {
        match &ty.resolved {
            Some(ResolvedPath::Builtin(builtin)) => {
                if !ty.generic_args.is_empty() {
                    self.push(
                        source_id,
                        MD_INVALID_TYPE_ARITY,
                        ty.span,
                        format!("内建类型 `{}` 不接受泛型实参", builtin.as_str()),
                        "请移除该类型后的泛型实参",
                    );
                }
                self.builtin_type(*builtin)
            }
            Some(ResolvedPath::Generic(name)) => {
                if generics.contains(name) {
                    self.intern(TypeKind::Generic(name.clone()))
                } else {
                    self.intern(TypeKind::Error)
                }
            }
            Some(ResolvedPath::Symbol(symbol)) => {
                self.resolve_symbol_type(source_id, *symbol, ty, generics)
            }
            Some(ResolvedPath::Local(_)) | None => self.intern(TypeKind::Error),
        }
    }

    fn resolve_symbol_type(
        &mut self,
        source_id: SourceId,
        symbol: SymbolId,
        ty: &HirTypeRef,
        generics: &HashSet<String>,
    ) -> TypeId {
        let args = ty
            .generic_args
            .iter()
            .map(|arg| self.resolve_type_ref(source_id, arg, generics))
            .collect::<Vec<_>>();
        let expected = self.generic_counts.get(&symbol).copied().unwrap_or(0);
        if expected != args.len() {
            self.push(
                source_id,
                MD_INVALID_TYPE_ARITY,
                ty.span,
                format!(
                    "类型 `{}` 需要 {expected} 个泛型实参，但提供了 {} 个",
                    ty.display(),
                    args.len()
                ),
                "请调整泛型实参数量",
            );
        }
        self.intern(TypeKind::Nominal { symbol, args })
    }

    fn builtin_type(&mut self, builtin: BuiltinType) -> TypeId {
        match builtin {
            BuiltinType::Int | BuiltinType::I32 => self.intern(TypeKind::I32),
            BuiltinType::Bool | BuiltinType::BoolLower => self.intern(TypeKind::Bool),
            BuiltinType::String => self.intern(TypeKind::String),
            BuiltinType::Unit => self.intern(TypeKind::Unit),
        }
    }

    fn result_parts(&self, ty: TypeId) -> Option<(TypeId, TypeId)> {
        let TypeKind::Nominal { symbol, args } = self.kind(ty) else {
            return None;
        };
        if args.len() != 2 || !self.symbol_has_name(*symbol, "Result") {
            return None;
        }
        Some((args[0], args[1]))
    }

    fn symbol_has_name(&self, symbol: SymbolId, name: &str) -> bool {
        self.package
            .symbols
            .get(symbol.get())
            .and_then(|symbol| symbol.path.last())
            .is_some_and(|last| last == name)
    }

    fn is_core_log_callee(&self, callee: &HirExpr) -> bool {
        let HirExpr::Path {
            resolved: Some(ResolvedPath::Symbol(symbol)),
            ..
        } = callee
        else {
            return false;
        };
        self.package
            .symbols
            .get(symbol.get())
            .is_some_and(|symbol| {
                symbol.kind == SymbolKind::Function
                    && symbol.path.as_slice() == ["core".to_owned(), "log".to_owned()]
            })
    }

    fn expect_exact(
        &mut self,
        source_id: SourceId,
        expected: TypeId,
        actual: TypeId,
        span: TextRange,
    ) {
        if self.is_error(expected) || self.is_error(actual) || expected == actual {
            return;
        }
        self.push_mismatch(source_id, expected, actual, span);
    }

    fn expect_assignable(
        &mut self,
        source_id: SourceId,
        expected: TypeId,
        actual: TypeId,
        span: TextRange,
    ) {
        let mut substitutions = HashMap::new();
        self.unify(source_id, expected, actual, span, &mut substitutions);
    }

    fn unify(
        &mut self,
        source_id: SourceId,
        expected: TypeId,
        actual: TypeId,
        span: TextRange,
        substitutions: &mut HashMap<String, TypeId>,
    ) {
        if self.is_error(expected) || self.is_error(actual) || expected == actual {
            return;
        }

        match (self.kind(expected).clone(), self.kind(actual).clone()) {
            (TypeKind::Infer(name), _) => {
                self.bind_infer(source_id, &name, actual, span, substitutions);
            }
            (_, TypeKind::Infer(name)) => {
                self.bind_infer(source_id, &name, expected, span, substitutions);
            }
            (
                TypeKind::Nominal {
                    symbol: expected_symbol,
                    args: expected_args,
                },
                TypeKind::Nominal {
                    symbol: actual_symbol,
                    args: actual_args,
                },
            ) if expected_symbol == actual_symbol && expected_args.len() == actual_args.len() => {
                for (expected_arg, actual_arg) in expected_args.into_iter().zip(actual_args) {
                    self.unify(source_id, expected_arg, actual_arg, span, substitutions);
                }
            }
            (TypeKind::Generic(expected_name), TypeKind::Generic(actual_name))
                if expected_name == actual_name => {}
            _ => self.push_mismatch(source_id, expected, actual, span),
        }
    }

    fn bind_infer(
        &mut self,
        source_id: SourceId,
        name: &str,
        replacement: TypeId,
        span: TextRange,
        substitutions: &mut HashMap<String, TypeId>,
    ) {
        if let Some(existing) = substitutions.get(name).copied() {
            self.unify(source_id, existing, replacement, span, substitutions);
        } else {
            substitutions.insert(name.to_owned(), replacement);
        }
    }

    fn apply_substitution(
        &mut self,
        ty: TypeId,
        substitutions: &HashMap<String, TypeId>,
    ) -> TypeId {
        match self.kind(ty).clone() {
            TypeKind::Generic(name) | TypeKind::Infer(name) => {
                substitutions.get(&name).copied().unwrap_or(ty)
            }
            TypeKind::Nominal { symbol, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.apply_substitution(arg, substitutions))
                    .collect::<Vec<_>>();
                self.intern(TypeKind::Nominal { symbol, args })
            }
            TypeKind::Function {
                generics,
                params,
                return_type,
            } => {
                let params = params
                    .into_iter()
                    .map(|param| self.apply_substitution(param, substitutions))
                    .collect::<Vec<_>>();
                let return_type = self.apply_substitution(return_type, substitutions);
                self.intern(TypeKind::Function {
                    generics,
                    params,
                    return_type,
                })
            }
            TypeKind::Error
            | TypeKind::Unit
            | TypeKind::I32
            | TypeKind::Bool
            | TypeKind::String => ty,
        }
    }

    fn instantiate_flexible(&mut self, ty: TypeId, generics: &[String]) -> TypeId {
        match self.kind(ty).clone() {
            TypeKind::Generic(name) if generics.contains(&name) => {
                self.intern(TypeKind::Infer(name))
            }
            TypeKind::Nominal { symbol, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.instantiate_flexible(arg, generics))
                    .collect::<Vec<_>>();
                self.intern(TypeKind::Nominal { symbol, args })
            }
            TypeKind::Function {
                generics: inner_generics,
                params,
                return_type,
            } => {
                let params = params
                    .into_iter()
                    .map(|param| self.instantiate_flexible(param, generics))
                    .collect::<Vec<_>>();
                let return_type = self.instantiate_flexible(return_type, generics);
                self.intern(TypeKind::Function {
                    generics: inner_generics,
                    params,
                    return_type,
                })
            }
            TypeKind::Error
            | TypeKind::Unit
            | TypeKind::I32
            | TypeKind::Bool
            | TypeKind::String
            | TypeKind::Generic(_)
            | TypeKind::Infer(_) => ty,
        }
    }

    fn push_mismatch(
        &mut self,
        source_id: SourceId,
        expected: TypeId,
        actual: TypeId,
        span: TextRange,
    ) {
        let expected = self.display_type(expected);
        let actual = self.display_type(actual);
        self.push(
            source_id,
            MD_TYPE_MISMATCH,
            span,
            format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`"),
            "请调整表达式类型或显式类型标注",
        );
    }

    fn record_item_type(&mut self, item: &HirItem, ty: TypeId) {
        self.item_types.insert(item.symbol, ty);
        self.items.push(ItemType {
            item: item.id,
            symbol: item.symbol,
            ty,
        });
    }

    fn intern(&mut self, kind: TypeKind) -> TypeId {
        if let Some(id) = self.type_ids.get(&kind) {
            return *id;
        }
        let id = TypeId::new(self.types.len());
        self.types.push(kind.clone());
        self.type_ids.insert(kind, id);
        id
    }

    fn kind(&self, ty: TypeId) -> &TypeKind {
        self.types.get(ty.get()).unwrap_or(&TypeKind::Error)
    }

    fn is_error(&self, ty: TypeId) -> bool {
        matches!(self.kind(ty), TypeKind::Error)
    }

    fn display_type(&self, ty: TypeId) -> String {
        let table = TypeTable {
            types: self.types.clone(),
            items: Vec::new(),
            locals: Vec::new(),
            expressions: Vec::new(),
            substitutions: Vec::new(),
        };
        table.display_type(ty)
    }

    fn push(
        &mut self,
        source_id: SourceId,
        code: &str,
        span: TextRange,
        message: impl Into<String>,
        note: &'static str,
    ) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(code).expect("type diagnostic code must be valid"),
            DiagnosticSeverity::Error,
            message,
        )
        .with_note(note);

        let diagnostic = self
            .sources
            .get(&source_id)
            .map_or(diagnostic.clone(), |source| {
                DiagnosticSpan::from_source(source, span)
                    .map_or(diagnostic.clone(), |resolved_span| {
                        diagnostic.with_span(resolved_span)
                    })
            });
        self.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Debug)]
struct BodyContext {
    source_id: SourceId,
    owner: String,
    return_type: TypeId,
    bindings: HashMap<LocalId, BindingInfo>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PatternCoverage {
    All,
    Variant(SymbolId),
    Partial,
}

fn generic_scope(outer: &[String], inner: &[String]) -> HashSet<String> {
    outer.iter().chain(inner).cloned().collect()
}

fn nominal_symbol(kind: &TypeKind) -> Option<SymbolId> {
    match kind {
        TypeKind::Nominal { symbol, .. } => Some(*symbol),
        _ => None,
    }
}

fn expr_span(expr: &HirExpr) -> TextRange {
    match expr {
        HirExpr::Missing { span }
        | HirExpr::Literal { span, .. }
        | HirExpr::Path { span, .. }
        | HirExpr::Call { span, .. }
        | HirExpr::If { span, .. }
        | HirExpr::Match { span, .. }
        | HirExpr::Binary { span, .. }
        | HirExpr::Try { span, .. } => *span,
        HirExpr::Block(block) => block.span,
    }
}

fn should_check_body_tail(block: &HirBlock) -> bool {
    matches!(block.statements.last(), Some(HirStatement::Expr(_)))
        || !block
            .statements
            .iter()
            .any(|statement| matches!(statement, HirStatement::Return { .. }))
}

#[derive(Default)]
struct TypeDumper {
    lines: Vec<String>,
}

impl TypeDumper {
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

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId};

    use crate::core::check_source_with_core;

    use super::{
        check_source, MD_CALL_ARITY, MD_IMMUTABLE_ASSIGNMENT, MD_INVALID_LOG_FORMAT,
        MD_INVALID_TRY, MD_MISSING_TRAIT_METHOD, MD_NON_EXHAUSTIVE_MATCH, MD_TYPE_MISMATCH,
    };

    #[test]
    fn accepts_generic_option_result_and_records_type_dump() {
        let source = SourceFile::new(
            SourceId::new(1),
            "generics.mao",
            "\
module demo
enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
fn id<T>(value: T) -> T { return value }
fn main(value: i32) -> Option<i32> {
  let wrapped: Option<i32> = Option.Some(value)
  let none: Option<i32> = Option.None
  let result: Result<i32, String> = Result.Ok(id(value))
  return wrapped
}
",
        );

        let result = check_source(&source);

        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        let dump = result.dump();
        assert!(dump.contains("Types"));
        assert!(dump.contains("Substitutions"));
        assert!(dump.contains("T="));
    }

    #[test]
    fn reports_wrong_return_and_let_annotation_types() {
        let source = SourceFile::new(
            SourceId::new(1),
            "mismatch.mao",
            "\
module demo
fn broken() -> bool {
  let value: bool = 1
  return 1
}
",
        );

        let result = check_source(&source);

        assert!(
            result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code.as_str() == MD_TYPE_MISMATCH)
                .count()
                >= 2
        );
    }

    #[test]
    fn reports_assignment_to_immutable_local() {
        let source = SourceFile::new(
            SourceId::new(1),
            "immutable.mao",
            "\
module demo
fn main() {
  let value: i32 = 1
  value = 2
}
",
        );

        let result = check_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.code.as_str() == MD_IMMUTABLE_ASSIGNMENT
                    && diagnostic.message.contains("value")
            ));
    }

    #[test]
    fn reports_missing_trait_method_in_impl() {
        let source = SourceFile::new(
            SourceId::new(1),
            "impl.mao",
            "\
module demo
struct Point {}
trait Show { fn show(value: Point) -> String; }
impl Show for Point {}
",
        );

        let result = check_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.code.as_str() == MD_MISSING_TRAIT_METHOD
                    && diagnostic.message.contains("show")
            ));
    }

    #[test]
    fn reports_non_exhaustive_enum_match() {
        let source = SourceFile::new(
            SourceId::new(1),
            "match.mao",
            "\
module demo
enum Color { Red, Green }
fn score(color: Color) -> i32 {
  match color {
    Color.Red => 1
  }
}
",
        );

        let result = check_source(&source);

        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code.as_str() == MD_NON_EXHAUSTIVE_MATCH
                && diagnostic.message.contains("Green")
        }));
    }

    #[test]
    fn accepts_wildcard_and_pattern_binding_in_match() {
        let source = SourceFile::new(
            SourceId::new(1),
            "match_binding.mao",
            "\
module demo
enum Color { Red, Green }
fn score(color: Color, value: i32) -> i32 {
  let copied: i32 = match value { bound => bound }
  match color {
    Color.Red => copied,
    _ => 0
  }
}
",
        );

        let result = check_source(&source);

        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert!(result.dump().contains("bound t1 owner=score"));
    }

    #[test]
    fn accepts_result_try_propagation() {
        let source = SourceFile::new(
            SourceId::new(1),
            "try.mao",
            "\
module demo
enum Result<T, E> { Ok(T), Err(E) }
fn parse(value: i32) -> Result<i32, String> { return Result.Ok(value) }
fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = parse(value)?
  return Result.Ok(parsed)
}
",
        );

        let result = check_source(&source);

        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn reports_try_in_non_result_function() {
        let source = SourceFile::new(
            SourceId::new(1),
            "try_bad.mao",
            "\
module demo
enum Result<T, E> { Ok(T), Err(E) }
fn parse(value: i32) -> Result<i32, String> { return Result.Ok(value) }
fn main(value: i32) -> i32 {
  let parsed: i32 = parse(value)?
  return parsed
}
",
        );

        let result = check_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_INVALID_TRY));
    }

    #[test]
    fn accepts_core_log_format_arguments() {
        let source = SourceFile::new(
            SourceId::new(1),
            "log_format.mao",
            "\
module demo
import core.Result
import core.log
fn label() -> String { return \"ok\" }
fn main(value: i32) -> Result<i32, String> {
  let enabled: bool = true
  let message: String = label()
  log(\"value is {} {} {}\", value, enabled, message)
  return Result.Ok(value)
}
",
        );

        let result = check_source_with_core(&source);

        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    }

    #[test]
    fn reports_core_log_format_errors() {
        let source = SourceFile::new(
            SourceId::new(1),
            "log_format_bad.mao",
            "\
module demo
import core.Result
import core.log
struct Point {}
fn bad(point: Point) {
  log(\"value {}\", point)
}
fn main(value: i32) -> Result<i32, String> {
  let format: String = \"value {}\"
  log(format, value)
  log(\"value {} {}\", value)
  return Result.Ok(value)
}
",
        );

        let result = check_source_with_core(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_INVALID_LOG_FORMAT));
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_CALL_ARITY));
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_TYPE_MISMATCH));
    }
}
