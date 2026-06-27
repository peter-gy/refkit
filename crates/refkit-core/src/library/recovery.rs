use std::collections::HashSet;
use std::ops::Range;

use biblatex::{
    Bibliography as BiblatexBibliography, ChunksExt, Entry as BiblatexEntry,
    TypeError as BiblatexTypeError,
};
use hayagriva::{Entry as HayEntry, Library as HayLibrary};

use crate::quoted;
use crate::raw::{
    remove_block_containing_span, sanitize_biblatex_for_library,
    sanitize_biblatex_for_library_literals,
};

use super::ParsedLibrary;

pub(super) fn recover_biblatex_library(
    source: &str,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    let (sanitized, mut recovery_diagnostics) =
        sanitize_biblatex_for_library(source, false, diagnostics);
    let mut bibliography =
        recover_biblatex_syntax(&sanitized, &mut recovery_diagnostics, diagnostics)
            .map_err(|err| format_recovery_failure(&err))?;
    sanitize_biblatex_typed_fields(&mut bibliography, &mut recovery_diagnostics, diagnostics);
    let inner =
        convert_biblatex_with_recovery(&bibliography, &mut recovery_diagnostics, diagnostics)
            .map_err(|err| format_recovery_failure(&err))?;

    Ok(ParsedLibrary {
        inner,
        diagnostics: if diagnostics {
            recovery_diagnostics
        } else {
            Vec::new()
        },
    })
}

fn format_recovery_failure(recovery_error: &str) -> String {
    format!("non-strict recovery failed:\n{recovery_error}")
}

fn sanitize_biblatex_typed_fields(
    bibliography: &mut BiblatexBibliography,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) {
    for entry in bibliography.iter_mut() {
        remove_field_if(entry, diagnostics, collect_diagnostics, "month", |value| {
            is_valid_month_field(value)
        });
        remove_field_if(entry, diagnostics, collect_diagnostics, "year", |value| {
            is_valid_year_field(value)
        });
        remove_field_if(entry, diagnostics, collect_diagnostics, "day", |value| {
            is_valid_day_field(value)
        });
        for field in ["endyear", "endmonth", "endday"] {
            remove_field_if(entry, diagnostics, collect_diagnostics, field, |value| {
                field.ends_with("year") && is_valid_year_field(value)
                    || field.ends_with("month") && is_valid_month_field(value)
                    || field.ends_with("day") && is_valid_day_field(value)
            });
        }
    }
}

fn remove_field_if(
    entry: &mut BiblatexEntry,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
    field: &str,
    is_valid: impl FnOnce(&str) -> bool,
) {
    let Some(value) = entry
        .fields
        .get(field)
        .map(|chunks| chunks.format_verbatim())
    else {
        return;
    };
    if is_valid(value.trim()) {
        return;
    }
    entry.fields.remove(field);
    if collect_diagnostics {
        diagnostics.push(format!(
            "ignored BibTeX field {} in entry {} because value {} is not valid for normalization",
            quoted(field),
            quoted(&entry.key),
            quoted(value.trim())
        ));
    }
}

fn recover_biblatex_syntax(
    source: &str,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<BiblatexBibliography, String> {
    const MAX_SYNTAX_RECOVERY_PASSES: usize = 16;
    let mut candidate = source.to_string();
    match BiblatexBibliography::parse(&candidate) {
        Ok(bibliography) => return Ok(bibliography),
        Err(first_err) => {
            let (validated, validation_diagnostics) =
                sanitize_biblatex_for_library(&candidate, true, collect_diagnostics);
            if validated != candidate {
                diagnostics.extend(validation_diagnostics);
                candidate = validated;
                if let Ok(bibliography) = BiblatexBibliography::parse(&candidate) {
                    return Ok(bibliography);
                }
            } else if collect_diagnostics {
                diagnostics.push(format!(
                    "syntax recovery could not pre-filter BibTeX entries after parse error: {first_err}"
                ));
            }

            let (literal, literal_diagnostics) =
                sanitize_biblatex_for_library_literals(&candidate, collect_diagnostics);
            if literal != candidate {
                diagnostics.extend(literal_diagnostics);
                candidate = literal;
                if let Ok(bibliography) = BiblatexBibliography::parse(&candidate) {
                    return Ok(bibliography);
                }
            }
        }
    }

    for _ in 0..MAX_SYNTAX_RECOVERY_PASSES {
        match BiblatexBibliography::parse(&candidate) {
            Ok(bibliography) => return Ok(bibliography),
            Err(err) => {
                let Some((next, diagnostic)) =
                    remove_block_containing_span(&candidate, err.span.clone())
                else {
                    return Err(format!("biblatex parse error: {err}"));
                };
                if collect_diagnostics {
                    diagnostics.push(format!(
                        "ignored BibTeX block during syntax recovery because {err}: {diagnostic}"
                    ));
                }
                if next.len() >= candidate.len() {
                    return Err(format!("biblatex parse error did not make progress: {err}"));
                }
                candidate = next;
            }
        }
    }

    Err(format!(
        "biblatex syntax recovery exceeded {MAX_SYNTAX_RECOVERY_PASSES} passes"
    ))
}

fn convert_biblatex_with_recovery(
    bibliography: &BiblatexBibliography,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<HayLibrary, String> {
    if let Ok(inner) = hayagriva::io::from_biblatex(bibliography) {
        return Ok(inner);
    }

    let mut converted = Vec::with_capacity(bibliography.len());
    for entry in bibliography.iter() {
        if let Some(entry) =
            convert_biblatex_entry_with_recovery(entry, diagnostics, collect_diagnostics)?
        {
            converted.push(entry);
        }
    }
    Ok(converted.into_iter().collect())
}

fn convert_biblatex_entry_with_recovery(
    source_entry: &BiblatexEntry,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<Option<HayEntry>, String> {
    const MAX_FIELD_RECOVERY_PASSES: usize = 64;
    let mut entry = source_entry.clone();
    let mut removed_fields = HashSet::new();

    for _ in 0..MAX_FIELD_RECOVERY_PASSES {
        match HayEntry::try_from(&entry) {
            Ok(entry) => return Ok(Some(entry)),
            Err(err) => {
                let Some(field) = field_for_type_error(&entry, &err, &removed_fields) else {
                    if collect_diagnostics {
                        diagnostics.push(format!(
                            "ignored BibTeX entry {} because type recovery failed: {err}",
                            quoted(&entry.key)
                        ));
                    }
                    return Ok(None);
                };
                if collect_diagnostics {
                    diagnostics.push(format!(
                        "ignored BibTeX field {} in entry {} because type conversion failed: {err}",
                        quoted(&field),
                        quoted(&entry.key)
                    ));
                }
                entry.fields.remove(&field);
                removed_fields.insert(field);
            }
        }
    }

    Err(format!(
        "biblatex type recovery exceeded {MAX_FIELD_RECOVERY_PASSES} passes for entry {}",
        quoted(&source_entry.key)
    ))
}

fn field_for_type_error(
    entry: &BiblatexEntry,
    err: &BiblatexTypeError,
    removed_fields: &HashSet<String>,
) -> Option<String> {
    field_containing_span(entry, err.span.clone(), removed_fields).or_else(|| {
        TYPED_RECOVERY_FIELDS
            .iter()
            .find(|field| entry.fields.contains_key(**field) && !removed_fields.contains(**field))
            .map(|field| (*field).to_string())
    })
}

fn field_containing_span(
    entry: &BiblatexEntry,
    span: Range<usize>,
    removed_fields: &HashSet<String>,
) -> Option<String> {
    entry
        .fields
        .iter()
        .filter(|(field, _)| !removed_fields.contains(*field))
        .find_map(|(field, chunks)| {
            chunks
                .iter()
                .any(|chunk| range_contains(&chunk.span, &span))
                .then(|| field.clone())
        })
}

fn range_contains(container: &Range<usize>, inner: &Range<usize>) -> bool {
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

const TYPED_RECOVERY_FIELDS: &[&str] = &[
    "date",
    "year",
    "month",
    "day",
    "endyear",
    "endmonth",
    "endday",
    "origdate",
    "urldate",
    "eventdate",
    "edition",
    "volume",
    "volumes",
    "number",
    "issue",
    "pages",
    "pagetotal",
    "pagination",
    "language",
    "langid",
    "gender",
    "editortype",
    "editoratype",
    "editorbtype",
    "editorctype",
];
