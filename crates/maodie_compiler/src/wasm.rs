//! WASM v1 backend from MIR.
//!
//! The backend is deliberately MIR-only: codegen consumes [`MirPackage`] plus
//! the backend-visible metadata copied into MIR lowering, and never reads HIR or
//! AST expression shapes.

use std::collections::HashMap;
use std::fmt;
use std::fmt::Write as _;

use crate::core::{
    WASM_HOST_MODULE, WASM_IMPORT_DEBUG_BOOL, WASM_IMPORT_DEBUG_I32, WASM_IMPORT_DEBUG_LOG_END,
    WASM_IMPORT_DEBUG_STRING, WASM_IMPORT_PANIC, WASM_MEMORY_EXPORT,
};
use crate::hir::{SymbolId, SymbolKind};
use crate::log_format::{parse_log_format, string_literal_value, LogFormat};
use crate::mir::{
    BasicBlock, BasicBlockId, MirBranchPattern, MirConstant, MirFunction, MirLocalId, MirLocalKind,
    MirOperand, MirPackage, MirPlace, MirRvalue, MirStatement, MirTerminator, MirTypeKind,
};

/// Stable WAT debug dump artifact name.
pub const WAT_DUMP_NAME: &str = "module.wat";
/// Stable WASM binary artifact name.
pub const WASM_BINARY_NAME: &str = "module.wasm";

const IMPORT_COUNT: u32 = 5;
const DEBUG_STRING_IMPORT_INDEX: u32 = 1;
const DEBUG_I32_IMPORT_INDEX: u32 = 2;
const DEBUG_BOOL_IMPORT_INDEX: u32 = 3;
const DEBUG_LOG_END_IMPORT_INDEX: u32 = 4;
const VARIANT_TAG_MASK: i32 = 0xff;
const VARIANT_PAYLOAD_SHIFT: i32 = 8;
const STRING_DATA_BASE: u32 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmValueType {
    I32,
    I64,
}

impl WasmValueType {
    fn wat(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
        }
    }

    fn binary(self) -> u8 {
        match self {
            Self::I32 => 0x7f,
            Self::I64 => 0x7e,
        }
    }
}

/// Structured output from the MIR-to-WASM backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmArtifacts {
    /// Stable WAT debug output.
    pub wat: String,
    /// Encoded WebAssembly binary module.
    pub wasm: Vec<u8>,
    /// Backend artifact fields for task 11 handoff.
    pub artifact_names: WasmArtifactNames,
    /// Non-fatal backend diagnostics and v1 limitations.
    pub diagnostics: Vec<WasmDiagnostic>,
}

/// Artifact field names exported by the v1 backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmArtifactNames {
    /// WAT debug dump name.
    pub wat_dump: &'static str,
    /// WASM binary artifact name.
    pub wasm_binary: &'static str,
}

impl Default for WasmArtifactNames {
    fn default() -> Self {
        Self {
            wat_dump: WAT_DUMP_NAME,
            wasm_binary: WASM_BINARY_NAME,
        }
    }
}

/// Backend diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmDiagnostic {
    /// Human-readable diagnostic text.
    pub message: String,
}

impl WasmDiagnostic {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Compile one MIR package into WAT and WASM artifacts.
#[must_use]
pub fn compile_mir_to_wasm(package: &MirPackage) -> WasmArtifacts {
    WasmBackend::new().compile(package)
}

/// MIR-to-WASM backend facade.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WasmBackend;

impl WasmBackend {
    /// Creates a WASM backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Compiles a MIR package into WAT and WASM artifacts.
    #[must_use]
    pub fn compile(&self, package: &MirPackage) -> WasmArtifacts {
        let mut diagnostics = vec![WasmDiagnostic::new(
            "v1 WASM layout maps i32, bool, Slice handles, and enum values to i32; direct String values use packed i64 ptr/len handles; managed allocation and GC are not emitted.",
        )];
        let layout = Layout::new(package);
        let strings = StringTable::new(package);
        let wat = WatWriter::new(package, &layout, &strings, &mut diagnostics).finish();
        let wasm = BinaryWriter::new(package, &layout, &strings, &mut diagnostics).finish();

        WasmArtifacts {
            wat,
            wasm,
            artifact_names: WasmArtifactNames::default(),
            diagnostics,
        }
    }
}

/// Result returned by the MIR execution helper used by backend golden tests.
pub type WasmRunResult = Result<i32, WasmRunError>;

/// Execution error from the MIR execution helper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmRunError {
    message: String,
}

impl WasmRunError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WasmRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// Executes one exported MIR function with the backend's v1 i32 layout.
///
/// This helper keeps golden tests independent from a host WebAssembly runtime
/// while checking the same value representation used by codegen.
///
/// # Errors
///
/// Returns an error when the export is missing, arguments do not match, or MIR
/// execution reaches unsupported or invalid control flow.
pub fn run_i32_export(package: &MirPackage, name: &str, args: &[i32]) -> WasmRunResult {
    MirRunner::new(package).run_i32(name, args)
}

#[derive(Clone, Debug)]
struct Layout {
    function_indices: HashMap<SymbolId, u32>,
    variant_tags: HashMap<SymbolId, u32>,
}

impl Layout {
    fn new(package: &MirPackage) -> Self {
        let function_indices = package
            .functions
            .iter()
            .filter_map(|function| function.symbol.map(|symbol| (symbol, function.id.get())))
            .map(|(symbol, index)| {
                (
                    symbol,
                    u32::try_from(index).expect("function index must fit in u32") + IMPORT_COUNT,
                )
            })
            .collect();

        let mut variant_ordinals = HashMap::<_, u32>::new();
        let mut variant_tags = HashMap::new();
        for symbol in package
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Variant)
        {
            let ordinal = variant_ordinals.entry(symbol.item).or_insert(0);
            variant_tags.insert(symbol.id, *ordinal);
            *ordinal += 1;
        }

        Self {
            function_indices,
            variant_tags,
        }
    }

    fn function_index(&self, symbol: SymbolId) -> Option<u32> {
        self.function_indices.get(&symbol).copied()
    }

    fn variant_tag(&self, symbol: SymbolId) -> u32 {
        self.variant_tags
            .get(&symbol)
            .copied()
            .unwrap_or_else(|| usize_to_u32(symbol.get()))
    }
}

#[derive(Clone, Debug, Default)]
struct StringTable {
    offsets: HashMap<String, u32>,
    entries: Vec<StringEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StringEntry {
    text: String,
    offset: u32,
    bytes: Vec<u8>,
}

impl StringTable {
    fn new(package: &MirPackage) -> Self {
        let mut table = Self::default();
        for function in &package.functions {
            for block in &function.blocks {
                for statement in &block.statements {
                    table.visit_statement(statement);
                }
                if let Some(terminator) = &block.terminator {
                    table.visit_terminator(terminator);
                }
            }
        }
        table
    }

    fn intern_literal(&mut self, text: &str) {
        let Some(value) = string_literal_value(text) else {
            return;
        };
        self.intern_text(&value);
    }

    fn intern_text(&mut self, value: &str) {
        if self.offsets.contains_key(value) {
            return;
        }
        let used = self
            .entries
            .iter()
            .map(|entry| entry.bytes.len() + 1)
            .sum::<usize>();
        let offset =
            STRING_DATA_BASE + u32::try_from(used).expect("string table size must fit in u32");
        self.offsets.insert(value.to_owned(), offset);
        self.entries.push(StringEntry {
            bytes: value.as_bytes().to_vec(),
            text: value.to_owned(),
            offset,
        });
    }

    fn handle_for_literal(&self, text: &str) -> i64 {
        string_literal_value(text)
            .map(|value| self.handle_for_text(&value))
            .unwrap_or(0)
    }

    fn handle_for_text(&self, text: &str) -> i64 {
        let offset = u64::from(self.offset_for_text(text).unwrap_or(0));
        let len = u64::try_from(text.len()).expect("string length must fit in u64");
        i64::try_from((len << 32) | offset).expect("string handle must fit in i64")
    }

    fn offset_for_text(&self, text: &str) -> Option<u32> {
        self.offsets.get(text).copied()
    }

    fn visit_statement(&mut self, statement: &MirStatement) {
        let MirStatement::Assign { value, .. } = statement;
        self.visit_rvalue(value);
    }

    fn visit_terminator(&mut self, terminator: &MirTerminator) {
        match terminator {
            MirTerminator::Goto { .. } => {}
            MirTerminator::Return { value, .. } => {
                if let Some(value) = value {
                    self.visit_operand(value);
                }
            }
            MirTerminator::Branch { condition, .. } => self.visit_operand(condition),
            MirTerminator::Match {
                scrutinee, targets, ..
            } => {
                self.visit_operand(scrutinee);
                for target in targets {
                    if let MirBranchPattern::Literal(text) = &target.pattern {
                        self.intern_literal(text);
                    }
                }
            }
        }
    }

    fn visit_rvalue(&mut self, value: &MirRvalue) {
        match value {
            MirRvalue::Use(operand)
            | MirRvalue::ProjectVariant {
                source: operand, ..
            } => {
                self.visit_operand(operand);
            }
            MirRvalue::Call { callee, args } => {
                if let Some(MirOperand::Const(MirConstant::Literal(text))) = args.first() {
                    if let Some(format) = parse_log_format(text) {
                        for segment in &format.segments {
                            self.intern_text(segment);
                        }
                    }
                }
                self.visit_operand(callee);
                for arg in args {
                    self.visit_operand(arg);
                }
            }
            MirRvalue::Binary { left, right, .. } => {
                self.visit_operand(left);
                self.visit_operand(right);
            }
            MirRvalue::AggregateVariant { fields, .. } => {
                for field in fields {
                    self.visit_operand(field);
                }
            }
        }
    }

    fn visit_operand(&mut self, operand: &MirOperand) {
        if let MirOperand::Const(MirConstant::Literal(text)) = operand {
            self.intern_literal(text);
        }
    }
}

#[derive(Clone, Debug)]
struct FunctionLayout {
    locals: HashMap<MirLocalId, u32>,
    ret: u32,
    bb: u32,
    done: u32,
    matched: u32,
    string_scratch: u32,
}

impl FunctionLayout {
    fn new(function: &MirFunction) -> Self {
        let mut locals = HashMap::new();
        let mut next = 0_u32;
        for local in &function.locals {
            if local.kind == MirLocalKind::Param {
                locals.insert(local.id, next);
                next += 1;
            }
        }
        for local in &function.locals {
            if local.kind != MirLocalKind::Param {
                locals.insert(local.id, next);
                next += 1;
            }
        }
        let ret = next;
        let bb = next + 1;
        let done = next + 2;
        let matched = next + 3;
        let string_scratch = next + 4;

        Self {
            locals,
            ret,
            bb,
            done,
            matched,
            string_scratch,
        }
    }

    fn local(&self, local: MirLocalId) -> u32 {
        self.locals
            .get(&local)
            .copied()
            .expect("MIR local must have a WASM local")
    }

    fn declared_local_groups(function: &MirFunction, package: &MirPackage) -> Vec<WasmValueType> {
        let mut locals = function
            .locals
            .iter()
            .filter(|local| local.kind != MirLocalKind::Param)
            .map(|local| local_value_type(package, function, local))
            .collect::<Vec<_>>();
        locals.push(function_return_value_type(package, function).unwrap_or(WasmValueType::I32));
        locals.extend([
            WasmValueType::I32,
            WasmValueType::I32,
            WasmValueType::I32,
            WasmValueType::I64,
        ]);
        locals
    }
}

struct WatWriter<'a, 'd> {
    package: &'a MirPackage,
    layout: &'a Layout,
    strings: &'a StringTable,
    diagnostics: &'d mut Vec<WasmDiagnostic>,
    output: String,
}

impl<'a, 'd> WatWriter<'a, 'd> {
    fn new(
        package: &'a MirPackage,
        layout: &'a Layout,
        strings: &'a StringTable,
        diagnostics: &'d mut Vec<WasmDiagnostic>,
    ) -> Self {
        Self {
            package,
            layout,
            strings,
            diagnostics,
            output: String::new(),
        }
    }

    fn finish(mut self) -> String {
        self.line("(module");
        self.line(&format!(
            "  (import \"{WASM_HOST_MODULE}\" \"{WASM_IMPORT_PANIC}\" (func $__maodie_panic (param i32 i32)))"
        ));
        self.line(&format!(
            "  (import \"{WASM_HOST_MODULE}\" \"{WASM_IMPORT_DEBUG_STRING}\" (func $__maodie_debug_string (param i32 i32)))"
        ));
        self.line(&format!(
            "  (import \"{WASM_HOST_MODULE}\" \"{WASM_IMPORT_DEBUG_I32}\" (func $__maodie_debug_i32 (param i32)))"
        ));
        self.line(&format!(
            "  (import \"{WASM_HOST_MODULE}\" \"{WASM_IMPORT_DEBUG_BOOL}\" (func $__maodie_debug_bool (param i32)))"
        ));
        self.line(&format!(
            "  (import \"{WASM_HOST_MODULE}\" \"{WASM_IMPORT_DEBUG_LOG_END}\" (func $__maodie_debug_log_end))"
        ));
        self.line(&format!("  (memory (export \"{WASM_MEMORY_EXPORT}\") 1)"));

        for entry in &self.strings.entries {
            self.line(&format!(
                "  (data (i32.const {}) \"{}\")",
                entry.offset,
                escape_wat_bytes(&entry.bytes)
            ));
        }

        for function in &self.package.functions {
            self.write_function(function);
        }

        self.line(")");
        self.output
    }

    fn write_function(&mut self, function: &MirFunction) {
        let function_layout = FunctionLayout::new(function);
        let mut params = String::new();
        for local in function
            .locals
            .iter()
            .filter(|local| local.kind == MirLocalKind::Param)
        {
            write!(
                params,
                " (param $v{} {})",
                local.id.get(),
                wasm_value_type(self.package, local.ty).wat()
            )
            .expect("writing to a String cannot fail");
        }
        let result_text = function_return_value_type(self.package, function)
            .map(|ty| format!(" (result {})", ty.wat()))
            .unwrap_or_default();

        self.line(&format!(
            "  (func ${}{}{}",
            wasm_name(&function.name),
            params,
            result_text
        ));
        for local in function
            .locals
            .iter()
            .filter(|local| local.kind != MirLocalKind::Param)
        {
            self.line(&format!(
                "    (local $v{} {})",
                local.id.get(),
                local_value_type(self.package, function, local).wat()
            ));
        }
        self.line(&format!(
            "    (local $__ret {})",
            function_return_value_type(self.package, function)
                .unwrap_or(WasmValueType::I32)
                .wat()
        ));
        self.line("    (local $__bb i32)");
        self.line("    (local $__done i32)");
        self.line("    (local $__matched i32)");
        self.line("    (local $__string i64)");
        self.line("    (block $__exit");
        self.line("      (loop $__dispatch");
        self.line("        (br_if $__exit (local.get $__done))");

        for block in &function.blocks {
            self.line(&format!(
                "        (if (i32.eq (local.get $__bb) (i32.const {}))",
                block.id.get()
            ));
            self.line("          (then");
            self.write_block(block, function, &function_layout);
            self.line("            (br $__dispatch)");
            self.line("          )");
            self.line("        )");
        }

        self.line("        (br $__dispatch)");
        self.line("      )");
        self.line("    )");
        if self.function_result(function) {
            self.line("    (return (local.get $__ret))");
        } else {
            self.line("    (return)");
        }
        self.line("  )");
        self.line(&format!(
            "  (export \"{}\" (func ${}))",
            function.name,
            wasm_name(&function.name)
        ));
    }

    fn write_block(
        &mut self,
        block: &BasicBlock,
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) {
        for statement in &block.statements {
            let MirStatement::Assign { place, value, .. } = statement;
            let MirPlace::Local(local) = place;
            let value = self.rvalue(value, function, function_layout);
            self.line(&format!(
                "            (local.set $v{} {})",
                local.get(),
                value
            ));
        }

        if let Some(terminator) = &block.terminator {
            self.write_terminator(terminator, function, function_layout);
        } else {
            self.line("            (unreachable)");
        }
    }

    fn write_terminator(
        &mut self,
        terminator: &MirTerminator,
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) {
        match terminator {
            MirTerminator::Goto { target, .. } => self.set_bb(*target),
            MirTerminator::Return { value, .. } => {
                if let Some(value) = value {
                    self.line(&format!(
                        "            (local.set $__ret {})",
                        self.operand(value, function, function_layout)
                    ));
                }
                self.line("            (local.set $__done (i32.const 1))");
            }
            MirTerminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                self.line(&format!(
                    "            (if {}",
                    self.operand(condition, function, function_layout)
                ));
                self.line("              (then");
                self.set_bb(*then_target);
                self.line("              )");
                self.line("              (else");
                self.set_bb(*else_target);
                self.line("              )");
                self.line("            )");
            }
            MirTerminator::Match {
                scrutinee, targets, ..
            } => {
                self.line("            (local.set $__matched (i32.const 0))");
                let scrutinee = self.operand(scrutinee, function, function_layout);
                for target in targets {
                    let condition = match &target.pattern {
                        MirBranchPattern::Wildcard => "(i32.const 1)".to_owned(),
                        MirBranchPattern::Literal(text) => {
                            format!(
                                "(i32.eq {scrutinee} {})",
                                self.literal(text)
                            )
                        }
                        MirBranchPattern::Variant(symbol) => format!(
                            "(i32.eq (i32.and {scrutinee} (i32.const {VARIANT_TAG_MASK})) (i32.const {}))",
                            self.layout.variant_tag(*symbol)
                        ),
                    };
                    self.line(&format!(
                        "            (if (i32.and (i32.eqz (local.get $__matched)) {condition})"
                    ));
                    self.line("              (then");
                    self.set_bb(target.target);
                    self.line("                (local.set $__matched (i32.const 1))");
                    self.line("              )");
                    self.line("            )");
                }
                self.line("            (if (i32.eqz (local.get $__matched)) (then (unreachable)))");
            }
        }
    }

    fn rvalue(
        &mut self,
        value: &MirRvalue,
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) -> String {
        match value {
            MirRvalue::Use(operand) => self.operand(operand, function, function_layout),
            MirRvalue::Call { callee, args } => match callee {
                MirOperand::Function(symbol) => {
                    if self.is_core_log(*symbol) {
                        return self.log_call(args, function, function_layout);
                    }
                    let args = args
                        .iter()
                        .map(|arg| self.operand(arg, function, function_layout))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let name = function_name(self.package, *symbol).unwrap_or("unknown");
                    format!("(call ${} {args})", wasm_name(name))
                }
                MirOperand::Variant(symbol) => {
                    self.aggregate_variant(*symbol, args, function, function_layout)
                }
                _ => {
                    self.diagnostics.push(WasmDiagnostic::new(
                        "unsupported dynamic call lowered to i32.const 0",
                    ));
                    "(i32.const 0)".to_owned()
                }
            },
            MirRvalue::Binary { op, left, right } => {
                let op = match *op {
                    "+" => "i32.add",
                    "-" => "i32.sub",
                    "*" => "i32.mul",
                    "/" => "i32.div_s",
                    "<" => "i32.lt_s",
                    ">" => "i32.gt_s",
                    _ => {
                        self.diagnostics.push(WasmDiagnostic::new(format!(
                            "unsupported binary operator `{op}` lowered to i32.const 0"
                        )));
                        return "(i32.const 0)".to_owned();
                    }
                };
                format!(
                    "({op} {} {})",
                    self.operand(left, function, function_layout),
                    self.operand(right, function, function_layout)
                )
            }
            MirRvalue::AggregateVariant { variant, fields } => {
                self.aggregate_variant(*variant, fields, function, function_layout)
            }
            MirRvalue::ProjectVariant { source, .. } => format!(
                "(i32.shr_s {} (i32.const {VARIANT_PAYLOAD_SHIFT}))",
                self.operand(source, function, function_layout)
            ),
        }
    }

    fn aggregate_variant(
        &mut self,
        variant: SymbolId,
        fields: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) -> String {
        let tag = self.layout.variant_tag(variant);
        match fields {
            [] => format!("(i32.const {tag})"),
            [field] => format!(
                "(i32.or (i32.shl {} (i32.const {VARIANT_PAYLOAD_SHIFT})) (i32.const {tag}))",
                self.operand(field, function, function_layout)
            ),
            [first, ..] => {
                self.diagnostics.push(WasmDiagnostic::new(
                    "multi-field enum variant lowered using only field 0 in v1",
                ));
                format!(
                    "(i32.or (i32.shl {} (i32.const {VARIANT_PAYLOAD_SHIFT})) (i32.const {tag}))",
                    self.operand(first, function, function_layout)
                )
            }
        }
    }

    fn operand(
        &self,
        operand: &MirOperand,
        _function: &MirFunction,
        _function_layout: &FunctionLayout,
    ) -> String {
        match operand {
            MirOperand::Copy(MirPlace::Local(local)) => format!("(local.get $v{})", local.get()),
            MirOperand::Const(MirConstant::Unit) | MirOperand::Function(_) => {
                "(i32.const 0)".to_owned()
            }
            MirOperand::Const(MirConstant::Literal(text)) => self.literal(text),
            MirOperand::Variant(symbol) => {
                format!("(i32.const {})", self.layout.variant_tag(*symbol))
            }
        }
    }

    fn literal(&self, text: &str) -> String {
        if let Some(value) = int_literal_value(text) {
            format!("(i32.const {value})")
        } else if let Some(value) = bool_literal_value(text) {
            format!("(i32.const {})", i32::from(value))
        } else if text.starts_with("string(") {
            format!("(i64.const {})", self.strings.handle_for_literal(text))
        } else {
            "(i32.const 0)".to_owned()
        }
    }

    fn log_call(
        &mut self,
        args: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) -> String {
        let Some(message) = args.first() else {
            self.diagnostics.push(WasmDiagnostic::new(
                "core.log call without message lowered to unit",
            ));
            return "(i32.const 0)".to_owned();
        };

        if args.len() == 1 {
            let chunk = self.log_string_chunk(message, function, function_layout);
            return format!(
                "(block (result i32) {chunk} (call $__maodie_debug_log_end) (i32.const 0))"
            );
        }

        let Some(format) = self.log_format(message) else {
            self.diagnostics.push(WasmDiagnostic::new(
                "core.log formatted call without literal format lowered to unit",
            ));
            return "(i32.const 0)".to_owned();
        };

        let mut chunks = Vec::new();
        self.push_formatted_log_chunks(&format, &args[1..], function, function_layout, &mut chunks);
        chunks.push("(call $__maodie_debug_log_end)".to_owned());

        format!("(block (result i32) {} (i32.const 0))", chunks.join(" "))
    }

    fn push_formatted_log_chunks(
        &mut self,
        format: &LogFormat,
        args: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
        chunks: &mut Vec<String>,
    ) {
        for (index, segment) in format.segments.iter().enumerate() {
            if !segment.is_empty() {
                chunks.push(self.log_text_chunk(segment));
            }
            if let Some(arg) = args.get(index) {
                chunks.push(self.log_value_chunk(arg, function, function_layout));
            }
        }
    }

    fn log_format(&mut self, operand: &MirOperand) -> Option<LogFormat> {
        if let MirOperand::Const(MirConstant::Literal(text)) = operand {
            parse_log_format(text)
        } else {
            None
        }
    }

    fn log_text_chunk(&self, text: &str) -> String {
        let pointer = self.strings.offset_for_text(text).unwrap_or(0);
        format!(
            "(call $__maodie_debug_string (i32.const {pointer}) (i32.const {}))",
            text.len()
        )
    }

    fn log_string_chunk(
        &self,
        operand: &MirOperand,
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) -> String {
        if let MirOperand::Const(MirConstant::Literal(text)) = operand {
            if let Some(value) = string_literal_value(text) {
                return self.log_text_chunk(&value);
            }
        }

        let value = self.operand(operand, function, function_layout);
        format!(
            "(local.set $__string {value}) (call $__maodie_debug_string (i32.wrap_i64 (local.get $__string)) (i32.wrap_i64 (i64.shr_u (local.get $__string) (i64.const 32))))"
        )
    }

    fn log_value_chunk(
        &self,
        operand: &MirOperand,
        function: &MirFunction,
        function_layout: &FunctionLayout,
    ) -> String {
        match self.operand_value_type(operand, function) {
            WasmValueType::I64 => self.log_string_chunk(operand, function, function_layout),
            WasmValueType::I32 => {
                let value = self.operand(operand, function, function_layout);
                if self.operand_is_bool(operand, function) {
                    format!("(call $__maodie_debug_bool {value})")
                } else {
                    format!("(call $__maodie_debug_i32 {value})")
                }
            }
        }
    }

    fn operand_value_type(&self, operand: &MirOperand, function: &MirFunction) -> WasmValueType {
        match operand {
            MirOperand::Const(MirConstant::Literal(text)) if text.starts_with("string(") => {
                WasmValueType::I64
            }
            MirOperand::Copy(MirPlace::Local(local)) => function
                .locals
                .iter()
                .find(|candidate| candidate.id == *local)
                .map_or(WasmValueType::I32, |local| {
                    local_value_type(self.package, function, local)
                }),
            _ => WasmValueType::I32,
        }
    }

    fn operand_is_bool(&self, operand: &MirOperand, function: &MirFunction) -> bool {
        match operand {
            MirOperand::Const(MirConstant::Literal(text)) => text.starts_with("bool("),
            MirOperand::Copy(MirPlace::Local(local)) => function
                .locals
                .iter()
                .find(|candidate| candidate.id == *local)
                .and_then(|local| local.ty)
                .and_then(|ty| self.package.types.get(ty.get()))
                .is_some_and(|ty| matches!(ty, MirTypeKind::Bool)),
            _ => false,
        }
    }

    fn is_core_log(&self, symbol: SymbolId) -> bool {
        is_core_log_symbol(self.package, symbol)
    }

    fn function_result(&self, function: &MirFunction) -> bool {
        function
            .return_type
            .and_then(|ty| self.package.types.get(ty.get()))
            .is_some_and(|ty| !matches!(ty, MirTypeKind::Unit | MirTypeKind::Error))
    }

    fn set_bb(&mut self, target: BasicBlockId) {
        self.line(&format!(
            "                (local.set $__bb (i32.const {}))",
            target.get()
        ));
    }

    fn line(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
    }
}

struct BinaryWriter<'a, 'd> {
    package: &'a MirPackage,
    layout: &'a Layout,
    strings: &'a StringTable,
    diagnostics: &'d mut Vec<WasmDiagnostic>,
}

impl<'a, 'd> BinaryWriter<'a, 'd> {
    fn new(
        package: &'a MirPackage,
        layout: &'a Layout,
        strings: &'a StringTable,
        diagnostics: &'d mut Vec<WasmDiagnostic>,
    ) -> Self {
        Self {
            package,
            layout,
            strings,
            diagnostics,
        }
    }

    fn finish(mut self) -> Vec<u8> {
        let mut module = Vec::new();
        module.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d]);
        module.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        self.type_section(&mut module);
        Self::import_section(&mut module);
        self.function_section(&mut module);
        Self::memory_section(&mut module);
        self.export_section(&mut module);
        self.code_section(&mut module);
        self.data_section(&mut module);
        module
    }

    fn type_section(&self, module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(
            3 + u32::try_from(self.package.functions.len()).expect("function count fits u32"),
            &mut payload,
        );
        payload.extend_from_slice(&[0x60, 0x02, 0x7f, 0x7f, 0x00]);
        payload.extend_from_slice(&[0x60, 0x01, 0x7f, 0x00]);
        payload.extend_from_slice(&[0x60, 0x00, 0x00]);
        for function in &self.package.functions {
            payload.push(0x60);
            let params = function
                .locals
                .iter()
                .filter(|local| local.kind == MirLocalKind::Param)
                .collect::<Vec<_>>();
            encode_u32(
                u32::try_from(params.len()).expect("param count fits u32"),
                &mut payload,
            );
            for param in params {
                payload.push(wasm_value_type(self.package, param.ty).binary());
            }
            if let Some(result) = function_return_value_type(self.package, function) {
                payload.push(0x01);
                payload.push(result.binary());
            } else {
                payload.push(0x00);
            }
        }
        section(1, payload, module);
    }

    fn import_section(module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(IMPORT_COUNT, &mut payload);
        import_func(WASM_HOST_MODULE, WASM_IMPORT_PANIC, 0, &mut payload);
        import_func(WASM_HOST_MODULE, WASM_IMPORT_DEBUG_STRING, 0, &mut payload);
        import_func(WASM_HOST_MODULE, WASM_IMPORT_DEBUG_I32, 1, &mut payload);
        import_func(WASM_HOST_MODULE, WASM_IMPORT_DEBUG_BOOL, 1, &mut payload);
        import_func(WASM_HOST_MODULE, WASM_IMPORT_DEBUG_LOG_END, 2, &mut payload);
        section(2, payload, module);
    }

    fn function_section(&self, module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(
            u32::try_from(self.package.functions.len()).expect("function count fits u32"),
            &mut payload,
        );
        for (index, _) in self.package.functions.iter().enumerate() {
            encode_u32(
                3 + u32::try_from(index).expect("type index fits u32"),
                &mut payload,
            );
        }
        section(3, payload, module);
    }

    fn memory_section(module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(1, &mut payload);
        payload.push(0x00);
        encode_u32(1, &mut payload);
        section(5, payload, module);
    }

    fn export_section(&self, module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(
            1 + u32::try_from(self.package.functions.len()).expect("export count fits u32"),
            &mut payload,
        );
        encode_name(WASM_MEMORY_EXPORT, &mut payload);
        payload.push(0x02);
        encode_u32(0, &mut payload);
        for (index, function) in self.package.functions.iter().enumerate() {
            encode_name(&function.name, &mut payload);
            payload.push(0x00);
            encode_u32(
                IMPORT_COUNT + u32::try_from(index).expect("function index fits u32"),
                &mut payload,
            );
        }
        section(7, payload, module);
    }

    fn code_section(&mut self, module: &mut Vec<u8>) {
        let mut payload = Vec::new();
        encode_u32(
            u32::try_from(self.package.functions.len()).expect("function count fits u32"),
            &mut payload,
        );
        for function in &self.package.functions {
            let body = self.function_body(function);
            encode_u32(
                u32::try_from(body.len()).expect("body size fits u32"),
                &mut payload,
            );
            payload.extend(body);
        }
        section(10, payload, module);
    }

    fn data_section(&self, module: &mut Vec<u8>) {
        if self.strings.entries.is_empty() {
            return;
        }
        let mut payload = Vec::new();
        encode_u32(
            u32::try_from(self.strings.entries.len()).expect("data count fits u32"),
            &mut payload,
        );
        for entry in &self.strings.entries {
            payload.push(0x00);
            payload.push(0x41);
            encode_i32(u32_to_i32(entry.offset), &mut payload);
            payload.push(0x0b);
            encode_u32(
                u32::try_from(entry.bytes.len()).expect("data size fits u32"),
                &mut payload,
            );
            payload.extend_from_slice(&entry.bytes);
        }
        section(11, payload, module);
    }

    fn function_body(&mut self, function: &MirFunction) -> Vec<u8> {
        let function_layout = FunctionLayout::new(function);
        let mut body = Vec::new();
        let local_groups = FunctionLayout::declared_local_groups(function, self.package);
        encode_u32(
            u32::try_from(local_groups.len()).expect("local group count fits u32"),
            &mut body,
        );
        for value_type in local_groups {
            encode_u32(1, &mut body);
            body.push(value_type.binary());
        }

        body.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
        instr_local_get(function_layout.done, &mut body);
        instr_br_if(1, &mut body);

        for block in &function.blocks {
            instr_local_get(function_layout.bb, &mut body);
            instr_i32_const(usize_to_i32(block.id.get()), &mut body);
            body.push(0x46);
            body.extend_from_slice(&[0x04, 0x40]);
            self.block_body(block, function, &function_layout, &mut body);
            instr_br(1, &mut body);
            body.push(0x0b);
        }

        instr_br(0, &mut body);
        body.push(0x0b);
        body.push(0x0b);
        if self.function_result(function) {
            instr_local_get(function_layout.ret, &mut body);
        }
        body.push(0x0f);
        body.push(0x0b);
        body
    }

    fn block_body(
        &mut self,
        block: &BasicBlock,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        for statement in &block.statements {
            let MirStatement::Assign { place, value, .. } = statement;
            self.rvalue(value, function, function_layout, body);
            let MirPlace::Local(local) = place;
            instr_local_set(function_layout.local(*local), body);
        }
        if let Some(terminator) = &block.terminator {
            self.terminator(terminator, function, function_layout, body);
        } else {
            body.push(0x00);
        }
    }

    fn terminator(
        &mut self,
        terminator: &MirTerminator,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        match terminator {
            MirTerminator::Goto { target, .. } => set_bb(*target, function_layout, body),
            MirTerminator::Return { value, .. } => {
                if let Some(value) = value {
                    self.operand(value, function, function_layout, body);
                    instr_local_set(function_layout.ret, body);
                }
                instr_i32_const(1, body);
                instr_local_set(function_layout.done, body);
            }
            MirTerminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                self.operand(condition, function, function_layout, body);
                body.extend_from_slice(&[0x04, 0x40]);
                set_bb(*then_target, function_layout, body);
                body.push(0x05);
                set_bb(*else_target, function_layout, body);
                body.push(0x0b);
            }
            MirTerminator::Match {
                scrutinee, targets, ..
            } => {
                instr_i32_const(0, body);
                instr_local_set(function_layout.matched, body);
                for target in targets {
                    instr_local_get(function_layout.matched, body);
                    instr_i32_const(0, body);
                    body.push(0x46);
                    self.match_condition(
                        &target.pattern,
                        scrutinee,
                        function,
                        function_layout,
                        body,
                    );
                    body.push(0x71);
                    body.extend_from_slice(&[0x04, 0x40]);
                    set_bb(target.target, function_layout, body);
                    instr_i32_const(1, body);
                    instr_local_set(function_layout.matched, body);
                    body.push(0x0b);
                }
                instr_local_get(function_layout.matched, body);
                instr_i32_const(0, body);
                body.push(0x46);
                body.extend_from_slice(&[0x04, 0x40, 0x00, 0x0b]);
            }
        }
    }

    fn match_condition(
        &mut self,
        pattern: &MirBranchPattern,
        scrutinee: &MirOperand,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        match pattern {
            MirBranchPattern::Wildcard => instr_i32_const(1, body),
            MirBranchPattern::Literal(text) => {
                self.operand(scrutinee, function, function_layout, body);
                self.literal(text, body);
                body.push(0x46);
            }
            MirBranchPattern::Variant(symbol) => {
                self.operand(scrutinee, function, function_layout, body);
                instr_i32_const(VARIANT_TAG_MASK, body);
                body.push(0x71);
                instr_i32_const(u32_to_i32(self.layout.variant_tag(*symbol)), body);
                body.push(0x46);
            }
        }
    }

    fn rvalue(
        &mut self,
        value: &MirRvalue,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        match value {
            MirRvalue::Use(operand) => self.operand(operand, function, function_layout, body),
            MirRvalue::Call { callee, args } => match callee {
                MirOperand::Function(symbol) => {
                    if self.is_core_log(*symbol) {
                        self.log_call(args, function, function_layout, body);
                        return;
                    }
                    for arg in args {
                        self.operand(arg, function, function_layout, body);
                    }
                    if let Some(index) = self.layout.function_index(*symbol) {
                        body.push(0x10);
                        encode_u32(index, body);
                    } else {
                        self.diagnostics
                            .push(WasmDiagnostic::new("unknown function call lowered to 0"));
                        instr_i32_const(0, body);
                    }
                }
                MirOperand::Variant(symbol) => {
                    self.aggregate_variant(*symbol, args, function, function_layout, body);
                }
                _ => {
                    self.diagnostics
                        .push(WasmDiagnostic::new("unsupported dynamic call lowered to 0"));
                    instr_i32_const(0, body);
                }
            },
            MirRvalue::Binary { op, left, right } => {
                self.operand(left, function, function_layout, body);
                self.operand(right, function, function_layout, body);
                match *op {
                    "+" => body.push(0x6a),
                    "-" => body.push(0x6b),
                    "*" => body.push(0x6c),
                    "/" => body.push(0x6d),
                    "<" => body.push(0x48),
                    ">" => body.push(0x4a),
                    _ => {
                        self.diagnostics.push(WasmDiagnostic::new(format!(
                            "unsupported binary operator `{op}` in binary output"
                        )));
                        body.push(0x1a);
                        instr_i32_const(0, body);
                    }
                }
            }
            MirRvalue::AggregateVariant { variant, fields } => {
                self.aggregate_variant(*variant, fields, function, function_layout, body);
            }
            MirRvalue::ProjectVariant { source, .. } => {
                self.operand(source, function, function_layout, body);
                instr_i32_const(VARIANT_PAYLOAD_SHIFT, body);
                body.push(0x75);
            }
        }
    }

    fn log_call(
        &mut self,
        args: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        let Some(message) = args.first() else {
            self.diagnostics.push(WasmDiagnostic::new(
                "core.log call without message lowered to unit",
            ));
            instr_i32_const(0, body);
            return;
        };

        if args.len() == 1 {
            self.log_string_chunk(message, function, function_layout, body);
            instr_call(DEBUG_LOG_END_IMPORT_INDEX, body);
            instr_i32_const(0, body);
            return;
        }

        let Some(format) = self.log_format(message) else {
            self.diagnostics.push(WasmDiagnostic::new(
                "core.log formatted call without literal format lowered to unit",
            ));
            instr_i32_const(0, body);
            return;
        };

        self.formatted_log_chunks(&format, &args[1..], function, function_layout, body);
        instr_call(DEBUG_LOG_END_IMPORT_INDEX, body);
        instr_i32_const(0, body);
    }

    fn log_format(&mut self, operand: &MirOperand) -> Option<LogFormat> {
        if let MirOperand::Const(MirConstant::Literal(text)) = operand {
            parse_log_format(text)
        } else {
            None
        }
    }

    fn formatted_log_chunks(
        &mut self,
        format: &LogFormat,
        args: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        for (index, segment) in format.segments.iter().enumerate() {
            if !segment.is_empty() {
                self.log_text_chunk(segment, body);
            }
            if let Some(arg) = args.get(index) {
                self.log_value_chunk(arg, function, function_layout, body);
            }
        }
    }

    fn log_text_chunk(&self, text: &str, body: &mut Vec<u8>) {
        instr_i32_const(
            u32_to_i32(self.strings.offset_for_text(text).unwrap_or(0)),
            body,
        );
        instr_i32_const(usize_to_i32(text.len()), body);
        instr_call(DEBUG_STRING_IMPORT_INDEX, body);
    }

    fn log_string_chunk(
        &self,
        operand: &MirOperand,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        if let MirOperand::Const(MirConstant::Literal(text)) = operand {
            if let Some(value) = string_literal_value(text) {
                self.log_text_chunk(&value, body);
                return;
            }
        }

        self.operand(operand, function, function_layout, body);
        instr_local_set(function_layout.string_scratch, body);
        instr_local_get(function_layout.string_scratch, body);
        body.push(0xa7);
        instr_local_get(function_layout.string_scratch, body);
        instr_i64_const(32, body);
        body.push(0x88);
        body.push(0xa7);
        instr_call(DEBUG_STRING_IMPORT_INDEX, body);
    }

    fn log_value_chunk(
        &self,
        operand: &MirOperand,
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        match self.operand_value_type(operand, function) {
            WasmValueType::I64 => self.log_string_chunk(operand, function, function_layout, body),
            WasmValueType::I32 => {
                self.operand(operand, function, function_layout, body);
                if self.operand_is_bool(operand, function) {
                    instr_call(DEBUG_BOOL_IMPORT_INDEX, body);
                } else {
                    instr_call(DEBUG_I32_IMPORT_INDEX, body);
                }
            }
        }
    }

    fn operand_value_type(&self, operand: &MirOperand, function: &MirFunction) -> WasmValueType {
        match operand {
            MirOperand::Const(MirConstant::Literal(text)) if text.starts_with("string(") => {
                WasmValueType::I64
            }
            MirOperand::Copy(MirPlace::Local(local)) => function
                .locals
                .iter()
                .find(|candidate| candidate.id == *local)
                .map_or(WasmValueType::I32, |local| {
                    local_value_type(self.package, function, local)
                }),
            _ => WasmValueType::I32,
        }
    }

    fn operand_is_bool(&self, operand: &MirOperand, function: &MirFunction) -> bool {
        match operand {
            MirOperand::Const(MirConstant::Literal(text)) => text.starts_with("bool("),
            MirOperand::Copy(MirPlace::Local(local)) => function
                .locals
                .iter()
                .find(|candidate| candidate.id == *local)
                .and_then(|local| local.ty)
                .and_then(|ty| self.package.types.get(ty.get()))
                .is_some_and(|ty| matches!(ty, MirTypeKind::Bool)),
            _ => false,
        }
    }

    fn is_core_log(&self, symbol: SymbolId) -> bool {
        is_core_log_symbol(self.package, symbol)
    }

    fn aggregate_variant(
        &mut self,
        variant: SymbolId,
        fields: &[MirOperand],
        function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        let tag = u32_to_i32(self.layout.variant_tag(variant));
        if let Some(first) = fields.first() {
            self.operand(first, function, function_layout, body);
            instr_i32_const(VARIANT_PAYLOAD_SHIFT, body);
            body.push(0x74);
            instr_i32_const(tag, body);
            body.push(0x72);
            if fields.len() > 1 {
                self.diagnostics.push(WasmDiagnostic::new(
                    "multi-field enum variant lowered using only field 0 in binary output",
                ));
            }
        } else {
            instr_i32_const(tag, body);
        }
    }

    fn operand(
        &self,
        operand: &MirOperand,
        _function: &MirFunction,
        function_layout: &FunctionLayout,
        body: &mut Vec<u8>,
    ) {
        match operand {
            MirOperand::Copy(MirPlace::Local(local)) => {
                instr_local_get(function_layout.local(*local), body);
            }
            MirOperand::Const(MirConstant::Unit) | MirOperand::Function(_) => {
                instr_i32_const(0, body);
            }
            MirOperand::Const(MirConstant::Literal(text)) => self.literal(text, body),
            MirOperand::Variant(symbol) => {
                instr_i32_const(u32_to_i32(self.layout.variant_tag(*symbol)), body);
            }
        }
    }

    fn literal(&self, text: &str, body: &mut Vec<u8>) {
        if let Some(value) = int_literal_value(text) {
            instr_i32_const(value, body);
        } else if let Some(value) = bool_literal_value(text) {
            instr_i32_const(i32::from(value), body);
        } else if text.starts_with("string(") {
            instr_i64_const(self.strings.handle_for_literal(text), body);
        } else {
            instr_i32_const(0, body);
        }
    }

    fn function_result(&self, function: &MirFunction) -> bool {
        function
            .return_type
            .and_then(|ty| self.package.types.get(ty.get()))
            .is_some_and(|ty| !matches!(ty, MirTypeKind::Unit | MirTypeKind::Error))
    }
}

struct MirRunner<'a> {
    package: &'a MirPackage,
    layout: Layout,
}

impl<'a> MirRunner<'a> {
    fn new(package: &'a MirPackage) -> Self {
        Self {
            package,
            layout: Layout::new(package),
        }
    }

    fn run_i32(&self, name: &str, args: &[i32]) -> WasmRunResult {
        let function = self
            .package
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or_else(|| WasmRunError::new(format!("unknown export `{name}`")))?;
        self.run_function(function, args)
    }

    fn run_function(&self, function: &MirFunction, args: &[i32]) -> WasmRunResult {
        let mut locals = vec![0; function.locals.len()];
        let params = function
            .locals
            .iter()
            .filter(|local| local.kind == MirLocalKind::Param)
            .collect::<Vec<_>>();
        if args.len() != params.len() {
            return Err(WasmRunError::new(format!(
                "export `{}` expects {} args, got {}",
                function.name,
                params.len(),
                args.len()
            )));
        }
        for (local, value) in params.iter().zip(args) {
            locals[local.id.get()] = *value;
        }

        let mut current = BasicBlockId::new(0);
        for _ in 0..10_000 {
            let block = function
                .blocks
                .get(current.get())
                .ok_or_else(|| WasmRunError::new("invalid basic block"))?;
            for statement in &block.statements {
                let MirStatement::Assign { place, value, .. } = statement;
                let value = self.eval_rvalue(value, &locals)?;
                let MirPlace::Local(local) = place;
                locals[local.get()] = value;
            }
            match block
                .terminator
                .as_ref()
                .ok_or_else(|| WasmRunError::new("unreachable block"))?
            {
                MirTerminator::Goto { target, .. } => current = *target,
                MirTerminator::Return { value, .. } => {
                    return value
                        .as_ref()
                        .map_or(Ok(0), |value| self.eval_operand(value, &locals));
                }
                MirTerminator::Branch {
                    condition,
                    then_target,
                    else_target,
                    ..
                } => {
                    current = if self.eval_operand(condition, &locals)? != 0 {
                        *then_target
                    } else {
                        *else_target
                    };
                }
                MirTerminator::Match {
                    scrutinee, targets, ..
                } => {
                    let value = self.eval_operand(scrutinee, &locals)?;
                    current = targets
                        .iter()
                        .find(|target| self.pattern_matches(&target.pattern, value))
                        .map(|target| target.target)
                        .ok_or_else(|| WasmRunError::new("non-exhaustive MIR match"))?;
                }
            }
        }
        Err(WasmRunError::new("MIR execution exceeded step limit"))
    }

    fn eval_rvalue(&self, value: &MirRvalue, locals: &[i32]) -> WasmRunResult {
        match value {
            MirRvalue::Use(operand) => self.eval_operand(operand, locals),
            MirRvalue::Call { callee, args } => match callee {
                MirOperand::Function(symbol) => {
                    let function = self
                        .package
                        .functions
                        .iter()
                        .find(|function| function.symbol == Some(*symbol))
                        .ok_or_else(|| WasmRunError::new("unknown function symbol"))?;
                    let args = args
                        .iter()
                        .map(|arg| self.eval_operand(arg, locals))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.run_function(function, &args)
                }
                MirOperand::Variant(symbol) => self.eval_variant(*symbol, args, locals),
                _ => Err(WasmRunError::new("unsupported dynamic call")),
            },
            MirRvalue::Binary { op, left, right } => {
                let left = self.eval_operand(left, locals)?;
                let right = self.eval_operand(right, locals)?;
                match *op {
                    "+" => Ok(left + right),
                    "-" => Ok(left - right),
                    "*" => Ok(left * right),
                    "/" => Ok(left / right),
                    "<" => Ok(i32::from(left < right)),
                    ">" => Ok(i32::from(left > right)),
                    _ => Err(WasmRunError::new(format!("unsupported operator `{op}`"))),
                }
            }
            MirRvalue::AggregateVariant { variant, fields } => {
                self.eval_variant(*variant, fields, locals)
            }
            MirRvalue::ProjectVariant { source, .. } => {
                Ok(self.eval_operand(source, locals)? >> VARIANT_PAYLOAD_SHIFT)
            }
        }
    }

    fn eval_variant(
        &self,
        variant: SymbolId,
        fields: &[MirOperand],
        locals: &[i32],
    ) -> WasmRunResult {
        let tag = u32_to_i32(self.layout.variant_tag(variant));
        fields.first().map_or(Ok(tag), |field| {
            Ok((self.eval_operand(field, locals)? << VARIANT_PAYLOAD_SHIFT) | tag)
        })
    }

    fn eval_operand(&self, operand: &MirOperand, locals: &[i32]) -> WasmRunResult {
        match operand {
            MirOperand::Copy(MirPlace::Local(local)) => locals
                .get(local.get())
                .copied()
                .ok_or_else(|| WasmRunError::new("invalid local")),
            MirOperand::Const(MirConstant::Unit) | MirOperand::Function(_) => Ok(0),
            MirOperand::Const(MirConstant::Literal(text)) => Ok(int_literal_value(text)
                .or_else(|| bool_literal_value(text).map(i32::from))
                .unwrap_or(0)),
            MirOperand::Variant(symbol) => Ok(u32_to_i32(self.layout.variant_tag(*symbol))),
        }
    }

    fn pattern_matches(&self, pattern: &MirBranchPattern, value: i32) -> bool {
        match pattern {
            MirBranchPattern::Wildcard => true,
            MirBranchPattern::Literal(text) => {
                int_literal_value(text).or_else(|| bool_literal_value(text).map(i32::from))
                    == Some(value)
            }
            MirBranchPattern::Variant(symbol) => {
                (value & VARIANT_TAG_MASK) == u32_to_i32(self.layout.variant_tag(*symbol))
            }
        }
    }
}

fn set_bb(target: BasicBlockId, function_layout: &FunctionLayout, body: &mut Vec<u8>) {
    instr_i32_const(usize_to_i32(target.get()), body);
    instr_local_set(function_layout.bb, body);
}

fn function_name(package: &MirPackage, symbol: SymbolId) -> Option<&str> {
    package
        .symbols
        .iter()
        .find(|mir_symbol| mir_symbol.id == symbol)
        .map(crate::mir::MirSymbol::name)
}

fn is_core_log_symbol(package: &MirPackage, symbol: SymbolId) -> bool {
    package
        .symbols
        .iter()
        .find(|mir_symbol| mir_symbol.id == symbol)
        .is_some_and(|mir_symbol| {
            mir_symbol.kind == SymbolKind::Function
                && mir_symbol.path.as_slice() == ["core".to_owned(), "log".to_owned()]
        })
}

fn function_return_value_type(
    package: &MirPackage,
    function: &MirFunction,
) -> Option<WasmValueType> {
    function.return_type.and_then(|ty| {
        let kind = package.types.get(ty.get())?;
        if matches!(kind, MirTypeKind::Unit | MirTypeKind::Error) {
            None
        } else {
            Some(wasm_value_type(package, Some(ty)))
        }
    })
}

fn wasm_value_type(package: &MirPackage, ty: Option<crate::typeck::TypeId>) -> WasmValueType {
    ty.and_then(|ty| package.types.get(ty.get()))
        .map_or(WasmValueType::I32, |kind| match kind {
            MirTypeKind::String => WasmValueType::I64,
            _ => WasmValueType::I32,
        })
}

fn local_value_type(
    package: &MirPackage,
    function: &MirFunction,
    local: &crate::mir::MirLocal,
) -> WasmValueType {
    if wasm_value_type(package, local.ty) == WasmValueType::I64
        && local_is_variant_projection(function, local.id)
    {
        return WasmValueType::I32;
    }
    wasm_value_type(package, local.ty)
}

fn local_is_variant_projection(function: &MirFunction, local: MirLocalId) -> bool {
    function.blocks.iter().any(|block| {
        block.statements.iter().any(|statement| {
            matches!(
                statement,
                MirStatement::Assign {
                    place: MirPlace::Local(target),
                    value: MirRvalue::ProjectVariant { .. },
                    ..
                } if *target == local
            )
        })
    })
}

fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("MIR index must fit in u32")
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).expect("MIR index must fit in i32")
}

fn u32_to_i32(value: u32) -> i32 {
    i32::try_from(value).expect("WASM v1 value must fit in i32")
}

fn int_literal_value(text: &str) -> Option<i32> {
    text.strip_prefix("int(")?.strip_suffix(')')?.parse().ok()
}

fn bool_literal_value(text: &str) -> Option<bool> {
    text.strip_prefix("bool(")?.strip_suffix(')')?.parse().ok()
}

fn wasm_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn escape_wat_bytes(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for byte in bytes {
        match byte {
            b'"' => escaped.push_str("\\22"),
            b'\\' => escaped.push_str("\\5c"),
            0x20..=0x7e => escaped.push(char::from(*byte)),
            _ => write!(escaped, "\\{byte:02x}").expect("writing to a String cannot fail"),
        }
    }
    escaped
}

fn section(id: u8, payload: Vec<u8>, module: &mut Vec<u8>) {
    module.push(id);
    encode_u32(
        u32::try_from(payload.len()).expect("section size fits u32"),
        module,
    );
    module.extend(payload);
}

fn import_func(module: &str, field: &str, type_index: u32, payload: &mut Vec<u8>) {
    encode_name(module, payload);
    encode_name(field, payload);
    payload.push(0x00);
    encode_u32(type_index, payload);
}

fn encode_name(name: &str, bytes: &mut Vec<u8>) {
    encode_u32(u32::try_from(name.len()).expect("name len fits u32"), bytes);
    bytes.extend_from_slice(name.as_bytes());
}

fn instr_i32_const(value: i32, body: &mut Vec<u8>) {
    body.push(0x41);
    encode_i32(value, body);
}

fn instr_i64_const(value: i64, body: &mut Vec<u8>) {
    body.push(0x42);
    encode_i64(value, body);
}

fn instr_local_get(index: u32, body: &mut Vec<u8>) {
    body.push(0x20);
    encode_u32(index, body);
}

fn instr_local_set(index: u32, body: &mut Vec<u8>) {
    body.push(0x21);
    encode_u32(index, body);
}

fn instr_br(depth: u32, body: &mut Vec<u8>) {
    body.push(0x0c);
    encode_u32(depth, body);
}

fn instr_br_if(depth: u32, body: &mut Vec<u8>) {
    body.push(0x0d);
    encode_u32(depth, body);
}

fn instr_call(index: u32, body: &mut Vec<u8>) {
    body.push(0x10);
    encode_u32(index, body);
}

fn encode_u32(mut value: u32, bytes: &mut Vec<u8>) {
    loop {
        let mut byte = u8::try_from(value & 0x7f).expect("masked LEB128 byte must fit u8");
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn encode_i32(mut value: i32, bytes: &mut Vec<u8>) {
    loop {
        let byte = u8::try_from(value & 0x7f).expect("masked LEB128 byte must fit u8");
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        bytes.push(if done { byte } else { byte | 0x80 });
        if done {
            break;
        }
    }
}

fn encode_i64(mut value: i64, bytes: &mut Vec<u8>) {
    loop {
        let byte = u8::try_from(value & 0x7f).expect("masked LEB128 byte must fit u8");
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        bytes.push(if done { byte } else { byte | 0x80 });
        if done {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId};

    use super::{compile_mir_to_wasm, run_i32_export, WASM_BINARY_NAME, WAT_DUMP_NAME};
    use crate::core::check_source_with_core;
    use crate::mir::lower_package;

    #[test]
    fn emits_wat_wasm_and_runs_golden_result() {
        let source = SourceFile::new(
            SourceId::new(1),
            "wasm_golden.mao",
            "\
module demo
import core.Result

enum Color { Red, Green }

fn parse(value: i32) -> Result<i32, String> {
  if value > 0 { Result.Ok(value) } else { Result.Ok(1) }
}

fn score(color: Color, value: i32) -> i32 {
  match color {
    Color.Red => value + 10,
    Color.Green => value + 20
  }
}

fn main(value: i32) -> Result<i32, String> {
  let parsed: i32 = parse(value)?
  let color: Color = Color.Green
  let total: i32 = score(color, parsed)
  return Result.Ok(total)
}
",
        );

        let typed = check_source_with_core(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        let mir = lower_package(&typed);
        let artifacts = compile_mir_to_wasm(&mir);

        assert_eq!(artifacts.artifact_names.wat_dump, WAT_DUMP_NAME);
        assert_eq!(artifacts.artifact_names.wasm_binary, WASM_BINARY_NAME);
        assert!(artifacts.wat.contains("(module"));
        assert!(
            artifacts.wat.contains("(export \"memory\" (memory 0))")
                || artifacts.wat.contains("(memory (export \"memory\") 1)")
        );
        assert!(artifacts.wat.contains("(export \"main\""));
        assert!(artifacts.wat.contains("i32.gt_s"));
        assert!(artifacts.wat.contains("i32.and"));
        assert_eq!(&artifacts.wasm[..4], b"\0asm");
        assert_eq!(artifacts.wasm[4..8], [1, 0, 0, 0]);

        let encoded_result = run_i32_export(&mir, "main", &[2]).expect("golden runs");
        assert_eq!(encoded_result >> 8, 22);
        assert_eq!(encoded_result & 0xff, 0);
    }

    #[test]
    fn lowers_core_log_to_debug_string_host_import() {
        let source = SourceFile::new(
            SourceId::new(1),
            "hello_log.mao",
            "\
module demo
import core.Result
import core.log

fn main(value: i32) -> Result<i32, String> {
  log(\"Hello world\")
  return Result.Ok(value)
}
",
        );

        let typed = check_source_with_core(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        let mir = lower_package(&typed);
        let artifacts = compile_mir_to_wasm(&mir);

        assert!(artifacts.wat.contains("Hello world"));
        assert!(artifacts.wat.contains("call $__maodie_debug_string"));
        assert!(artifacts.wat.contains("call $__maodie_debug_log_end"));
        assert!(artifacts.wat.contains("(i32.const 11)"));
        assert_eq!(
            run_i32_export(&mir, "main", &[7]).expect("golden runs") >> 8,
            7
        );
    }

    #[test]
    fn lowers_formatted_core_log_chunks_and_string_handles() {
        let source = SourceFile::new(
            SourceId::new(1),
            "formatted_log.mao",
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

        let typed = check_source_with_core(&source);
        assert!(typed.diagnostics.is_empty(), "{:#?}", typed.diagnostics);
        let mir = lower_package(&typed);
        let artifacts = compile_mir_to_wasm(&mir);

        assert!(artifacts.wat.contains("debug_i32"));
        assert!(artifacts.wat.contains("debug_bool"));
        assert!(artifacts.wat.contains("debug_log_end"));
        assert!(artifacts.wat.contains("(result i64)"));
        assert!(artifacts.wat.contains("(local $__string i64)"));
        assert!(artifacts.wat.contains("i64.shr_u"));
        assert!(artifacts.wat.contains("value is "));
        assert!(artifacts.wat.contains("(i32.const 1)"));
        assert_eq!(&artifacts.wasm[..4], b"\0asm");
    }
}
