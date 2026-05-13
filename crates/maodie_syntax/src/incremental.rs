use std::{error::Error, fmt};

use maodie_diagnostics::{Diagnostic, DiagnosticSpan, SourceFile, TextRange};
use serde::{Deserialize, Serialize};

use crate::{highlight::highlight_tokens_from_lex_tokens, HighlightToken, Lexer, Token, TokenKind};

/// A source replacement edit expressed against the session's old UTF-8 byte offsets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HighlightEdit {
    /// Old-source byte range to replace.
    pub range: TextRange,
    /// Replacement source text.
    pub replacement: String,
}

/// Result returned after a reset or update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncrementalHighlightUpdate {
    /// Session version after applying the change.
    pub version: u64,
    /// New-source byte range whose highlight data was recomputed or invalidated.
    pub changed_range: TextRange,
    /// Current full highlight token set after patching the session cache.
    pub tokens: Vec<HighlightToken>,
    /// Current lexer diagnostics after patching the session cache.
    pub diagnostics: Vec<Diagnostic>,
    /// True when the session intentionally rebuilt from the full source.
    pub full_rehighlight: bool,
}

/// Error returned when an incremental highlight edit cannot be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncrementalHighlightError {
    /// The edit range is outside the old source or splits a UTF-8 code point.
    InvalidEditRange { range: TextRange, source_len: usize },
}

impl fmt::Display for IncrementalHighlightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEditRange { range, source_len } => write!(
                formatter,
                "invalid highlight edit range {}..{} for source length {}",
                range.start, range.end, source_len
            ),
        }
    }
}

impl Error for IncrementalHighlightError {}

/// Stateful lexer-backed syntax highlight session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncrementalHighlightSession {
    source: SourceFile,
    lex_tokens: Vec<Token>,
    highlight_tokens: Vec<HighlightToken>,
    diagnostics: Vec<Diagnostic>,
    version: u64,
}

impl IncrementalHighlightSession {
    /// Creates a session at version 0 by fully highlighting the initial source.
    #[must_use]
    pub fn new(source: SourceFile) -> Self {
        let lex_result = crate::lex_source(&source);
        let highlight_tokens = highlight_tokens_from_lex_tokens(&lex_result.tokens);

        Self {
            source,
            lex_tokens: lex_result.tokens,
            highlight_tokens,
            diagnostics: lex_result.diagnostics,
            version: 0,
        }
    }

    /// Returns the current session version.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Returns the current source snapshot.
    #[must_use]
    pub const fn source(&self) -> &SourceFile {
        &self.source
    }

    /// Returns the cached full lexer token stream, including trivia and EOF.
    #[must_use]
    pub fn lex_tokens(&self) -> &[Token] {
        &self.lex_tokens
    }

    /// Returns the cached full highlight token stream.
    #[must_use]
    pub fn tokens(&self) -> &[HighlightToken] {
        &self.highlight_tokens
    }

    /// Returns the cached lexer diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Replaces the entire source snapshot and increments the session version.
    pub fn reset(&mut self, source: SourceFile) -> IncrementalHighlightUpdate {
        self.version += 1;
        self.source = source;
        self.rebuild_full(TextRange::new(0, self.source.len_bytes()), true)
    }

    /// Applies one source replacement edit against the old session source.
    ///
    /// The edit range must be a valid UTF-8 byte range in the old source.
    ///
    /// # Errors
    ///
    /// Returns [`IncrementalHighlightError::InvalidEditRange`] when the edit range is outside the
    /// old source or splits a UTF-8 code point.
    pub fn update(
        &mut self,
        edit: HighlightEdit,
    ) -> Result<IncrementalHighlightUpdate, IncrementalHighlightError> {
        if !self.source.is_valid_range(edit.range) {
            return Err(IncrementalHighlightError::InvalidEditRange {
                range: edit.range,
                source_len: self.source.len_bytes(),
            });
        }

        self.version += 1;

        let old_source = self.source.clone();
        let old_tokens = self.lex_tokens.clone();
        let old_diagnostics = self.diagnostics.clone();
        let new_text = apply_replacement(old_source.text(), edit.range, &edit.replacement);
        let new_source = SourceFile::new(old_source.id(), old_source.name(), new_text);
        let delta = edit.replacement.len() as isize - edit.range.len() as isize;
        let relex_start = safe_relex_start(&old_tokens, edit.range);
        self.source = new_source;

        let old_relex_index = first_token_starting_at_or_after(&old_tokens, relex_start);
        let old_sync_index = first_token_starting_at_or_after(&old_tokens, edit.range.end);
        let prefix_tokens = old_tokens[..old_relex_index].to_vec();
        let prefix_diagnostics = old_diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic_start(diagnostic).is_some_and(|start| start < relex_start)
            })
            .cloned()
            .collect::<Vec<_>>();

        match relex_until_sync(
            &self.source,
            relex_start,
            &old_tokens,
            old_sync_index,
            delta,
        ) {
            Some(sync) => {
                let mut lex_tokens = prefix_tokens;
                lex_tokens.extend(sync.patch_tokens);
                lex_tokens.extend(
                    old_tokens[sync.old_suffix_index..]
                        .iter()
                        .filter_map(|token| shift_token(token, delta)),
                );

                let mut diagnostics = prefix_diagnostics;
                diagnostics.extend(sync.patch_diagnostics);
                diagnostics.extend(
                    old_diagnostics
                        .iter()
                        .filter(|diagnostic| {
                            diagnostic_start(diagnostic)
                                .is_some_and(|start| start >= sync.old_suffix_start)
                        })
                        .filter_map(|diagnostic| shift_diagnostic(diagnostic, delta, &self.source)),
                );

                let changed_range = TextRange::new(
                    relex_start,
                    sync.new_suffix_start
                        .max(edit.range.start + edit.replacement.len()),
                );
                Ok(self.replace_caches(lex_tokens, diagnostics, changed_range, false))
            }
            None => Ok(self.rebuild_full(TextRange::new(0, self.source.len_bytes()), true)),
        }
    }

    fn replace_caches(
        &mut self,
        lex_tokens: Vec<Token>,
        diagnostics: Vec<Diagnostic>,
        changed_range: TextRange,
        full_rehighlight: bool,
    ) -> IncrementalHighlightUpdate {
        let highlight_tokens = highlight_tokens_from_lex_tokens(&lex_tokens);
        self.lex_tokens = lex_tokens;
        self.highlight_tokens = highlight_tokens;
        self.diagnostics = diagnostics;
        self.update_response(changed_range, full_rehighlight)
    }

    fn rebuild_full(
        &mut self,
        changed_range: TextRange,
        full_rehighlight: bool,
    ) -> IncrementalHighlightUpdate {
        let lex_result = crate::lex_source(&self.source);
        self.highlight_tokens = highlight_tokens_from_lex_tokens(&lex_result.tokens);
        self.lex_tokens = lex_result.tokens;
        self.diagnostics = lex_result.diagnostics;
        self.update_response(changed_range, full_rehighlight)
    }

    fn update_response(
        &self,
        changed_range: TextRange,
        full_rehighlight: bool,
    ) -> IncrementalHighlightUpdate {
        IncrementalHighlightUpdate {
            version: self.version,
            changed_range,
            tokens: self.highlight_tokens.clone(),
            diagnostics: self.diagnostics.clone(),
            full_rehighlight,
        }
    }
}

struct IncrementalSync {
    patch_tokens: Vec<Token>,
    patch_diagnostics: Vec<Diagnostic>,
    old_suffix_index: usize,
    old_suffix_start: usize,
    new_suffix_start: usize,
}

fn apply_replacement(source: &str, range: TextRange, replacement: &str) -> String {
    let mut patched = String::with_capacity(source.len() - range.len() + replacement.len());
    patched.push_str(&source[..range.start]);
    patched.push_str(replacement);
    patched.push_str(&source[range.end..]);
    patched
}

fn safe_relex_start(tokens: &[Token], edit_range: TextRange) -> usize {
    if let Some(token) = tokens
        .iter()
        .find(|token| should_fallback_to_token_start(token, edit_range))
    {
        return token.range.start;
    }

    let edit_start = edit_range.start;
    let mut token_index = tokens
        .iter()
        .position(|token| token.range.end >= edit_start)
        .unwrap_or_else(|| tokens.len().saturating_sub(1));

    if token_index > 0 {
        token_index -= 1;
    }

    let token = &tokens[token_index];
    if token.kind != TokenKind::Eof {
        token.range.start
    } else {
        edit_start
    }
}

fn should_fallback_to_token_start(token: &Token, edit_range: TextRange) -> bool {
    matches!(
        token.kind,
        TokenKind::BlockComment | TokenKind::StringLiteral | TokenKind::Error
    ) && ranges_touch_or_overlap(token.range, edit_range)
}

fn ranges_touch_or_overlap(left: TextRange, right: TextRange) -> bool {
    left.start <= right.end && right.start <= left.end
}

fn first_token_starting_at_or_after(tokens: &[Token], offset: usize) -> usize {
    tokens
        .iter()
        .position(|token| token.range.start >= offset)
        .unwrap_or(tokens.len())
}

fn relex_until_sync(
    source: &SourceFile,
    relex_start: usize,
    old_tokens: &[Token],
    old_sync_index: usize,
    delta: isize,
) -> Option<IncrementalSync> {
    let mut lexer = Lexer::with_offset(source, relex_start);
    let mut patch_tokens = Vec::new();
    let mut patch_diagnostics = Vec::new();

    while let Some(new_token) = lexer.next_token() {
        patch_diagnostics.extend(lexer.drain_diagnostics());

        if let Some(old_suffix_index) =
            find_sync_token(&new_token, old_tokens, old_sync_index, delta)
        {
            let old_suffix_start = old_tokens[old_suffix_index].range.start;
            let new_suffix_start = new_token.range.start;
            return Some(IncrementalSync {
                patch_tokens,
                patch_diagnostics,
                old_suffix_index,
                old_suffix_start,
                new_suffix_start,
            });
        }

        let is_eof = new_token.kind == TokenKind::Eof;
        patch_tokens.push(new_token);
        if is_eof {
            return Some(IncrementalSync {
                patch_tokens,
                patch_diagnostics,
                old_suffix_index: old_tokens.len(),
                old_suffix_start: usize::MAX,
                new_suffix_start: source.len_bytes(),
            });
        }
    }

    None
}

fn find_sync_token(
    new_token: &Token,
    old_tokens: &[Token],
    old_sync_index: usize,
    delta: isize,
) -> Option<usize> {
    old_tokens[old_sync_index..]
        .iter()
        .position(|old_token| token_matches_shifted_old(new_token, old_token, delta))
        .map(|relative_index| old_sync_index + relative_index)
}

fn token_matches_shifted_old(new_token: &Token, old_token: &Token, delta: isize) -> bool {
    old_token.kind == new_token.kind
        && old_token.text == new_token.text
        && shift_range(old_token.range, delta).is_some_and(|range| range == new_token.range)
}

fn shift_token(token: &Token, delta: isize) -> Option<Token> {
    Some(Token {
        kind: token.kind,
        range: shift_range(token.range, delta)?,
        text: token.text.clone(),
    })
}

fn shift_diagnostic(
    diagnostic: &Diagnostic,
    delta: isize,
    source: &SourceFile,
) -> Option<Diagnostic> {
    let span = diagnostic.span.as_ref()?;
    let range = shift_range(span.range, delta)?;
    let resolved_span = DiagnosticSpan::from_source(source, range)?;

    let mut shifted = diagnostic.clone();
    shifted.span = Some(resolved_span);
    Some(shifted)
}

fn shift_range(range: TextRange, delta: isize) -> Option<TextRange> {
    Some(TextRange::new(
        shift_offset(range.start, delta)?,
        shift_offset(range.end, delta)?,
    ))
}

fn shift_offset(offset: usize, delta: isize) -> Option<usize> {
    if delta >= 0 {
        offset.checked_add(delta as usize)
    } else {
        offset.checked_sub(delta.unsigned_abs())
    }
}

fn diagnostic_start(diagnostic: &Diagnostic) -> Option<usize> {
    diagnostic.span.as_ref().map(|span| span.range.start)
}

#[cfg(test)]
mod tests {
    use maodie_diagnostics::{SourceFile, SourceId, TextRange};

    use super::{HighlightEdit, IncrementalHighlightError, IncrementalHighlightSession};
    use crate::{highlight_source, HighlightResult};

    #[test]
    fn creates_session_matching_full_highlight() {
        let source = source("module 示例\nfn main() -> bool { true }\n");
        let session = IncrementalHighlightSession::new(source.clone());

        assert_eq!(session.version(), 0);
        assert_eq!(session.tokens(), highlight_source(&source).tokens);
        assert_eq!(session.diagnostics(), highlight_source(&source).diagnostics);
    }

    #[test]
    fn updates_chinese_identifier_and_emoji_edits() {
        assert_edit_sequence_matches_full(
            "fn main() {\n  let 名称 = 1\n}\n",
            &[
                TestEdit::replace("名称", "值称"),
                TestEdit::insert_after_text("值称", "🙂"),
            ],
        );
    }

    #[test]
    fn updates_arrow_and_fat_arrow_boundaries() {
        assert_edit_sequence_matches_full(
            "fn map<T>(x: T) - bool { match x { _ = x } }\n",
            &[TestEdit::insert_after("-"), TestEdit::insert_after("=")],
        );
    }

    #[test]
    fn updates_block_comment_open_and_close() {
        assert_edit_sequence_matches_full(
            "fn main() {\n  /* comment\n  let x = 1\n}\n",
            &[
                TestEdit::insert_after_text("comment", " */"),
                TestEdit::delete("/*"),
            ],
        );
    }

    #[test]
    fn updates_unclosed_string_recovery() {
        assert_edit_sequence_matches_full(
            "fn main() {\n  let text = \"未闭合\n  return text\n}\n",
            &[TestEdit::insert_after_text("未闭合", "\"")],
        );
    }

    #[test]
    fn updates_invalid_character_recovery() {
        assert_edit_sequence_matches_full(
            "fn main() {\n  let x = @\n}\n",
            &[TestEdit::replace("@", "42")],
        );
    }

    #[test]
    fn handles_full_source_replacement() {
        let initial = "module old\nfn main() { false }\n";
        let mut session = IncrementalHighlightSession::new(source(initial));

        session
            .update(HighlightEdit {
                range: TextRange::new(0, initial.len()),
                replacement: "module 新\nfn main() -> bool { true }\n".to_owned(),
            })
            .expect("full source replacement range is valid");

        assert_session_matches_full(&session);
    }

    #[test]
    fn reset_increments_version_and_rebuilds() {
        let mut session = IncrementalHighlightSession::new(source("let x = 1\n"));
        let next_source = source("let 名 = \"ok\"\n");

        let update = session.reset(next_source.clone());

        assert_eq!(update.version, 1);
        assert!(update.full_rehighlight);
        assert_eq!(
            update.changed_range,
            TextRange::new(0, next_source.len_bytes())
        );
        assert_session_matches_full(&session);
    }

    #[test]
    fn rejects_ranges_that_split_utf8_codepoints() {
        let mut session = IncrementalHighlightSession::new(source("let 名 = 1\n"));

        let error = session
            .update(HighlightEdit {
                range: TextRange::new(5, 6),
                replacement: "x".to_owned(),
            })
            .expect_err("range splits a Chinese codepoint");

        assert_eq!(
            error,
            IncrementalHighlightError::InvalidEditRange {
                range: TextRange::new(5, 6),
                source_len: "let 名 = 1\n".len(),
            }
        );
        assert_eq!(session.version(), 0);
    }

    fn assert_edit_sequence_matches_full(initial: &str, edits: &[TestEdit]) {
        let mut session = IncrementalHighlightSession::new(source(initial));

        for edit in edits {
            let edit = edit.resolve(session.source().text());
            let update = session
                .update(edit)
                .expect("edit range is valid for current source");
            assert_eq!(update.version, session.version());
            assert_session_matches_full(&session);
        }
    }

    fn assert_session_matches_full(session: &IncrementalHighlightSession) {
        let full = highlight_source(session.source());
        assert_eq!(session.tokens(), full.tokens);
        assert_eq!(session.diagnostics(), full.diagnostics);
        assert_eq!(
            HighlightResult {
                tokens: session.tokens().to_vec(),
                diagnostics: session.diagnostics().to_vec(),
            },
            full
        );
    }

    fn source(text: &str) -> SourceFile {
        SourceFile::new(SourceId::new(1), "incremental.mao", text)
    }

    struct TestEdit {
        needle: &'static str,
        offset_in_needle: usize,
        delete_len: usize,
        replacement: &'static str,
    }

    impl TestEdit {
        const fn replace(needle: &'static str, replacement: &'static str) -> Self {
            Self {
                needle,
                offset_in_needle: 0,
                delete_len: needle.len(),
                replacement,
            }
        }

        const fn delete(needle: &'static str) -> Self {
            Self::replace(needle, "")
        }

        const fn insert_after(needle: &'static str) -> Self {
            Self::insert_after_text(needle, ">")
        }

        const fn insert_after_text(needle: &'static str, replacement: &'static str) -> Self {
            Self {
                needle,
                offset_in_needle: needle.len(),
                delete_len: 0,
                replacement,
            }
        }

        fn resolve(&self, source: &str) -> HighlightEdit {
            let start =
                source.find(self.needle).expect("test edit needle exists") + self.offset_in_needle;
            HighlightEdit {
                range: TextRange::new(start, start + self.delete_len),
                replacement: self.replacement.to_owned(),
            }
        }
    }
}
