use super::guard::validate_source;
use super::recovery::parse_biblatex;
use super::{Diagnostic, ParseFailure, ParseReport, ParsedLibrary, RecoveryPolicy};

pub(super) fn parse_biblatex_library(
    source: &str,
    recovery: RecoveryPolicy,
) -> Result<ParsedLibrary, ParseFailure> {
    validate_source(source)?;
    let mut parsed = parse_biblatex(source, recovery)?;
    parsed
        .diagnostics
        .sort_by_key(|diagnostic| diagnostic.span.as_ref().map(|span| span.start));
    if !source.trim().is_empty() && parsed.records.is_empty() && !parsed.diagnostics.is_empty() {
        return Err(ParseFailure {
            diagnostics: parsed.diagnostics,
        });
    }
    Ok(parsed)
}

pub(super) fn parse_hayagriva_yaml(source: &str) -> Result<ParsedLibrary, ParseFailure> {
    validate_source(source)?;
    hayagriva::io::from_yaml_str(source)
        .and_then(|inner| {
            let original: serde_yaml::Value = serde_yaml::from_str(source)?;
            let records = inner
                .iter()
                .map(|entry| {
                    let mut record = crate::EntryRecord::from_engine(entry);
                    if let Some(value) = original.get(entry.key()) {
                        record.capture_yaml_extensions(value);
                    }
                    record
                })
                .collect();
            Ok(ParsedLibrary {
                records,
                diagnostics: Vec::new(),
            })
        })
        .map_err(|error| {
            let span = error
                .location()
                .map(|location| location.index()..location.index());
            Diagnostic::error(
                "yaml_parse_error",
                span,
                format!("yaml parse error: {error}"),
            )
            .into()
        })
}

pub fn parse_bibtex_report(source: &str, recovery: RecoveryPolicy) -> ParseReport {
    match parse_biblatex_library(source, recovery) {
        Ok(parsed) => ParseReport {
            ok: true,
            entry_count: Some(parsed.records.len()),
            keys: Some(
                parsed
                    .records
                    .iter()
                    .map(|record| record.key.clone())
                    .collect(),
            ),
            diagnostics: parsed.diagnostics,
        },
        Err(mut error) => {
            error
                .diagnostics
                .sort_by_key(|diagnostic| diagnostic.span.as_ref().map(|span| span.start));
            ParseReport {
                ok: false,
                entry_count: None,
                keys: None,
                diagnostics: error.diagnostics,
            }
        }
    }
}

pub(crate) fn parse_error(error: &biblatex::ParseError) -> Diagnostic {
    let code = match error.kind {
        biblatex::ParseErrorKind::UnknownAbbreviation(_) => "unknown_abbreviation",
        biblatex::ParseErrorKind::DuplicateKey(_) => "duplicate_key",
        _ => "syntax_error",
    };
    Diagnostic::error(
        code,
        Some(error.span.clone()),
        format!("biblatex parse error: {error}"),
    )
}
