use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use biblatex::{
    Bibliography as BiblatexBibliography, ChunksExt, Entry as BiblatexEntry,
    TypeError as BiblatexTypeError,
};
use hayagriva::{Entry as HayEntry, Library as HayLibrary};
use serde_json::{Value, json};

use crate::{entry_type_name, quoted};

pub struct ParsedLibrary {
    pub inner: HayLibrary,
    pub diagnostics: Vec<String>,
}

pub struct SourceText {
    pub source: String,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRecord {
    pub key: String,
    pub entry_type: String,
    pub title: Option<String>,
    pub volume: Option<String>,
    pub doi: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProjectField {
    Key,
    EntryType,
    Type,
    Title,
    Doi,
    Volume,
}

pub fn read_bibliography_text(path: &Path) -> Result<SourceText, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(source) => Ok(SourceText {
            source,
            diagnostic: None,
        }),
        Err(err) => Ok(SourceText {
            source: decode_windows_1252(&err.into_bytes()),
            diagnostic: Some(format!(
                "decoded {} as Windows-1252-compatible text because it is not valid UTF-8",
                path.display()
            )),
        }),
    }
}

pub fn parse_library_path(
    path: PathBuf,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    let text = read_bibliography_text(&path)?;
    let mut parsed = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bib") => parse_library_source(&text.source, "bibtex", strict, diagnostics),
        Some("yaml" | "yml") => parse_library_source(&text.source, "yaml", strict, diagnostics),
        Some(ext) => Err(format!(
            "unsupported bibliography extension {}",
            quoted(ext)
        )),
        None => Err("bibliography path has no extension".to_string()),
    }?;
    if diagnostics && let Some(diagnostic) = text.diagnostic {
        parsed.diagnostics.insert(0, diagnostic);
    }
    Ok(parsed)
}

pub fn parse_library_source(
    source: &str,
    format: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    match format.to_ascii_lowercase().as_str() {
        "bib" | "bibtex" | "biblatex" => parse_biblatex_library(source, strict, diagnostics),
        "yaml" | "yml" => hayagriva::io::from_yaml_str(source)
            .map(|inner| ParsedLibrary {
                inner,
                diagnostics: Vec::new(),
            })
            .map_err(|err| format!("yaml parse error: {err}")),
        other => Err(format!("unsupported bibliography format {}", quoted(other))),
    }
}

pub fn entry_record(entry: &HayEntry) -> EntryRecord {
    EntryRecord {
        key: entry.key().to_string(),
        entry_type: entry_type_name(entry.entry_type()).to_string(),
        title: entry.title().map(ToString::to_string),
        volume: entry
            .volume()
            .or_else(|| entry.parents().first().and_then(|parent| parent.volume()))
            .map(|value| value.to_string()),
        doi: entry
            .serial_number()
            .and_then(|serial| serial.0.get("doi").cloned()),
    }
}

pub fn library_to_normalized_json(library: &HayLibrary) -> Result<String, String> {
    let entries = library
        .iter()
        .map(entry_to_normalized_json_value)
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_string(&entries).map_err(|err| err.to_string())
}

fn entry_to_normalized_json_value(entry: &HayEntry) -> Result<Value, String> {
    let mut value = serde_json::to_value(entry).map_err(|err| err.to_string())?;
    if let Some(map) = value.as_object_mut() {
        map.insert("id".to_string(), json!(entry.key()));
        map.insert("key".to_string(), json!(entry.key()));
    }
    Ok(value)
}

pub fn parse_project_field(field: &str) -> Result<ProjectField, String> {
    match field {
        "key" => Ok(ProjectField::Key),
        "entry_type" => Ok(ProjectField::EntryType),
        "type" => Ok(ProjectField::Type),
        "title" => Ok(ProjectField::Title),
        "doi" => Ok(ProjectField::Doi),
        "volume" => Ok(ProjectField::Volume),
        _ => Err(format!("unsupported projection field {}", quoted(field))),
    }
}

pub fn project_records(
    library: &HayLibrary,
    fields: &[ProjectField],
    keys: Option<&[String]>,
) -> Result<Vec<Vec<Option<String>>>, String> {
    let entries = library.iter().map(entry_record).collect::<Vec<_>>();
    match keys {
        Some(keys) => keys
            .iter()
            .map(|key| {
                let Some(record) = entries.iter().find(|entry| &entry.key == key) else {
                    return Err(format!("missing reference {}", quoted(key)));
                };
                Ok(project_record(record, fields))
            })
            .collect(),
        None => Ok(entries
            .iter()
            .map(|record| project_record(record, fields))
            .collect()),
    }
}

fn project_record(record: &EntryRecord, fields: &[ProjectField]) -> Vec<Option<String>> {
    fields
        .iter()
        .map(|field| match field {
            ProjectField::Key => Some(record.key.clone()),
            ProjectField::EntryType | ProjectField::Type => Some(record.entry_type.clone()),
            ProjectField::Title => record.title.clone(),
            ProjectField::Doi => record.doi.clone(),
            ProjectField::Volume => record.volume.clone(),
        })
        .collect()
}

fn parse_biblatex_library(
    source: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    if strict {
        return match hayagriva::io::from_biblatex_str(source) {
            Ok(inner) => Ok(ParsedLibrary {
                inner,
                diagnostics: Vec::new(),
            }),
            Err(errors) => Err(format_biblatex_errors(&errors)),
        };
    }

    let mut recovery_diagnostics = Vec::new();
    let mut bibliography = BiblatexBibliography::parse(source)
        .map_err(|err| format!("biblatex parse error: {err}"))?;
    sanitize_biblatex_typed_fields(&mut bibliography, &mut recovery_diagnostics, diagnostics);
    let inner =
        convert_biblatex_with_recovery(&bibliography, &mut recovery_diagnostics, diagnostics)?;
    Ok(ParsedLibrary {
        inner,
        diagnostics: if diagnostics {
            recovery_diagnostics
        } else {
            Vec::new()
        },
    })
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

fn format_biblatex_errors(errors: &[hayagriva::io::BibLaTeXError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
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

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x80 => '\u{20ac}',
            0x82 => '\u{201a}',
            0x83 => '\u{0192}',
            0x84 => '\u{201e}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02c6}',
            0x89 => '\u{2030}',
            0x8a => '\u{0160}',
            0x8b => '\u{2039}',
            0x8c => '\u{0152}',
            0x8e => '\u{017d}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201c}',
            0x94 => '\u{201d}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02dc}',
            0x99 => '\u{2122}',
            0x9a => '\u{0161}',
            0x9b => '\u{203a}',
            0x9c => '\u{0153}',
            0x9e => '\u{017e}',
            0x9f => '\u{0178}',
            0x81 | 0x8d | 0x8f | 0x90 | 0x9d => '\u{fffd}',
            _ => char::from(*byte),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bibtex_and_projects_scalar_records() {
        let parsed = parse_library_source(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}, doi = {10.1/test}}",
            "bibtex",
            false,
            true,
        )
        .unwrap();

        let rows = project_records(
            &parsed.inner,
            &[
                ProjectField::Key,
                ProjectField::Title,
                ProjectField::Doi,
                ProjectField::Volume,
            ],
            None,
        )
        .unwrap();

        assert_eq!(
            rows,
            vec![vec![
                Some("doe2024".to_string()),
                Some("Core".to_string()),
                Some("10.1/test".to_string()),
                None,
            ]]
        );
    }

    #[test]
    fn decodes_windows_1252_when_utf8_fails() {
        assert_eq!(decode_windows_1252(&[0x48, 0x80]), "H€");
    }
}
