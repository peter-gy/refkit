mod parse;
mod recovery;

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use hayagriva::{Entry as HayEntry, Library as HayLibrary, Selector};

use crate::quoted;
use crate::strings::entry_type_name;

pub use self::parse::parse_bibtex_report;
use self::parse::{parse_biblatex_library, parse_hayagriva_yaml};

pub(crate) struct ParsedLibrary {
    pub(crate) inner: HayLibrary,
    pub(crate) diagnostics: Vec<String>,
}

pub struct Library {
    inner: Arc<HayLibrary>,
    diagnostics: Vec<String>,
    keys: OnceLock<Vec<String>>,
    records: OnceLock<RecordCache>,
}

struct RecordCache {
    records: Vec<EntryRecord>,
    index: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport {
    pub ok: bool,
    pub entry_count: Option<usize>,
    pub keys: Option<Vec<String>>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryError {
    Biblatex(String),
    HayagrivaYaml(String),
    Selector(String),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Biblatex(message) | Self::HayagrivaYaml(message) => f.write_str(message),
            Self::Selector(message) => write!(f, "invalid selector: {message}"),
        }
    }
}

impl std::error::Error for LibraryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPolicy {
    Error,
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRecord {
    pub key: String,
    pub entry_type: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub volume: Option<String>,
    pub doi: Option<String>,
    pub parents: Vec<EntryRecord>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EntryField {
    Key,
    Type,
    Title,
    Date,
    Doi,
    Volume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFieldError(String);

impl fmt::Display for EntryFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported projection field {}", quoted(&self.0))
    }
}

impl std::error::Error for EntryFieldError {}

impl FromStr for EntryField {
    type Err = EntryFieldError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "key" => Ok(Self::Key),
            "entry_type" | "type" => Ok(Self::Type),
            "title" => Ok(Self::Title),
            "date" => Ok(Self::Date),
            "doi" => Ok(Self::Doi),
            "volume" => Ok(Self::Volume),
            _ => Err(EntryFieldError(value.to_string())),
        }
    }
}

impl EntryRecord {
    pub fn field(&self, field: EntryField) -> Option<&str> {
        match field {
            EntryField::Key => Some(&self.key),
            EntryField::Type => Some(&self.entry_type),
            EntryField::Title => self.title.as_deref(),
            EntryField::Date => self.date.as_deref(),
            EntryField::Doi => self.doi.as_deref(),
            EntryField::Volume => self.volume.as_deref(),
        }
    }
}

impl Library {
    pub fn parse_biblatex(source: &str, recovery: RecoveryPolicy) -> Result<Self, LibraryError> {
        parse_biblatex_library(source, recovery)
            .map(Self::from_parsed)
            .map_err(LibraryError::Biblatex)
    }

    pub fn parse_hayagriva_yaml(source: &str) -> Result<Self, LibraryError> {
        parse_hayagriva_yaml(source)
            .map(Self::from_parsed)
            .map_err(LibraryError::HayagrivaYaml)
    }

    pub(crate) fn from_parsed(parsed: ParsedLibrary) -> Self {
        Self {
            inner: Arc::new(parsed.inner),
            diagnostics: parsed.diagnostics,
            keys: OnceLock::new(),
            records: OnceLock::new(),
        }
    }

    pub(crate) fn inner(&self) -> &HayLibrary {
        self.inner.as_ref()
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    pub fn keys(&self) -> &[String] {
        self.keys
            .get_or_init(|| self.inner.keys().map(str::to_string).collect())
    }

    pub fn records(&self) -> &[EntryRecord] {
        &self.record_cache().records
    }

    pub fn get_record(&self, key: &str) -> Option<&EntryRecord> {
        let cache = self.record_cache();
        cache.index.get(key).map(|index| &cache.records[*index])
    }

    pub fn select_records(&self, selector: &str) -> Result<Vec<EntryRecord>, LibraryError> {
        let selector =
            Selector::parse(selector).map_err(|err| LibraryError::Selector(err.to_string()))?;
        Ok(self
            .inner
            .iter()
            .filter(|entry| selector.matches(entry))
            .map(entry_record)
            .collect())
    }

    fn record_cache(&self) -> &RecordCache {
        self.records
            .get_or_init(|| RecordCache::from_library(&self.inner))
    }
}

impl RecordCache {
    fn from_library(library: &HayLibrary) -> Self {
        let mut records = Vec::with_capacity(library.len());
        let mut index = HashMap::with_capacity(library.len());

        for entry in library.iter() {
            let key = entry.key().to_string();
            index.insert(key, records.len());
            records.push(entry_record(entry));
        }

        Self { records, index }
    }
}

pub(crate) fn entry_record(entry: &HayEntry) -> EntryRecord {
    EntryRecord {
        key: entry.key().to_string(),
        entry_type: entry_type_name(entry.entry_type()).to_string(),
        title: entry.title().map(ToString::to_string),
        date: entry.date().map(ToString::to_string),
        volume: entry
            .volume()
            .or_else(|| entry.parents().first().and_then(|parent| parent.volume()))
            .map(|value| value.to_string()),
        doi: entry
            .serial_number()
            .and_then(|serial| serial.0.get("doi").cloned()),
        parents: entry.parents().iter().map(entry_record).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bibtex_and_projects_scalar_records() {
        let parsed = parse_biblatex_library(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}, doi = {10.1/test}}",
            RecoveryPolicy::Report,
        )
        .unwrap();

        let library = Library::from_parsed(parsed);
        let record = &library.records()[0];

        assert_eq!(
            [
                record.field(EntryField::Key),
                record.field(EntryField::Title),
                record.field(EntryField::Doi),
                record.field(EntryField::Volume),
            ],
            [Some("doe2024"), Some("Core"), Some("10.1/test"), None]
        );
    }

    #[test]
    fn non_strict_recovery_keeps_valid_entries_and_reports_malformed_blocks() {
        let parsed = parse_biblatex_library(
            concat!(
                "@article{valid,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Kept Entry},\n",
                "  year = {2024}\n",
                "}\n",
                "@broken{missing,\n",
                "  title = {No close}\n",
            ),
            RecoveryPolicy::Report,
        )
        .unwrap();

        let keys = parsed
            .inner
            .iter()
            .map(|entry| entry.key().to_string())
            .collect::<Vec<_>>();

        assert_eq!(keys, vec!["valid"]);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("ignored malformed BibTeX block"))
        );
    }

    #[test]
    fn bibtex_value_parse_treats_recovered_empty_source_as_failure() {
        let err = match parse_biblatex_library("@broken{missing", RecoveryPolicy::Report) {
            Ok(_) => panic!("expected recovered empty source to fail"),
            Err(err) => err,
        };
        let report = parse_bibtex_report("@broken{missing", RecoveryPolicy::Report);

        assert!(err.contains("malformed BibTeX block"));
        assert!(!report.ok);
        assert_eq!(report.entry_count, None);
        assert_eq!(report.keys, None);
        assert!(report.diagnostics[0].contains("malformed BibTeX block"));
    }

    #[test]
    fn non_strict_recovery_removes_invalid_typed_fields() {
        let parsed = parse_biblatex_library(
            concat!(
                "@article{badmonth,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Bad Month},\n",
                "  year = {2024},\n",
                "  month = {16}\n",
                "}\n",
            ),
            RecoveryPolicy::Report,
        )
        .unwrap();

        assert_eq!(parsed.inner.len(), 1);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("ignored BibTeX field \"month\""))
        );
    }

    #[test]
    fn non_strict_recovery_literalizes_unknown_abbreviations() {
        let parsed = parse_biblatex_library(
            concat!(
                "@article{macro,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Macro Journal},\n",
                "  year = {2024},\n",
                "  journal = JMLR # { Extra}\n",
                "}\n",
            ),
            RecoveryPolicy::Report,
        )
        .unwrap();

        assert_eq!(parsed.inner.len(), 1);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unknown abbreviation"))
        );
    }
}
