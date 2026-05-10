//! Name resolution from syntax AST to HIR.

use std::collections::{HashMap, HashSet};

use maodie_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticSpan, SourceFile, TextRange,
};
use maodie_syntax::{
    parse_source, AstFile, BinaryOp, BlockExpr, EnumDecl, Expr, FunctionDecl, ImplDecl, Item,
    Literal, Pattern, Statement, StructDecl, TraitDecl, TypeRef,
};

use crate::hir::{
    BuiltinType, HirBlock, HirEnum, HirExpr, HirField, HirFunction, HirImpl, HirImport, HirItem,
    HirItemKind, HirLet, HirLocal, HirMatchArm, HirModule, HirPackage, HirParam, HirPattern,
    HirStatement, HirStruct, HirTrait, HirTypeRef, HirVariant, ItemId, LocalId, LocalKind,
    ModuleId, ResolvedPath, Symbol, SymbolId, SymbolKind,
};

/// Duplicate definition diagnostic.
pub const MD_DUPLICATE_NAME: &str = "MD0301";
/// Unresolved path diagnostic.
pub const MD_UNRESOLVED_NAME: &str = "MD0302";
/// Invalid import diagnostic.
pub const MD_INVALID_IMPORT: &str = "MD0303";

/// Resolver output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolveResult {
    /// HIR package produced from parsed inputs.
    pub package: HirPackage,
    /// Parser and resolver diagnostics in stable order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Name resolver for one compiler session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Resolver;

impl Resolver {
    /// Creates a resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Parses and resolves one source file.
    #[must_use]
    pub fn resolve_source(&self, source: &SourceFile) -> ResolveResult {
        self.resolve_sources(&[source])
    }

    /// Parses and resolves a set of source files as one package.
    #[must_use]
    pub fn resolve_sources(&self, sources: &[&SourceFile]) -> ResolveResult {
        let parsed = sources
            .iter()
            .map(|source| {
                let result = parse_source(source);
                ParsedUnit {
                    ast: result.ast,
                    diagnostics: result.diagnostics,
                }
            })
            .collect::<Vec<_>>();

        ResolverSession::new(sources, parsed).resolve()
    }
}

/// Parses and resolves one source file.
#[must_use]
pub fn resolve_source(source: &SourceFile) -> ResolveResult {
    Resolver::new().resolve_source(source)
}

/// Parses and resolves a set of source files as one package.
#[must_use]
pub fn resolve_sources(sources: &[&SourceFile]) -> ResolveResult {
    Resolver::new().resolve_sources(sources)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedUnit {
    ast: AstFile,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
struct ResolverSession<'source> {
    sources: &'source [&'source SourceFile],
    parsed: Vec<ParsedUnit>,
    diagnostics: Vec<Diagnostic>,
    modules: Vec<ModuleBuild>,
    symbols: Vec<Symbol>,
    global_paths: HashMap<String, SymbolId>,
    item_symbols: HashMap<(usize, usize), SymbolId>,
    variant_symbols: HashMap<(usize, usize, usize), SymbolId>,
}

impl<'source> ResolverSession<'source> {
    fn new(sources: &'source [&'source SourceFile], parsed: Vec<ParsedUnit>) -> Self {
        Self {
            sources,
            parsed,
            diagnostics: Vec::new(),
            modules: Vec::new(),
            symbols: Vec::new(),
            global_paths: HashMap::new(),
            item_symbols: HashMap::new(),
            variant_symbols: HashMap::new(),
        }
    }

    fn resolve(mut self) -> ResolveResult {
        for unit in &self.parsed {
            self.diagnostics.extend(unit.diagnostics.clone());
        }

        self.declare_modules();
        self.declare_items();
        self.resolve_imports();
        let package = self.lower_package();

        ResolveResult {
            package,
            diagnostics: self.diagnostics,
        }
    }

    fn declare_modules(&mut self) {
        let mut module_paths = HashMap::<String, ModuleId>::new();
        for source_index in 0..self.parsed.len() {
            let id = ModuleId::new(self.modules.len());
            let ast = &self.parsed[source_index].ast;
            let path = ast
                .module
                .as_ref()
                .filter(|module| !module.path.is_empty())
                .map_or_else(
                    || vec![default_module_name(self.sources[source_index])],
                    |module| module.path.clone(),
                );
            let path_key = path_key(&path);
            let span = ast.span;
            let duplicate_span = ast.module.as_ref().map_or(span, |module| module.span);
            let symbol = self.alloc_symbol(SymbolKind::Module, path.clone(), id, None, span);
            if module_paths.insert(path_key.clone(), id).is_some() {
                self.push_duplicate(source_index, duplicate_span, &path_key);
            }
            self.global_paths.entry(path_key).or_insert(symbol);
            self.modules.push(ModuleBuild {
                id,
                source_index,
                path,
                imports: Vec::new(),
                visible: HashMap::new(),
            });
        }
    }

    fn declare_items(&mut self) {
        for module_index in 0..self.parsed.len() {
            let module_id = self.modules[module_index].id;
            let items = self.parsed[module_index].ast.items.clone();
            for (item_index, item) in items.iter().enumerate() {
                let item_id = ItemId::new(self.item_symbols.len());
                let (kind, name, span) = item_symbol_parts(item, item_index);
                let mut path = self.modules[module_index].path.clone();
                path.push(name.clone());
                let symbol = self.alloc_symbol(kind, path.clone(), module_id, Some(item_id), span);
                self.item_symbols.insert((module_index, item_index), symbol);
                self.add_visible_name(module_index, &name, symbol, span);
                self.global_paths.entry(path_key(&path)).or_insert(symbol);

                if let Item::Enum(enum_) = item {
                    self.declare_variants(module_index, item_index, item_id, enum_);
                }
            }
        }
    }

    fn declare_variants(
        &mut self,
        module_index: usize,
        item_index: usize,
        item_id: ItemId,
        enum_: &EnumDecl,
    ) {
        let module_id = self.modules[module_index].id;
        let Some(enum_name) = &enum_.name else {
            return;
        };
        let mut seen = HashMap::<String, TextRange>::new();
        for (variant_index, variant) in enum_.variants.iter().enumerate() {
            let Some(name) = &variant.name else {
                continue;
            };
            if seen.insert(name.clone(), variant.span).is_some() {
                self.push_duplicate(module_index, variant.span, name);
            }
            let mut path = self.modules[module_index].path.clone();
            path.push(enum_name.clone());
            path.push(name.clone());
            let symbol = self.alloc_symbol(
                SymbolKind::Variant,
                path.clone(),
                module_id,
                Some(item_id),
                variant.span,
            );
            self.variant_symbols
                .insert((module_index, item_index, variant_index), symbol);
            self.global_paths.entry(path_key(&path)).or_insert(symbol);
        }
    }

    fn resolve_imports(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports = self.parsed[module_index].ast.imports.clone();
            for import in imports {
                let key = path_key(&import.path);
                let resolved = self.global_paths.get(&key).copied();
                if resolved.is_none() {
                    self.push_invalid_import(module_index, import.span, &key);
                } else if let Some(alias) = import.path.last() {
                    self.add_visible_name(
                        module_index,
                        alias,
                        resolved.expect("checked above"),
                        import.span,
                    );
                }
                self.modules[module_index].imports.push(HirImport {
                    path: import.path,
                    resolved,
                    span: import.span,
                });
            }
        }
    }

    fn lower_package(&mut self) -> HirPackage {
        let mut modules = Vec::with_capacity(self.modules.len());
        for module_index in 0..self.modules.len() {
            let span = self.parsed[module_index].ast.span;
            let items = self.parsed[module_index].ast.items.clone();
            let hir_items = items
                .iter()
                .enumerate()
                .map(|(item_index, item)| self.lower_item(module_index, item_index, item))
                .collect::<Vec<_>>();
            let module = &self.modules[module_index];
            modules.push(HirModule {
                id: module.id,
                source_id: self.sources[module.source_index].id(),
                path: module.path.clone(),
                imports: module.imports.clone(),
                items: hir_items,
                span,
            });
        }

        HirPackage {
            modules,
            symbols: self.symbols.clone(),
        }
    }

    fn lower_item(&mut self, module_index: usize, item_index: usize, item: &Item) -> HirItem {
        let symbol = self.item_symbols[&(module_index, item_index)];
        let id = self.symbols[symbol.get()]
            .item
            .expect("top-level item symbol has item id");
        let kind = match item {
            Item::Function(function) => {
                HirItemKind::Function(self.lower_function(module_index, function))
            }
            Item::Struct(struct_) => HirItemKind::Struct(self.lower_struct(module_index, struct_)),
            Item::Enum(enum_) => {
                HirItemKind::Enum(self.lower_enum(module_index, item_index, enum_))
            }
            Item::Trait(trait_) => HirItemKind::Trait(self.lower_trait(module_index, trait_)),
            Item::Impl(impl_) => HirItemKind::Impl(self.lower_impl(module_index, impl_)),
        };

        HirItem {
            id,
            symbol,
            kind,
            span: item_span(item),
        }
    }

    fn lower_struct(&mut self, module_index: usize, struct_: &StructDecl) -> HirStruct {
        let mut type_scope = TypeScope::new(struct_.generics.clone());
        let mut fields = Vec::with_capacity(struct_.fields.len());
        let mut seen = HashMap::<String, TextRange>::new();
        for field in &struct_.fields {
            if let Some(name) = &field.name {
                if seen.insert(name.clone(), field.span).is_some() {
                    self.push_duplicate(module_index, field.span, name);
                }
                fields.push(HirField {
                    name: name.clone(),
                    ty: field
                        .ty
                        .as_ref()
                        .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope)),
                    span: field.span,
                });
            }
        }

        HirStruct {
            name: struct_
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned()),
            generics: struct_.generics.clone(),
            fields,
        }
    }

    fn lower_enum(&mut self, module_index: usize, item_index: usize, enum_: &EnumDecl) -> HirEnum {
        let mut type_scope = TypeScope::new(enum_.generics.clone());
        let mut variants = Vec::with_capacity(enum_.variants.len());
        for (variant_index, variant) in enum_.variants.iter().enumerate() {
            let Some(name) = &variant.name else {
                continue;
            };
            let fields = variant
                .fields
                .iter()
                .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope))
                .collect::<Vec<_>>();
            variants.push(HirVariant {
                symbol: self.variant_symbols[&(module_index, item_index, variant_index)],
                name: name.clone(),
                fields,
                span: variant.span,
            });
        }

        HirEnum {
            name: enum_.name.clone().unwrap_or_else(|| "<missing>".to_owned()),
            generics: enum_.generics.clone(),
            variants,
        }
    }

    fn lower_trait(&mut self, module_index: usize, trait_: &TraitDecl) -> HirTrait {
        let mut seen = HashMap::<String, TextRange>::new();
        let functions = trait_
            .functions
            .iter()
            .map(|function| {
                if let Some(name) = &function.name {
                    if seen.insert(name.clone(), function.span).is_some() {
                        self.push_duplicate(module_index, function.span, name);
                    }
                }
                self.lower_function_with_generics(module_index, function, &trait_.generics)
            })
            .collect::<Vec<_>>();

        HirTrait {
            name: trait_
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned()),
            generics: trait_.generics.clone(),
            functions,
        }
    }

    fn lower_impl(&mut self, module_index: usize, impl_: &ImplDecl) -> HirImpl {
        let mut type_scope = TypeScope::default();
        let trait_path = impl_
            .trait_path
            .as_ref()
            .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope));
        let target = impl_
            .target
            .as_ref()
            .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope));
        let mut seen = HashMap::<String, TextRange>::new();
        let methods = impl_
            .methods
            .iter()
            .map(|function| {
                if let Some(name) = &function.name {
                    if seen.insert(name.clone(), function.span).is_some() {
                        self.push_duplicate(module_index, function.span, name);
                    }
                }
                self.lower_function(module_index, function)
            })
            .collect::<Vec<_>>();

        HirImpl {
            trait_path,
            target,
            methods,
        }
    }

    fn lower_function(&mut self, module_index: usize, function: &FunctionDecl) -> HirFunction {
        self.lower_function_with_generics(module_index, function, &[])
    }

    fn lower_function_with_generics(
        &mut self,
        module_index: usize,
        function: &FunctionDecl,
        outer_generics: &[String],
    ) -> HirFunction {
        let mut scope = FunctionScope::default();
        let mut generics = outer_generics.to_vec();
        generics.extend(function.generics.clone());
        let mut type_scope = TypeScope::new(generics);
        let params = function
            .params
            .params
            .iter()
            .filter_map(|param| {
                let name = param.name.clone()?;
                let local = scope.declare(
                    &mut self.diagnostics,
                    self.sources[module_index],
                    &name,
                    param.span,
                    LocalKind::Param,
                );
                Some(HirParam {
                    local,
                    name,
                    ty: param
                        .ty
                        .as_ref()
                        .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope)),
                    span: param.span,
                })
            })
            .collect::<Vec<_>>();
        let return_type = function
            .return_type
            .as_ref()
            .map(|ty| self.lower_type_ref(module_index, ty, &mut type_scope));
        let body = function
            .body
            .as_ref()
            .map(|body| self.lower_block(module_index, body, &mut scope, &mut type_scope));

        HirFunction {
            name: function
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned()),
            generics: function.generics.clone(),
            params,
            return_type,
            body,
            locals: scope.locals,
            span: function.span,
        }
    }

    fn lower_block(
        &mut self,
        module_index: usize,
        block: &BlockExpr,
        scope: &mut FunctionScope,
        type_scope: &mut TypeScope,
    ) -> HirBlock {
        let statements = block
            .statements
            .iter()
            .map(|statement| self.lower_statement(module_index, statement, scope, type_scope))
            .collect::<Vec<_>>();

        HirBlock {
            statements,
            span: block.span,
        }
    }

    fn lower_statement(
        &mut self,
        module_index: usize,
        statement: &Statement,
        scope: &mut FunctionScope,
        type_scope: &mut TypeScope,
    ) -> HirStatement {
        match statement {
            Statement::Let(statement) => {
                let ty = statement
                    .ty
                    .as_ref()
                    .map(|ty| self.lower_type_ref(module_index, ty, type_scope));
                let value = statement
                    .value
                    .as_ref()
                    .map(|expr| self.lower_expr(module_index, expr, scope, type_scope));
                let local = statement.name.as_ref().map(|name| {
                    scope.declare(
                        &mut self.diagnostics,
                        self.sources[module_index],
                        name,
                        statement.span,
                        LocalKind::Let,
                    )
                });
                HirStatement::Let(HirLet {
                    mutable: statement.mutable,
                    local,
                    name: statement.name.clone(),
                    ty,
                    value,
                    span: statement.span,
                })
            }
            Statement::Return { expr, span } => HirStatement::Return {
                expr: expr
                    .as_ref()
                    .map(|expr| self.lower_expr(module_index, expr, scope, type_scope)),
                span: *span,
            },
            Statement::Expr(expr) => {
                HirStatement::Expr(self.lower_expr(module_index, expr, scope, type_scope))
            }
        }
    }

    fn lower_expr(
        &mut self,
        module_index: usize,
        expr: &Expr,
        scope: &mut FunctionScope,
        type_scope: &mut TypeScope,
    ) -> HirExpr {
        match expr {
            Expr::Missing { span } => HirExpr::Missing { span: *span },
            Expr::Literal { literal, span } => HirExpr::Literal {
                text: literal_text(literal),
                span: *span,
            },
            Expr::Path { path, span } => HirExpr::Path {
                path: path.clone(),
                resolved: self.resolve_expr_path(module_index, scope, path, *span),
                span: *span,
            },
            Expr::Call { callee, args, span } => HirExpr::Call {
                callee: Box::new(self.lower_expr(module_index, callee, scope, type_scope)),
                args: args
                    .iter()
                    .map(|arg| self.lower_expr(module_index, arg, scope, type_scope))
                    .collect(),
                span: *span,
            },
            Expr::Block(block) => {
                HirExpr::Block(self.lower_block(module_index, block, scope, type_scope))
            }
            Expr::If {
                condition,
                then_block,
                else_branch,
                span,
            } => HirExpr::If {
                condition: Box::new(self.lower_expr(module_index, condition, scope, type_scope)),
                then_block: self.lower_block(module_index, then_block, scope, type_scope),
                else_branch: else_branch.as_ref().map(|branch| {
                    Box::new(self.lower_expr(module_index, branch, scope, type_scope))
                }),
                span: *span,
            },
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => HirExpr::Match {
                scrutinee: Box::new(self.lower_expr(module_index, scrutinee, scope, type_scope)),
                arms: arms
                    .iter()
                    .map(|arm| {
                        let saved_bindings = scope.bindings.clone();
                        let pattern = self.lower_pattern(module_index, &arm.pattern, scope);
                        let expr = self.lower_expr(module_index, &arm.expr, scope, type_scope);
                        scope.bindings = saved_bindings;
                        HirMatchArm {
                            pattern,
                            expr,
                            span: arm.span,
                        }
                    })
                    .collect(),
                span: *span,
            },
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => HirExpr::Binary {
                op: binary_op_text(*op),
                left: Box::new(self.lower_expr(module_index, left, scope, type_scope)),
                right: Box::new(self.lower_expr(module_index, right, scope, type_scope)),
                span: *span,
            },
            Expr::Try { expr, span } => HirExpr::Try {
                expr: Box::new(self.lower_expr(module_index, expr, scope, type_scope)),
                span: *span,
            },
        }
    }

    fn lower_pattern(
        &mut self,
        module_index: usize,
        pattern: &Pattern,
        scope: &mut FunctionScope,
    ) -> HirPattern {
        match pattern {
            Pattern::Wildcard { span } => HirPattern::Wildcard { span: *span },
            Pattern::Binding { name, span } => {
                let local = scope.declare(
                    &mut self.diagnostics,
                    self.sources[module_index],
                    name,
                    *span,
                    LocalKind::Pattern,
                );
                HirPattern::Binding {
                    local,
                    name: name.clone(),
                    span: *span,
                }
            }
            Pattern::Literal { literal, span } => HirPattern::Literal {
                text: literal_text(literal),
                span: *span,
            },
            Pattern::Path { path, span } => HirPattern::Path {
                path: path.clone(),
                resolved: self.resolve_item_path(module_index, path, *span),
                span: *span,
            },
        }
    }

    fn lower_type_ref(
        &mut self,
        module_index: usize,
        ty: &TypeRef,
        type_scope: &mut TypeScope,
    ) -> HirTypeRef {
        HirTypeRef {
            path: ty.path.clone(),
            resolved: self.resolve_type_path(module_index, type_scope, &ty.path, ty.span),
            generic_args: ty
                .generic_args
                .iter()
                .map(|arg| self.lower_type_ref(module_index, arg, type_scope))
                .collect(),
            span: ty.span,
        }
    }

    fn resolve_type_path(
        &mut self,
        module_index: usize,
        type_scope: &TypeScope,
        path: &[String],
        span: TextRange,
    ) -> Option<ResolvedPath> {
        if path.len() == 1 {
            let name = &path[0];
            if type_scope.generics.contains(name) {
                return Some(ResolvedPath::Generic(name.clone()));
            }
            if let Some(symbol) = self.modules[module_index].visible.get(name) {
                return Some(ResolvedPath::Symbol(*symbol));
            }
            if let Some(builtin) = BuiltinType::from_name(name) {
                return Some(ResolvedPath::Builtin(builtin));
            }
        }
        self.resolve_global_path(module_index, path, span)
    }

    fn resolve_expr_path(
        &mut self,
        module_index: usize,
        scope: &FunctionScope,
        path: &[String],
        span: TextRange,
    ) -> Option<ResolvedPath> {
        if path.len() == 1 {
            let name = &path[0];
            if let Some(local) = scope.bindings.get(name) {
                return Some(ResolvedPath::Local(*local));
            }
            if let Some(symbol) = self.modules[module_index].visible.get(name) {
                return Some(ResolvedPath::Symbol(*symbol));
            }
        }
        self.resolve_global_path(module_index, path, span)
    }

    fn resolve_item_path(
        &mut self,
        module_index: usize,
        path: &[String],
        span: TextRange,
    ) -> Option<ResolvedPath> {
        if path.len() == 1 {
            if let Some(symbol) = self.modules[module_index].visible.get(&path[0]) {
                return Some(ResolvedPath::Symbol(*symbol));
            }
        }
        self.resolve_global_path(module_index, path, span)
    }

    fn resolve_global_path(
        &mut self,
        module_index: usize,
        path: &[String],
        span: TextRange,
    ) -> Option<ResolvedPath> {
        let key = path_key(path);
        if let Some(symbol) = self.global_paths.get(&key) {
            Some(ResolvedPath::Symbol(*symbol))
        } else if let Some(symbol) = self.resolve_relative_qualified_path(module_index, path) {
            Some(ResolvedPath::Symbol(symbol))
        } else {
            self.push_unresolved(module_index, span, &key);
            None
        }
    }

    fn resolve_relative_qualified_path(
        &self,
        module_index: usize,
        path: &[String],
    ) -> Option<SymbolId> {
        let (head, tail) = path.split_first()?;
        if tail.is_empty() {
            return None;
        }

        let base = self.modules[module_index].visible.get(head)?;
        let mut absolute = self.symbols[base.get()].path.clone();
        absolute.extend(tail.iter().cloned());
        self.global_paths.get(&path_key(&absolute)).copied()
    }

    fn alloc_symbol(
        &mut self,
        kind: SymbolKind,
        path: Vec<String>,
        owner: ModuleId,
        item: Option<ItemId>,
        span: TextRange,
    ) -> SymbolId {
        let id = SymbolId::new(self.symbols.len());
        self.symbols.push(Symbol {
            id,
            kind,
            path,
            owner,
            item,
            span,
        });
        id
    }

    fn add_visible_name(
        &mut self,
        module_index: usize,
        name: &str,
        symbol: SymbolId,
        span: TextRange,
    ) {
        if self.modules[module_index]
            .visible
            .insert(name.to_owned(), symbol)
            .is_some()
        {
            self.push_duplicate(module_index, span, name);
        }
    }

    fn push_duplicate(&mut self, source_index: usize, span: TextRange, name: &str) {
        self.push_diagnostic(
            source_index,
            MD_DUPLICATE_NAME,
            span,
            format!("重复定义名称 `{name}`"),
            "同一作用域中的名称必须唯一",
        );
    }

    fn push_unresolved(&mut self, source_index: usize, span: TextRange, name: &str) {
        self.push_diagnostic(
            source_index,
            MD_UNRESOLVED_NAME,
            span,
            format!("无法解析名称 `{name}`"),
            "请确认名称已在当前模块中定义，或通过 import 导入",
        );
    }

    fn push_invalid_import(&mut self, source_index: usize, span: TextRange, name: &str) {
        self.push_diagnostic(
            source_index,
            MD_INVALID_IMPORT,
            span,
            format!("无效的 import `{name}`"),
            "import 路径必须指向当前包中已声明的模块或符号",
        );
    }

    fn push_diagnostic(
        &mut self,
        source_index: usize,
        code: &str,
        span: TextRange,
        message: String,
        note: &'static str,
    ) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(code).expect("resolver diagnostic code must be valid"),
            DiagnosticSeverity::Error,
            message,
        )
        .with_note(note);
        let diagnostic = DiagnosticSpan::from_source(self.sources[source_index], span)
            .map_or(diagnostic.clone(), |resolved_span| {
                diagnostic.with_span(resolved_span)
            });
        self.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Debug)]
struct ModuleBuild {
    id: ModuleId,
    source_index: usize,
    path: Vec<String>,
    imports: Vec<HirImport>,
    visible: HashMap<String, SymbolId>,
}

#[derive(Clone, Debug, Default)]
struct FunctionScope {
    locals: Vec<HirLocal>,
    bindings: HashMap<String, LocalId>,
}

impl FunctionScope {
    fn declare(
        &mut self,
        diagnostics: &mut Vec<Diagnostic>,
        source: &SourceFile,
        name: &str,
        span: TextRange,
        kind: LocalKind,
    ) -> LocalId {
        let local = LocalId::new(self.locals.len());
        if self.bindings.insert(name.to_owned(), local).is_some() {
            diagnostics.push(
                diagnostic(
                    source,
                    MD_DUPLICATE_NAME,
                    span,
                    format!("重复定义名称 `{name}`"),
                )
                .with_note("同一函数作用域中的名称必须唯一"),
            );
        }
        self.locals.push(HirLocal {
            id: local,
            name: name.to_owned(),
            kind,
            span,
        });
        local
    }
}

#[derive(Clone, Debug, Default)]
struct TypeScope {
    generics: HashSet<String>,
}

impl TypeScope {
    fn new(generics: Vec<String>) -> Self {
        Self {
            generics: generics.into_iter().collect(),
        }
    }
}

fn diagnostic(source: &SourceFile, code: &str, span: TextRange, message: String) -> Diagnostic {
    let diagnostic = Diagnostic::new(
        DiagnosticCode::new(code).expect("resolver diagnostic code must be valid"),
        DiagnosticSeverity::Error,
        message,
    );
    DiagnosticSpan::from_source(source, span).map_or(diagnostic.clone(), |resolved_span| {
        diagnostic.with_span(resolved_span)
    })
}

fn default_module_name(source: &SourceFile) -> String {
    source
        .name()
        .rsplit_once('/')
        .map_or(source.name(), |(_, name)| name)
        .split_once('.')
        .map_or_else(
            || source.name().replace('-', "_"),
            |(stem, _)| stem.replace('-', "_"),
        )
}

fn item_symbol_parts(item: &Item, index: usize) -> (SymbolKind, String, TextRange) {
    match item {
        Item::Function(function) => (
            SymbolKind::Function,
            function
                .name
                .clone()
                .unwrap_or_else(|| format!("<missing_fn_{index}>")),
            function.span,
        ),
        Item::Struct(struct_) => (
            SymbolKind::Struct,
            struct_
                .name
                .clone()
                .unwrap_or_else(|| format!("<missing_struct_{index}>")),
            struct_.span,
        ),
        Item::Enum(enum_) => (
            SymbolKind::Enum,
            enum_
                .name
                .clone()
                .unwrap_or_else(|| format!("<missing_enum_{index}>")),
            enum_.span,
        ),
        Item::Trait(trait_) => (
            SymbolKind::Trait,
            trait_
                .name
                .clone()
                .unwrap_or_else(|| format!("<missing_trait_{index}>")),
            trait_.span,
        ),
        Item::Impl(impl_) => (SymbolKind::Impl, format!("<impl_{index}>"), impl_.span),
    }
}

fn item_span(item: &Item) -> TextRange {
    match item {
        Item::Function(function) => function.span,
        Item::Struct(struct_) => struct_.span,
        Item::Enum(enum_) => enum_.span,
        Item::Trait(trait_) => trait_.span,
        Item::Impl(impl_) => impl_.span,
    }
}

fn path_key(path: &[String]) -> String {
    path.join(".")
}

fn literal_text(literal: &Literal) -> String {
    match literal {
        Literal::Integer(value) => format!("int({value})"),
        Literal::Bool(value) => format!("bool({value})"),
        Literal::String(value) => format!("string({value})"),
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Assign => "=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId};

    use super::{
        resolve_source, resolve_sources, MD_DUPLICATE_NAME, MD_INVALID_IMPORT, MD_UNRESOLVED_NAME,
    };

    #[test]
    fn resolves_multi_module_imports_and_stable_hir_dump() {
        let core = SourceFile::new(
            SourceId::new(1),
            "core.mao",
            "module demo.core\nstruct Point { x: Int }\nfn make() -> Point { return Point }\n",
        );
        let app = SourceFile::new(
            SourceId::new(2),
            "app.mao",
            "module demo.app\nimport demo.core.Point\nfn main(value: Point) -> Bool { let copy: Point = value; return true }\n",
        );

        let result = resolve_sources(&[&core, &app]);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.package.dump(),
            "\
Package
  Symbols
    s0 demo.core m0 kind=module
    s1 demo.app m1 kind=module
    s2 demo.core.Point m0 kind=struct
    s3 demo.core.make m0 kind=function
    s4 demo.app.main m1 kind=function
  Module m0 demo.core source=1 @0..77
    Item i0 s2 Struct Point @17..40
      Field x @32..38
        Type Int -> builtin:Int @35..38
    Item i1 s3 Fn make @41..76
      ReturnType
        Type Point -> s2 @54..59
      Block @60..76
        Return @62..74
          Path Point -> s2 @69..74
  Module m1 demo.app source=2 @0..110
    Import demo.core.Point -> s2 @16..38
    Item i2 s4 Fn main @39..109
      Locals
        l0 value kind=param @47..59
        l1 copy kind=let @71..94
      Param l0 value @47..59
        Type Point -> s2 @54..59
      ReturnType
        Type Bool -> builtin:Bool @64..68
      Block @69..109
        Let l1 copy @71..94
          Type
            Type Point -> s2 @81..86
          Value
            Path value -> l0 @89..94
        Return @96..107
          Literal bool(true) @103..107"
        );
    }

    #[test]
    fn reports_duplicate_top_level_names() {
        let source = SourceFile::new(
            SourceId::new(1),
            "dup.mao",
            "module demo\nstruct Point {}\nfn Point() {}\n",
        );

        let result = resolve_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_DUPLICATE_NAME
                && diagnostic.message.contains("Point")));
    }

    #[test]
    fn reports_invalid_imports() {
        let source = SourceFile::new(
            SourceId::new(1),
            "bad_import.mao",
            "module demo\nimport missing.Symbol\nstruct Point {}\n",
        );

        let result = resolve_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_INVALID_IMPORT
                && diagnostic.message.contains("missing.Symbol")));
    }

    #[test]
    fn reports_unresolved_paths() {
        let source = SourceFile::new(
            SourceId::new(1),
            "missing.mao",
            "module demo\nfn main() -> Missing { return value }\n",
        );

        let result = resolve_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_UNRESOLVED_NAME
                && diagnostic.message.contains("Missing")));
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_UNRESOLVED_NAME
                && diagnostic.message.contains("value")));
    }
}
