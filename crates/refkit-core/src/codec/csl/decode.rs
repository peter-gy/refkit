use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::{identifier, record_type, scalar};
use crate::codec::{CodecError, ConversionIssue, issue};
use crate::{
    Contributors, Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, Publisher,
    ScalarValue, Text, Url,
};

pub(in crate::codec) fn decode(
    source: &str,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Vec<EntryRecord>, CodecError> {
    let value =
        crate::record::parse_json(source).map_err(|error| CodecError::new(error.to_string()))?;
    let rows = value
        .as_array()
        .ok_or_else(|| CodecError::new("CSL-JSON must be an array of items"))?;
    if rows.len() > 100_000 {
        return Err(CodecError::new("CSL-JSON exceeds 100000 items"));
    }
    rows.iter()
        .enumerate()
        .map(|(index, value)| decode_entry(value, index, issues))
        .collect()
}

fn decode_entry(
    value: &Value,
    index: usize,
    issues: &mut Vec<ConversionIssue>,
) -> Result<EntryRecord, CodecError> {
    let (mut record, mut fields) = prepare_record(value, index, issues)?;
    decode_names(&mut record, &mut fields, issues)?;
    decode_dates(&mut record, &mut fields, value, issues)?;
    decode_publication(&mut record, &mut fields)?;
    decode_container(&mut record, &mut fields, issues)?;
    let key = &record.key;
    if let Some(value) = take_string(&mut fields, "keyword")? {
        record.keywords = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
    }
    for (field, value) in fields {
        issue(
            issues,
            "retained_extension",
            "decode",
            key,
            &format!("extensions.csl-json.{field}"),
            false,
            "CSL field is retained as source-specific data",
        );
        record
            .extensions
            .entry("csl-json".to_string())
            .or_default()
            .insert(
                field,
                serde_json::from_value(value)
                    .map_err(|error| CodecError::new(error.to_string()))?,
            );
    }
    Ok(record)
}

fn prepare_record(
    value: &Value,
    index: usize,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(EntryRecord, Map<String, Value>), CodecError> {
    let mut fields = value
        .as_object()
        .ok_or_else(|| CodecError::new(format!("CSL-JSON item {index} must be an object")))?
        .clone();
    let id = fields
        .remove("id")
        .ok_or_else(|| CodecError::new(format!("CSL-JSON item {index} requires id")))?;
    let key = identifier(&id)?;
    let source_type = take_string(&mut fields, "type")?
        .ok_or_else(|| CodecError::new(format!("CSL-JSON item {key:?} requires type")))?;
    let (kind, parent) = record_type(&source_type).unwrap_or_else(|| {
        issue(issues, "type_projected", "decode", &key, "entry_type", false, format!("CSL type {source_type:?} uses the generic Misc record type and is retained in source type metadata"));
        ("Misc", None)
    });
    let mut record = EntryRecord {
        key: key.clone(),
        entry_type: kind.into(),
        ..EntryRecord::default()
    };
    record.extensions.insert(
        "csl-json".into(),
        BTreeMap::from([
            ("@type".into(), ExtensionValue::String(source_type)),
            (
                "@id".into(),
                serde_json::from_value(id).map_err(|error| CodecError::new(error.to_string()))?,
            ),
            ("@source".into(), ExtensionValue::String(value.to_string())),
        ]),
    );
    if let Some(kind) = parent {
        record.parents.push(EntryRecord {
            key,
            entry_type: kind.into(),
            ..EntryRecord::default()
        });
    }
    Ok((record, fields))
}

fn decode_names(
    record: &mut EntryRecord,
    fields: &mut Map<String, Value>,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    let key = &record.key;
    record.title = take_text(fields, "title")?;
    if let Some(short) = take_string(fields, "title-short")? {
        if let Some(title) = &mut record.title {
            title.short = Some(Text::plain(short).chunks);
        } else {
            record.title = Some(Text {
                chunks: Vec::new(),
                short: Some(Text::plain(short).chunks),
            });
        }
    }
    record.authors = take_names(fields, "author", key, issues)?;
    record.editors = take_names(fields, "editor", key, issues)?;
    for (field, role) in [
        ("translator", "translator"),
        ("composer", "composer"),
        ("director", "director"),
        ("illustrator", "illustrator"),
        ("narrator", "narrator"),
        ("producer", "producer"),
        ("executive-producer", "executive-producer"),
    ] {
        let names = take_names(fields, field, key, issues)?;
        if !names.is_empty() {
            record.affiliated.push(Contributors {
                role: role.into(),
                names,
            });
        }
    }
    Ok(())
}

fn decode_dates(
    record: &mut EntryRecord,
    fields: &mut Map<String, Value>,
    source: &Value,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    let key = &record.key;
    record.date = take_date(fields, "issued", key, issues)?;
    record.event_date = take_date(fields, "event-date", key, issues)?;
    record.original_date = take_date(fields, "original-date", key, issues)?;
    let accessed = take_date(fields, "accessed", key, issues)?;
    if let Some(value) = take_string(fields, "URL")? {
        record.url = Some(Url { value, accessed });
    } else if accessed.is_some() {
        record
            .extensions
            .entry("csl-json".to_string())
            .or_default()
            .insert(
                "accessed".into(),
                serde_json::from_value(source["accessed"].clone())
                    .map_err(|error| CodecError::new(error.to_string()))?,
            );
        issue(
            issues,
            "retained_extension",
            "decode",
            key,
            "extensions.csl-json.accessed",
            false,
            "access date without a URL is retained as source-specific data",
        );
    }
    Ok(())
}

fn decode_publication(
    record: &mut EntryRecord,
    fields: &mut Map<String, Value>,
) -> Result<(), CodecError> {
    for (field, scheme) in [
        ("DOI", "doi"),
        ("ISBN", "isbn"),
        ("ISSN", "issn"),
        ("PMID", "pmid"),
        ("PMCID", "pmcid"),
    ] {
        if let Some(value) = fields.remove(field) {
            record
                .identifiers
                .insert(scheme.into(), scalar(&value, field)?);
        }
    }
    let publisher = take_text(fields, "publisher")?;
    let location = take_text(fields, "publisher-place")?;
    if publisher.is_some() || location.is_some() {
        record.publisher = Some(Publisher {
            name: publisher,
            location,
        });
    }
    record.archive = take_text(fields, "archive")?;
    record.archive_location = take_text(fields, "archive_location")?;
    record.call_number = take_text(fields, "call-number")?;
    record.abstract_text = take_text(fields, "abstract")?;
    record.note = take_text(fields, "note")?;
    record.genre = take_text(fields, "genre")?;
    record.language = take_string(fields, "language")?;
    record.edition = take_scalar(fields, "edition")?;
    record.chapter = take_scalar(fields, "chapter-number")?;
    record.page_range = take_scalar(fields, "page")?;
    record.page_total = take_scalar(fields, "number-of-pages")?;
    record.volume_total = take_scalar(fields, "number-of-volumes")?;
    Ok(())
}

fn decode_container(
    record: &mut EntryRecord,
    fields: &mut Map<String, Value>,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    let key = &record.key;
    let volume = take_scalar(fields, "volume")?;
    let issue_value = take_scalar(fields, "issue")?;
    let container = take_text(fields, "container-title")?;
    let container_authors = take_names(fields, "container-author", key, issues)?;
    if (container.is_some() || !container_authors.is_empty()) && record.parents.is_empty() {
        record.parents.push(EntryRecord {
            key: key.clone(),
            entry_type: "Book".into(),
            ..EntryRecord::default()
        });
    }
    if let Some(parent) = record.parents.first_mut() {
        parent.title = container;
        parent.authors = container_authors;
        parent.editors = std::mem::take(&mut record.editors);
        parent.publisher = record.publisher.take();
        if let Some(short) = take_string(fields, "container-title-short")? {
            parent.title.get_or_insert_with(Text::default).short = Some(Text::plain(short).chunks);
        }
        if parent.entry_type == "Periodical" || parent.entry_type == "Newspaper" {
            parent.volume = volume;
            parent.issue = issue_value;
        } else {
            record.volume = volume;
            record.issue = issue_value;
        }
    } else {
        record.volume = volume;
        record.issue = issue_value;
    }
    Ok(())
}

fn take_string(fields: &mut Map<String, Value>, name: &str) -> Result<Option<String>, CodecError> {
    fields
        .remove(name)
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| CodecError::at(name, format!("CSL {name} must be a string")))
        })
        .transpose()
}
fn take_text(fields: &mut Map<String, Value>, name: &str) -> Result<Option<Text>, CodecError> {
    Ok(take_string(fields, name)?.map(Text::plain))
}
fn take_scalar(
    fields: &mut Map<String, Value>,
    name: &str,
) -> Result<Option<ScalarValue>, CodecError> {
    fields
        .remove(name)
        .map(|value| {
            let value = scalar(&value, name)?;
            let typed = if name == "page" {
                value.parse::<hayagriva::types::PageRanges>().is_ok()
            } else {
                value.parse::<hayagriva::types::Numeric>().is_ok()
            };
            Ok(if typed {
                ScalarValue::Typed(value)
            } else {
                ScalarValue::Literal(value)
            })
        })
        .transpose()
}

fn take_names(
    fields: &mut Map<String, Value>,
    field: &str,
    key: &str,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Vec<Name>, CodecError> {
    let Some(value) = fields.remove(field) else {
        return Ok(Vec::new());
    };
    let names = value
        .as_array()
        .ok_or_else(|| CodecError::new(format!("CSL {field} must be an array of names")))?;
    names
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let mut name = value
                .as_object()
                .ok_or_else(|| {
                    CodecError::new(format!("CSL {field}[{index}] must be a name object"))
                })?
                .clone();
            let result = if let Some(literal) = take_string(&mut name, "literal")? {
                Name::Organization { name: literal }
            } else {
                Name::Person {
                    family: take_string(&mut name, "family")?.unwrap_or_default(),
                    given: take_string(&mut name, "given")?,
                    prefix: take_string(&mut name, "dropping-particle")?,
                    non_dropping_particle: take_string(&mut name, "non-dropping-particle")?,
                    suffix: take_string(&mut name, "suffix")?,
                    comma_suffix: name
                        .remove("comma-suffix")
                        .map(|value| flag(&value, "comma-suffix"))
                        .transpose()?
                        .unwrap_or(false),
                    alias: None,
                    id: None,
                    given_initials: None,
                    prefix_initials: None,
                    use_prefix: None,
                }
            };
            for extra in name.keys() {
                issue(
                    issues,
                    "unsupported_name_field",
                    "decode",
                    key,
                    &format!("{field}[{index}].{extra}"),
                    true,
                    "name field is not represented by the normalized name model",
                );
            }
            Ok(result)
        })
        .collect()
}

fn flag(value: &Value, field: &str) -> Result<bool, CodecError> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::Number(value) if value.as_f64() == Some(0.0) => Ok(false),
        Value::Number(value) if value.as_f64() == Some(1.0) => Ok(true),
        Value::String(value) if value == "true" || value == "1" => Ok(true),
        Value::String(value) if value == "false" || value == "0" => Ok(false),
        _ => Err(CodecError::new(format!("CSL {field} must be a boolean"))),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "The range and fractional-part checks admit only exactly representable i32 values, including JSON numbers spelled with a decimal point."
)]
fn integer(value: &Value) -> Option<i32> {
    let number = value.as_f64()?;
    (number.fract() == 0.0 && number >= f64::from(i32::MIN) && number <= f64::from(i32::MAX))
        .then_some(number as i32)
}

fn take_date(
    fields: &mut Map<String, Value>,
    field: &str,
    key: &str,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Option<Date>, CodecError> {
    let Some(value) = fields.remove(field) else {
        return Ok(None);
    };
    let mut value = value
        .as_object()
        .ok_or_else(|| CodecError::new(format!("CSL {field} must be a date object")))?
        .clone();
    let approximate = value
        .remove("circa")
        .map(|value| flag(&value, "circa"))
        .transpose()?
        .unwrap_or(false);
    let literal = take_string(&mut value, "literal")?;
    let raw = take_string(&mut value, "raw")?;
    let parts = value.remove("date-parts");
    let season = take_season(&mut value)?;
    let mut date = if let Some(parts) = parts {
        Date {
            value: date_value_from_parts(&parts)?,
            uncertain: false,
            approximate,
        }
    } else if let Some(raw) = raw {
        if let Ok(date) = Date::parse_biblatex(&raw) {
            date
        } else {
            issue(
                issues,
                "date_literalized",
                "decode",
                key,
                field,
                true,
                "raw date could not be interpreted and was retained as literal text",
            );
            Date {
                value: DateValue::Literal { text: raw },
                uncertain: false,
                approximate,
            }
        }
    } else if let Some(literal) = literal.clone() {
        Date {
            value: DateValue::Literal { text: literal },
            uncertain: false,
            approximate,
        }
    } else {
        return Err(CodecError::new(format!(
            "CSL {field} requires date-parts, raw, or literal"
        )));
    };
    date.approximate |= approximate;
    if let Some(season) = season {
        match &mut date.value {
            DateValue::Point { date }
            | DateValue::Range {
                start: Some(date), ..
            } => date.season = Some(season),
            _ => issue(
                issues,
                "season_removed",
                "decode",
                key,
                field,
                true,
                "season cannot be attached to a literal date",
            ),
        }
    }
    if literal.is_some() && !matches!(date.value, DateValue::Literal { .. }) {
        issue(
            issues,
            "date_literal_removed",
            "decode",
            key,
            field,
            true,
            "literal date label accompanies structured date parts and is not retained",
        );
    }
    for extra in value.keys() {
        issue(
            issues,
            "unsupported_date_field",
            "decode",
            key,
            &format!("{field}.{extra}"),
            true,
            "date field is not represented in the normalized date model",
        );
    }
    Ok(Some(date))
}

fn date_value_from_parts(value: &Value) -> Result<DateValue, CodecError> {
    let parts = value
        .as_array()
        .ok_or_else(|| CodecError::new("CSL date-parts must be an array"))?;
    match parts.as_slice() {
        [date] => Ok(DateValue::Point {
            date: date_parts(date)?,
        }),
        [start, end] => Ok(DateValue::Range {
            start: Some(date_parts(start)?),
            end: Some(date_parts(end)?),
        }),
        _ => Err(CodecError::new(
            "CSL date-parts requires one date or two range endpoints",
        )),
    }
}

fn date_parts(value: &Value) -> Result<DateParts, CodecError> {
    let values = value
        .as_array()
        .ok_or_else(|| CodecError::new("CSL date endpoint must be an array"))?;
    if values.is_empty() || values.len() > 3 {
        return Err(CodecError::new(
            "CSL date endpoint requires year and optional month/day",
        ));
    }
    let year = values
        .first()
        .and_then(integer)
        .ok_or_else(|| CodecError::new("CSL year must be a 32-bit integer"))?;
    let part = |index: usize| {
        values
            .get(index)
            .map(|value| {
                integer(value)
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| CodecError::new("CSL month/day must be an integer"))
            })
            .transpose()
    };
    Ok(DateParts {
        year,
        month: part(1)?,
        day: part(2)?,
        season: None,
        time: None,
    })
}

fn take_season(value: &mut Map<String, Value>) -> Result<Option<u8>, CodecError> {
    value
        .remove("season")
        .map(|value| {
            if value.is_number() {
                integer(&value)
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| CodecError::new("CSL season must be an integer"))
            } else {
                scalar(&value, "season")?
                    .parse::<u8>()
                    .map_err(|error| CodecError::new(error.to_string()))
            }
        })
        .transpose()
}
