mod normalize;
mod parse;
mod project;
mod read;
mod recovery;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use hayagriva::{Entry as HayEntry, Library as HayLibrary, Selector};
use serde_json::Number;

use crate::{entry_type_name, quoted};

use self::normalize::normalized_entries;
pub use self::parse::parse_bibtex_report_source;
use self::parse::{parse_bibtex_value_source, parse_library_path, parse_library_source};
pub use self::project::parse_project_field;
use self::project::project_record;
pub use self::read::{SourceText, read_bibliography_text};

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
pub struct EntryRecord {
    pub key: String,
    pub entry_type: String,
    pub title: Option<String>,
    pub volume: Option<String>,
    pub doi: Option<String>,
    pub parents: Vec<EntryRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedEntry {
    pub value: NormalizedValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NormalizedValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<NormalizedValue>),
    Object(BTreeMap<String, NormalizedValue>),
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

impl Library {
    pub fn read_path(
        path: impl AsRef<Path>,
        strict: bool,
        diagnostics: bool,
    ) -> Result<Self, String> {
        parse_library_path(path.as_ref().to_path_buf(), strict, diagnostics).map(Self::from_parsed)
    }

    pub fn parse_source(
        source: &str,
        format: &str,
        strict: bool,
        diagnostics: bool,
    ) -> Result<Self, String> {
        parse_library_source(source, format, strict, diagnostics).map(Self::from_parsed)
    }

    pub fn parse_bibtex(source: &str, strict: bool) -> Result<Self, String> {
        parse_bibtex_value_source(source, strict).map(Self::from_parsed)
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

    pub fn select_records(&self, selector: &str) -> Result<Vec<EntryRecord>, String> {
        let selector =
            Selector::parse(selector).map_err(|err| format!("invalid selector: {err}"))?;
        Ok(self
            .inner
            .iter()
            .filter(|entry| selector.matches(entry))
            .map(entry_record)
            .collect())
    }

    pub fn project_records(
        &self,
        fields: &[ProjectField],
        keys: Option<&[String]>,
    ) -> Result<Vec<Vec<Option<String>>>, String> {
        if fields
            .iter()
            .all(|field| matches!(field, ProjectField::Key))
        {
            return self.project_key_fields(fields, keys);
        }

        match keys {
            Some(keys) => keys
                .iter()
                .map(|key| {
                    let Some(entry) = self.inner.get(key) else {
                        return Err(format!("missing reference {}", quoted(key)));
                    };
                    let record = entry_record(entry);
                    Ok(project_record(&record, fields))
                })
                .collect(),
            None => Ok(self
                .record_cache()
                .records
                .iter()
                .map(|record| project_record(record, fields))
                .collect()),
        }
    }

    pub fn normalized_entries(&self) -> Result<Vec<NormalizedEntry>, String> {
        normalized_entries(self.inner.as_ref())
    }

    fn record_cache(&self) -> &RecordCache {
        self.records
            .get_or_init(|| RecordCache::from_library(&self.inner))
    }

    fn project_key_fields(
        &self,
        fields: &[ProjectField],
        keys: Option<&[String]>,
    ) -> Result<Vec<Vec<Option<String>>>, String> {
        let keys = match keys {
            Some(keys) => {
                for key in keys {
                    if self.inner.get(key).is_none() {
                        return Err(format!("missing reference {}", quoted(key)));
                    }
                }
                keys.to_vec()
            }
            None => self.keys().to_vec(),
        };

        Ok(keys
            .into_iter()
            .map(|key| fields.iter().map(|_| Some(key.clone())).collect())
            .collect())
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
        let parsed = parse_library_source(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}, doi = {10.1/test}}",
            "bibtex",
            false,
            true,
        )
        .unwrap();

        let library = Library::from_parsed(parsed);
        let rows = library
            .project_records(
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
    fn non_strict_recovery_keeps_valid_entries_and_reports_malformed_blocks() {
        let parsed = parse_library_source(
            concat!(
                "@article{valid,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Kept Entry},\n",
                "  year = {2024}\n",
                "}\n",
                "@broken{missing,\n",
                "  title = {No close}\n",
            ),
            "bibtex",
            false,
            true,
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
        let err = match parse_bibtex_value_source("@broken{missing", false) {
            Ok(_) => panic!("expected recovered empty source to fail"),
            Err(err) => err,
        };
        let report = parse_bibtex_report_source("@broken{missing", false);

        assert!(err.contains("malformed BibTeX block"));
        assert!(!report.ok);
        assert_eq!(report.entry_count, None);
        assert_eq!(report.keys, None);
        assert!(report.diagnostics[0].contains("malformed BibTeX block"));
    }

    #[test]
    fn non_strict_recovery_removes_invalid_typed_fields() {
        let parsed = parse_library_source(
            concat!(
                "@article{badmonth,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Bad Month},\n",
                "  year = {2024},\n",
                "  month = {16}\n",
                "}\n",
            ),
            "bibtex",
            false,
            true,
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
        let parsed = parse_library_source(
            concat!(
                "@article{macro,\n",
                "  author = {Doe, Jane},\n",
                "  title = {Macro Journal},\n",
                "  year = {2024},\n",
                "  journal = JMLR # { Extra}\n",
                "}\n",
            ),
            "bibtex",
            false,
            true,
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
