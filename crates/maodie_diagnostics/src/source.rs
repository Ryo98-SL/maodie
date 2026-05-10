use serde::{Deserialize, Serialize};

/// Stable identifier for a source file inside one compiler session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceId(usize);

impl SourceId {
    /// Creates a source identifier from a caller-owned numeric id.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw numeric id.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Half-open byte range in a UTF-8 source string.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextRange {
    /// Inclusive byte offset where the range starts.
    pub start: usize,
    /// Exclusive byte offset where the range ends.
    pub end: usize,
}

impl TextRange {
    /// Creates a half-open byte range.
    ///
    /// # Panics
    ///
    /// Panics when `start` is greater than `end`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "text range start must not exceed end");
        Self { start, end }
    }

    /// Creates an empty range at one byte offset.
    #[must_use]
    pub const fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Returns the byte length of the range.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Returns true when the range has no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns true when `offset` is inside the half-open range.
    #[must_use]
    pub const fn contains(self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
}

/// 1-based human-readable line and column paired with the original byte offset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextPosition {
    /// 1-based line number.
    pub line: usize,
    /// 1-based Unicode scalar column number.
    pub column: usize,
    /// Original byte offset in the source string.
    pub byte_offset: usize,
}

/// Source text plus line index information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Creates a source file and indexes line starts.
    #[must_use]
    pub fn new(id: SourceId, name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let line_starts = line_starts(&text);

        Self {
            id,
            name: name.into(),
            text,
            line_starts,
        }
    }

    /// Returns the file id.
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }

    /// Returns the display name or path for this source file.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the full source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the source length in bytes.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    /// Returns true when the source text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the number of indexed lines.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns all 0-based byte offsets that start a line.
    #[must_use]
    pub fn line_start_offsets(&self) -> &[usize] {
        &self.line_starts
    }

    /// Converts a valid UTF-8 byte offset into a 1-based line and column.
    ///
    /// Returns `None` when the offset is out of bounds or splits a UTF-8 code point.
    #[must_use]
    pub fn position_at(&self, offset: usize) -> Option<TextPosition> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }

        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..offset].chars().count() + 1;

        Some(TextPosition {
            line: line_index + 1,
            column,
            byte_offset: offset,
        })
    }

    /// Returns true when the range is ordered, in bounds, and aligned to UTF-8 boundaries.
    #[must_use]
    pub fn is_valid_range(&self, range: TextRange) -> bool {
        range.start <= range.end
            && range.end <= self.text.len()
            && self.text.is_char_boundary(range.start)
            && self.text.is_char_boundary(range.end)
    }

    /// Returns the 1-based line text without a trailing line ending.
    #[must_use]
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());

        Some(self.text[start..end].trim_end_matches(['\r', '\n']))
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];

    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }

    starts
}

#[cfg(test)]
mod tests {
    use super::{SourceFile, SourceId, TextRange};

    #[test]
    fn maps_utf8_byte_offsets_to_line_and_column() {
        let source = SourceFile::new(SourceId::new(7), "main.mao", "let 名 = 1\n打印(名)\n");

        let name_offset = source.text().find('名').expect("Chinese identifier exists");
        let name_position = source
            .position_at(name_offset)
            .expect("offset starts at a valid UTF-8 boundary");
        assert_eq!(name_position.line, 1);
        assert_eq!(name_position.column, 5);
        assert_eq!(name_position.byte_offset, name_offset);

        let call_offset = source.text().find('打').expect("Chinese call exists");
        let call_position = source
            .position_at(call_offset)
            .expect("offset starts at a valid UTF-8 boundary");
        assert_eq!(call_position.line, 2);
        assert_eq!(call_position.column, 1);
    }

    #[test]
    fn rejects_offsets_inside_utf8_codepoints() {
        let source = SourceFile::new(SourceId::new(1), "main.mao", "名");

        assert!(source.position_at(1).is_none());
        assert!(!source.is_valid_range(TextRange::new(0, 1)));
        assert!(source.is_valid_range(TextRange::new(0, 3)));
    }

    #[test]
    fn exposes_ranges_and_line_text() {
        let source = SourceFile::new(SourceId::new(1), "main.mao", "one\r\ntwo\nthree");
        let range = TextRange::new(5, 8);

        assert_eq!(range.len(), 3);
        assert!(range.contains(5));
        assert!(!range.contains(8));
        assert_eq!(source.line_count(), 3);
        assert_eq!(source.line_text(2), Some("two"));
    }

    #[test]
    fn maps_eof_after_trailing_newline_to_next_line_start() {
        let source = SourceFile::new(SourceId::new(1), "main.mao", "let x = 1\n");
        let eof = source
            .position_at(source.len_bytes())
            .expect("EOF offset is valid");

        assert_eq!(source.line_count(), 2);
        assert_eq!(eof.line, 2);
        assert_eq!(eof.column, 1);
        assert_eq!(source.line_text(2), Some(""));
    }
}
