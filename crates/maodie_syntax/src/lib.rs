//! Source-level syntax utilities for Maodie.

mod ast;
mod lexer;
mod parser;

pub use ast::{
    AstFile, AstNode, BinaryOp, BlockExpr, EnumDecl, EnumVariant, Expr, FieldDecl, FunctionDecl,
    FunctionParam, ImplDecl, ImportDecl, Item, LetStmt, Literal, MatchArm, ModuleDecl, ParamList,
    Pattern, Statement, StructDecl, TraitDecl, TypeRef,
};
pub use lexer::{
    lex_source, Keyword, LexResult, Lexer, Token, TokenKind, MD_INVALID_CHARACTER,
    MD_UNTERMINATED_BLOCK_COMMENT, MD_UNTERMINATED_STRING,
};
pub use parser::{parse_source, ParseResult, Parser, MD_EXPECTED_SYNTAX, MD_UNEXPECTED_TOKEN};
