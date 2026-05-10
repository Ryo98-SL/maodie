//! Mid-level intermediate representation lowered from typed HIR.

use std::collections::HashMap;
use std::fmt;

use maodie_diagnostics::{SourceId, TextRange};

use crate::hir::{
    HirBlock, HirExpr, HirFunction, HirItemKind, HirLet, HirMatchArm, HirPackage, HirPattern,
    HirStatement, ItemId, LocalId, LocalKind, ResolvedPath, SymbolId, SymbolKind,
};
use crate::typeck::{
    ExprType, ItemType, LocalType, TypeCheckResult, TypeId, TypeKind, TypeSubstitution,
};

/// Stable MIR function identifier inside one lowered package.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MirFunctionId(usize);

impl MirFunctionId {
    /// Creates a MIR function id from a stable numeric index.
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

impl fmt::Display for MirFunctionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "f{}", self.0)
    }
}

/// Stable basic block identifier inside one MIR function.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BasicBlockId(usize);

impl BasicBlockId {
    /// Creates a basic block id from a stable numeric index.
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

impl fmt::Display for BasicBlockId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "bb{}", self.0)
    }
}

/// Stable MIR local identifier inside one MIR function.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MirLocalId(usize);

impl MirLocalId {
    /// Creates a MIR local id from a stable numeric index.
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

impl fmt::Display for MirLocalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v{}", self.0)
    }
}

/// Lowered package ready for monomorphization and backend codegen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirPackage {
    /// MIR functions in source order.
    pub functions: Vec<MirFunction>,
    /// Type shapes copied from type checking for backend-only layout decisions.
    pub types: Vec<MirTypeKind>,
    /// Symbol metadata copied from name resolution for backend symbol lookup.
    pub symbols: Vec<MirSymbol>,
    /// Generic instantiation observations carried forward for the backend.
    pub instantiations: Vec<MirInstantiation>,
}

impl MirPackage {
    /// Renders a stable dump for snapshot-style tests and handoff docs.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut dumper = MirDumper::default();
        dumper.line("MIR");

        if !self.instantiations.is_empty() {
            dumper.indented(1, "Instantiations");
            for instantiation in &self.instantiations {
                dumper.indented(
                    2,
                    format!(
                        "@{}..{} {}={}",
                        instantiation.span.start,
                        instantiation.span.end,
                        instantiation.generic,
                        instantiation.replacement
                    ),
                );
            }
        }

        for function in &self.functions {
            function.dump(&mut dumper, 1);
        }

        dumper.finish()
    }
}

/// Backend-visible type shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirTypeKind {
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

impl From<&TypeKind> for MirTypeKind {
    fn from(kind: &TypeKind) -> Self {
        match kind {
            TypeKind::Error => Self::Error,
            TypeKind::Unit => Self::Unit,
            TypeKind::I32 => Self::I32,
            TypeKind::Bool => Self::Bool,
            TypeKind::String => Self::String,
            TypeKind::Generic(name) => Self::Generic(name.clone()),
            TypeKind::Infer(name) => Self::Infer(name.clone()),
            TypeKind::Nominal { symbol, args } => Self::Nominal {
                symbol: *symbol,
                args: args.clone(),
            },
            TypeKind::Function {
                generics,
                params,
                return_type,
            } => Self::Function {
                generics: generics.clone(),
                params: params.clone(),
                return_type: *return_type,
            },
        }
    }
}

/// Backend-visible symbol metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirSymbol {
    /// Symbol id.
    pub id: SymbolId,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Fully qualified path.
    pub path: Vec<String>,
    /// Optional owning item for top-level items and child symbols.
    pub item: Option<ItemId>,
}

impl MirSymbol {
    /// Returns the symbol's final path segment.
    #[must_use]
    pub fn name(&self) -> &str {
        self.path.last().map_or("", String::as_str)
    }
}

/// Generic substitution preserved as a backend instantiation entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirInstantiation {
    /// Source expression span that caused the instantiation.
    pub span: TextRange,
    /// Generic parameter name.
    pub generic: String,
    /// Concrete replacement type.
    pub replacement: TypeId,
}

impl From<&TypeSubstitution> for MirInstantiation {
    fn from(substitution: &TypeSubstitution) -> Self {
        Self {
            span: substitution.span,
            generic: substitution.generic.clone(),
            replacement: substitution.replacement,
        }
    }
}

/// Lowered function body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirFunction {
    /// MIR function id.
    pub id: MirFunctionId,
    /// Parent HIR item id.
    pub item: ItemId,
    /// Function symbol for top-level functions, if one exists.
    pub symbol: Option<SymbolId>,
    /// Source file id.
    pub source_id: SourceId,
    /// Function name.
    pub name: String,
    /// Return type.
    pub return_type: Option<TypeId>,
    /// MIR locals in deterministic allocation order.
    pub locals: Vec<MirLocal>,
    /// Basic blocks in deterministic allocation order.
    pub blocks: Vec<BasicBlock>,
    /// Source span.
    pub span: TextRange,
}

impl MirFunction {
    fn dump(&self, dumper: &mut MirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!(
                "Function {} item={} symbol={} {} source={} return={} @{}..{}",
                self.id,
                self.item,
                self.symbol
                    .map_or_else(|| "_".to_owned(), |symbol| symbol.to_string()),
                self.name,
                self.source_id.get(),
                format_type(self.return_type),
                self.span.start,
                self.span.end
            ),
        );

        if !self.locals.is_empty() {
            dumper.indented(indent + 1, "Locals");
            for local in &self.locals {
                let hir = local
                    .hir_local
                    .map_or_else(|| "_".to_owned(), |local| local.to_string());
                dumper.indented(
                    indent + 2,
                    format!(
                        "{} {} {} type={} hir={} @{}..{}",
                        local.id,
                        local.kind.as_str(),
                        local.name,
                        format_type(local.ty),
                        hir,
                        local.span.start,
                        local.span.end
                    ),
                );
            }
        }

        for block in &self.blocks {
            block.dump(dumper, indent + 1);
        }
    }
}

/// MIR local declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirLocal {
    /// Local id.
    pub id: MirLocalId,
    /// Source or generated name.
    pub name: String,
    /// Local kind.
    pub kind: MirLocalKind,
    /// HIR local this MIR local mirrors, if any.
    pub hir_local: Option<LocalId>,
    /// Typed local or temporary type.
    pub ty: Option<TypeId>,
    /// Source span.
    pub span: TextRange,
}

/// MIR local kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirLocalKind {
    /// Function parameter.
    Param,
    /// `let` binding.
    Let,
    /// Pattern binding.
    Pattern,
    /// Compiler-generated temporary.
    Temp,
}

impl MirLocalKind {
    /// Returns the stable dump string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Param => "param",
            Self::Let => "let",
            Self::Pattern => "pattern",
            Self::Temp => "temp",
        }
    }
}

impl From<LocalKind> for MirLocalKind {
    fn from(kind: LocalKind) -> Self {
        match kind {
            LocalKind::Param => Self::Param,
            LocalKind::Let => Self::Let,
            LocalKind::Pattern => Self::Pattern,
        }
    }
}

/// A MIR basic block with straight-line statements and one terminator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicBlock {
    /// Block id.
    pub id: BasicBlockId,
    /// Statements before the terminator.
    pub statements: Vec<MirStatement>,
    /// Explicit control-flow terminator.
    pub terminator: Option<MirTerminator>,
    /// Source span most closely associated with this block.
    pub span: TextRange,
}

impl BasicBlock {
    fn dump(&self, dumper: &mut MirDumper, indent: usize) {
        dumper.indented(
            indent,
            format!("{} @{}..{}", self.id, self.span.start, self.span.end),
        );
        for statement in &self.statements {
            statement.dump(dumper, indent + 1);
        }
        if let Some(terminator) = &self.terminator {
            terminator.dump(dumper, indent + 1);
        } else {
            dumper.indented(indent + 1, "unreachable");
        }
    }
}

/// MIR statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirStatement {
    /// Assign an rvalue to a local place.
    Assign {
        /// Destination place.
        place: MirPlace,
        /// Source value.
        value: MirRvalue,
        /// Source span.
        span: TextRange,
    },
}

impl MirStatement {
    fn dump(&self, dumper: &mut MirDumper, indent: usize) {
        match self {
            Self::Assign { place, value, span } => {
                dumper.indented(
                    indent,
                    format!(
                        "{} = {} @{}..{}",
                        place.display(),
                        value.display(),
                        span.start,
                        span.end
                    ),
                );
            }
        }
    }
}

/// MIR terminator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirTerminator {
    /// Jump to another block.
    Goto {
        /// Target block.
        target: BasicBlockId,
        /// Source span.
        span: TextRange,
    },
    /// Return from the current function.
    Return {
        /// Optional returned operand.
        value: Option<MirOperand>,
        /// Source span.
        span: TextRange,
    },
    /// Branch on a bool condition.
    Branch {
        /// Bool condition.
        condition: MirOperand,
        /// Target when true.
        then_target: BasicBlockId,
        /// Target when false.
        else_target: BasicBlockId,
        /// Source span.
        span: TextRange,
    },
    /// Branch on literal or enum-variant patterns.
    Match {
        /// Matched value.
        scrutinee: MirOperand,
        /// Ordered branch targets.
        targets: Vec<MirMatchTarget>,
        /// Source span.
        span: TextRange,
    },
}

impl MirTerminator {
    fn dump(&self, dumper: &mut MirDumper, indent: usize) {
        match self {
            Self::Goto { target, span } => {
                dumper.indented(
                    indent,
                    format!("goto {target} @{}..{}", span.start, span.end),
                );
            }
            Self::Return { value, span } => {
                let value = value
                    .as_ref()
                    .map_or_else(|| "_".to_owned(), MirOperand::display);
                dumper.indented(
                    indent,
                    format!("return {value} @{}..{}", span.start, span.end),
                );
            }
            Self::Branch {
                condition,
                then_target,
                else_target,
                span,
            } => {
                dumper.indented(
                    indent,
                    format!(
                        "branch {} ? {} : {} @{}..{}",
                        condition.display(),
                        then_target,
                        else_target,
                        span.start,
                        span.end
                    ),
                );
            }
            Self::Match {
                scrutinee,
                targets,
                span,
            } => {
                let targets = targets
                    .iter()
                    .map(MirMatchTarget::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                dumper.indented(
                    indent,
                    format!(
                        "match {} [{}] @{}..{}",
                        scrutinee.display(),
                        targets,
                        span.start,
                        span.end
                    ),
                );
            }
        }
    }
}

/// One match-like branch target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MirMatchTarget {
    /// Pattern tested by this target.
    pub pattern: MirBranchPattern,
    /// Block entered when the pattern matches.
    pub target: BasicBlockId,
}

impl MirMatchTarget {
    fn display(&self) -> String {
        format!("{} -> {}", self.pattern.display(), self.target)
    }
}

/// MIR branch pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirBranchPattern {
    /// Match all remaining values.
    Wildcard,
    /// Match one literal dump string.
    Literal(String),
    /// Match one enum variant.
    Variant(SymbolId),
}

impl MirBranchPattern {
    fn display(&self) -> String {
        match self {
            Self::Wildcard => "_".to_owned(),
            Self::Literal(text) => format!("literal({text})"),
            Self::Variant(symbol) => format!("variant({symbol})"),
        }
    }
}

/// MIR place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirPlace {
    /// Local place.
    Local(MirLocalId),
}

impl MirPlace {
    fn display(&self) -> String {
        match self {
            Self::Local(local) => local.to_string(),
        }
    }
}

/// MIR operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirOperand {
    /// Copy a place.
    Copy(MirPlace),
    /// Constant value.
    Const(MirConstant),
    /// Function item.
    Function(SymbolId),
    /// Enum variant value or constructor.
    Variant(SymbolId),
}

impl MirOperand {
    fn display(&self) -> String {
        match self {
            Self::Copy(place) => format!("copy {}", place.display()),
            Self::Const(constant) => constant.display(),
            Self::Function(symbol) => format!("fn({symbol})"),
            Self::Variant(symbol) => format!("variant({symbol})"),
        }
    }
}

/// MIR constant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirConstant {
    /// Unit value.
    Unit,
    /// Literal text as recorded in HIR.
    Literal(String),
}

impl MirConstant {
    fn display(&self) -> String {
        match self {
            Self::Unit => "unit".to_owned(),
            Self::Literal(text) => format!("const({text})"),
        }
    }
}

/// MIR rvalue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MirRvalue {
    /// Use an operand directly.
    Use(MirOperand),
    /// Function or variant call.
    Call {
        /// Callable operand.
        callee: MirOperand,
        /// Argument operands.
        args: Vec<MirOperand>,
    },
    /// Binary operation.
    Binary {
        /// Operator text.
        op: &'static str,
        /// Left operand.
        left: MirOperand,
        /// Right operand.
        right: MirOperand,
    },
    /// Build an enum variant value.
    AggregateVariant {
        /// Variant symbol.
        variant: SymbolId,
        /// Field operands.
        fields: Vec<MirOperand>,
    },
    /// Project one field from a matched enum variant.
    ProjectVariant {
        /// Source enum value.
        source: MirOperand,
        /// Known variant symbol.
        variant: SymbolId,
        /// Field index.
        field: usize,
    },
}

impl MirRvalue {
    fn display(&self) -> String {
        match self {
            Self::Use(operand) => operand.display(),
            Self::Call { callee, args } => {
                let args = args
                    .iter()
                    .map(MirOperand::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("call {}({args})", callee.display())
            }
            Self::Binary { op, left, right } => {
                format!("binary {op} {}, {}", left.display(), right.display())
            }
            Self::AggregateVariant { variant, fields } => {
                let fields = fields
                    .iter()
                    .map(MirOperand::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("aggregate variant({variant})({fields})")
            }
            Self::ProjectVariant {
                source,
                variant,
                field,
            } => {
                format!("project {} as variant({variant}).{field}", source.display())
            }
        }
    }
}

/// Typed HIR to MIR lowering facade.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MirLowerer;

impl MirLowerer {
    /// Creates a MIR lowerer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Lowers a typed package to MIR.
    #[must_use]
    pub fn lower(&self, typed: &TypeCheckResult) -> MirPackage {
        let type_index = TypeIndex::new(
            &typed.type_table.items,
            &typed.type_table.locals,
            &typed.type_table.expressions,
        );
        let mut functions = Vec::new();

        for module in &typed.package.modules {
            for item in &module.items {
                match &item.kind {
                    HirItemKind::Function(function) => {
                        let id = MirFunctionId::new(functions.len());
                        functions.push(
                            FunctionLowerer::new(
                                id,
                                item.id,
                                Some(item.symbol),
                                module.source_id,
                                function,
                                typed,
                                &type_index,
                            )
                            .lower(),
                        );
                    }
                    HirItemKind::Impl(impl_) => {
                        for method in &impl_.methods {
                            let id = MirFunctionId::new(functions.len());
                            functions.push(
                                FunctionLowerer::new(
                                    id,
                                    item.id,
                                    None,
                                    module.source_id,
                                    method,
                                    typed,
                                    &type_index,
                                )
                                .lower(),
                            );
                        }
                    }
                    HirItemKind::Struct(_) | HirItemKind::Enum(_) | HirItemKind::Trait(_) => {}
                }
            }
        }

        let mir_types = typed
            .type_table
            .types
            .iter()
            .map(MirTypeKind::from)
            .collect();
        let symbols = typed
            .package
            .symbols
            .iter()
            .map(|symbol| MirSymbol {
                id: symbol.id,
                kind: symbol.kind,
                path: symbol.path.clone(),
                item: symbol.item,
            })
            .collect();
        let instantiations = typed
            .type_table
            .substitutions
            .iter()
            .map(MirInstantiation::from)
            .collect();

        MirPackage {
            functions,
            types: mir_types,
            symbols,
            instantiations,
        }
    }
}

/// Lowers a typed package to MIR.
#[must_use]
pub fn lower_package(typed: &TypeCheckResult) -> MirPackage {
    MirLowerer::new().lower(typed)
}

#[derive(Clone, Debug)]
struct TypeIndex {
    items: HashMap<SymbolId, TypeId>,
    locals: HashMap<(String, LocalId), TypeId>,
    expressions: HashMap<(String, TextRangeKey), TypeId>,
}

impl TypeIndex {
    fn new(items: &[ItemType], locals: &[LocalType], expressions: &[ExprType]) -> Self {
        Self {
            items: items.iter().map(|item| (item.symbol, item.ty)).collect(),
            locals: locals
                .iter()
                .map(|local| ((local.owner.clone(), local.local), local.ty))
                .collect(),
            expressions: expressions
                .iter()
                .map(|expr| ((expr.owner.clone(), TextRangeKey::from(expr.span)), expr.ty))
                .collect(),
        }
    }

    fn local_type(&self, owner: &str, local: LocalId) -> Option<TypeId> {
        self.locals.get(&(owner.to_owned(), local)).copied()
    }

    fn expr_type(&self, owner: &str, span: TextRange) -> Option<TypeId> {
        self.expressions
            .get(&(owner.to_owned(), TextRangeKey::from(span)))
            .copied()
    }

    fn item_type(&self, symbol: SymbolId) -> Option<TypeId> {
        self.items.get(&symbol).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TextRangeKey {
    start: usize,
    end: usize,
}

impl From<TextRange> for TextRangeKey {
    fn from(span: TextRange) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

struct FunctionLowerer<'typed, 'index> {
    id: MirFunctionId,
    item: ItemId,
    symbol: Option<SymbolId>,
    source_id: SourceId,
    function: &'typed HirFunction,
    package: &'typed HirPackage,
    types: &'typed [TypeKind],
    type_index: &'index TypeIndex,
    locals: Vec<MirLocal>,
    local_map: HashMap<LocalId, MirLocalId>,
    blocks: Vec<BasicBlock>,
    current: BasicBlockId,
}

impl<'typed, 'index> FunctionLowerer<'typed, 'index> {
    fn new(
        id: MirFunctionId,
        item: ItemId,
        symbol: Option<SymbolId>,
        source_id: SourceId,
        function: &'typed HirFunction,
        typed: &'typed TypeCheckResult,
        type_index: &'index TypeIndex,
    ) -> Self {
        let mut lowerer = Self {
            id,
            item,
            symbol,
            source_id,
            function,
            package: &typed.package,
            types: &typed.type_table.types,
            type_index,
            locals: Vec::new(),
            local_map: HashMap::new(),
            blocks: Vec::new(),
            current: BasicBlockId::new(0),
        };
        lowerer.alloc_hir_locals();
        lowerer.current = lowerer.new_block(function.span);
        lowerer
    }

    fn lower(mut self) -> MirFunction {
        let return_type = self
            .function
            .body
            .as_ref()
            .and_then(|body| self.type_index.expr_type(&self.function.name, body.span))
            .or_else(|| self.return_type_from_item());

        if let Some(body) = &self.function.body {
            let tail = self.lower_block(body);
            if !self.current_is_terminated() {
                self.set_terminator(MirTerminator::Return {
                    value: tail,
                    span: body.span,
                });
            }
        } else if !self.current_is_terminated() {
            self.set_terminator(MirTerminator::Return {
                value: None,
                span: self.function.span,
            });
        }

        MirFunction {
            id: self.id,
            item: self.item,
            symbol: self.symbol,
            source_id: self.source_id,
            name: self.function.name.clone(),
            return_type,
            locals: self.locals,
            blocks: self.blocks,
            span: self.function.span,
        }
    }

    fn alloc_hir_locals(&mut self) {
        for local in &self.function.locals {
            let id = MirLocalId::new(self.locals.len());
            self.local_map.insert(local.id, id);
            self.locals.push(MirLocal {
                id,
                name: local.name.clone(),
                kind: MirLocalKind::from(local.kind),
                hir_local: Some(local.id),
                ty: self.type_index.local_type(&self.function.name, local.id),
                span: local.span,
            });
        }
    }

    fn return_type_from_item(&self) -> Option<TypeId> {
        let symbol = self.symbol?;
        let ty = self.type_index.item_type(symbol)?;
        let TypeKind::Function { return_type, .. } = self.types.get(ty.get())? else {
            return None;
        };
        Some(*return_type)
    }

    fn lower_block(&mut self, block: &HirBlock) -> Option<MirOperand> {
        let mut tail = None;
        let last_index = block.statements.len().saturating_sub(1);

        for (index, statement) in block.statements.iter().enumerate() {
            if self.current_is_terminated() {
                break;
            }

            tail = match statement {
                HirStatement::Let(statement) => {
                    self.lower_let(statement);
                    None
                }
                HirStatement::Return { expr, span } => {
                    let value = expr.as_ref().map(|expr| self.lower_expr(expr));
                    self.set_terminator(MirTerminator::Return { value, span: *span });
                    None
                }
                HirStatement::Expr(expr) => {
                    let value = self.lower_expr(expr);
                    if index == last_index {
                        Some(value)
                    } else {
                        None
                    }
                }
            };
        }

        tail
    }

    fn lower_let(&mut self, statement: &HirLet) {
        let Some(local) = statement
            .local
            .and_then(|local| self.local_map.get(&local).copied())
        else {
            if let Some(value) = &statement.value {
                self.lower_expr(value);
            }
            return;
        };

        if let Some(value) = &statement.value {
            let value = self.lower_expr(value);
            self.push_assign(
                MirPlace::Local(local),
                MirRvalue::Use(value),
                statement.span,
            );
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> MirOperand {
        match expr {
            HirExpr::Missing { span } => {
                let temp = self.alloc_temp("_missing", self.expr_type(expr), *span);
                MirOperand::Copy(MirPlace::Local(temp))
            }
            HirExpr::Literal { text, .. } => MirOperand::Const(MirConstant::Literal(text.clone())),
            HirExpr::Path { resolved, span, .. } => self.lower_path(resolved.as_ref(), *span),
            HirExpr::Call { callee, args, span } => {
                let callee = self.lower_expr(callee);
                let args = args.iter().map(|arg| self.lower_expr(arg)).collect();
                let temp = self.alloc_temp("call", self.expr_type(expr), *span);
                self.push_assign(
                    MirPlace::Local(temp),
                    MirRvalue::Call { callee, args },
                    *span,
                );
                MirOperand::Copy(MirPlace::Local(temp))
            }
            HirExpr::Block(block) => self
                .lower_block(block)
                .unwrap_or(MirOperand::Const(MirConstant::Unit)),
            HirExpr::If {
                condition,
                then_block,
                else_branch,
                span,
            } => self.lower_if(condition, then_block, else_branch.as_deref(), *span, expr),
            HirExpr::Match {
                scrutinee,
                arms,
                span,
            } => self.lower_match(scrutinee, arms, *span, expr),
            HirExpr::Binary {
                op,
                left,
                right,
                span,
            } if *op == "=" => self.lower_assignment(left, right, *span, expr),
            HirExpr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left = self.lower_expr(left);
                let right = self.lower_expr(right);
                let temp = self.alloc_temp("binary", self.expr_type(expr), *span);
                self.push_assign(
                    MirPlace::Local(temp),
                    MirRvalue::Binary { op, left, right },
                    *span,
                );
                MirOperand::Copy(MirPlace::Local(temp))
            }
            HirExpr::Try { expr: inner, span } => self.lower_try(inner, *span, expr),
        }
    }

    fn lower_path(&mut self, resolved: Option<&ResolvedPath>, span: TextRange) -> MirOperand {
        match resolved {
            Some(ResolvedPath::Local(local)) => self.local_map.get(local).copied().map_or_else(
                || self.unit_temp(span),
                |local| MirOperand::Copy(MirPlace::Local(local)),
            ),
            Some(ResolvedPath::Symbol(symbol)) => self.lower_symbol_operand(*symbol),
            Some(ResolvedPath::Builtin(_) | ResolvedPath::Generic(_)) | None => {
                self.unit_temp(span)
            }
        }
    }

    fn lower_symbol_operand(&self, symbol: SymbolId) -> MirOperand {
        let Some(symbol_info) = self.package.symbols.get(symbol.get()) else {
            return MirOperand::Variant(symbol);
        };
        match symbol_info.kind {
            SymbolKind::Function => MirOperand::Function(symbol),
            SymbolKind::Module
            | SymbolKind::Struct
            | SymbolKind::Enum
            | SymbolKind::Variant
            | SymbolKind::Trait
            | SymbolKind::Impl => MirOperand::Variant(symbol),
        }
    }

    fn lower_assignment(
        &mut self,
        left: &HirExpr,
        right: &HirExpr,
        span: TextRange,
        expr: &HirExpr,
    ) -> MirOperand {
        let value = self.lower_expr(right);
        if let HirExpr::Path {
            resolved: Some(ResolvedPath::Local(local)),
            ..
        } = left
        {
            if let Some(local) = self.local_map.get(local).copied() {
                self.push_assign(MirPlace::Local(local), MirRvalue::Use(value), span);
            }
        } else {
            self.lower_expr(left);
        }

        let temp = self.alloc_temp("assign", self.expr_type(expr), span);
        self.push_assign(
            MirPlace::Local(temp),
            MirRvalue::Use(MirOperand::Const(MirConstant::Unit)),
            span,
        );
        MirOperand::Copy(MirPlace::Local(temp))
    }

    fn lower_if(
        &mut self,
        condition: &HirExpr,
        then_block: &HirBlock,
        else_branch: Option<&HirExpr>,
        span: TextRange,
        expr: &HirExpr,
    ) -> MirOperand {
        let condition = self.lower_expr(condition);
        let result = self.alloc_temp("if", self.expr_type(expr), span);
        let then_target = self.new_block(then_block.span);
        let else_target = self.new_block(else_branch.map_or(span, expr_span));
        let join_target = self.new_block(span);

        self.set_terminator(MirTerminator::Branch {
            condition,
            then_target,
            else_target,
            span,
        });

        self.current = then_target;
        if let Some(value) = self.lower_block(then_block) {
            self.assign_result_and_goto(result, value, join_target, then_block.span);
        } else if !self.current_is_terminated() {
            self.assign_result_and_goto(
                result,
                MirOperand::Const(MirConstant::Unit),
                join_target,
                then_block.span,
            );
        }

        self.current = else_target;
        if let Some(else_branch) = else_branch {
            let value = self.lower_expr(else_branch);
            if !self.current_is_terminated() {
                self.assign_result_and_goto(result, value, join_target, expr_span(else_branch));
            }
        } else {
            self.assign_result_and_goto(
                result,
                MirOperand::Const(MirConstant::Unit),
                join_target,
                span,
            );
        }

        self.current = join_target;
        MirOperand::Copy(MirPlace::Local(result))
    }

    fn lower_match(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        span: TextRange,
        expr: &HirExpr,
    ) -> MirOperand {
        let scrutinee = self.lower_expr(scrutinee);
        let result = self.alloc_temp("match", self.expr_type(expr), span);
        let join_target = self.new_block(span);
        let arm_blocks = arms
            .iter()
            .map(|arm| (self.new_block(arm.span), arm))
            .collect::<Vec<_>>();
        let targets = arm_blocks
            .iter()
            .map(|(target, arm)| MirMatchTarget {
                pattern: branch_pattern(&arm.pattern),
                target: *target,
            })
            .collect();

        self.set_terminator(MirTerminator::Match {
            scrutinee: scrutinee.clone(),
            targets,
            span,
        });

        for (target, arm) in arm_blocks {
            self.current = target;
            self.bind_match_pattern(&arm.pattern, &scrutinee);
            let value = self.lower_expr(&arm.expr);
            if !self.current_is_terminated() {
                self.assign_result_and_goto(result, value, join_target, arm.span);
            }
        }

        self.current = join_target;
        MirOperand::Copy(MirPlace::Local(result))
    }

    fn bind_match_pattern(&mut self, pattern: &HirPattern, scrutinee: &MirOperand) {
        if let HirPattern::Binding { local, span, .. } = pattern {
            if let Some(local) = self.local_map.get(local).copied() {
                self.push_assign(
                    MirPlace::Local(local),
                    MirRvalue::Use(scrutinee.clone()),
                    *span,
                );
            }
        }
    }

    fn lower_try(&mut self, inner: &HirExpr, span: TextRange, expr: &HirExpr) -> MirOperand {
        let result_value = self.lower_expr(inner);
        let ok_target = self.new_block(span);
        let err_target = self.new_block(span);
        let continue_target = self.new_block(span);
        let ok_temp = self.alloc_temp("try_ok", self.expr_type(expr), span);

        let Some((_, _, ok_variant, err_variant)) = self.result_shape_for_expr(inner) else {
            self.set_terminator(MirTerminator::Goto {
                target: ok_target,
                span,
            });
            self.current = ok_target;
            self.assign_result_and_goto(ok_temp, result_value, continue_target, span);
            self.current = continue_target;
            return MirOperand::Copy(MirPlace::Local(ok_temp));
        };

        self.set_terminator(MirTerminator::Match {
            scrutinee: result_value.clone(),
            targets: vec![
                MirMatchTarget {
                    pattern: MirBranchPattern::Variant(ok_variant),
                    target: ok_target,
                },
                MirMatchTarget {
                    pattern: MirBranchPattern::Variant(err_variant),
                    target: err_target,
                },
            ],
            span,
        });

        self.current = ok_target;
        self.push_assign(
            MirPlace::Local(ok_temp),
            MirRvalue::ProjectVariant {
                source: result_value.clone(),
                variant: ok_variant,
                field: 0,
            },
            span,
        );
        self.set_terminator(MirTerminator::Goto {
            target: continue_target,
            span,
        });

        self.current = err_target;
        let err_ty = self
            .result_shape_for_expr(inner)
            .map(|(_, err_ty, _, _)| err_ty);
        let err_temp = self.alloc_temp("try_err", err_ty, span);
        self.push_assign(
            MirPlace::Local(err_temp),
            MirRvalue::ProjectVariant {
                source: result_value,
                variant: err_variant,
                field: 0,
            },
            span,
        );
        let return_temp =
            self.alloc_temp("try_return", self.return_type_of_current_function(), span);
        self.push_assign(
            MirPlace::Local(return_temp),
            MirRvalue::AggregateVariant {
                variant: err_variant,
                fields: vec![MirOperand::Copy(MirPlace::Local(err_temp))],
            },
            span,
        );
        self.set_terminator(MirTerminator::Return {
            value: Some(MirOperand::Copy(MirPlace::Local(return_temp))),
            span,
        });

        self.current = continue_target;
        MirOperand::Copy(MirPlace::Local(ok_temp))
    }

    fn result_shape_for_expr(
        &self,
        expr: &HirExpr,
    ) -> Option<(TypeId, TypeId, SymbolId, SymbolId)> {
        let ty = self.expr_type(expr)?;
        let TypeKind::Nominal { symbol, args } = self.types.get(ty.get())? else {
            return None;
        };
        if args.len() != 2 || !self.symbol_named(*symbol, "Result") {
            return None;
        }
        let ok_variant = self.variant_named(*symbol, "Ok")?;
        let err_variant = self.variant_named(*symbol, "Err")?;
        Some((args[0], args[1], ok_variant, err_variant))
    }

    fn return_type_of_current_function(&self) -> Option<TypeId> {
        self.return_type_from_item().or_else(|| {
            self.function
                .body
                .as_ref()
                .and_then(|body| self.type_index.expr_type(&self.function.name, body.span))
        })
    }

    fn symbol_named(&self, symbol: SymbolId, name: &str) -> bool {
        self.package
            .symbols
            .get(symbol.get())
            .and_then(|symbol| symbol.path.last())
            .is_some_and(|last| last == name)
    }

    fn variant_named(&self, enum_symbol: SymbolId, name: &str) -> Option<SymbolId> {
        let item = self.package.symbols.get(enum_symbol.get())?.item?;
        self.package
            .symbols
            .iter()
            .find(|symbol| {
                symbol.kind == SymbolKind::Variant
                    && symbol.item == Some(item)
                    && symbol.path.last().is_some_and(|last| last == name)
            })
            .map(|symbol| symbol.id)
    }

    fn assign_result_and_goto(
        &mut self,
        result: MirLocalId,
        value: MirOperand,
        target: BasicBlockId,
        span: TextRange,
    ) {
        self.push_assign(MirPlace::Local(result), MirRvalue::Use(value), span);
        self.set_terminator(MirTerminator::Goto { target, span });
    }

    fn unit_temp(&mut self, span: TextRange) -> MirOperand {
        let temp = self.alloc_temp("unit", None, span);
        self.push_assign(
            MirPlace::Local(temp),
            MirRvalue::Use(MirOperand::Const(MirConstant::Unit)),
            span,
        );
        MirOperand::Copy(MirPlace::Local(temp))
    }

    fn alloc_temp(&mut self, name: &str, ty: Option<TypeId>, span: TextRange) -> MirLocalId {
        let id = MirLocalId::new(self.locals.len());
        self.locals.push(MirLocal {
            id,
            name: format!("_{name}{}", id.get()),
            kind: MirLocalKind::Temp,
            hir_local: None,
            ty,
            span,
        });
        id
    }

    fn push_assign(&mut self, place: MirPlace, value: MirRvalue, span: TextRange) {
        self.current_block_mut()
            .statements
            .push(MirStatement::Assign { place, value, span });
    }

    fn new_block(&mut self, span: TextRange) -> BasicBlockId {
        let id = BasicBlockId::new(self.blocks.len());
        self.blocks.push(BasicBlock {
            id,
            statements: Vec::new(),
            terminator: None,
            span,
        });
        id
    }

    fn set_terminator(&mut self, terminator: MirTerminator) {
        self.current_block_mut().terminator = Some(terminator);
    }

    fn current_is_terminated(&self) -> bool {
        self.blocks
            .get(self.current.get())
            .is_none_or(|block| block.terminator.is_some())
    }

    fn current_block_mut(&mut self) -> &mut BasicBlock {
        self.blocks
            .get_mut(self.current.get())
            .expect("current MIR block must exist")
    }

    fn expr_type(&self, expr: &HirExpr) -> Option<TypeId> {
        self.type_index
            .expr_type(&self.function.name, expr_span(expr))
    }
}

fn branch_pattern(pattern: &HirPattern) -> MirBranchPattern {
    match pattern {
        HirPattern::Literal { text, .. } => MirBranchPattern::Literal(text.clone()),
        HirPattern::Path {
            resolved: Some(ResolvedPath::Symbol(symbol)),
            ..
        } => MirBranchPattern::Variant(*symbol),
        HirPattern::Wildcard { .. } | HirPattern::Binding { .. } | HirPattern::Path { .. } => {
            MirBranchPattern::Wildcard
        }
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

fn format_type(ty: Option<TypeId>) -> String {
    ty.map_or_else(|| "_".to_owned(), |ty| ty.to_string())
}

#[derive(Default)]
struct MirDumper {
    lines: Vec<String>,
}

impl MirDumper {
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

    use super::lower_package;
    use crate::typeck::check_source;

    #[test]
    fn lowers_if_expression_to_branch_blocks() {
        let source = SourceFile::new(
            SourceId::new(1),
            "if.mao",
            "\
module demo
fn choose(flag: bool) -> i32 {
  if flag { 1 } else { 2 }
}
",
        );

        let typed = check_source(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);

        let mir = lower_package(&typed);
        let dump = mir.dump();

        assert!(dump.contains("Function f0"));
        assert!(dump.contains("branch copy v0 ? bb1 : bb2"));
        assert!(dump.contains("goto bb3"));
        assert!(dump.contains("return copy"));
    }

    #[test]
    fn lowers_match_expression_to_variant_targets() {
        let source = SourceFile::new(
            SourceId::new(1),
            "match.mao",
            "\
module demo
enum Color { Red, Green }
fn score(color: Color) -> i32 {
  match color {
    Color.Red => 1,
    Color.Green => 2
  }
}
",
        );

        let typed = check_source(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);

        let dump = lower_package(&typed).dump();

        assert!(dump.contains("match copy v0 [variant("));
        assert!(dump.contains("-> bb"));
        assert!(dump.contains("const(int(1))"));
        assert!(dump.contains("const(int(2))"));
    }

    #[test]
    fn lowers_try_to_result_branch_and_early_return() {
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

        let typed = check_source(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);

        let dump = lower_package(&typed).dump();

        assert!(dump.contains("match copy"));
        assert!(dump.contains("project copy"));
        assert!(dump.contains("aggregate variant("));
        assert!(dump.contains("return copy"));
    }
}
