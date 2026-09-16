use std::collections::HashMap;
use std::ops::Range;

use crate::EntryRecord;
use biblatex::{Bibliography, ChunksExt, Entry, RawBibliography};
use hayagriva::Entry as HayEntry;

use super::guard::validate_raw_report;
use super::parse::parse_error;
use super::source::mask;
use super::{Diagnostic, DiagnosticAction, ParseFailure, ParsedLibrary, RecoveryPolicy};
use crate::raw::{RawBlockInfo, RawDocument, sanitize_biblatex_for_library};

mod raw;

const MAX_RECOVERY_RESCAN_BYTES: usize = 128 * 1024 * 1024;
const DATE_PART_FIELDS: [&str; 6] = ["year", "month", "day", "endyear", "endmonth", "endday"];

type DatePartSpans<'a> = HashMap<(&'a str, &'static str), Range<usize>>;

pub(super) fn parse_biblatex(
    source: &str,
    policy: RecoveryPolicy,
) -> Result<ParsedLibrary, ParseFailure> {
    let mut source = source.to_string();
    let mut diagnostics = Vec::new();
    let mut remaining = MAX_RECOVERY_RESCAN_BYTES;
    loop {
        let Some(next_budget) = remaining.checked_sub(source.len()) else {
            diagnostics.push(Diagnostic::error(
                "resource_limit",
                None,
                "bibliography recovery rescanning exceeds 128 MiB".to_string(),
            ));
            return Err(ParseFailure { diagnostics });
        };
        remaining = next_budget;
        let mut raw = match RawBibliography::parse(&source) {
            Ok(raw) => raw,
            Err(error) => {
                recover_parse_error(&mut source, &error, policy, &mut diagnostics)?;
                continue;
            }
        };
        if policy == RecoveryPolicy::Report && raw::has_duplicate_entries(&raw) {
            let (filtered, found) = sanitize_biblatex_for_library(&source);
            if filtered != source {
                diagnostics.extend(found);
                source = filtered;
                continue;
            }
            raw::remove_duplicate_entries(&mut raw, &mut diagnostics);
        }
        let checkpoint = diagnostics.len();
        if policy == RecoveryPolicy::Report {
            raw::repair_independent_values(&mut raw, &mut diagnostics);
        }
        if let Err(diagnostic) = validate_with_recovery(&mut raw, policy, &mut diagnostics) {
            return Err(failure(&source, diagnostics, diagnostic));
        }
        let date_part_spans = direct_date_part_spans(&raw);
        match Bibliography::from_raw(raw) {
            Ok(mut bibliography) => {
                if bibliography.is_empty() && !source.trim().is_empty() {
                    collect_sanitizer_diagnostics(&source, &mut diagnostics);
                }
                let records = convert(
                    &mut bibliography,
                    &date_part_spans,
                    policy,
                    &mut diagnostics,
                )?;
                return Ok(ParsedLibrary {
                    records,
                    diagnostics,
                });
            }
            Err(error) => {
                diagnostics.truncate(checkpoint);
                recover_parse_error(&mut source, &error, policy, &mut diagnostics)?;
            }
        }
    }
}

fn direct_date_part_spans<'a>(raw: &RawBibliography<'a>) -> DatePartSpans<'a> {
    raw.entries
        .iter()
        .flat_map(|entry| {
            entry.v.fields.iter().filter_map(move |field| {
                DATE_PART_FIELDS
                    .iter()
                    .copied()
                    .find(|name| field.key.v.eq_ignore_ascii_case(name))
                    .map(|name| ((entry.v.key.v, name), field.value.span.clone()))
            })
        })
        .collect()
}

fn validate_with_recovery(
    raw: &mut RawBibliography<'_>,
    policy: RecoveryPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Diagnostic> {
    let mut remaining = super::guard::MAX_STEPS;
    loop {
        let found = validate_raw_report(raw)?;
        if policy == RecoveryPolicy::Error || found.is_empty() {
            return found.into_iter().next().map_or(Ok(()), Err);
        }
        let work = raw
            .abbreviations
            .iter()
            .map(|pair| pair.value.v.len())
            .chain(
                raw.entries
                    .iter()
                    .flat_map(|entry| &entry.v.fields)
                    .map(|field| field.value.v.len().max(1)),
            )
            .sum::<usize>();
        remaining = remaining.checked_sub(work).ok_or_else(|| {
            Diagnostic::error(
                "resource_limit",
                None,
                "bibliography recovery exceeds its cumulative traversal budget".to_string(),
            )
        })?;
        if !raw::apply_guard_repairs(raw, &found) {
            return found.into_iter().next().map_or(Ok(()), Err);
        }
        diagnostics.extend(found.into_iter().map(|diagnostic| {
            let action = if diagnostic.code == "invalid_field" {
                DiagnosticAction::DroppedField
            } else {
                DiagnosticAction::Literalized
            };
            diagnostic.recovered(action)
        }));
    }
}

fn collect_sanitizer_diagnostics(source: &str, diagnostics: &mut Vec<Diagnostic>) {
    let (_, found) = sanitize_biblatex_for_library(source);
    diagnostics.extend(found);
}

fn recover_parse_error(
    source: &mut String,
    error: &biblatex::ParseError,
    policy: RecoveryPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ParseFailure> {
    let mut diagnostic = parse_error(error);
    if policy == RecoveryPolicy::Error {
        return Err(failure(source, std::mem::take(diagnostics), diagnostic));
    }
    let (filtered, found) = sanitize_biblatex_for_library(source);
    if filtered != *source {
        diagnostics.extend(found);
        *source = filtered;
        return Ok(());
    }
    if drop_block(source, &mut diagnostic) {
        diagnostics.push(diagnostic);
        return Ok(());
    }
    Err(failure(source, std::mem::take(diagnostics), diagnostic))
}

fn failure(
    source: &str,
    mut diagnostics: Vec<Diagnostic>,
    mut diagnostic: Diagnostic,
) -> ParseFailure {
    identify_field(source, &mut diagnostic);
    diagnostics.push(diagnostic);
    ParseFailure { diagnostics }
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

fn drop_block(source: &mut String, diagnostic: &mut Diagnostic) -> bool {
    let Some(span) = diagnostic.span.clone() else {
        return false;
    };
    let document = RawDocument::parse(source);
    let block = document.blocks().into_iter().find_map(|block| {
        let (block_span, key) = match block {
            RawBlockInfo::Entry { span, key, .. } => (span, Some(key)),
            RawBlockInfo::Failed { span, .. }
            | RawBlockInfo::StringDef { span, .. }
            | RawBlockInfo::Preamble { span, .. } => (span, None),
            _ => return None,
        };
        (contains(&block_span, &span)
            || (span.start >= source.len() && block_span.end == source.len()))
        .then_some((block_span, key))
    });
    let Some((span, key)) = block else {
        return false;
    };
    diagnostic.span = Some(span.clone());
    diagnostic.entry = key;
    diagnostic.action = DiagnosticAction::DroppedBlock;
    diagnostic.severity = super::DiagnosticSeverity::Warning;
    mask(source, span);
    true
}

fn convert(
    bibliography: &mut Bibliography,
    date_part_spans: &DatePartSpans<'_>,
    policy: RecoveryPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<EntryRecord>, ParseFailure> {
    let mut entries = Vec::with_capacity(bibliography.len());
    for entry in bibliography.iter_mut() {
        if policy == RecoveryPolicy::Report {
            drop_invalid_date_parts(entry, date_part_spans, diagnostics);
        }
        if let Some(record) = convert_entry(entry, date_part_spans, policy, diagnostics) {
            entries.push(record);
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
    Ok(entries)
}

fn drop_invalid_date_parts(
    entry: &mut Entry,
    date_part_spans: &DatePartSpans<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in DATE_PART_FIELDS {
        let Some(chunks) = entry.fields.get(field) else {
            continue;
        };
        let value = chunks.format_verbatim();
        let valid = if field.ends_with("year") {
            is_valid_year_field(value.trim())
        } else if field.ends_with("month") {
            is_valid_month_field(value.trim())
        } else {
            is_valid_day_field(value.trim())
        };
        if !valid {
            let span = date_part_spans.get(&(entry.key.as_str(), field)).cloned();
            diagnostics.push(field_diagnostic(entry, field, span, "invalid_field", format!("ignored BibTeX field {field:?} in entry {:?} because value {value:?} is not valid for normalization", entry.key)).recovered(DiagnosticAction::DroppedField));
            entry.fields.remove(field);
        }
    }
}

fn convert_entry(
    entry: &mut Entry,
    date_part_spans: &DatePartSpans<'_>,
    policy: RecoveryPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<EntryRecord> {
    loop {
        let error = match HayEntry::try_from(&*entry) {
            Ok(converted) => {
                return Some(EntryRecord::from_biblatex(entry, &converted));
            }
            Err(error) => error,
        };
        let target = conversion_error_target(entry, &error, date_part_spans);
        let field = target.as_ref().map(|target| target.field.as_str());
        let mut diagnostic = field_diagnostic(
            entry,
            field.unwrap_or(""),
            target
                .as_ref()
                .map_or_else(|| Some(error.span.clone()), |target| target.span.clone()),
            "invalid_field",
            format!("biblatex type error: {error}"),
        );
        if policy == RecoveryPolicy::Error {
            diagnostics.push(diagnostic);
            return None;
        }
        if let Some(field) = field {
            diagnostic = diagnostic.recovered(DiagnosticAction::DroppedField);
            entry.fields.remove(field);
            diagnostics.push(diagnostic);
        } else {
            diagnostics.push(diagnostic.recovered(DiagnosticAction::DroppedBlock));
            return None;
        }
    }
}

struct ConversionErrorTarget {
    field: String,
    span: Option<Range<usize>>,
}

fn conversion_error_target(
    entry: &Entry,
    error: &biblatex::TypeError,
    date_part_spans: &DatePartSpans<'_>,
) -> Option<ConversionErrorTarget> {
    if is_trailing_month_error(entry, error) {
        return Some(ConversionErrorTarget {
            field: "month".to_string(),
            span: date_part_spans.get(&(entry.key.as_str(), "month")).cloned(),
        });
    }
    entry.fields.iter().find_map(|(name, chunks)| {
        chunks
            .iter()
            .any(|chunk| contains(&chunk.span, &error.span))
            .then(|| ConversionErrorTarget {
                field: name.clone(),
                span: Some(error.span.clone()),
            })
    })
}

fn is_trailing_month_error(entry: &Entry, error: &biblatex::TypeError) -> bool {
    if error.kind != biblatex::TypeErrorKind::MissingNumber
        || entry.fields.contains_key("date")
        || entry.fields.contains_key("day")
    {
        return false;
    }
    let Some(year) = entry.fields.get("year") else {
        return false;
    };
    let Some(month) = entry.fields.get("month") else {
        return false;
    };
    if !month.format_verbatim().ends_with(char::is_whitespace) {
        return false;
    }
    // This upstream error uses a month-local offset. Exclude year failures and
    // reproduce the guarded split-date error before attributing its source field.
    biblatex::Date::parse_three_fields(year, None, None).is_ok()
        && biblatex::Date::parse_three_fields(year, Some(month), None)
            .err()
            .as_ref()
            == Some(error)
}

fn field_diagnostic(
    entry: &Entry,
    field: &str,
    span: Option<Range<usize>>,
    code: &'static str,
    message: String,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(code, span, message);
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
