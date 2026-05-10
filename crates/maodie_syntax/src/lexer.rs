use maodie_diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticSpan, SourceFile, TextRange,
};
use serde::{Deserialize, Serialize};
use unicode_ident::{is_xid_continue, is_xid_start};

/// Illegal character diagnostic code.
pub const MD_INVALID_CHARACTER: &str = "MD0101";
/// Unterminated string literal diagnostic code.
pub const MD_UNTERMINATED_STRING: &str = "MD0102";
/// Unterminated block comment diagnostic code.
pub const MD_UNTERMINATED_BLOCK_COMMENT: &str = "MD0103";

/// Result produced by a lexing pass.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LexResult {
    /// Stable token stream, including trivia and EOF.
    pub tokens: Vec<Token>,
    /// Lexical diagnostics produced while scanning.
    pub diagnostics: Vec<Diagnostic>,
}

/// One token with its source text and byte range.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Half-open byte range in the source file.
    pub range: TextRange,
    /// Exact source text for this token.
    pub text: String,
}

/// Maodie token kinds produced by the lexer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// Whitespace trivia.
    Whitespace,
    /// `// ...` line comment trivia.
    LineComment,
    /// `/* ... */` block comment trivia.
    BlockComment,
    /// Language keyword.
    Keyword(Keyword),
    /// Unicode identifier.
    Identifier,
    /// Integer literal without suffix.
    IntegerLiteral,
    /// `true` or `false`.
    BoolLiteral,
    /// Double-quoted string literal.
    StringLiteral,
    /// Invalid or incomplete source text.
    Error,
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `.`
    Dot,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `?`
    Question,
    /// `=`
    Equal,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// End of file marker.
    Eof,
}

/// Reserved Maodie keywords.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Keyword {
    /// `module`
    Module,
    /// `import`
    Import,
    /// `fn`
    Fn,
    /// `let`
    Let,
    /// `mut`
    Mut,
    /// `struct`
    Struct,
    /// `enum`
    Enum,
    /// `trait`
    Trait,
    /// `impl`
    Impl,
    /// `if`
    If,
    /// `else`
    Else,
    /// `match`
    Match,
    /// `return`
    Return,
}

/// Lexes one source file.
#[must_use]
pub fn lex_source(source: &SourceFile) -> LexResult {
    Lexer::new(source).lex()
}

/// Stateful lexer for `.mao` source text.
#[derive(Debug)]
pub struct Lexer<'source> {
    source: &'source SourceFile,
    offset: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'source> Lexer<'source> {
    /// Creates a lexer for one source file.
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        Self {
            source,
            offset: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Runs the lexer to EOF.
    #[must_use]
    pub fn lex(mut self) -> LexResult {
        while !self.is_eof() {
            self.lex_one();
        }

        self.push_token(TokenKind::Eof, self.offset, self.offset);

        LexResult {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn lex_one(&mut self) {
        let start = self.offset;
        let Some(character) = self.current_char() else {
            return;
        };

        match character {
            character if character.is_whitespace() => self.lex_whitespace(start),
            '/' if self.starts_with("//") => self.lex_line_comment(start),
            '/' if self.starts_with("/*") => self.lex_block_comment(start),
            '"' => self.lex_string(start),
            character if character.is_ascii_digit() => self.lex_integer(start),
            character if is_identifier_start(character) => self.lex_identifier_or_keyword(start),
            '(' => self.consume_symbol(TokenKind::LeftParen, start),
            ')' => self.consume_symbol(TokenKind::RightParen, start),
            '{' => self.consume_symbol(TokenKind::LeftBrace, start),
            '}' => self.consume_symbol(TokenKind::RightBrace, start),
            '[' => self.consume_symbol(TokenKind::LeftBracket, start),
            ']' => self.consume_symbol(TokenKind::RightBracket, start),
            ',' => self.consume_symbol(TokenKind::Comma, start),
            ':' => self.consume_symbol(TokenKind::Colon, start),
            ';' => self.consume_symbol(TokenKind::Semicolon, start),
            '.' => self.consume_symbol(TokenKind::Dot, start),
            '<' => self.consume_symbol(TokenKind::Less, start),
            '>' => self.consume_symbol(TokenKind::Greater, start),
            '?' => self.consume_symbol(TokenKind::Question, start),
            '+' => self.consume_symbol(TokenKind::Plus, start),
            '*' => self.consume_symbol(TokenKind::Star, start),
            '/' => self.consume_symbol(TokenKind::Slash, start),
            '-' if self.starts_with("->") => self.consume_fixed(TokenKind::Arrow, start, 2),
            '-' => self.consume_symbol(TokenKind::Minus, start),
            '=' if self.starts_with("=>") => self.consume_fixed(TokenKind::FatArrow, start, 2),
            '=' => self.consume_symbol(TokenKind::Equal, start),
            _ => self.lex_invalid_character(start, character),
        }
    }

    fn lex_whitespace(&mut self, start: usize) {
        while self.current_char().is_some_and(char::is_whitespace) {
            self.bump();
        }

        self.push_token(TokenKind::Whitespace, start, self.offset);
    }

    fn lex_line_comment(&mut self, start: usize) {
        self.consume_bytes(2);

        while let Some(character) = self.current_char() {
            if character == '\n' {
                break;
            }
            self.bump();
        }

        self.push_token(TokenKind::LineComment, start, self.offset);
    }

    fn lex_block_comment(&mut self, start: usize) {
        self.consume_bytes(2);

        while !self.is_eof() {
            if self.starts_with("*/") {
                self.consume_bytes(2);
                self.push_token(TokenKind::BlockComment, start, self.offset);
                return;
            }

            self.bump();
        }

        self.push_token(TokenKind::Error, start, self.offset);
        self.push_diagnostic(
            MD_UNTERMINATED_BLOCK_COMMENT,
            "块注释没有闭合",
            TextRange::new(start, self.offset),
        );
    }

    fn lex_string(&mut self, start: usize) {
        self.bump();

        while let Some(character) = self.current_char() {
            match character {
                '"' => {
                    self.bump();
                    self.push_token(TokenKind::StringLiteral, start, self.offset);
                    return;
                }
                '\n' | '\r' => {
                    self.push_token(TokenKind::Error, start, self.offset);
                    self.push_diagnostic(
                        MD_UNTERMINATED_STRING,
                        "字符串字面量没有闭合",
                        TextRange::new(start, self.offset),
                    );
                    return;
                }
                '\\' => {
                    self.bump();
                    if !self.is_eof() {
                        self.bump();
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }

        self.push_token(TokenKind::Error, start, self.offset);
        self.push_diagnostic(
            MD_UNTERMINATED_STRING,
            "字符串字面量没有闭合",
            TextRange::new(start, self.offset),
        );
    }

    fn lex_integer(&mut self, start: usize) {
        while self
            .current_char()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.bump();
        }

        self.push_token(TokenKind::IntegerLiteral, start, self.offset);
    }

    fn lex_identifier_or_keyword(&mut self, start: usize) {
        self.bump();

        while self.current_char().is_some_and(is_identifier_continue) {
            self.bump();
        }

        let text = self.slice(start, self.offset);
        let kind = match text {
            "true" | "false" => TokenKind::BoolLiteral,
            "module" => TokenKind::Keyword(Keyword::Module),
            "import" => TokenKind::Keyword(Keyword::Import),
            "fn" => TokenKind::Keyword(Keyword::Fn),
            "let" => TokenKind::Keyword(Keyword::Let),
            "mut" => TokenKind::Keyword(Keyword::Mut),
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "enum" => TokenKind::Keyword(Keyword::Enum),
            "trait" => TokenKind::Keyword(Keyword::Trait),
            "impl" => TokenKind::Keyword(Keyword::Impl),
            "if" => TokenKind::Keyword(Keyword::If),
            "else" => TokenKind::Keyword(Keyword::Else),
            "match" => TokenKind::Keyword(Keyword::Match),
            "return" => TokenKind::Keyword(Keyword::Return),
            _ => TokenKind::Identifier,
        };

        self.push_token(kind, start, self.offset);
    }

    fn lex_invalid_character(&mut self, start: usize, character: char) {
        self.bump();
        self.push_token(TokenKind::Error, start, self.offset);
        self.push_diagnostic(
            MD_INVALID_CHARACTER,
            format!("发现无法识别的字符 `{character}`"),
            TextRange::new(start, self.offset),
        );
    }

    fn consume_symbol(&mut self, kind: TokenKind, start: usize) {
        self.bump();
        self.push_token(kind, start, self.offset);
    }

    fn consume_fixed(&mut self, kind: TokenKind, start: usize, byte_len: usize) {
        self.consume_bytes(byte_len);
        self.push_token(kind, start, self.offset);
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            range: TextRange::new(start, end),
            text: self.slice(start, end).to_owned(),
        });
    }

    fn push_diagnostic(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        range: TextRange,
    ) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new(code).expect("lexer diagnostic code must be valid"),
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

    fn starts_with(&self, needle: &str) -> bool {
        self.remaining_text().starts_with(needle)
    }

    fn current_char(&self) -> Option<char> {
        self.remaining_text().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.current_char()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn consume_bytes(&mut self, byte_len: usize) {
        self.offset += byte_len;
    }

    fn is_eof(&self) -> bool {
        self.offset >= self.source.text().len()
    }

    fn remaining_text(&self) -> &str {
        &self.source.text()[self.offset..]
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.source.text()[start..end]
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || is_xid_start(character)
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || is_xid_continue(character)
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId, TextRange};

    use super::{
        lex_source, Keyword, Token, TokenKind, MD_INVALID_CHARACTER, MD_UNTERMINATED_BLOCK_COMMENT,
        MD_UNTERMINATED_STRING,
    };

    #[test]
    fn lexes_stable_token_stream_for_mao_source() {
        let source = SourceFile::new(
            SourceId::new(1),
            "examples/main.mao",
            "module 示例\nfn main<T>(value: T) -> bool {\n  let mut 名 = 42\n  // 注释\n  return true\n}\n",
        );

        let result = lex_source(&source);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            dump_tokens(&result.tokens),
            "\
keyword(module)@0..6 `module`
whitespace@6..7 ` `
identifier@7..13 `示例`
whitespace@13..14 `\\n`
keyword(fn)@14..16 `fn`
whitespace@16..17 ` `
identifier@17..21 `main`
less@21..22 `<`
identifier@22..23 `T`
greater@23..24 `>`
left_paren@24..25 `(`
identifier@25..30 `value`
colon@30..31 `:`
whitespace@31..32 ` `
identifier@32..33 `T`
right_paren@33..34 `)`
whitespace@34..35 ` `
arrow@35..37 `->`
whitespace@37..38 ` `
identifier@38..42 `bool`
whitespace@42..43 ` `
left_brace@43..44 `{`
whitespace@44..47 `\\n  `
keyword(let)@47..50 `let`
whitespace@50..51 ` `
keyword(mut)@51..54 `mut`
whitespace@54..55 ` `
identifier@55..58 `名`
whitespace@58..59 ` `
equal@59..60 `=`
whitespace@60..61 ` `
integer_literal@61..63 `42`
whitespace@63..66 `\\n  `
line_comment@66..75 `// 注释`
whitespace@75..78 `\\n  `
keyword(return)@78..84 `return`
whitespace@84..85 ` `
bool_literal@85..89 `true`
whitespace@89..90 `\\n`
right_brace@90..91 `}`
whitespace@91..92 `\\n`
eof@92..92 ``"
        );
    }

    #[test]
    fn lexes_string_comment_and_symbol_tokens() {
        let source = SourceFile::new(
            SourceId::new(1),
            "symbols.mao",
            "import foo.bar\nmatch x { _ => \"值\\\"\"; ? + - * / [] }\n/* ok */",
        );

        let result = lex_source(&source);

        assert!(result.diagnostics.is_empty());
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::FatArrow && token.text == "=>"));
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::StringLiteral && token.text == "\"值\\\"\""));
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::BlockComment && token.text == "/* ok */"));
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Question));
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::LeftBracket));
    }

    #[test]
    fn reports_invalid_character_with_chinese_diagnostic_and_span() {
        let source = SourceFile::new(SourceId::new(1), "bad.mao", "let x = @\n");
        let result = lex_source(&source);

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];

        assert_eq!(diagnostic.code.as_str(), MD_INVALID_CHARACTER);
        assert_eq!(diagnostic.message, "发现无法识别的字符 `@`");
        assert_eq!(
            diagnostic.span.as_ref().expect("diagnostic has span").range,
            TextRange::new(8, 9)
        );
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Error && token.text == "@"));
    }

    #[test]
    fn reports_unterminated_string_without_consuming_newline() {
        let source = SourceFile::new(SourceId::new(1), "bad.mao", "let x = \"未闭合\nreturn x");
        let result = lex_source(&source);

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];

        assert_eq!(diagnostic.code.as_str(), MD_UNTERMINATED_STRING);
        assert_eq!(diagnostic.message, "字符串字面量没有闭合");
        assert_eq!(
            diagnostic.span.as_ref().expect("diagnostic has span").range,
            TextRange::new(8, 18)
        );
        assert!(result
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Whitespace && token.text == "\n"));
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let source = SourceFile::new(SourceId::new(1), "bad.mao", "fn main() { /* 未闭合");
        let result = lex_source(&source);

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];

        assert_eq!(diagnostic.code.as_str(), MD_UNTERMINATED_BLOCK_COMMENT);
        assert_eq!(diagnostic.message, "块注释没有闭合");
        assert_eq!(
            diagnostic.span.as_ref().expect("diagnostic has span").range,
            TextRange::new(12, source.len_bytes())
        );
    }

    #[test]
    fn keeps_angle_brackets_as_plain_tokens() {
        let source = SourceFile::new(SourceId::new(1), "angles.mao", "if a < b { return a > b }");
        let result = lex_source(&source);
        let kinds = result
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(kinds.contains(&TokenKind::Less));
        assert!(kinds.contains(&TokenKind::Greater));
        assert!(result.diagnostics.is_empty());
    }

    fn dump_tokens(tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|token| {
                format!(
                    "{}@{}..{} `{}`",
                    dump_kind(token.kind),
                    token.range.start,
                    token.range.end,
                    escape_text(&token.text)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn dump_kind(kind: TokenKind) -> String {
        match kind {
            TokenKind::Keyword(keyword) => format!("keyword({})", dump_keyword(keyword)),
            TokenKind::Whitespace => "whitespace".to_owned(),
            TokenKind::LineComment => "line_comment".to_owned(),
            TokenKind::BlockComment => "block_comment".to_owned(),
            TokenKind::Identifier => "identifier".to_owned(),
            TokenKind::IntegerLiteral => "integer_literal".to_owned(),
            TokenKind::BoolLiteral => "bool_literal".to_owned(),
            TokenKind::StringLiteral => "string_literal".to_owned(),
            TokenKind::Error => "error".to_owned(),
            TokenKind::LeftParen => "left_paren".to_owned(),
            TokenKind::RightParen => "right_paren".to_owned(),
            TokenKind::LeftBrace => "left_brace".to_owned(),
            TokenKind::RightBrace => "right_brace".to_owned(),
            TokenKind::LeftBracket => "left_bracket".to_owned(),
            TokenKind::RightBracket => "right_bracket".to_owned(),
            TokenKind::Comma => "comma".to_owned(),
            TokenKind::Colon => "colon".to_owned(),
            TokenKind::Semicolon => "semicolon".to_owned(),
            TokenKind::Dot => "dot".to_owned(),
            TokenKind::Arrow => "arrow".to_owned(),
            TokenKind::FatArrow => "fat_arrow".to_owned(),
            TokenKind::Less => "less".to_owned(),
            TokenKind::Greater => "greater".to_owned(),
            TokenKind::Question => "question".to_owned(),
            TokenKind::Equal => "equal".to_owned(),
            TokenKind::Plus => "plus".to_owned(),
            TokenKind::Minus => "minus".to_owned(),
            TokenKind::Star => "star".to_owned(),
            TokenKind::Slash => "slash".to_owned(),
            TokenKind::Eof => "eof".to_owned(),
        }
    }

    fn dump_keyword(keyword: Keyword) -> &'static str {
        match keyword {
            Keyword::Module => "module",
            Keyword::Import => "import",
            Keyword::Fn => "fn",
            Keyword::Let => "let",
            Keyword::Mut => "mut",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Trait => "trait",
            Keyword::Impl => "impl",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::Match => "match",
            Keyword::Return => "return",
        }
    }

    fn escape_text(text: &str) -> String {
        text.replace('\\', "\\\\").replace('\n', "\\n")
    }
}
