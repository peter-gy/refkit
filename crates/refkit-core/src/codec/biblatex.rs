use ::biblatex::{Chunk, Chunks, Entry, EntryType, Spanned};

use super::{CodecError, ConversionIssue, issue};
use crate::{Date, EntryRecord, ExtensionValue, Name, RecoveryPolicy, Text, TextKind};

pub(super) fn redundant_extension(
    record: &EntryRecord,
    field: &str,
    value: &ExtensionValue,
) -> bool {
    let Ok(text) = serde_json::from_value::<Text>(serde_json::to_value(value).unwrap_or_default())
    else {
        return false;
    };
    fn find_text<'a>(record: &'a EntryRecord, field: &str) -> Option<&'a Text> {
        let own = match field {
            "publisher" => record
                .publisher
                .as_ref()
                .and_then(|value| value.name.as_ref()),
            "location" | "address" => record
                .publisher
                .as_ref()
                .and_then(|value| value.location.as_ref())
                .or(record.location.as_ref()),
            "organization" | "institution" | "school" => record.organization.as_ref(),
            "journal" | "journaltitle" | "booktitle" | "maintitle" | "issuetitle" => {
                record.title.as_ref()
            }
            "type" => record.genre.as_ref(),
            "howpublished" | "annotation" | "annote" | "addendum" => record.note.as_ref(),
            _ => None,
        };
        own.or_else(|| {
            record
                .parents
                .iter()
                .find_map(|parent| find_text(parent, field))
        })
    }
    let actual_text = match field {
        "journal" | "journaltitle" | "booktitle" | "maintitle" | "issuetitle" => record
            .parents
            .iter()
            .find_map(|parent| find_text(parent, field)),
        _ => find_text(record, field),
    };
    if let Some(actual) = actual_text {
        return actual.chunks == text.chunks;
    }
    let scalar = match field {
        "langid" => record.language.as_deref().and_then(language_name),
        "volumes" => record.volume_total.as_ref().map(|value| value.value()),
        "pagetotal" => record.page_total.as_ref().map(|value| value.value()),
        "chapter" => record.chapter.as_ref().map(|value| value.value()),
        "number" | "issue" => record
            .issue
            .as_ref()
            .or_else(|| {
                record
                    .parents
                    .first()
                    .and_then(|parent| parent.issue.as_ref())
            })
            .map(|value| value.value()),
        "isan" | "ismn" | "iswc" | "version" => record.identifiers.get(field).map(String::as_str),
        _ => None,
    };
    if let Some(actual) = scalar {
        let source = text.plain_text();
        if field == "langid"
            && let (Ok(left), Ok(right)) = (
                actual.parse::<::biblatex::Language>(),
                source.parse::<::biblatex::Language>(),
            )
        {
            return left == right;
        }
        if ["volumes", "pagetotal", "chapter", "number", "issue"].contains(&field)
            && let (Ok(left), Ok(right)) = (
                actual.parse::<hayagriva::types::Numeric>(),
                source.parse::<hayagriva::types::Numeric>(),
            )
        {
            return left == right;
        }
        return actual == source;
    }
    false
}

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
        ("Article", Some("Proceedings" | "Conference")) => "inproceedings",
        ("Article", _) => "article",
        ("Chapter", _) => "inbook",
        ("Anthos", _) => "incollection",
        ("Entry", _) => "inreference",
        ("Book", _) => "book",
        ("Anthology", _) => "collection",
        ("Proceedings", _) => "proceedings",
        ("Periodical", _) => "periodical",
        ("Reference", _) => "reference",
        ("Report", _) => "report",
        ("Thesis", _) => "thesis",
        ("Web", _) => "online",
        ("Repository", _) => "dataset",
        ("Manuscript", _) => "unpublished",
        ("Patent", _) => "patent",
        _ => "misc",
    }
}

pub(super) fn source_type_matches(record: &EntryRecord, original: &str) -> bool {
    if !original
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return false;
    }
    let probe = format!("@{original}{{probe,title={{Probe}}}}");
    crate::Library::parse_biblatex(&probe, RecoveryPolicy::Error)
        .ok()
        .is_some_and(|library| {
            library.records()[0].entry_type == record.entry_type
                && library.records()[0]
                    .parents
                    .first()
                    .map(|parent| &parent.entry_type)
                    == record.parents.first().map(|parent| &parent.entry_type)
        })
}

pub(super) fn decode_issues(records: &[EntryRecord], issues: &mut Vec<ConversionIssue>) {
    for record in records {
        if let Some(fields) = record.extensions.get("biblatex") {
            for (field, path, actual) in [
                (
                    "@volume",
                    "volume",
                    record
                        .field(crate::EntryField::Volume)
                        .map(|value| value.into_owned()),
                ),
                (
                    "@edition",
                    "edition",
                    record
                        .edition
                        .as_ref()
                        .map(|value| value.value().to_string()),
                ),
                (
                    "@pagetotal",
                    "page_total",
                    record
                        .page_total
                        .as_ref()
                        .map(|value| value.value().to_string()),
                ),
                (
                    "@volumes",
                    "volume_total",
                    record
                        .volume_total
                        .as_ref()
                        .map(|value| value.value().to_string()),
                ),
            ] {
                if let Some(ExtensionValue::String(original)) = fields.get(field)
                    && let Ok(expected) = original.trim().parse::<i128>()
                {
                    if ["volume", "edition"].contains(&path)
                        && (expected < 0 || expected > i128::from(i32::MAX))
                    {
                        issue(
                            issues,
                            "numeric_literalized",
                            "decode",
                            &record.key,
                            path,
                            true,
                            "numeric value is preserved as literal text because it exceeds the engine's supported numeric representation",
                        );
                    }
                    if actual
                        .as_deref()
                        .and_then(|value| value.parse::<i128>().ok())
                        != Some(expected)
                    {
                        issue(
                            issues,
                            "numeric_approximated",
                            "decode",
                            &record.key,
                            path,
                            true,
                            "source numeric value is not retained by the normalized engine representation; the original is retained in source annotations",
                        );
                    }
                }
            }
            for field in ["@date", "@eventdate", "@origdate", "@urldate"] {
                if let Some(ExtensionValue::String(value)) = fields.get(field)
                    && value.contains(['X', 'x'])
                {
                    issue(
                        issues,
                        "date_precision_approximated",
                        "decode",
                        &record.key,
                        field.trim_start_matches('@'),
                        true,
                        "masked date precision is approximated in structured date parts; the original form is retained in source annotations",
                    );
                }
            }
        }
    }
}

pub(super) fn encode(
    records: &[EntryRecord],
    issues: &mut Vec<ConversionIssue>,
) -> Result<String, CodecError> {
    let mut bibliography = ::biblatex::Bibliography::new();
    for record in records {
        bibliography.insert(encode_entry(record, issues)?);
    }
    Ok(bibliography.to_biblatex_string())
}

fn encode_entry(
    record: &EntryRecord,
    issues: &mut Vec<ConversionIssue>,
) -> Result<Entry, CodecError> {
    if record
        .key
        .chars()
        .any(|character| character.is_whitespace() || "{}(),=%#\\\"".contains(character))
    {
        return Err(CodecError::new(format!(
            "entry key {:?} cannot be represented in BibLaTeX",
            record.key
        )));
    }
    let mut kind = canonical_type(record).to_string();
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("csl-json")
        .and_then(|fields| fields.get("@type"))
    {
        match (original.as_str(), record.entry_type.as_str()) {
            ("software", "Repository") => kind = "software".into(),
            ("dataset", "Repository") => kind = "dataset".into(),
            ("pamphlet", "Misc") => kind = "booklet".into(),
            ("article-magazine", "Article") => issue(
                issues,
                "type_approximated",
                "encode",
                &record.key,
                "entry_type",
                true,
                "BibLaTeX article does not distinguish the CSL magazine subtype",
            ),
            _ if original != super::csl::canonical_type(record)
                && super::csl::record_type(original)
                    .map(|(kind, _)| kind == record.entry_type)
                    .unwrap_or(record.entry_type == "Misc") =>
            {
                issue(
                    issues,
                    "source_type_omitted",
                    "encode",
                    &record.key,
                    "entry_type",
                    true,
                    "source CSL type specialization is not retained by the BibLaTeX output",
                )
            }
            _ => {}
        }
    }
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("biblatex")
        .and_then(|fields| fields.get("@type"))
        && original != &kind
        && source_type_matches(record, original)
    {
        kind = original.clone();
    }
    if EntryType::new(&kind).to_biblatex().to_string() != kind {
        issue(
            issues,
            "type_approximated",
            "encode",
            &record.key,
            "entry_type",
            true,
            "BibLaTeX serializer normalizes the source entry type",
        );
    }
    let mut entry = Entry::new(record.key.clone(), EntryType::new(&kind));
    if let Some(extra) = record.extensions.get("biblatex") {
        for (field, value) in extra.iter().filter(|(field, value)| {
            !field.starts_with('@') && !redundant_extension(record, field, value)
        }) {
            if ["crossref", "xref", "xdata"].contains(&field.as_str()) {
                continue;
            }
            if field.is_empty()
                || !field.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '-'
                })
            {
                issue(
                    issues,
                    "unsupported_field_name",
                    "encode",
                    &record.key,
                    &format!("extensions.biblatex.{field}"),
                    true,
                    "extension field name cannot be represented in BibLaTeX",
                );
                continue;
            }
            let text = match value {
                ExtensionValue::String(value) => Some(Text::plain(value.clone())),
                value => serde_json::from_value::<Text>(
                    serde_json::to_value(value).map_err(CodecError::new)?,
                )
                .ok(),
            };
            if let Some(text) = text {
                let normalized = field.to_ascii_lowercase();
                let normalized = match normalized.as_str() {
                    "journal" => "journaltitle",
                    "address" => "location",
                    "school" => "institution",
                    field => field,
                };
                entry.set(normalized, chunks(&text));
            } else {
                issue(
                    issues,
                    "unsupported_extension",
                    "encode",
                    &record.key,
                    &format!("extensions.biblatex.{field}"),
                    true,
                    "BibLaTeX extension requires text chunks or a string",
                );
            }
        }
    }
    put_text(&mut entry, "title", record.title.as_ref());
    if let Some(short) = record.title.as_ref().and_then(|title| title.short.as_ref()) {
        entry.set(
            "shorttitle",
            chunks(&Text {
                chunks: short.clone(),
                short: None,
            }),
        );
    }
    if !record.authors.is_empty() {
        entry.set("author", names(&record.authors));
    }
    let parent = record
        .parents
        .iter()
        .find(|parent| !["Original", "Conference"].contains(&parent.entry_type.as_str()));
    let editors = if record.editors.is_empty() {
        parent
            .map(|parent| parent.editors.as_slice())
            .unwrap_or_default()
    } else {
        &record.editors
    };
    if !editors.is_empty() {
        entry.set("editor", names(editors));
    }
    let groups = record
        .affiliated
        .iter()
        .chain(parent.into_iter().flat_map(|parent| &parent.affiliated));
    for (index, group) in groups.enumerate() {
        if index >= 3 {
            break;
        }
        let field = ["editora", "editorb", "editorc"][index];
        entry.set(field, names(&group.names));
        put_scalar(&mut entry, &format!("{field}type"), Some(&group.role));
    }
    for (field, date) in [
        ("date", record.date.as_ref()),
        (
            "eventdate",
            record.event_date.as_ref().or_else(|| {
                record
                    .parents
                    .iter()
                    .find(|parent| parent.entry_type == "Conference")
                    .and_then(|parent| parent.date.as_ref())
            }),
        ),
        (
            "origdate",
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
            put_date(&mut entry, field, date, record);
        }
    }
    if let Some(url) = &record.url {
        put_scalar(&mut entry, "url", Some(&url.value));
        if let Some(date) = &url.accessed {
            put_date(&mut entry, "urldate", date, record);
        }
    }
    for field in ["doi", "isbn", "issn", "isan", "ismn", "iswc"] {
        put_scalar(
            &mut entry,
            field,
            record.identifiers.get(field).map(String::as_str),
        );
    }
    for (scheme, name) in [("arxiv", "arxiv"), ("pmid", "pubmed"), ("pmcid", "pmcid")] {
        if let Some(value) = record.identifiers.get(scheme) {
            put_scalar(&mut entry, "eprint", Some(value));
            put_scalar(&mut entry, "eprinttype", Some(name));
            break;
        }
    }
    let publisher = record
        .publisher
        .as_ref()
        .or_else(|| parent.and_then(|parent| parent.publisher.as_ref()));
    if let Some(publisher) = publisher {
        put_text(&mut entry, "publisher", publisher.name.as_ref());
        put_text(&mut entry, "location", publisher.location.as_ref());
    }
    if entry.get("location").is_none() {
        put_text(&mut entry, "location", record.location.as_ref());
    }
    put_text(&mut entry, "organization", record.organization.as_ref());
    put_text(&mut entry, "note", record.note.as_ref());
    put_text(&mut entry, "abstract", record.abstract_text.as_ref());
    put_text(&mut entry, "type", record.genre.as_ref());
    if let Some(language) = &record.language {
        if let Some(langid) = language_name(language) {
            put_scalar(&mut entry, "langid", Some(langid));
        } else {
            issue(
                issues,
                "unsupported_language",
                "encode",
                &record.key,
                "language",
                true,
                "language has no supported BibLaTeX language name",
            );
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
        ("edition", record.edition.as_ref()),
        ("pages", record.page_range.as_ref()),
        ("pagetotal", record.page_total.as_ref()),
        ("volumes", record.volume_total.as_ref()),
        ("chapter", record.chapter.as_ref()),
        (
            "number",
            record
                .issue
                .as_ref()
                .or_else(|| parent.and_then(|parent| parent.issue.as_ref())),
        ),
    ] {
        put_scalar(&mut entry, field, value.map(|value| value.value()));
    }
    if !record.keywords.is_empty() {
        put_scalar(&mut entry, "keywords", Some(&record.keywords.join(", ")));
    }
    if let Some(parent) = parent {
        let field = if ["Periodical", "Newspaper"].contains(&parent.entry_type.as_str()) {
            "journaltitle"
        } else {
            "booktitle"
        };
        put_text(&mut entry, field, parent.title.as_ref());
        if let Some(short) = parent.title.as_ref().and_then(|title| title.short.as_ref()) {
            put_text(
                &mut entry,
                if field == "journaltitle" {
                    "shortjournal"
                } else {
                    "shorttitle"
                },
                Some(&Text {
                    chunks: short.clone(),
                    short: None,
                }),
            );
        }
        if !parent.authors.is_empty() {
            entry.set("bookauthor", names(&parent.authors));
        }
        if let Some(grandparent) = parent.parents.first() {
            put_text(&mut entry, "maintitle", grandparent.title.as_ref());
        }
    }
    Ok(entry)
}

fn language_name(language: &str) -> Option<&'static str> {
    Some(match language {
        "en" | "en-US" => "english",
        "en-GB" | "en-UK" => "british",
        "en-CA" => "canadian",
        "en-AU" => "australian",
        "en-NZ" => "newzealand",
        "de" | "de-DE" => "ngerman",
        "de-AT" => "naustrian",
        "de-CH" => "nswissgerman",
        "fr" => "french",
        "es" => "spanish",
        "it" => "italian",
        "pt" | "pt-PT" => "portuguese",
        "pt-BR" => "brazil",
        "eu" => "basque",
        "bg" => "bulgarian",
        "ca" => "catalan",
        "hr" => "croatian",
        "cs" => "czech",
        "da" => "danish",
        "nl" => "dutch",
        "et" => "estonian",
        "fi" => "finnish",
        "el" => "greek",
        "hu" => "hungarian",
        "is" => "icelandic",
        "lv" => "latvian",
        "lt" => "lithuanian",
        "mr" => "marathi",
        "nb" => "norsk",
        "nn" => "nynorsk",
        "pl" => "polish",
        "ro" => "romanian",
        "ru" => "russian",
        "sr-Latn" => "serbian",
        "sr-Cyrl" => "serbianc",
        "sk" => "slovak",
        "sl" => "slovene",
        "sv" => "swedish",
        "tr" => "turkish",
        "uk" => "ukrainian",
        _ => return None,
    })
}

fn put_date(entry: &mut Entry, field: &str, date: &Date, record: &EntryRecord) {
    let mut value = date.display();
    if let Some(ExtensionValue::String(original)) = record
        .extensions
        .get("biblatex")
        .and_then(|fields| fields.get(&format!("@{field}")))
        && ::biblatex::Date::parse(&[Spanned::detached(Chunk::Normal(original.clone()))])
            .ok()
            .map(|value| Date::from_biblatex(::biblatex::PermissiveType::Typed(value)))
            .as_ref()
            == Some(date)
    {
        value = original.clone();
    }
    put_scalar(entry, field, Some(&value));
}

fn chunks(text: &Text) -> Chunks {
    text.chunks
        .iter()
        .map(|chunk| {
            Spanned::detached(match chunk.kind {
                TextKind::Normal => Chunk::Normal(chunk.text.clone()),
                TextKind::Protected => Chunk::Verbatim(chunk.text.clone()),
                TextKind::Math => Chunk::Math(chunk.text.clone()),
            })
        })
        .collect()
}
fn put_text(entry: &mut Entry, field: &str, value: Option<&Text>) {
    if let Some(value) = value {
        entry.set(field, chunks(value));
    }
}
fn put_scalar(entry: &mut Entry, field: &str, value: Option<&str>) {
    if let Some(value) = value {
        entry.set(field, vec![Spanned::detached(Chunk::Normal(value.into()))]);
    }
}

fn names(names: &[Name]) -> Chunks {
    let mut result = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if index > 0 {
            result.push(Spanned::detached(Chunk::Normal(" and ".into())));
        }
        match name {
            Name::Organization { name } => {
                result.push(Spanned::detached(Chunk::Verbatim(name.clone())))
            }
            Name::Person {
                family,
                given,
                prefix,
                suffix,
                id,
                given_initials,
                prefix_initials,
                use_prefix,
                non_dropping_particle,
                ..
            } => {
                let family = non_dropping_particle
                    .as_ref()
                    .map(|particle| format!("{particle} {family}"))
                    .unwrap_or_else(|| family.clone());
                let use_prefix = use_prefix.map(|value| value.to_string());
                let fields = [
                    ("family", Some(family.as_str())),
                    ("given", given.as_deref()),
                    ("prefix", prefix.as_deref()),
                    ("suffix", suffix.as_deref()),
                    ("id", id.as_deref()),
                    ("given-i", given_initials.as_deref()),
                    ("prefix-i", prefix_initials.as_deref()),
                    ("useprefix", use_prefix.as_deref()),
                ];
                let mut first = true;
                for (field, value) in fields {
                    if let Some(value) = value {
                        result.push(Spanned::detached(Chunk::Normal(format!(
                            "{}{field}=",
                            if first { "" } else { ", " }
                        ))));
                        result.push(Spanned::detached(Chunk::Verbatim(value.into())));
                        first = false;
                    }
                }
            }
        }
    }
    result
}
