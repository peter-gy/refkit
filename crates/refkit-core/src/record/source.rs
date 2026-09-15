use biblatex::{Chunk, ChunksExt};
use std::fmt::Write as _;

use super::{
    BTreeMap, Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, ScalarValue, Text,
    TextChunk, TextKind, Url,
};

impl Date {
    pub(crate) fn parse_biblatex(source: &str) -> Result<Self, String> {
        crate::library::validate_date_parser_input("date", source)?;
        let date = biblatex::Date::parse(&[biblatex::Spanned::detached(biblatex::Chunk::Normal(
            source.to_string(),
        ))])
        .map_err(|error| error.to_string())?;
        Ok(Self::from_biblatex(biblatex::PermissiveType::Typed(date)))
    }

    pub(crate) fn from_biblatex(value: biblatex::PermissiveType<biblatex::Date>) -> Self {
        let date = match value {
            biblatex::PermissiveType::Typed(date) => date,
            biblatex::PermissiveType::Chunks(chunks) => {
                return Self {
                    value: DateValue::Literal {
                        text: chunks.format_verbatim(),
                    },
                    uncertain: false,
                    approximate: false,
                };
            }
        };
        Self {
            value: match date.value {
                biblatex::DateValue::At(date) => DateValue::Point {
                    date: date_parts_from_biblatex(date),
                },
                biblatex::DateValue::After(date) => DateValue::Range {
                    start: Some(date_parts_from_biblatex(date)),
                    end: None,
                },
                biblatex::DateValue::Before(date) => DateValue::Range {
                    start: None,
                    end: Some(date_parts_from_biblatex(date)),
                },
                biblatex::DateValue::Between(start, end) => DateValue::Range {
                    start: Some(date_parts_from_biblatex(start)),
                    end: Some(date_parts_from_biblatex(end)),
                },
            },
            uncertain: date.uncertain,
            approximate: date.approximate,
        }
    }
}

impl Text {
    pub(crate) fn from_biblatex(chunks: biblatex::ChunksRef<'_>) -> Self {
        Self {
            chunks: chunks
                .iter()
                .map(|chunk| match &chunk.v {
                    Chunk::Normal(text) => TextChunk {
                        kind: TextKind::Normal,
                        text: text.clone(),
                    },
                    Chunk::Verbatim(text) => TextChunk {
                        kind: TextKind::Protected,
                        text: text.clone(),
                    },
                    Chunk::Math(text) => TextChunk {
                        kind: TextKind::Math,
                        text: text.clone(),
                    },
                })
                .collect(),
            short: None,
        }
    }
}

impl Name {
    fn from_biblatex(person: &biblatex::Person) -> Self {
        fn nonempty(value: &str) -> Option<String> {
            (!value.is_empty()).then(|| value.to_string())
        }
        Self::Person {
            family: person.name.clone(),
            given: nonempty(&person.given_name),
            prefix: nonempty(&person.prefix),
            suffix: nonempty(&person.suffix),
            alias: None,
            comma_suffix: false,
            id: person.id.clone(),
            given_initials: person.given_initials.clone(),
            prefix_initials: person.prefix_initials.clone(),
            use_prefix: person.use_prefix,
            non_dropping_particle: None,
        }
    }
}

fn is_literal_name(person: &biblatex::Person, chunks: biblatex::ChunksRef<'_>) -> bool {
    person.given_name.is_empty()
        && person.prefix.is_empty()
        && person.suffix.is_empty()
        && chunks
            .iter()
            .any(|chunk| matches!(&chunk.v, Chunk::Verbatim(value) if value.trim() == person.name))
}

impl EntryRecord {
    #[expect(
        clippy::expect_used,
        reason = "The fallback serializes a bounded, parsed serde_yaml Value using its built-in serialization."
    )]
    pub(crate) fn capture_yaml_extensions(&mut self, value: &serde_yaml::Value) {
        let Some(fields) = value.as_mapping() else {
            return;
        };
        let known = [
            "type",
            "title",
            "author",
            "date",
            "editor",
            "affiliated",
            "publisher",
            "location",
            "organization",
            "issue",
            "chapter",
            "volume",
            "volume-total",
            "edition",
            "page-range",
            "page-total",
            "time-range",
            "runtime",
            "url",
            "serial-number",
            "serial",
            "language",
            "archive",
            "archive-location",
            "call-number",
            "note",
            "abstract",
            "genre",
            "parent",
        ];
        for (key, value) in fields {
            let Some(key) = key.as_str().filter(|key| !known.contains(key)) else {
                continue;
            };
            let extension =
                serde_yaml::from_value::<ExtensionValue>(value.clone()).unwrap_or_else(|_| {
                    ExtensionValue::Object(BTreeMap::from([(
                        "yaml".into(),
                        ExtensionValue::String(
                            serde_yaml::to_string(value).expect("parsed YAML serializes"),
                        ),
                    )]))
                });
            self.extensions
                .entry("hayagriva".into())
                .or_default()
                .insert(key.to_string(), extension);
        }
        let Some(parent) = value.get("parent") else {
            return;
        };
        let parents = match parent {
            serde_yaml::Value::Sequence(parents) => parents.as_slice(),
            value => std::slice::from_ref(value),
        };
        for (record, value) in self.parents.iter_mut().zip(parents) {
            record.capture_yaml_extensions(value);
        }
    }

    pub(crate) fn from_biblatex(entry: &biblatex::Entry, prepared: &hayagriva::Entry) -> Self {
        let mut record = Self::from_engine(prepared);
        restore_scalar_ranges(&mut record, entry);
        record.date = entry.date().ok().map(Date::from_biblatex).or(record.date);
        record.event_date = entry.event_date().ok().map(Date::from_biblatex);
        record.original_date = entry.orig_date().ok().map(Date::from_biblatex);
        if let Ok(value) = entry.url() {
            record.url = Some(Url {
                value,
                accessed: entry.url_date().ok().map(Date::from_biblatex),
            });
        }
        restore_names(&mut record, entry);
        if let Ok(keywords) = entry.get_as::<Vec<String>>("keywords") {
            record.keywords = keywords;
        }
        capture_biblatex_fields(&mut record, entry);
        if let Some(chunks) = entry.get("shorttitle")
            && let Some(title) = &mut record.title
        {
            title.short = Some(Text::from_biblatex(chunks).chunks);
        }
        record
    }
}

fn restore_scalar_ranges(record: &mut EntryRecord, entry: &biblatex::Entry) {
    for (field, value) in [
        ("volume", entry.volume().ok()),
        ("edition", entry.edition().ok()),
    ] {
        if let Some(biblatex::PermissiveType::Typed(value)) = value
            && (value < 0 || value > i64::from(i32::MAX))
        {
            let value = ScalarValue::Literal(value.to_string());
            if replace_scalar(record, field, &value) {
                continue;
            }
            if field == "volume" {
                record.volume = Some(value);
            } else {
                record.edition = Some(value);
            }
        }
    }
}

fn restore_names(record: &mut EntryRecord, entry: &biblatex::Entry) {
    if let Ok(authors) = entry.author() {
        let chunks = entry.get("author").unwrap_or_default();
        record.authors = authors
            .iter()
            .map(|person| {
                if is_literal_name(person, chunks) {
                    Name::Organization {
                        name: person.name.clone(),
                    }
                } else {
                    Name::from_biblatex(person)
                }
            })
            .collect();
    }
    let Ok(groups) = entry.editors() else {
        return;
    };
    for person in groups.iter().flat_map(|(people, _)| people) {
        let replacement = if ["editor", "editora", "editorb", "editorc"]
            .iter()
            .filter_map(|field| entry.get(field))
            .any(|chunks| is_literal_name(person, chunks))
        {
            Name::Organization {
                name: person.name.clone(),
            }
        } else {
            Name::from_biblatex(person)
        };

        for name in record.editors.iter_mut().chain(
            record
                .affiliated
                .iter_mut()
                .flat_map(|group| &mut group.names),
        ) {
            if let Name::Person {
                family,
                given,
                prefix,
                suffix,
                ..
            } = name
                && *family == person.name
                && given.as_deref().unwrap_or_default() == person.given_name
                && prefix.as_deref().unwrap_or_default() == person.prefix
                && suffix.as_deref().unwrap_or_default() == person.suffix
            {
                name.clone_from(&replacement);
            }
        }
    }
}

fn capture_biblatex_fields(record: &mut EntryRecord, entry: &biblatex::Entry) {
    let mapped = [
        "title",
        "shorttitle",
        "author",
        "editor",
        "date",
        "year",
        "month",
        "day",
        "endyear",
        "endmonth",
        "endday",
        "eventdate",
        "origdate",
        "urldate",
        "keywords",
        "doi",
        "isbn",
        "issn",
        "url",
        "volume",
        "edition",
        "pages",
        "abstract",
        "note",
        "language",
    ];
    let fields: BTreeMap<_, _> = entry
        .fields
        .iter()
        .filter(|(field, _)| !mapped.contains(&field.as_str()))
        .map(|(field, chunks)| {
            let text = Text::from_biblatex(chunks);
            let value = ExtensionValue::Object(BTreeMap::from([(
                "chunks".into(),
                ExtensionValue::Array(
                    text.chunks
                        .iter()
                        .map(|chunk| {
                            ExtensionValue::Object(BTreeMap::from([
                                (
                                    "kind".into(),
                                    ExtensionValue::String(
                                        match chunk.kind {
                                            TextKind::Normal => "normal",
                                            TextKind::Protected => "protected",
                                            TextKind::Math => "math",
                                        }
                                        .into(),
                                    ),
                                ),
                                ("text".into(), ExtensionValue::String(chunk.text.clone())),
                            ]))
                        })
                        .collect(),
                ),
            )]));
            (field.clone(), value)
        })
        .collect();
    record.extensions.insert("biblatex".into(), fields);
    let source_fields = record.extensions.entry("biblatex".to_string()).or_default();
    source_fields.insert(
        "@type".into(),
        ExtensionValue::String(entry.entry_type.to_string()),
    );
    for field in [
        "date",
        "eventdate",
        "origdate",
        "urldate",
        "volume",
        "edition",
        "pagetotal",
        "volumes",
    ] {
        if let Some(chunks) = entry.get(field) {
            source_fields.insert(
                format!("@{field}"),
                ExtensionValue::String(chunks.format_verbatim()),
            );
        }
    }
}

fn replace_scalar(record: &mut EntryRecord, field: &str, value: &ScalarValue) -> bool {
    let slot = if field == "volume" {
        &mut record.volume
    } else {
        &mut record.edition
    };
    if slot.is_some() {
        *slot = Some(value.clone());
        return true;
    }
    record
        .parents
        .iter_mut()
        .any(|parent| replace_scalar(parent, field, value))
}

#[expect(
    clippy::expect_used,
    reason = "Only standard integer and character formatting is written into String, whose fmt::Write implementation cannot fail."
)]
fn date_parts_from_biblatex(date: biblatex::Datetime) -> DateParts {
    DateParts {
        year: date.year,
        month: date.month.map(|month| month + 1),
        day: date.day.map(|day| day + 1),
        season: None,
        time: date.time.map(|time| {
            let mut value = format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);
            match time.offset {
                Some(biblatex::TimeOffset::Utc) => value.push('Z'),
                Some(biblatex::TimeOffset::Offset {
                    positive,
                    hours,
                    minutes,
                }) => {
                    write!(
                        value,
                        "{}{:02}:{:02}",
                        if positive { '+' } else { '-' },
                        hours,
                        minutes
                    )
                    .expect("String formatting is infallible");
                }
                None => {}
            }
            value
        }),
    }
}
