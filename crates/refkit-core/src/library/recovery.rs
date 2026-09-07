use std::ops::Range;

use biblatex::{Bibliography, ChunksExt, Entry, RawBibliography};
use hayagriva::{Entry as HayEntry, Library as HayLibrary};

use super::guard::validate_raw;
use super::parse::parse_error;
use super::source::RecoverySource;
use super::{Diagnostic, DiagnosticAction, ParseFailure, ParsedLibrary, RecoveryPolicy};
use crate::raw::{RawBlockInfo, RawDocument, sanitize_biblatex_for_library};

const MAX_RECOVERY_PASSES: usize = 128;

pub(super) fn parse_biblatex(
    source: &str,
    policy: RecoveryPolicy,
) -> Result<ParsedLibrary, ParseFailure> {
    let mut source = RecoverySource::new(source.to_string());
    let mut diagnostics = Vec::new();
    for _ in 0..MAX_RECOVERY_PASSES {
        let raw = match RawBibliography::parse(&source.text) {
            Ok(raw) => raw,
            Err(error) => {
                let mut diagnostic = parse_error(&error);
                if policy == RecoveryPolicy::Report {
                    let (filtered, mut found) = sanitize_biblatex_for_library(&source.text);
                    if filtered != source.text {
                        for diagnostic in &mut found {
                            map_diagnostic(&source, diagnostic);
                        }
                        diagnostics.extend(found);
                        source.text = filtered;
                        continue;
                    }
                    if drop_block(&mut source, &mut diagnostic) {
                        diagnostics.push(diagnostic);
                        continue;
                    }
                }
                return Err(failure(&source, diagnostics, diagnostic));
            }
        };
        if let Err(mut diagnostic) = validate_raw(&raw) {
            if policy == RecoveryPolicy::Report && diagnostic.code != "resource_limit" {
                let span = diagnostic
                    .span
                    .clone()
                    .expect("dependency errors carry spans");
                let clear_reference = diagnostic.code == "cyclic_reference";
                map_diagnostic(&source, &mut diagnostic);
                diagnostics.push(diagnostic.recovered(DiagnosticAction::Literalized));
                if clear_reference {
                    source.replace(span, "{}");
                } else {
                    source.literalize(span);
                }
                continue;
            }
            return Err(failure(&source, diagnostics, diagnostic));
        }
        match Bibliography::from_raw(raw) {
            Ok(mut bibliography) => {
                if bibliography.is_empty() && !source.text.trim().is_empty() {
                    let (_, mut found) = sanitize_biblatex_for_library(&source.text);
                    for diagnostic in &mut found {
                        map_diagnostic(&source, diagnostic);
                    }
                    diagnostics.extend(found);
                }
                let inner = convert(&mut bibliography, &source, policy, &mut diagnostics)?;
                return Ok(ParsedLibrary { inner, diagnostics });
            }
            Err(error) => {
                let mut diagnostic = parse_error(&error);
                if policy == RecoveryPolicy::Report {
                    if matches!(error.kind, biblatex::ParseErrorKind::UnknownAbbreviation(_)) {
                        identify_field(&source.text, &mut diagnostic);
                        map_diagnostic(&source, &mut diagnostic);
                        diagnostics.push(diagnostic.recovered(DiagnosticAction::Literalized));
                        source.literalize(error.span);
                        continue;
                    }
                    let (filtered, mut found) = sanitize_biblatex_for_library(&source.text);
                    if filtered != source.text {
                        for diagnostic in &mut found {
                            map_diagnostic(&source, diagnostic);
                        }
                        diagnostics.extend(found);
                        source.text = filtered;
                        continue;
                    }
                    if drop_block(&mut source, &mut diagnostic) {
                        diagnostics.push(diagnostic);
                        continue;
                    }
                }
                return Err(failure(&source, diagnostics, diagnostic));
            }
        }
    }
    diagnostics.push(Diagnostic::error(
        "resource_limit",
        None,
        "BibTeX recovery exceeded 128 changes".to_string(),
    ));
    Err(ParseFailure { diagnostics })
}

fn failure(
    source: &RecoverySource,
    mut diagnostics: Vec<Diagnostic>,
    mut diagnostic: Diagnostic,
) -> ParseFailure {
    identify_field(&source.text, &mut diagnostic);
    map_diagnostic(source, &mut diagnostic);
    diagnostics.push(diagnostic);
    ParseFailure { diagnostics }
}

fn map_diagnostic(source: &RecoverySource, diagnostic: &mut Diagnostic) {
    diagnostic.span = diagnostic
        .span
        .take()
        .map(|span| source.original_span(span));
}

fn identify_field(source: &str, diagnostic: &mut Diagnostic) {
    let Some(span) = diagnostic.span.as_ref() else {
        return;
    };
    let Ok(raw) = RawBibliography::parse(source) else {
        return;
    };
    for entry in raw.entries {
        for field in entry.v.fields {
            if contains(&field.value.span, span) {
                diagnostic.entry = Some(entry.v.key.v.to_string());
                diagnostic.field = Some(field.key.v.to_ascii_lowercase());
                return;
            }
        }
    }
}

fn drop_block(source: &mut RecoverySource, diagnostic: &mut Diagnostic) -> bool {
    let Some(span) = diagnostic.span.clone() else {
        return false;
    };
    let document = RawDocument::parse(&source.text);
    let block = document.blocks().into_iter().find_map(|block| {
        let (block_span, key) = match block {
            RawBlockInfo::Entry { span, key, .. } => (span, Some(key)),
            RawBlockInfo::Failed { span, .. }
            | RawBlockInfo::StringDef { span, .. }
            | RawBlockInfo::Preamble { span, .. } => (span, None),
            _ => return None,
        };
        (contains(&block_span, &span)
            || (span.start >= source.text.len() && block_span.end == source.text.len()))
        .then_some((block_span, key))
    });
    let Some((span, key)) = block else {
        return false;
    };
    diagnostic.span = Some(source.original_span(span.clone()));
    diagnostic.entry = key;
    diagnostic.action = DiagnosticAction::DroppedBlock;
    diagnostic.severity = super::DiagnosticSeverity::Warning;
    source.replace(span, "");
    true
}

fn convert(
    bibliography: &mut Bibliography,
    source: &RecoverySource,
    policy: RecoveryPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<HayLibrary, ParseFailure> {
    let mut entries = Vec::with_capacity(bibliography.len());
    for entry in bibliography.iter_mut() {
        if policy == RecoveryPolicy::Report {
            for field in ["year", "month", "day", "endyear", "endmonth", "endday"] {
                if let Some(chunks) = entry.fields.get(field) {
                    let value = chunks.format_verbatim();
                    let valid = if field.ends_with("year") {
                        is_valid_year_field(value.trim())
                    } else if field.ends_with("month") {
                        is_valid_month_field(value.trim())
                    } else {
                        is_valid_day_field(value.trim())
                    };
                    if !valid {
                        let span = chunks
                            .first()
                            .map(|first| first.span.start)
                            .zip(chunks.last().map(|last| last.span.end))
                            .map(|(start, end)| start..end);
                        diagnostics.push(field_diagnostic(source, entry, field, span, "invalid_field", format!("ignored BibTeX field {field:?} in entry {:?} because value {value:?} is not valid for normalization", entry.key)).recovered(DiagnosticAction::DroppedField));
                        entry.fields.remove(field);
                    }
                }
            }
        }
        loop {
            match HayEntry::try_from(&*entry) {
                Ok(converted) => {
                    entries.push(converted);
                    break;
                }
                Err(error) => {
                    let field = entry
                        .fields
                        .iter()
                        .find(|(_, chunks)| {
                            chunks
                                .iter()
                                .any(|chunk| contains(&chunk.span, &error.span))
                        })
                        .map(|(name, _)| name.clone());
                    let mut diagnostic = field_diagnostic(
                        source,
                        entry,
                        field.as_deref().unwrap_or(""),
                        Some(error.span.clone()),
                        "invalid_field",
                        format!("biblatex type error: {error}"),
                    );
                    if policy == RecoveryPolicy::Error {
                        diagnostics.push(diagnostic);
                        break;
                    }
                    if let Some(field) = field {
                        diagnostic = diagnostic.recovered(DiagnosticAction::DroppedField);
                        entry.fields.remove(&field);
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(diagnostic.recovered(DiagnosticAction::DroppedBlock));
                        break;
                    }
                }
            }
        }
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == super::DiagnosticSeverity::Error)
    {
        return Err(ParseFailure {
            diagnostics: std::mem::take(diagnostics),
        });
    }
    Ok(entries.into_iter().collect())
}

fn field_diagnostic(
    source: &RecoverySource,
    entry: &Entry,
    field: &str,
    span: Option<Range<usize>>,
    code: &'static str,
    message: String,
) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::error(code, span.map(|span| source.original_span(span)), message);
    diagnostic.entry = Some(entry.key.clone());
    diagnostic.field = (!field.is_empty()).then(|| field.to_string());
    diagnostic
}

fn contains(container: &Range<usize>, inner: &Range<usize>) -> bool {
    container.start <= inner.start && inner.end <= container.end
}

fn is_valid_year_field(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    (1..=4).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_valid_month_field(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "jan"
            | "january"
            | "feb"
            | "february"
            | "mar"
            | "march"
            | "apr"
            | "april"
            | "may"
            | "jun"
            | "june"
            | "jul"
            | "july"
            | "aug"
            | "august"
            | "sep"
            | "september"
            | "oct"
            | "october"
            | "nov"
            | "november"
            | "dec"
            | "december"
    ) || normalized
        .parse::<u8>()
        .is_ok_and(|month| (1..=12).contains(&month))
}

fn is_valid_day_field(value: &str) -> bool {
    value.parse::<u8>().is_ok_and(|day| (1..=31).contains(&day))
}
