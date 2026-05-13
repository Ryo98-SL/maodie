//! Shared helpers for the `core.log` formatting surface.

/// Parsed `core.log` format string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogFormat {
    /// Literal text segments surrounding `{}` placeholders.
    pub segments: Vec<String>,
}

impl LogFormat {
    /// Number of `{}` placeholders in the format string.
    #[must_use]
    pub fn placeholder_count(&self) -> usize {
        self.segments.len().saturating_sub(1)
    }
}

/// Extracts the raw Maodie string literal value from resolver literal text.
#[must_use]
pub fn string_literal_value(text: &str) -> Option<String> {
    let raw = text.strip_prefix("string(")?.strip_suffix(')')?;
    Some(raw.trim_matches('"').to_owned())
}

/// Parses a `core.log` format string.
#[must_use]
pub fn parse_log_format(text: &str) -> Option<LogFormat> {
    let value = string_literal_value(text)?;
    let mut segments = Vec::new();
    let mut rest = value.as_str();

    loop {
        if let Some(index) = rest.find("{}") {
            segments.push(rest[..index].to_owned());
            rest = &rest[index + 2..];
        } else {
            segments.push(rest.to_owned());
            break;
        }
    }

    Some(LogFormat { segments })
}
