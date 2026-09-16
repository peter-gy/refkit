use serde_json::{Map, Value, json};

use super::{canonical_type, identifier, record_type};
use crate::codec::{CodecError, ConversionIssue, biblatex, issue};
use crate::{Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, Text};

pub(in crate::codec) fn encode(
    records: &[EntryRecord],
    issues: &mut Vec<ConversionIssue>,
) -> Result<String, CodecError> {
    let mut source_types = biblatex::SourceTypes::default();
    let values = records
        .iter()
        .map(|record| encode_entry(record, issues, &mut source_types))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_string_pretty(&values).map_err(|error| CodecError::new(error.to_string()))
}

fn encode_entry(
    record: &EntryRecord,
    issues: &mut Vec<ConversionIssue>,
    source_types: &mut biblatex::SourceTypes,
) -> Result<Value, CodecError> {
    let parent = record
        .parents
        .iter()
        .find(|parent| !["Original", "Conference"].contains(&parent.entry_type.as_str()));
    let mut fields = encode_identity(record, parent, issues, source_types)?;
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
    source_types: &mut biblatex::SourceTypes,
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
            _ if original != biblatex::canonical_type(record)
                && source_types.matches(record, original) =>
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
