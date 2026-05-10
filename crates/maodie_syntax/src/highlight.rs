use maodie_diagnostics::{Diagnostic, SourceFile, TextRange};
use serde::{Deserialize, Serialize};

use crate::{lex_source, TokenKind};

/// Result produced by a syntax highlighting pass.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HighlightResult {
    /// Stable syntax highlight tokens, excluding whitespace and EOF.
    pub tokens: Vec<HighlightToken>,
    /// Lexical diagnostics produced while scanning.
    pub diagnostics: Vec<Diagnostic>,
}

/// One syntax highlight token with its source byte range.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HighlightToken {
    /// Syntax-level highlight classification.
    pub kind: HighlightKind,
    /// Half-open byte range in the source file.
    pub range: TextRange,
}

/// Stable syntax-level highlight kinds shared by editor integrations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightKind {
    /// Language keyword.
    Keyword,
    /// Identifier without semantic classification.
    Identifier,
    /// Line or block comment.
    Comment,
    /// String literal.
    String,
    /// Numeric literal.
    Number,
    /// Boolean literal.
    Boolean,
    /// Operator token.
    Operator,
    /// Delimiter or separator punctuation.
    Punctuation,
    /// Invalid or incomplete source text.
    Error,
}

/// Highlights one source file using lexer-level token classifications.
#[must_use]
pub fn highlight_source(source: &SourceFile) -> HighlightResult {
    let lex_result = lex_source(source);
    let tokens = lex_result
        .tokens
        .into_iter()
        .filter_map(|token| {
            highlight_kind_for_token(token.kind).map(|kind| HighlightToken {
                kind,
                range: token.range,
            })
        })
        .collect();

    HighlightResult {
        tokens,
        diagnostics: lex_result.diagnostics,
    }
}

fn highlight_kind_for_token(kind: TokenKind) -> Option<HighlightKind> {
    match kind {
        TokenKind::Whitespace | TokenKind::Eof => None,
        TokenKind::LineComment | TokenKind::BlockComment => Some(HighlightKind::Comment),
        TokenKind::Keyword(_) => Some(HighlightKind::Keyword),
        TokenKind::Identifier => Some(HighlightKind::Identifier),
        TokenKind::IntegerLiteral => Some(HighlightKind::Number),
        TokenKind::BoolLiteral => Some(HighlightKind::Boolean),
        TokenKind::StringLiteral => Some(HighlightKind::String),
        TokenKind::Error => Some(HighlightKind::Error),
        TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::Question
        | TokenKind::Equal
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash => Some(HighlightKind::Operator),
        TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Comma
        | TokenKind::Colon
        | TokenKind::Semicolon
        | TokenKind::Dot => Some(HighlightKind::Punctuation),
    }
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId, TextRange};
    use serde::Deserialize;

    use super::{highlight_source, HighlightKind, HighlightToken};
    use crate::MD_INVALID_CHARACTER;

    const SHARED_FIXTURE_SOURCE: &str =
        include_str!("../../../docs/tasks/highlighting/fixtures/syntax-highlight.mao");
    const SHARED_FIXTURE_GOLDEN: &str =
        include_str!("../../../docs/tasks/highlighting/fixtures/syntax-highlight.tokens.json");

    #[test]
    fn maps_lexer_tokens_to_highlight_kinds() {
        let source = SourceFile::new(
            SourceId::new(1),
            "highlight.mao",
            "module demo\nfn main<T>(value: T) -> bool {\n  let flag = true\n  let n = 42\n  let s = \"ok\"\n  // hi\n  flag?\n}\n",
        );

        let result = highlight_source(&source);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            dump_tokens(&result.tokens),
            "\
keyword@0..6
identifier@7..11
keyword@12..14
identifier@15..19
operator@19..20
identifier@20..21
operator@21..22
punctuation@22..23
identifier@23..28
punctuation@28..29
identifier@30..31
punctuation@31..32
operator@33..35
identifier@36..40
punctuation@41..42
keyword@45..48
identifier@49..53
operator@54..55
boolean@56..60
keyword@63..66
identifier@67..68
operator@69..70
number@71..73
keyword@76..79
identifier@80..81
operator@82..83
string@84..88
comment@91..96
identifier@99..103
operator@103..104
punctuation@105..106"
        );
    }

    #[test]
    fn omits_whitespace_and_eof_tokens() {
        let source = SourceFile::new(SourceId::new(1), "empty-ish.mao", "  \n\t");
        let result = highlight_source(&source);

        assert!(result.tokens.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn preserves_error_tokens_and_lexer_diagnostics() {
        let source = SourceFile::new(SourceId::new(1), "bad.mao", "let x = @");
        let result = highlight_source(&source);

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code.as_str(), MD_INVALID_CHARACTER);
        assert!(result.tokens.contains(&HighlightToken {
            kind: HighlightKind::Error,
            range: TextRange::new(8, 9),
        }));
    }

    #[test]
    fn matches_shared_highlight_golden_fixture() {
        let golden: GoldenFixture =
            serde_json::from_str(SHARED_FIXTURE_GOLDEN).expect("highlight golden is valid JSON");
        let source = SourceFile::new(SourceId::new(1), &golden.source_path, SHARED_FIXTURE_SOURCE);
        let result = highlight_source(&source);

        assert_eq!(result.tokens, golden.tokens);
        assert_eq!(result.diagnostics.len(), golden.diagnostics.len());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|diagnostic| DiagnosticGolden {
                    code: diagnostic.code.to_string(),
                    range: diagnostic
                        .span
                        .as_ref()
                        .expect("fixture diagnostic has a source span")
                        .range,
                })
                .collect::<Vec<_>>(),
            golden.diagnostics
        );
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GoldenFixture {
        source_path: String,
        tokens: Vec<HighlightToken>,
        diagnostics: Vec<DiagnosticGolden>,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct DiagnosticGolden {
        code: String,
        range: TextRange,
    }

    fn dump_tokens(tokens: &[HighlightToken]) -> String {
        tokens
            .iter()
            .map(|token| {
                format!(
                    "{}@{}..{}",
                    dump_kind(token.kind),
                    token.range.start,
                    token.range.end
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn dump_kind(kind: HighlightKind) -> &'static str {
        match kind {
            HighlightKind::Keyword => "keyword",
            HighlightKind::Identifier => "identifier",
            HighlightKind::Comment => "comment",
            HighlightKind::String => "string",
            HighlightKind::Number => "number",
            HighlightKind::Boolean => "boolean",
            HighlightKind::Operator => "operator",
            HighlightKind::Punctuation => "punctuation",
            HighlightKind::Error => "error",
        }
    }
}
