use serde_yaml::{Mapping, Value};

use super::{CodecError, ConversionIssue, issue};
use crate::{Date, DateValue, EntryRecord, ExtensionValue, Name};

pub(super) fn encode(
    records: &[EntryRecord],
    issues: &mut Vec<ConversionIssue>,
) -> Result<String, CodecError> {
    let mut library = Mapping::new();
    for record in records {
        library.insert(Value::String(record.key.clone()), entry(record, issues)?);
    }
    serde_yaml::to_string(&library).map_err(|error| CodecError::new(error.to_string()))
}

fn entry(record: &EntryRecord, issues: &mut Vec<ConversionIssue>) -> Result<Value, CodecError> {
    let prepared = record
        .to_engine()
        .map_err(|error| CodecError::new(error.to_string()))?;
    let mut value =
        serde_yaml::to_value(prepared).map_err(|error| CodecError::new(error.to_string()))?;
    decorate(record, &mut value, None, issues)?;
    Ok(value)
}

fn decorate(
    record: &EntryRecord,
    value: &mut Value,
    date_override: Option<&Date>,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    report_type_projection(record, issues);
    let fields = value
        .as_mapping_mut()
        .ok_or_else(|| CodecError::new("expected a YAML entry mapping"))?;
    put_creators(record, fields)?;
    if let Some(date) = date_override.or(record.date.as_ref()) {
        put_date(fields, date)?;
    }
    if let Some(url) = &record.url
        && let Some(accessed) = url.accessed.as_ref().and_then(date_value)
        && fields.contains_key(Value::String("url".into()))
    {
        fields.insert(
            "url".into(),
            Value::Mapping(Mapping::from_iter([
                ("value".into(), url.value.clone().into()),
                ("date".into(), accessed),
            ])),
        );
    }
    decorate_parents(record, fields, issues)?;
    put_extensions(record, fields, issues)?;
    Ok(())
}

fn put_creators(record: &EntryRecord, fields: &mut Mapping) -> Result<(), CodecError> {
    for (field, names) in [("author", &record.authors), ("editor", &record.editors)] {
        if !names.is_empty() {
            fields.insert(field.into(), names_value(names));
        }
    }
    if record.affiliated.is_empty() {
        return Ok(());
    }
    let groups = match fields.get_mut(Value::String("affiliated".into())) {
        Some(Value::Sequence(groups)) => groups.as_mut_slice(),
        Some(group @ Value::Mapping(_)) => std::slice::from_mut(group),
        _ => return Err(CodecError::new("expected YAML affiliated mappings")),
    };
    if groups.len() != record.affiliated.len() {
        return Err(CodecError::new(
            "prepared YAML contributor groups do not match records",
        ));
    }
    for (group, contributors) in groups.iter_mut().zip(&record.affiliated) {
        let group = group
            .as_mapping_mut()
            .ok_or_else(|| CodecError::new("expected a YAML affiliated mapping"))?;
        group.insert("names".into(), names_value(&contributors.names));
    }
    Ok(())
}

fn names_value(names: &[Name]) -> Value {
    Value::Sequence(names.iter().map(name_value).collect())
}

fn name_value(name: &Name) -> Value {
    let person = name.to_engine();
    let mut fields = Mapping::from_iter([
        ("name".into(), person.name.into()),
        ("comma-suffix".into(), person.comma_suffix.into()),
    ]);
    for (field, value) in [
        ("given-name", person.given_name),
        ("prefix", person.prefix),
        ("suffix", person.suffix),
        ("alias", person.alias),
    ] {
        if let Some(value) = value {
            fields.insert(field.into(), value.into());
        }
    }
    Value::Mapping(fields)
}

fn decorate_parents(
    record: &EntryRecord,
    fields: &mut Mapping,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    let prepared = match fields.remove(Value::String("parent".into())) {
        Some(Value::Sequence(parents)) => parents,
        Some(parent @ Value::Mapping(_)) => vec![parent],
        None => Vec::new(),
        Some(_) => return Err(CodecError::new("expected YAML parent mappings")),
    };
    let mut prepared = prepared.into_iter();
    let mut parents = Vec::with_capacity(record.parents.len() + 2);
    let mut original = record.original_date.as_ref();
    let mut event = record.event_date.as_ref();
    for parent in &record.parents {
        let mut value = prepared
            .next()
            .ok_or_else(|| CodecError::new("prepared YAML parent is missing"))?;
        let date_override = match parent.entry_type.as_str() {
            "Original" => original.take(),
            "Conference" => event.take(),
            _ => None,
        };
        decorate(parent, &mut value, date_override, issues)?;
        parents.push(value);
    }
    for (kind, date) in [("Original", original), ("Conference", event)] {
        if let Some(date) = date {
            let mut parent = Mapping::new();
            parent.insert("type".into(), kind.into());
            put_date(&mut parent, date)?;
            parents.push(Value::Mapping(parent));
        }
    }
    if !parents.is_empty() {
        fields.insert("parent".into(), Value::Sequence(parents));
    }
    Ok(())
}

fn put_date(fields: &mut Mapping, date: &Date) -> Result<(), CodecError> {
    let value = match date_value(date) {
        Some(value) => Some(value),
        None => date
            .to_engine("date")
            .map_err(|error| CodecError::new(error.to_string()))?
            .map(serde_yaml::to_value)
            .transpose()
            .map_err(|error| CodecError::new(error.to_string()))?,
    };
    if let Some(value) = value {
        fields.insert("date".into(), value);
    } else {
        fields.remove(Value::String("date".into()));
    }
    Ok(())
}

fn date_value(date: &Date) -> Option<Value> {
    let DateValue::Point { date: parts } = &date.value else {
        return None;
    };
    let mut fields = Mapping::new();
    fields.insert("year".into(), parts.year.into());
    if let Some(month) = parts.month {
        fields.insert("month".into(), (month - 1).into());
    }
    if let Some(day) = parts.day {
        fields.insert("day".into(), (day - 1).into());
    }
    if let Some(season) = parts.season {
        fields.insert("season".into(), season.into());
    }
    fields.insert(
        "approximate".into(),
        (date.approximate || date.uncertain).into(),
    );
    Some(Value::Mapping(fields))
}

fn report_type_projection(record: &EntryRecord, issues: &mut Vec<ConversionIssue>) {
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("csl-json")
        .and_then(|fields| fields.get("@type"))
    {
        let compatible = super::csl::record_type(original)
            .map_or(record.entry_type == "Misc", |(kind, _)| {
                kind == record.entry_type
            });
        if original != super::csl::canonical_type(record) && compatible {
            issue(
                issues,
                "type_approximated",
                "encode",
                &record.key,
                "entry_type",
                true,
                "source type specialization is not retained by the Hayagriva type vocabulary",
            );
        }
    }
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("biblatex")
        .and_then(|fields| fields.get("@type"))
        && ((record.entry_type == "Repository" && original == "dataset")
            || (record.entry_type == "Misc"
                && ["software", "booklet"].contains(&original.as_str())))
    {
        issue(
            issues,
            "type_approximated",
            "encode",
            &record.key,
            "entry_type",
            true,
            "source type specialization is not retained by the Hayagriva type vocabulary",
        );
    }
}

fn put_extensions(
    record: &EntryRecord,
    fields: &mut Mapping,
    issues: &mut Vec<ConversionIssue>,
) -> Result<(), CodecError> {
    if let Some(extra) = record.extensions.get("hayagriva") {
        for (key, value) in extra {
            if fields.contains_key(Value::String(key.clone())) {
                issue(
                    issues,
                    "extension_conflict",
                    "encode",
                    &record.key,
                    &format!("extensions.hayagriva.{key}"),
                    true,
                    "structured field takes precedence over the extension",
                );
            } else {
                fields.insert(
                    key.clone().into(),
                    serde_yaml::to_value(value)
                        .map_err(|error| CodecError::new(error.to_string()))?,
                );
            }
        }
    }
    Ok(())
}
