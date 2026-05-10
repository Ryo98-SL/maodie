use std::{
    error::Error,
    fmt::{self, Write as _},
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{SourceFile, SourceId, TextPosition, TextRange};

/// Stable Maodie diagnostic code, such as `MD0001`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Creates a diagnostic code after validating the `MD0001` naming rule.
    ///
    /// # Errors
    ///
    /// Returns [`DiagnosticCodeError`] when the value is not `MD` followed by four digits.
    pub fn new(value: impl Into<String>) -> Result<Self, DiagnosticCodeError> {
        let value = value.into();

        if is_valid_code(&value) {
            Ok(Self(value))
        } else {
            Err(DiagnosticCodeError { value })
        }
    }

    /// Returns the stable string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DiagnosticCode {
    type Error = DiagnosticCodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for DiagnosticCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Error returned for invalid Maodie diagnostic codes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticCodeError {
    value: String,
}

impl DiagnosticCodeError {
    /// Returns the rejected code value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for DiagnosticCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid diagnostic code `{}`; expected MD followed by four digits",
            self.value
        )
    }
}

impl Error for DiagnosticCodeError {}

/// Diagnostic severity used by CLI, IDE, and JSON consumers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// Compilation error.
    Error,
    /// Warning that does not stop compilation.
    Warning,
    /// Informational compiler note.
    Info,
}

impl DiagnosticSeverity {
    /// Returns the stable machine-readable severity string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    /// Returns the Chinese label used in CLI diagnostics.
    #[must_use]
    pub const fn zh_label(self) -> &'static str {
        match self {
            Self::Error => "错误",
            Self::Warning => "警告",
            Self::Info => "信息",
        }
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Fully resolved source span for diagnostics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticSpan {
    /// Source file id.
    pub source_id: SourceId,
    /// Source display name or path.
    pub file_name: String,
    /// Half-open byte range.
    pub range: TextRange,
    /// Human-readable start position.
    pub start: TextPosition,
    /// Human-readable end position.
    pub end: TextPosition,
}

impl DiagnosticSpan {
    /// Resolves a byte range against a source file.
    #[must_use]
    pub fn from_source(file: &SourceFile, range: TextRange) -> Option<Self> {
        if !file.is_valid_range(range) {
            return None;
        }

        Some(Self {
            source_id: file.id(),
            file_name: file.name().to_owned(),
            range,
            start: file.position_at(range.start)?,
            end: file.position_at(range.end)?,
        })
    }
}

/// Compiler diagnostic with stable fields for JSON consumers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Chinese-first diagnostic message.
    pub message: String,
    /// Primary source span, or `null` when the diagnostic is not tied to one source range.
    pub span: Option<DiagnosticSpan>,
    /// Additional Chinese notes.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Creates a diagnostic without a source span.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    /// Adds a resolved source span.
    #[must_use]
    pub fn with_span(mut self, span: DiagnosticSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Adds one explanatory note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Renders a CLI-friendly Chinese diagnostic.
    #[must_use]
    pub fn render_chinese(&self, source: Option<&SourceFile>) -> String {
        let mut output = format!(
            "{}[{}]: {}",
            self.severity.zh_label(),
            self.code,
            self.message
        );

        if let Some(span) = &self.span {
            write!(
                output,
                "\n  --> {}:{}:{}",
                span.file_name, span.start.line, span.start.column
            )
            .expect("writing to String cannot fail");

            if let Some(source) = source.filter(|file| file.id() == span.source_id) {
                render_source_excerpt(&mut output, source, span);
            }
        }

        for note in &self.notes {
            write!(output, "\n  = 提示: {note}").expect("writing to String cannot fail");
        }

        output
    }
}

fn render_source_excerpt(output: &mut String, source: &SourceFile, span: &DiagnosticSpan) {
    let Some(line_text) = source.line_text(span.start.line) else {
        return;
    };

    let marker_width = marker_width(source, span);
    let gutter = span.start.line.to_string();
    let marker_offset = source
        .text()
        .get(line_start_byte(source, span)..span.range.start)
        .map_or(span.start.column.saturating_sub(1), display_width);
    output.push_str("\n   |");
    write!(output, "\n {gutter} | {line_text}").expect("writing to String cannot fail");
    write!(
        output,
        "\n   | {}{}",
        " ".repeat(marker_offset),
        "^".repeat(marker_width)
    )
    .expect("writing to String cannot fail");
}

fn marker_width(source: &SourceFile, span: &DiagnosticSpan) -> usize {
    if span.start.line != span.end.line || span.range.is_empty() {
        return 1;
    }

    source
        .text()
        .get(span.range.start..span.range.end)
        .map_or(1, |text| display_width(text).max(1))
}

fn line_start_byte(source: &SourceFile, span: &DiagnosticSpan) -> usize {
    source
        .line_start_offsets()
        .get(span.start.line.saturating_sub(1))
        .copied()
        .unwrap_or(span.range.start)
}

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|character| if character.is_ascii() { 1 } else { 2 })
        .sum()
}

fn is_valid_code(value: &str) -> bool {
    value.len() == 6
        && value.starts_with("MD")
        && value
            .as_bytes()
            .get(2..)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCode, DiagnosticSeverity, DiagnosticSpan};
    use crate::{SourceFile, SourceId, TextRange};

    #[test]
    fn validates_stable_diagnostic_codes() {
        assert_eq!(
            DiagnosticCode::new("MD0001")
                .expect("code follows naming rule")
                .as_str(),
            "MD0001"
        );

        let error = DiagnosticCode::new("M0001").expect_err("missing MD prefix is invalid");
        assert_eq!(error.value(), "M0001");
        assert!(DiagnosticCode::new("MD001").is_err());
        assert!(DiagnosticCode::new("MD00A1").is_err());
    }

    #[test]
    fn serializes_diagnostic_json_with_stable_fields() {
        let source = SourceFile::new(SourceId::new(3), "examples/hello.mao", "let 名 = @\n");
        let at_offset = source.text().find('@').expect("invalid char exists");
        let span = DiagnosticSpan::from_source(&source, TextRange::new(at_offset, at_offset + 1))
            .expect("span is valid");
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new("MD0001").expect("code is valid"),
            DiagnosticSeverity::Error,
            "发现无法识别的字符",
        )
        .with_span(span)
        .with_note("请删除该字符或替换为合法 token。");

        let json = serde_json::to_value(&diagnostic).expect("diagnostic serializes");

        assert_eq!(json["code"], "MD0001");
        assert_eq!(json["severity"], "error");
        assert_eq!(json["message"], "发现无法识别的字符");
        assert_eq!(json["span"]["source_id"], 3);
        assert_eq!(json["span"]["file_name"], "examples/hello.mao");
        assert_eq!(json["span"]["range"]["start"], at_offset);
        assert_eq!(json["span"]["range"]["end"], at_offset + 1);
        assert_eq!(json["span"]["start"]["line"], 1);
        assert_eq!(json["span"]["start"]["column"], 9);
        assert_eq!(json["span"]["end"]["column"], 10);
        assert_eq!(json["notes"][0], "请删除该字符或替换为合法 token。");
    }

    #[test]
    fn rejects_invalid_diagnostic_codes_during_deserialization() {
        let error = serde_json::from_str::<DiagnosticCode>("\"MD0X01\"")
            .expect_err("invalid code should not deserialize");

        assert!(error.to_string().contains("invalid diagnostic code"));
    }

    #[test]
    fn renders_chinese_cli_diagnostic_with_source_excerpt() {
        let source = SourceFile::new(SourceId::new(1), "main.mao", "let 名 = @\n");
        let at_offset = source.text().find('@').expect("invalid char exists");
        let span = DiagnosticSpan::from_source(&source, TextRange::new(at_offset, at_offset + 1))
            .expect("span is valid");
        let diagnostic = Diagnostic::new(
            DiagnosticCode::new("MD0001").expect("code is valid"),
            DiagnosticSeverity::Error,
            "发现无法识别的字符",
        )
        .with_span(span);

        let rendered = diagnostic.render_chinese(Some(&source));

        assert!(rendered.contains("错误[MD0001]: 发现无法识别的字符"));
        assert!(rendered.contains("--> main.mao:1:9"));
        assert!(rendered.contains("1 | let 名 = @"));
        assert!(rendered.contains("|          ^"));
    }

    #[test]
    fn serializes_all_severity_values_as_lowercase_strings() {
        assert_eq!(
            serde_json::to_string(&DiagnosticSeverity::Error).expect("severity serializes"),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticSeverity::Warning).expect("severity serializes"),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticSeverity::Info).expect("severity serializes"),
            "\"info\""
        );
    }
}
