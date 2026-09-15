use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::{CodecError, ConversionIssue, issue};
use crate::{
    Contributors, Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, Publisher,
    ScalarValue, Text, Url,
};

pub(super) fn canonical_type(record: &EntryRecord) -> &'static str {
    let parent = record
        .parents
        .iter()
        .find(|parent| !["Original", "Conference"].contains(&parent.entry_type.as_str()))
        .or_else(|| {
            record
                .parents
                .iter()
                .find(|parent| parent.entry_type == "Conference")
        });
    match (
        record.entry_type.as_str(),
        parent.map(|parent| parent.entry_type.as_str()),
    ) {
        ("Article", Some("Periodical")) => "article-journal",
        ("Article", Some("Newspaper")) => "article-newspaper",
        ("Article", Some("Proceedings" | "Conference")) => "paper-conference",
        ("Article", _) => "article",
        ("Chapter" | "Anthos", _) => "chapter",
        ("Entry", _) => "entry-encyclopedia",
        ("Book" | "Anthology" | "Reference", _) => "book",
        ("Report", _) => "report",
        ("Thesis", _) => "thesis",
        ("Web", _) => "webpage",
        ("Post", Some("Blog")) => "post-weblog",
        ("Post", _) => "post",
        ("Repository", _) => "software",
        ("Patent", _) => "patent",
        ("Case", _) => "legal_case",
        ("Legislation", _) => "legislation",
        ("Manuscript", _) => "manuscript",
        ("Video" | "Scene", _) => "motion_picture",
        ("Audio", _) => "song",
        ("Artwork", _) => "graphic",
        ("Performance", _) => "speech",
        _ => "document",
    }
}

pub(super) fn record_type(kind: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match kind {
        "article-journal" | "article-magazine" => ("Article", Some("Periodical")),
        "article-newspaper" => ("Article", Some("Newspaper")),
        "paper-conference" => ("Article", Some("Proceedings")),
        "article" => ("Article", None),
        "chapter" => ("Chapter", Some("Book")),
        "entry-dictionary" | "entry-encyclopedia" => ("Entry", Some("Reference")),
        "book" => ("Book", None),
        "report" => ("Report", None),
        "thesis" => ("Thesis", None),
        "webpage" => ("Web", None),
        "post" => ("Post", None),
        "post-weblog" => ("Post", Some("Blog")),
        "software" | "dataset" => ("Repository", None),
        "patent" => ("Patent", None),
        "legal_case" => ("Case", None),
        "legislation" => ("Legislation", None),
        "manuscript" => ("Manuscript", None),
        "motion_picture" => ("Video", None),
        "song" => ("Audio", None),
        "graphic" => ("Artwork", None),
        "speech" => ("Performance", None),
        "document" | "pamphlet" | "personal_communication" => ("Misc", None),
        _ => return None,
    })
}

pub(super) fn decode(
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

fn scalar(value: &Value, field: &str) -> Result<String, CodecError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(CodecError::new(format!(
            "CSL {field} must be a string or number"
        ))),
    }
}
fn identifier(value: &Value) -> Result<String, CodecError> {
    if let Value::Number(number) = value {
        if let Some(value) = number.as_i64() {
            return Ok(value.to_string());
        }
        if let Some(value) = number.as_u64() {
            return Ok(value.to_string());
        }
        if let Some(value) = number.as_f64() {
            return Ok(value.to_string());
        }
    }
    scalar(value, "id")
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

pub(super) fn encode(
    records: &[EntryRecord],
    issues: &mut Vec<ConversionIssue>,
) -> Result<String, CodecError> {
    let values = records
        .iter()
        .map(|record| encode_entry(record, issues))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_string_pretty(&values).map_err(|error| CodecError::new(error.to_string()))
}

fn encode_entry(
    record: &EntryRecord,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Value, CodecError> {
    let parent = record
        .parents
        .iter()
        .find(|parent| !["Original", "Conference"].contains(&parent.entry_type.as_str()));
    let mut fields = encode_identity(record, parent, issues)?;
    put_text(&mut fields, "title", record.title.as_ref());
    if let Some(short) = record.title.as_ref().and_then(|title| title.short.as_ref()) {
        fields.insert(
            "title-short".into(),
            Text {
                chunks: short.clone(),
                short: None,
            }
            .plain_text()
            .into(),
        );
    }
    encode_creators(&mut fields, record, parent);
    encode_dates(&mut fields, record, issues);
    encode_publication(&mut fields, record, parent);
    encode_numbers(&mut fields, record, parent);
    encode_container(&mut fields, parent);
    if !record.keywords.is_empty() {
        fields.insert("keyword".into(), record.keywords.join(", ").into());
    }
    encode_extensions(&mut fields, record, issues)?;
    Ok(Value::Object(fields))
}

fn encode_identity(
    record: &EntryRecord,
    parent: Option<&EntryRecord>,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Map<String, Value>, CodecError> {
    let mut fields = Map::new();
    let mut kind = canonical_type(record).to_string();
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("csl-json")
        .and_then(|fields| fields.get("@type"))
    {
        if record_type(original).is_some_and(|(kind, expected_parent)| {
            kind == record.entry_type
                && expected_parent == parent.map(|parent| parent.entry_type.as_str())
        }) {
            kind.clone_from(original);
        } else if record_type(original).is_none() && record.entry_type == "Misc" {
            kind.clone_from(original);
            issue(
                issues,
                "type_uninterpreted",
                "encode",
                &record.key,
                "entry_type",
                false,
                "source CSL type is retained but has no specialized normalized model",
            );
        }
    } else if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("biblatex")
        .and_then(|fields| fields.get("@type"))
    {
        match (original.as_str(), record.entry_type.as_str()) {
            ("dataset", "Repository") => kind = "dataset".into(),
            ("software", "Misc") => kind = "software".into(),
            ("booklet", "Misc") => kind = "pamphlet".into(),
            _ if original != super::biblatex::canonical_type(record)
                && super::biblatex::source_type_matches(record, original) =>
            {
                issue(
                    issues,
                    "source_type_omitted",
                    "encode",
                    &record.key,
                    "entry_type",
                    true,
                    "source BibLaTeX type specialization is not retained by the CSL-JSON output",
                );
            }
            _ => {}
        }
    }
    fields.insert("id".into(), record.key.clone().into());
    if let Some(original) = record
        .extensions
        .get("csl-json")
        .and_then(|fields| fields.get("@id"))
    {
        let value =
            serde_json::to_value(original).map_err(|error| CodecError::new(error.to_string()))?;
        if identifier(&value)? == record.key {
            fields.insert("id".into(), value);
        }
    }
    fields.insert("type".into(), kind.into());
    Ok(fields)
}

fn encode_creators(
    fields: &mut Map<String, Value>,
    record: &EntryRecord,
    parent: Option<&EntryRecord>,
) {
    put_names(fields, "author", &record.authors);
    put_names(
        fields,
        "editor",
        if record.editors.is_empty() {
            parent
                .map(|parent| parent.editors.as_slice())
                .unwrap_or_default()
        } else {
            &record.editors
        },
    );
    for group in &record.affiliated {
        if [
            "translator",
            "composer",
            "director",
            "illustrator",
            "narrator",
            "producer",
            "executive-producer",
        ]
        .contains(&group.role.as_str())
        {
            put_names(fields, &group.role, &group.names);
        }
    }
}

fn encode_dates(
    fields: &mut Map<String, Value>,
    record: &EntryRecord,
    issues: &mut Vec<ConversionIssue>,
) {
    for (field, date) in [
        ("issued", record.date.as_ref()),
        (
            "event-date",
            record.event_date.as_ref().or_else(|| {
                record
                    .parents
                    .iter()
                    .find(|parent| parent.entry_type == "Conference")
                    .and_then(|parent| parent.date.as_ref())
            }),
        ),
        (
            "original-date",
            record.original_date.as_ref().or_else(|| {
                record
                    .parents
                    .iter()
                    .find(|parent| parent.entry_type == "Original")
                    .and_then(|parent| parent.date.as_ref())
            }),
        ),
    ] {
        if let Some(date) = date {
            fields.insert(field.into(), encode_date(date, record, field, issues));
        }
    }
    if let Some(url) = &record.url {
        fields.insert("URL".into(), url.value.clone().into());
        if let Some(date) = &url.accessed {
            fields.insert(
                "accessed".into(),
                encode_date(date, record, "url.accessed", issues),
            );
        }
    }
}

fn encode_publication(
    fields: &mut Map<String, Value>,
    record: &EntryRecord,
    parent: Option<&EntryRecord>,
) {
    for (scheme, field) in [
        ("doi", "DOI"),
        ("isbn", "ISBN"),
        ("issn", "ISSN"),
        ("pmid", "PMID"),
        ("pmcid", "PMCID"),
    ] {
        if let Some(value) = record.identifiers.get(scheme) {
            fields.insert(field.into(), value.clone().into());
        }
    }
    if let Some(publisher) = record
        .publisher
        .as_ref()
        .or_else(|| parent.and_then(|parent| parent.publisher.as_ref()))
    {
        put_text(fields, "publisher", publisher.name.as_ref());
        put_text(fields, "publisher-place", publisher.location.as_ref());
    }
    for (field, text) in [
        ("archive", record.archive.as_ref()),
        ("archive_location", record.archive_location.as_ref()),
        ("call-number", record.call_number.as_ref()),
        ("abstract", record.abstract_text.as_ref()),
        ("note", record.note.as_ref()),
        ("genre", record.genre.as_ref()),
    ] {
        put_text(fields, field, text);
    }
    if let Some(language) = &record.language {
        fields.insert("language".into(), language.clone().into());
    }
}

fn encode_numbers(
    fields: &mut Map<String, Value>,
    record: &EntryRecord,
    parent: Option<&EntryRecord>,
) {
    for (field, scalar) in [
        ("edition", record.edition.as_ref()),
        ("chapter-number", record.chapter.as_ref()),
        ("page", record.page_range.as_ref()),
        ("number-of-pages", record.page_total.as_ref()),
        ("number-of-volumes", record.volume_total.as_ref()),
    ] {
        if let Some(value) = scalar {
            fields.insert(field.into(), value.value().into());
        }
    }
    for (field, value) in [
        (
            "volume",
            record
                .volume
                .as_ref()
                .or_else(|| parent.and_then(|parent| parent.volume.as_ref())),
        ),
        (
            "issue",
            record
                .issue
                .as_ref()
                .or_else(|| parent.and_then(|parent| parent.issue.as_ref())),
        ),
    ] {
        if let Some(value) = value {
            fields.insert(field.into(), value.value().into());
        }
    }
}

fn encode_container(fields: &mut Map<String, Value>, parent: Option<&EntryRecord>) {
    if let Some(parent) = parent {
        put_text(fields, "container-title", parent.title.as_ref());
        put_names(fields, "container-author", &parent.authors);
        if let Some(short) = parent.title.as_ref().and_then(|title| title.short.as_ref()) {
            fields.insert(
                "container-title-short".into(),
                Text {
                    chunks: short.clone(),
                    short: None,
                }
                .plain_text()
                .into(),
            );
        }
    }
}

fn encode_extensions(
    fields: &mut Map<String, Value>,
    record: &EntryRecord,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    if let Some(extra) = record.extensions.get("csl-json") {
        for (field, value) in extra.iter().filter(|(field, _)| !field.starts_with('@')) {
            if fields.contains_key(field) {
                issue(
                    issues,
                    "extension_conflict",
                    "encode",
                    &record.key,
                    &format!("extensions.csl-json.{field}"),
                    true,
                    "structured field takes precedence over source extension",
                );
            } else {
                fields.insert(
                    field.clone(),
                    serde_json::to_value(value)
                        .map_err(|error| CodecError::new(error.to_string()))?,
                );
            }
        }
    }
    Ok(())
}

fn put_text(fields: &mut Map<String, Value>, field: &str, text: Option<&Text>) {
    if let Some(text) = text {
        fields.insert(field.into(), text.plain_text().into());
    }
}
fn put_names(fields: &mut Map<String, Value>, field: &str, names: &[Name]) {
    if names.is_empty() {
        return;
    }
    fields.insert(
        field.into(),
        Value::Array(
            names
                .iter()
                .map(|name| match name {
                    Name::Organization { name } => json!({"literal": name}),
                    Name::Person {
                        family,
                        given,
                        prefix,
                        suffix,
                        comma_suffix,
                        non_dropping_particle,
                        ..
                    } => {
                        let mut value = Map::from_iter([("family".into(), family.clone().into())]);
                        for (field, text) in [
                            ("given", given),
                            ("dropping-particle", prefix),
                            ("suffix", suffix),
                            ("non-dropping-particle", non_dropping_particle),
                        ] {
                            if let Some(text) = text {
                                value.insert(field.into(), text.clone().into());
                            }
                        }
                        if *comma_suffix {
                            value.insert("comma-suffix".into(), true.into());
                        }
                        Value::Object(value)
                    }
                })
                .collect(),
        ),
    );
}

#[expect(
    clippy::expect_used,
    reason = "Library construction rejects date ranges with no endpoints before these retained dates reach any encoder."
)]
fn encode_date(
    date: &Date,
    record: &EntryRecord,
    field: &str,
    issues: &mut Vec<ConversionIssue>,
) -> Value {
    fn parts(date: &DateParts) -> Value {
        let mut parts = vec![json!(date.year)];
        if let Some(month) = date.month {
            parts.push(json!(month));
        }
        if let Some(day) = date.day {
            parts.push(json!(day));
        }
        Value::Array(parts)
    }
    let mut value = Map::new();
    match &date.value {
        DateValue::Point { date } => {
            value.insert("date-parts".into(), json!([parts(date)]));
            if let Some(season) = date.season {
                value.insert("season".into(), season.into());
            }
        }
        DateValue::Range {
            start: Some(start),
            end: Some(end),
        } => {
            value.insert("date-parts".into(), json!([parts(start), parts(end)]));
        }
        DateValue::Range { start, end } => {
            let endpoint = start
                .as_ref()
                .or(end.as_ref())
                .expect("validated range has endpoint");
            value.insert("date-parts".into(), json!([parts(endpoint)]));
            issue(
                issues,
                "open_range_approximated",
                "encode",
                &record.key,
                field,
                true,
                "CSL-JSON date-parts cannot represent an open range",
            );
        }
        DateValue::Literal { text } => {
            value.insert("literal".into(), text.clone().into());
        }
    }
    if date.uncertain || date.approximate {
        value.insert("circa".into(), true.into());
    }
    Value::Object(value)
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
