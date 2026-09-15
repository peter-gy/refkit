use serde_yaml::{Mapping, Value};

use super::{CodecError, ConversionIssue, issue};
use crate::{Date, DateValue, EntryRecord, ExtensionValue};

pub(super) fn encode(
    records: &[EntryRecord],
    issues: &mut Vec<ConversionIssue>,
) -> Result<String, CodecError> {
    let mut library = Mapping::new();
    for record in records {
        library.insert(Value::String(record.key.clone()), entry(record, issues)?);
    }
    serde_yaml::to_string(&library).map_err(CodecError::new)
}

fn entry(record: &EntryRecord, issues: &mut Vec<ConversionIssue>) -> Result<Value, CodecError> {
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("csl-json")
        .and_then(|fields| fields.get("@type"))
    {
        let compatible = super::csl::record_type(original)
            .map(|(kind, _)| kind == record.entry_type)
            .unwrap_or(record.entry_type == "Misc");
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
    let prepared = record.to_engine().map_err(CodecError::new)?;
    let mut value = serde_yaml::to_value(prepared).map_err(CodecError::new)?;
    let fields = value
        .as_mapping_mut()
        .ok_or_else(|| CodecError::new("expected a YAML entry mapping"))?;
    if let Some(date) = &record.date
        && let Some(date) = date_value(date)
    {
        fields.insert("date".into(), date);
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
    let mut parents = record.parents.clone();
    for (kind, date) in [
        ("Original", &record.original_date),
        ("Conference", &record.event_date),
    ] {
        if let Some(date) = date {
            if let Some(parent) = parents.iter_mut().find(|parent| parent.entry_type == kind) {
                parent.date = Some(date.clone());
            } else {
                parents.push(EntryRecord {
                    key: record.key.clone(),
                    entry_type: kind.into(),
                    date: Some(date.clone()),
                    ..EntryRecord::default()
                });
            }
        }
    }
    if !parents.is_empty() {
        fields.insert(
            "parent".into(),
            Value::Sequence(
                parents
                    .iter()
                    .map(|parent| entry(parent, issues))
                    .collect::<Result<_, _>>()?,
            ),
        );
    }
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
                    serde_yaml::to_value(value).map_err(CodecError::new)?,
                );
            }
        }
    }
    Ok(value)
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
