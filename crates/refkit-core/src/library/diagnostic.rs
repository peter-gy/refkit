use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether a source condition prevents parsing or was recovered.
pub enum DiagnosticSeverity {
    /// A source condition rejected by the active parsing policy.
    Error,
    /// A recovered condition requiring caller review.
    Warning,
}

impl DiagnosticSeverity {
    #[must_use]
    /// Return the host-facing severity identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Action taken when encountering a source condition.
pub enum DiagnosticAction {
    /// Parsing stopped without a normalized result.
    Rejected,
    /// Recovery discarded the affected source block.
    DroppedBlock,
    /// Recovery discarded an affected field.
    DroppedField,
    /// Recovery retained source content as literal text.
    Literalized,
    /// Source bytes were decoded through the fallback encoding.
    Decoded,
}

impl DiagnosticAction {
    #[must_use]
    /// Return the host-facing action identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::DroppedBlock => "dropped_block",
            Self::DroppedField => "dropped_field",
            Self::Literalized => "literalized",
            Self::Decoded => "decoded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A parser or recovery finding attributed to original source coordinates.
pub struct Diagnostic {
    /// Stable machine-readable diagnostic code.
    pub code: &'static str,
    /// Whether the condition was rejected or recovered.
    pub severity: DiagnosticSeverity,
    /// Source treatment performed by the parser or decoder.
    pub action: DiagnosticAction,
    /// UTF-8 byte range in the original source, when available.
    pub span: Option<Range<usize>>,
    /// Affected entry key, when attributable to an entry.
    pub entry: Option<String>,
    /// Affected source field name, when attributable to a field.
    pub field: Option<String>,
    /// Human-readable account of the source condition.
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn error(code: &'static str, span: Option<Range<usize>>, message: String) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            action: DiagnosticAction::Rejected,
            span,
            entry: None,
            field: None,
            message,
        }
    }

    pub(crate) fn recovered(mut self, action: DiagnosticAction) -> Self {
        self.severity = DiagnosticSeverity::Warning;
        self.action = action;
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered diagnostics for a bibliography that could not be normalized.
pub struct ParseFailure {
    /// Findings retained up to the parsing failure.
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for ParseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            diagnostic.fmt(formatter)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseFailure {}

impl From<Diagnostic> for ParseFailure {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }
}
