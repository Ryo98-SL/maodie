use maodie_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticSpan, SourceFile, TextRange,
};

use crate::{
    lex_source, AstFile, BinaryOp, BlockExpr, EnumDecl, EnumVariant, Expr, FieldDecl, FunctionDecl,
    FunctionParam, ImplDecl, ImportDecl, Item, Keyword, LetStmt, Literal, MatchArm, ModuleDecl,
    ParamList, Pattern, Statement, StructDecl, Token, TokenKind, TraitDecl, TypeRef,
};

/// Generic unexpected token parser diagnostic.
pub const MD_UNEXPECTED_TOKEN: &str = "MD0201";
/// Expected syntax parser diagnostic.
pub const MD_EXPECTED_SYNTAX: &str = "MD0202";

/// Result produced by parsing a source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    /// Parsed AST. It may contain missing nodes when diagnostics are present.
    pub ast: AstFile,
    /// Lexer and parser diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses and lexes one source file.
#[must_use]
pub fn parse_source(source: &SourceFile) -> ParseResult {
    let lex_result = lex_source(source);
    Parser::new(source, lex_result.tokens, lex_result.diagnostics).parse()
}

/// Hand-written recursive descent parser.
#[derive(Debug)]
pub struct Parser<'source> {
    source: &'source SourceFile,
    tokens: Vec<Token>,
    index: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'source> Parser<'source> {
    /// Creates a parser from an existing token stream.
    #[must_use]
    pub fn new(
        source: &'source SourceFile,
        tokens: Vec<Token>,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            source,
            tokens: tokens
                .into_iter()
                .filter(|token| !is_trivia(token.kind))
                .collect(),
            index: 0,
            diagnostics,
        }
    }

    /// Parses the token stream.
    #[must_use]
    pub fn parse(mut self) -> ParseResult {
        let mut module = None;
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while !self.at_eof() {
            if self.at_keyword(Keyword::Module) {
                if module.is_some() {
                    self.error_current("重复的 module 声明");
                }
                module = Some(self.parse_module_decl());
            } else if self.at_keyword(Keyword::Import) {
                imports.push(self.parse_import_decl());
            } else if self.at_item_start() {
                if let Some(item) = self.parse_item() {
                    items.push(item);
                }
            } else {
                self.error_current("顶层只能出现 module、import 或声明");
                self.recover_top_level();
            }
        }

        ParseResult {
            ast: AstFile {
                span: TextRange::new(0, self.source.len_bytes()),
                module,
                imports,
                items,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.advance().range.start;
        let (path, path_span) = self.parse_path_segments("需要 module 路径");
        self.consume_optional_semicolon();

        ModuleDecl {
            path,
            span: TextRange::new(start, path_span.end),
        }
    }

    fn parse_import_decl(&mut self) -> ImportDecl {
        let start = self.advance().range.start;
        let (path, path_span) = self.parse_path_segments("需要 import 路径");
        self.consume_optional_semicolon();

        ImportDecl {
            path,
            span: TextRange::new(start, path_span.end),
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        if self.at_keyword(Keyword::Fn) {
            Some(Item::Function(
                self.parse_function_decl(FunctionBodyMode::Optional),
            ))
        } else if self.at_keyword(Keyword::Struct) {
            Some(Item::Struct(self.parse_struct_decl()))
        } else if self.at_keyword(Keyword::Enum) {
            Some(Item::Enum(self.parse_enum_decl()))
        } else if self.at_keyword(Keyword::Trait) {
            Some(Item::Trait(self.parse_trait_decl()))
        } else if self.at_keyword(Keyword::Impl) {
            Some(Item::Impl(self.parse_impl_decl()))
        } else {
            self.error_current("需要声明");
            self.recover_top_level();
            None
        }
    }

    fn parse_function_decl(&mut self, body_mode: FunctionBodyMode) -> FunctionDecl {
        let start = self.expect_keyword(Keyword::Fn, "`fn`").range.start;
        let (name, name_span) = self.parse_identifier("需要函数名");
        let generics = self.parse_generic_params();
        let params = self.parse_param_list();
        let return_type = if self.at_kind(TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_ref("需要返回类型"))
        } else {
            None
        };

        let body = if self.at_kind(TokenKind::LeftBrace) {
            Some(self.parse_block_expr())
        } else {
            match body_mode {
                FunctionBodyMode::Required => {
                    self.error_current("需要函数体");
                }
                FunctionBodyMode::Optional => {}
            }
            self.consume_optional_semicolon();
            None
        };

        let end = body.as_ref().map_or_else(
            || return_type.as_ref().map_or(name_span.end, |ty| ty.span.end),
            |body| body.span.end,
        );

        FunctionDecl {
            name,
            generics,
            params,
            return_type,
            body,
            span: TextRange::new(start, end),
        }
    }

    fn parse_generic_params(&mut self) -> Vec<String> {
        if !self.at_kind(TokenKind::Less) {
            return Vec::new();
        }

        self.advance();
        let mut params = Vec::new();

        while !self.at_eof() && !self.at_kind(TokenKind::Greater) {
            let (name, _) = self.parse_identifier("需要泛型参数名");
            if let Some(name) = name {
                params.push(name);
            }

            if !self.consume_comma_or_stop(TokenKind::Greater) {
                break;
            }
        }

        self.expect_kind(TokenKind::Greater, "`>`");
        params
    }

    fn parse_param_list(&mut self) -> ParamList {
        let start = self.expect_kind(TokenKind::LeftParen, "`(`").range.start;
        let mut params = Vec::new();

        while !self.at_eof() && !self.at_kind(TokenKind::RightParen) {
            let param_start = self.current_range().start;
            let (name, name_span) = self.parse_identifier("需要参数名");
            let ty = if self.at_kind(TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_ref("需要参数类型"))
            } else {
                self.error_current("需要参数类型");
                None
            };
            let end = ty.as_ref().map_or(name_span.end, |ty| ty.span.end);

            params.push(FunctionParam {
                name,
                ty,
                span: TextRange::new(param_start, end),
            });

            if !self.consume_comma_or_stop(TokenKind::RightParen) {
                break;
            }
        }

        let end = self.expect_kind(TokenKind::RightParen, "`)`").range.end;

        ParamList {
            params,
            span: TextRange::new(start, end),
        }
    }

    fn parse_struct_decl(&mut self) -> StructDecl {
        let start = self.expect_keyword(Keyword::Struct, "`struct`").range.start;
        let (name, name_span) = self.parse_identifier("需要 struct 名称");
        let generics = self.parse_generic_params();
        let mut fields = Vec::new();

        if self.at_kind(TokenKind::LeftBrace) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
                fields.push(self.parse_field_decl());
                self.consume_comma_or_stop(TokenKind::RightBrace);
            }
            let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
            StructDecl {
                name,
                generics,
                fields,
                span: TextRange::new(start, end),
            }
        } else {
            self.error_current("需要 struct 字段块");
            StructDecl {
                name,
                generics,
                fields,
                span: TextRange::new(start, name_span.end),
            }
        }
    }

    fn parse_field_decl(&mut self) -> FieldDecl {
        let start = self.current_range().start;
        let (name, name_span) = self.parse_identifier("需要字段名");
        let ty = if self.at_kind(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_ref("需要字段类型"))
        } else {
            self.error_current("需要字段类型");
            None
        };
        let end = ty.as_ref().map_or(name_span.end, |ty| ty.span.end);

        FieldDecl {
            name,
            ty,
            span: TextRange::new(start, end),
        }
    }

    fn parse_enum_decl(&mut self) -> EnumDecl {
        let start = self.expect_keyword(Keyword::Enum, "`enum`").range.start;
        let (name, name_span) = self.parse_identifier("需要 enum 名称");
        let generics = self.parse_generic_params();
        let mut variants = Vec::new();

        if self.at_kind(TokenKind::LeftBrace) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
                variants.push(self.parse_enum_variant());
                self.consume_comma_or_stop(TokenKind::RightBrace);
            }
            let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
            EnumDecl {
                name,
                generics,
                variants,
                span: TextRange::new(start, end),
            }
        } else {
            self.error_current("需要 enum 变体块");
            EnumDecl {
                name,
                generics,
                variants,
                span: TextRange::new(start, name_span.end),
            }
        }
    }

    fn parse_enum_variant(&mut self) -> EnumVariant {
        let start = self.current_range().start;
        let (name, name_span) = self.parse_identifier("需要 enum 变体名");
        let mut fields = Vec::new();
        let mut end = name_span.end;

        if self.at_kind(TokenKind::LeftParen) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightParen) {
                fields.push(self.parse_type_ref("需要变体载荷类型"));
                if !self.consume_comma_or_stop(TokenKind::RightParen) {
                    break;
                }
            }
            end = self.expect_kind(TokenKind::RightParen, "`)`").range.end;
        }

        EnumVariant {
            name,
            fields,
            span: TextRange::new(start, end),
        }
    }

    fn parse_trait_decl(&mut self) -> TraitDecl {
        let start = self.expect_keyword(Keyword::Trait, "`trait`").range.start;
        let (name, name_span) = self.parse_identifier("需要 trait 名称");
        let generics = self.parse_generic_params();
        let mut functions = Vec::new();

        if self.at_kind(TokenKind::LeftBrace) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
                if self.at_keyword(Keyword::Fn) {
                    functions.push(self.parse_function_decl(FunctionBodyMode::Optional));
                } else {
                    self.error_current("trait 中只能出现函数签名");
                    self.recover_member();
                }
            }
            let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
            TraitDecl {
                name,
                generics,
                functions,
                span: TextRange::new(start, end),
            }
        } else {
            self.error_current("需要 trait 声明块");
            TraitDecl {
                name,
                generics,
                functions,
                span: TextRange::new(start, name_span.end),
            }
        }
    }

    fn parse_impl_decl(&mut self) -> ImplDecl {
        let start = self.expect_keyword(Keyword::Impl, "`impl`").range.start;
        let first_type = self.parse_type_ref("需要 impl 目标类型");
        let (trait_path, target) = if self.at_identifier_text("for") {
            self.advance();
            (
                Some(first_type),
                Some(self.parse_type_ref("需要 impl 目标类型")),
            )
        } else {
            (None, Some(first_type))
        };
        let mut methods = Vec::new();

        if self.at_kind(TokenKind::LeftBrace) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
                if self.at_keyword(Keyword::Fn) {
                    methods.push(self.parse_function_decl(FunctionBodyMode::Required));
                } else {
                    self.error_current("impl 中只能出现方法声明");
                    self.recover_member();
                }
            }
            let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
            ImplDecl {
                trait_path,
                target,
                methods,
                span: TextRange::new(start, end),
            }
        } else {
            self.error_current("需要 impl 声明块");
            let end = target.as_ref().map_or(start, |ty| ty.span.end);
            ImplDecl {
                trait_path,
                target,
                methods,
                span: TextRange::new(start, end),
            }
        }
    }

    fn parse_block_expr(&mut self) -> BlockExpr {
        let start = self.expect_kind(TokenKind::LeftBrace, "`{`").range.start;
        let mut statements = Vec::new();

        while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
            if self.at_keyword(Keyword::Let) {
                statements.push(Statement::Let(self.parse_let_stmt()));
            } else if self.at_keyword(Keyword::Return) {
                statements.push(self.parse_return_stmt());
            } else {
                let expr = self.parse_expr();
                self.consume_optional_semicolon();
                statements.push(Statement::Expr(expr));
            }
        }

        let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
        BlockExpr {
            statements,
            span: TextRange::new(start, end),
        }
    }

    fn parse_let_stmt(&mut self) -> LetStmt {
        let start = self.expect_keyword(Keyword::Let, "`let`").range.start;
        let mutable = if self.at_keyword(Keyword::Mut) {
            self.advance();
            true
        } else {
            false
        };
        let (name, name_span) = self.parse_identifier("需要绑定名");
        let ty = if self.at_kind(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_ref("需要类型标注"))
        } else {
            None
        };
        let value = if self.at_kind(TokenKind::Equal) {
            self.advance();
            Some(self.parse_expr())
        } else {
            None
        };
        self.consume_optional_semicolon();
        let end = value.as_ref().map_or_else(
            || ty.as_ref().map_or(name_span.end, |ty| ty.span.end),
            |expr| expr.span().end,
        );

        LetStmt {
            mutable,
            name,
            ty,
            value,
            span: TextRange::new(start, end),
        }
    }

    fn parse_return_stmt(&mut self) -> Statement {
        let start = self.expect_keyword(Keyword::Return, "`return`").range.start;
        let expr = if self.at_kind(TokenKind::Semicolon) || self.at_kind(TokenKind::RightBrace) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.consume_optional_semicolon();
        let end = expr
            .as_ref()
            .map_or(start + "return".len(), |expr| expr.span().end);

        Statement::Return {
            expr,
            span: TextRange::new(start, end),
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_binary_expr(0)
    }

    fn parse_binary_expr(&mut self, min_binding_power: u8) -> Expr {
        let mut left = self.parse_postfix_expr();

        while let Some((op, left_power, right_power)) = self.current_binary_op() {
            if left_power < min_binding_power {
                break;
            }
            self.advance();
            let right = self.parse_binary_expr(right_power);
            let span = TextRange::new(left.span().start, right.span().end);
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn parse_postfix_expr(&mut self) -> Expr {
        let mut expr = self.parse_primary_expr();

        loop {
            if self.at_kind(TokenKind::LeftParen) {
                let start = expr.span().start;
                let mut args = Vec::new();
                self.advance();
                while !self.at_eof() && !self.at_kind(TokenKind::RightParen) {
                    args.push(self.parse_expr());
                    if !self.consume_comma_or_stop(TokenKind::RightParen) {
                        break;
                    }
                }
                let end = self.expect_kind(TokenKind::RightParen, "`)`").range.end;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span: TextRange::new(start, end),
                };
            } else if self.at_kind(TokenKind::Question) {
                let start = expr.span().start;
                let end = self.advance().range.end;
                expr = Expr::Try {
                    expr: Box::new(expr),
                    span: TextRange::new(start, end),
                };
            } else {
                break;
            }
        }

        expr
    }

    fn parse_primary_expr(&mut self) -> Expr {
        if self.at_kind(TokenKind::IntegerLiteral) {
            let token = self.advance();
            Expr::Literal {
                literal: Literal::Integer(token.text),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::BoolLiteral) {
            let token = self.advance();
            Expr::Literal {
                literal: Literal::Bool(token.text == "true"),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::StringLiteral) {
            let token = self.advance();
            Expr::Literal {
                literal: Literal::String(token.text),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::Identifier) {
            let (path, span) = self.parse_path_segments("需要路径表达式");
            Expr::Path { path, span }
        } else if self.at_kind(TokenKind::LeftBrace) {
            Expr::Block(self.parse_block_expr())
        } else if self.at_keyword(Keyword::If) {
            self.parse_if_expr()
        } else if self.at_keyword(Keyword::Match) {
            self.parse_match_expr()
        } else {
            let span = self.current_range();
            self.error_current("需要表达式");
            if !self.at_eof() {
                self.advance();
            }
            Expr::Missing { span }
        }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.expect_keyword(Keyword::If, "`if`").range.start;
        let condition = self.parse_expr();
        let then_block = if self.at_kind(TokenKind::LeftBrace) {
            self.parse_block_expr()
        } else {
            self.error_current("需要 if 代码块");
            BlockExpr {
                statements: Vec::new(),
                span: TextRange::at(condition.span().end),
            }
        };
        let else_branch = if self.at_keyword(Keyword::Else) {
            self.advance();
            if self.at_keyword(Keyword::If) {
                Some(Box::new(self.parse_if_expr()))
            } else if self.at_kind(TokenKind::LeftBrace) {
                Some(Box::new(Expr::Block(self.parse_block_expr())))
            } else {
                self.error_current("需要 else 分支");
                None
            }
        } else {
            None
        };
        let end = else_branch
            .as_ref()
            .map_or(then_block.span.end, |expr| expr.span().end);

        Expr::If {
            condition: Box::new(condition),
            then_block,
            else_branch,
            span: TextRange::new(start, end),
        }
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.expect_keyword(Keyword::Match, "`match`").range.start;
        let scrutinee = self.parse_expr();
        let mut arms = Vec::new();

        if self.at_kind(TokenKind::LeftBrace) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::RightBrace) {
                arms.push(self.parse_match_arm());
                self.consume_comma_or_stop(TokenKind::RightBrace);
            }
            let end = self.expect_kind(TokenKind::RightBrace, "`}`").range.end;
            Expr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
                span: TextRange::new(start, end),
            }
        } else {
            self.error_current("需要 match 分支块");
            Expr::Match {
                scrutinee: Box::new(scrutinee.clone()),
                arms,
                span: TextRange::new(start, scrutinee.span().end),
            }
        }
    }

    fn parse_match_arm(&mut self) -> MatchArm {
        let start = self.current_range().start;
        let pattern = self.parse_pattern();
        self.expect_kind(TokenKind::FatArrow, "`=>`");
        let expr = self.parse_expr();
        let end = expr.span().end;

        MatchArm {
            pattern,
            expr,
            span: TextRange::new(start, end),
        }
    }

    fn parse_pattern(&mut self) -> Pattern {
        if self.at_identifier_text("_") {
            let token = self.advance();
            Pattern::Wildcard { span: token.range }
        } else if self.at_kind(TokenKind::IntegerLiteral) {
            let token = self.advance();
            Pattern::Literal {
                literal: Literal::Integer(token.text),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::BoolLiteral) {
            let token = self.advance();
            Pattern::Literal {
                literal: Literal::Bool(token.text == "true"),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::StringLiteral) {
            let token = self.advance();
            Pattern::Literal {
                literal: Literal::String(token.text),
                span: token.range,
            }
        } else if self.at_kind(TokenKind::Identifier) {
            let (path, span) = self.parse_path_segments("需要模式");
            if path.len() == 1 {
                Pattern::Binding {
                    name: path.into_iter().next().expect("one path segment exists"),
                    span,
                }
            } else {
                Pattern::Path { path, span }
            }
        } else {
            let span = self.current_range();
            self.error_current("需要模式");
            if !self.at_eof() {
                self.advance();
            }
            Pattern::Wildcard { span }
        }
    }

    fn parse_type_ref(&mut self, message: &'static str) -> TypeRef {
        let (path, mut span) = self.parse_path_segments(message);
        let mut generic_args = Vec::new();

        if self.at_kind(TokenKind::Less) {
            self.advance();
            while !self.at_eof() && !self.at_kind(TokenKind::Greater) {
                let arg = self.parse_type_ref("需要泛型实参类型");
                span = TextRange::new(span.start, arg.span.end);
                generic_args.push(arg);
                if !self.consume_comma_or_stop(TokenKind::Greater) {
                    break;
                }
            }
            span = TextRange::new(
                span.start,
                self.expect_kind(TokenKind::Greater, "`>`").range.end,
            );
        }

        TypeRef {
            path,
            generic_args,
            span,
        }
    }

    fn parse_path_segments(&mut self, message: &'static str) -> (Vec<String>, TextRange) {
        let start = self.current_range().start;
        let (first, mut span) = self.parse_identifier(message);
        let mut path = first.into_iter().collect::<Vec<_>>();

        while self.at_kind(TokenKind::Dot) {
            self.advance();
            let (segment, segment_span) = self.parse_identifier("需要路径片段");
            if let Some(segment) = segment {
                path.push(segment);
            }
            span = TextRange::new(start, segment_span.end);
        }

        if path.is_empty() {
            path.push("<missing>".to_owned());
        }

        (path, span)
    }

    fn parse_identifier(&mut self, message: &'static str) -> (Option<String>, TextRange) {
        if self.at_kind(TokenKind::Identifier) {
            let token = self.advance();
            (Some(token.text), token.range)
        } else {
            let span = self.current_range();
            self.push_expected(message, span);
            (None, span)
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        let op = match self.current_kind()? {
            TokenKind::Equal => (BinaryOp::Assign, 1, 1),
            TokenKind::Less => (BinaryOp::Less, 3, 4),
            TokenKind::Greater => (BinaryOp::Greater, 3, 4),
            TokenKind::Plus => (BinaryOp::Add, 5, 6),
            TokenKind::Minus => (BinaryOp::Subtract, 5, 6),
            TokenKind::Star => (BinaryOp::Multiply, 7, 8),
            TokenKind::Slash => (BinaryOp::Divide, 7, 8),
            _ => return None,
        };
        Some(op)
    }

    fn expect_keyword(&mut self, keyword: Keyword, expected: &'static str) -> Token {
        if self.at_keyword(keyword) {
            self.advance()
        } else {
            self.push_expected(format!("需要 {expected}"), self.current_range());
            self.synthetic_token()
        }
    }

    fn expect_kind(&mut self, kind: TokenKind, expected: &'static str) -> Token {
        if self.at_kind(kind) {
            self.advance()
        } else {
            self.push_expected(format!("需要 {expected}"), self.current_range());
            self.synthetic_token()
        }
    }

    fn consume_optional_semicolon(&mut self) -> bool {
        if self.at_kind(TokenKind::Semicolon) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_comma_or_stop(&mut self, stop: TokenKind) -> bool {
        if self.at_kind(TokenKind::Comma) {
            self.advance();
            true
        } else if self.at_kind(stop)
            || (stop == TokenKind::RightParen && self.at_kind(TokenKind::LeftBrace))
        {
            false
        } else {
            self.error_current("需要 `,` 或列表结束");
            self.recover_list(stop);
            self.at_kind(TokenKind::Comma) && {
                self.advance();
                true
            }
        }
    }

    fn at_item_start(&self) -> bool {
        self.at_keyword(Keyword::Fn)
            || self.at_keyword(Keyword::Struct)
            || self.at_keyword(Keyword::Enum)
            || self.at_keyword(Keyword::Trait)
            || self.at_keyword(Keyword::Impl)
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.current_kind(), Some(TokenKind::Keyword(current)) if current == keyword)
    }

    fn at_kind(&self, kind: TokenKind) -> bool {
        matches!(
            (self.current_kind(), kind),
            (Some(TokenKind::Whitespace), TokenKind::Whitespace)
                | (Some(TokenKind::LineComment), TokenKind::LineComment)
                | (Some(TokenKind::BlockComment), TokenKind::BlockComment)
                | (Some(TokenKind::Identifier), TokenKind::Identifier)
                | (Some(TokenKind::IntegerLiteral), TokenKind::IntegerLiteral)
                | (Some(TokenKind::BoolLiteral), TokenKind::BoolLiteral)
                | (Some(TokenKind::StringLiteral), TokenKind::StringLiteral)
                | (Some(TokenKind::Error), TokenKind::Error)
                | (Some(TokenKind::LeftParen), TokenKind::LeftParen)
                | (Some(TokenKind::RightParen), TokenKind::RightParen)
                | (Some(TokenKind::LeftBrace), TokenKind::LeftBrace)
                | (Some(TokenKind::RightBrace), TokenKind::RightBrace)
                | (Some(TokenKind::LeftBracket), TokenKind::LeftBracket)
                | (Some(TokenKind::RightBracket), TokenKind::RightBracket)
                | (Some(TokenKind::Comma), TokenKind::Comma)
                | (Some(TokenKind::Colon), TokenKind::Colon)
                | (Some(TokenKind::Semicolon), TokenKind::Semicolon)
                | (Some(TokenKind::Dot), TokenKind::Dot)
                | (Some(TokenKind::Arrow), TokenKind::Arrow)
                | (Some(TokenKind::FatArrow), TokenKind::FatArrow)
                | (Some(TokenKind::Less), TokenKind::Less)
                | (Some(TokenKind::Greater), TokenKind::Greater)
                | (Some(TokenKind::Question), TokenKind::Question)
                | (Some(TokenKind::Equal), TokenKind::Equal)
                | (Some(TokenKind::Plus), TokenKind::Plus)
                | (Some(TokenKind::Minus), TokenKind::Minus)
                | (Some(TokenKind::Star), TokenKind::Star)
                | (Some(TokenKind::Slash), TokenKind::Slash)
                | (Some(TokenKind::Eof), TokenKind::Eof)
        )
    }

    fn at_identifier_text(&self, text: &str) -> bool {
        matches!(self.current(), Some(token) if token.kind == TokenKind::Identifier && token.text == text)
    }

    fn at_eof(&self) -> bool {
        self.at_kind(TokenKind::Eof) || self.index >= self.tokens.len()
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|token| token.kind)
    }

    fn current_range(&self) -> TextRange {
        self.current().map_or_else(
            || TextRange::at(self.source.len_bytes()),
            |token| token.range,
        )
    }

    fn advance(&mut self) -> Token {
        let token = self
            .current()
            .cloned()
            .unwrap_or_else(|| self.synthetic_token());
        if self.index < self.tokens.len() {
            self.index += 1;
        }
        token
    }

    fn synthetic_token(&self) -> Token {
        let range = self.current_range();
        Token {
            kind: TokenKind::Error,
            range,
            text: String::new(),
        }
    }

    fn error_current(&mut self, message: impl Into<String>) {
        self.push_diagnostic(MD_UNEXPECTED_TOKEN, message, self.current_range());
    }

    fn push_expected(&mut self, message: impl Into<String>, range: TextRange) {
        self.push_diagnostic(MD_EXPECTED_SYNTAX, message, range);
    }

    fn push_diagnostic(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        range: TextRange,
    ) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(code).expect("parser diagnostic code must be valid"),
            DiagnosticSeverity::Error,
            message,
        );

        let diagnostic = if let Some(span) = DiagnosticSpan::from_source(self.source, range) {
            diagnostic.with_span(span)
        } else {
            diagnostic
        };

        self.diagnostics.push(diagnostic);
    }

    fn recover_top_level(&mut self) {
        while !self.at_eof()
            && !self.at_keyword(Keyword::Module)
            && !self.at_keyword(Keyword::Import)
            && !self.at_item_start()
        {
            self.advance();
        }
    }

    fn recover_member(&mut self) {
        while !self.at_eof()
            && !self.at_kind(TokenKind::RightBrace)
            && !self.at_kind(TokenKind::Semicolon)
            && !self.at_kind(TokenKind::Comma)
            && !self.at_keyword(Keyword::Fn)
        {
            self.advance();
        }
        self.consume_optional_semicolon();
    }

    fn recover_list(&mut self, stop: TokenKind) {
        while !self.at_eof() && !self.at_kind(stop) && !self.at_kind(TokenKind::Comma) {
            self.advance();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FunctionBodyMode {
    Required,
    Optional,
}

fn is_trivia(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment
    )
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId};

    use super::{parse_source, MD_EXPECTED_SYNTAX};

    #[test]
    fn parses_complete_file_into_stable_ast_dump() {
        let source = SourceFile::new(
            SourceId::new(1),
            "main.mao",
            "module demo.app\nimport core.io\nstruct Point { x: Int, y: Int }\nenum Option { Some(Int), None }\ntrait Show { fn show(value: Point) -> String; }\nimpl Show for Point { fn show(value: Point) -> String { return \"点\" } }\nfn main<T>(value: T) -> Bool { let mut count: Int = 1 + 2 * 3; match count { 0 => false, _ => true } }\n",
        );

        let result = parse_source(&source);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.ast.dump(),
            "\
File @0..319
  Module demo.app @0..15
  Import core.io @16..30
  Struct Point @31..62
    Field x @46..52
      Type Int @49..52
    Field y @54..60
      Type Int @57..60
  Enum Option @63..94
    Variant Some @77..86
      Type Int @82..85
    Variant None @88..92
  Trait Show @95..142
    Fn show @108..139
      Param value @116..128
        Type Point @123..128
      ReturnType
        Type String @133..139
  Impl @143..215
    Trait
      Type Show @148..152
    Target
      Type Point @157..162
    Fn show @165..213
      Param value @173..185
        Type Point @180..185
      ReturnType
        Type String @190..196
      Block @197..213
        Return @199..211
          Literal string(\"点\") @206..211
  Fn main @216..318
    Generics T
    Param value @227..235
      Type T @234..235
    ReturnType
      Type Bool @240..244
    Block @245..318
      Let mut count @247..277
        Type
          Type Int @262..265
        Value
          Binary + @268..277
            Literal int(1) @268..269
            Binary * @272..277
              Literal int(2) @272..273
              Literal int(3) @276..277
      Match @279..316
        Scrutinee
          Path count @285..290
        Arm @293..303
          Pattern Literal int(0) @293..294
          Literal bool(false) @298..303
        Arm @305..314
          Pattern _ @305..306
          Literal bool(true) @310..314"
        );
    }

    #[test]
    fn recovers_after_missing_type_and_continues_next_item() {
        let source = SourceFile::new(
            SourceId::new(1),
            "broken.mao",
            "fn broken(value:) { let x = ; }\nfn next() { return true }\n",
        );

        let result = parse_source(&source);
        let dump = result.ast.dump();

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_EXPECTED_SYNTAX
                && diagnostic.message.contains("需要参数类型")));
        assert!(dump.contains("Fn broken"));
        assert!(dump.contains("Fn next"));
    }

    #[test]
    fn recovers_after_missing_closing_paren() {
        let source = SourceFile::new(
            SourceId::new(1),
            "broken.mao",
            "fn broken(value: Int { return true }\nfn next() { return false }\n",
        );

        let result = parse_source(&source);
        let dump = result.ast.dump();

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == MD_EXPECTED_SYNTAX
                && diagnostic.message.contains("需要 `)`")));
        assert!(dump.contains("Fn broken"));
        assert!(dump.contains("Fn next"));
    }

    #[test]
    fn recovers_after_illegal_top_level_expression() {
        let source = SourceFile::new(SourceId::new(1), "broken.mao", "123\nstruct A {}\n");
        let result = parse_source(&source);

        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "顶层只能出现 module、import 或声明"));
        assert!(result.ast.dump().contains("Struct A"));
    }
}
