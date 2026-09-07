use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticAction {
    Rejected,
    DroppedBlock,
    DroppedField,
    Literalized,
    Decoded,
}

impl DiagnosticAction {
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
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub action: DiagnosticAction,
    pub span: Option<Range<usize>>,
    pub entry: Option<String>,
    pub field: Option<String>,
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
pub struct ParseFailure {
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
