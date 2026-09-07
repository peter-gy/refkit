mod diagnostic;
mod guard;
mod parse;
mod recovery;
mod source;

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use hayagriva::{Entry as HayEntry, Library as HayLibrary, Selector};

use crate::quoted;
use crate::strings::entry_type_name;

pub use self::diagnostic::{Diagnostic, DiagnosticAction, DiagnosticSeverity, ParseFailure};
pub(crate) use self::guard::{normalize_reference, validate_literal, validate_source};
pub use self::parse::parse_bibtex_report;
use self::parse::{parse_biblatex_library, parse_hayagriva_yaml};

pub(crate) struct ParsedLibrary {
    pub(crate) inner: HayLibrary,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub struct Library {
    inner: Arc<HayLibrary>,
    diagnostics: Vec<Diagnostic>,
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
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryError {
    Biblatex(ParseFailure),
    HayagrivaYaml(ParseFailure),
    Selector(String),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Biblatex(failure) | Self::HayagrivaYaml(failure) => failure.fmt(f),
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

    pub fn diagnostics(&self) -> &[Diagnostic] {
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
    fn bibtex_value_parse_treats_recovered_empty_source_as_failure() {
        let err = match parse_biblatex_library("@broken{missing", RecoveryPolicy::Report) {
            Ok(_) => panic!("expected recovered empty source to fail"),
            Err(err) => err,
        };
        let report = parse_bibtex_report("@broken{missing", RecoveryPolicy::Report);

        assert!(err.to_string().contains("malformed BibTeX block"));
        assert!(!report.ok);
        assert_eq!(report.entry_count, None);
        assert_eq!(report.keys, None);
        assert!(
            report.diagnostics[0]
                .message
                .contains("malformed BibTeX block")
        );
    }
}

#[cfg(test)]
mod recovery_contracts {
    use super::*;

    #[test]
    fn recovery_preserves_resolved_fields_and_original_diagnostic_spans() {
        let source = concat!(
            "@string{known={Correct title}}\n",
            "@book{good,title=known,note={Already \\% escaped}}\n",
            "@book{first,title=missing # { suffix}}\n",
            "@book{second,title=absent}\n",
        );
        let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
        assert_eq!(
            library.get_record("good").unwrap().title.as_deref(),
            Some("Correct title")
        );
        assert_eq!(
            library.get_record("first").unwrap().title.as_deref(),
            Some("missing suffix")
        );
        assert_eq!(
            library.get_record("second").unwrap().title.as_deref(),
            Some("absent")
        );
        let diagnostics = library.diagnostics();
        assert_eq!(diagnostics.len(), 2);
        for (diagnostic, word, key) in [
            (&diagnostics[0], "missing", "first"),
            (&diagnostics[1], "absent", "second"),
        ] {
            let start = source.find(word).unwrap();
            assert_eq!(diagnostic.span, Some(start..start + word.len()));
            assert_eq!(diagnostic.code, "unknown_abbreviation");
            assert_eq!(diagnostic.action, DiagnosticAction::Literalized);
            assert_eq!(diagnostic.entry.as_deref(), Some(key));
            assert_eq!(diagnostic.field.as_deref(), Some("title"));
        }
    }

    #[test]
    fn yaml_failure_has_a_structured_source_diagnostic() {
        let source = "broken:\n  title: Broken\n";
        let error = match Library::parse_hayagriva_yaml(source) {
            Ok(_) => panic!("expected a missing entry type error"),
            Err(error) => error,
        };
        let LibraryError::HayagrivaYaml(failure) = error else {
            panic!("expected a YAML failure");
        };
        assert_eq!(failure.diagnostics.len(), 1);
        let diagnostic = &failure.diagnostics[0];
        assert_eq!(diagnostic.code, "yaml_parse_error");
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostic.action, DiagnosticAction::Rejected);
        let span = diagnostic.span.as_ref().unwrap();
        assert!(source.is_char_boundary(span.start));
        assert!(source.is_char_boundary(span.end));
    }

    #[test]
    fn typed_recovery_keeps_original_spans_inside_literalized_values() {
        let source = "@book{a,title={A},year=bogus}\n@book{b,title=未定,month=錯誤}";
        let report = parse_bibtex_report(source, RecoveryPolicy::Report);
        assert!(report.ok);
        assert_eq!(report.entry_count, Some(2));
        for (token, field, count) in [
            ("bogus", "year", 2),
            ("未定", "title", 1),
            ("錯誤", "month", 2),
        ] {
            let start = source.find(token).unwrap();
            let diagnostics = report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.field.as_deref() == Some(field))
                .collect::<Vec<_>>();
            assert_eq!(diagnostics.len(), count);
            for diagnostic in diagnostics {
                assert_eq!(
                    diagnostic.span,
                    Some(start..start + token.len()),
                    "{diagnostic:?}"
                );
                assert_eq!(&source[diagnostic.span.clone().unwrap()], token);
            }
        }
    }

    #[test]
    fn typed_recovery_preserves_the_literalized_macro_definition_span() {
        let source = "@string{badyear=unknown}\n@book{a,title={A},year=badyear}";
        let report = parse_bibtex_report(source, RecoveryPolicy::Report);
        assert!(report.ok);
        assert_eq!(report.diagnostics.len(), 2);
        let start = source.find("unknown").unwrap();
        for diagnostic in &report.diagnostics {
            assert_eq!(diagnostic.span, Some(start..start + "unknown".len()));
        }
        let diagnostic = &report.diagnostics[1];
        assert_eq!(diagnostic.code, "invalid_field");
        assert_eq!(diagnostic.entry.as_deref(), Some("a"));
        assert_eq!(diagnostic.field.as_deref(), Some("year"));
    }

    #[test]
    fn unicode_macro_names_share_strict_recovery_and_tidy_grammar() {
        let source = "@string{café={Coffee}}\n@book{a,title=café}";
        for policy in [RecoveryPolicy::Error, RecoveryPolicy::Report] {
            let library = Library::parse_biblatex(source, policy).unwrap();
            assert_eq!(
                library.get_record("a").unwrap().title.as_deref(),
                Some("Coffee")
            );
            assert!(library.diagnostics().is_empty());
        }
        let tidy = crate::tidy_bibtex(source, crate::TidyOptions::default()).unwrap();
        let library = Library::parse_biblatex(&tidy.bibtex, RecoveryPolicy::Error).unwrap();
        assert_eq!(
            library.get_record("a").unwrap().title.as_deref(),
            Some("Coffee")
        );
    }

    #[test]
    fn recovery_keeps_first_live_duplicate_after_percent_comments() {
        let source = concat!(
            "% @string{hidden={Hidden}} @book{same,title={Hidden}} % tail\n",
            "@book{same,title={First}}\n",
            "@book{same,title={Second}}\n",
            "@book{other,title={Other}}\n",
            "@broken{unfinished\n",
        );
        let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.keys(), ["same", "other"]);
        assert_eq!(
            library.get_record("same").unwrap().title.as_deref(),
            Some("First")
        );
        assert_eq!(
            library
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["duplicate_key", "malformed_block"]
        );
    }

    #[test]
    fn strict_reference_cycles_return_typed_failures() {
        for field in ["crossref", "xdata"] {
            let source = format!(
                "@book{{a,title={{A}},{field}={{b}}}}\n@book{{b,title={{B}},{field}={{a}}}}"
            );
            let report = parse_bibtex_report(&source, RecoveryPolicy::Error);
            assert!(!report.ok);
            assert_eq!(report.diagnostics[0].code, "cyclic_reference");
            assert_eq!(report.diagnostics[0].severity, DiagnosticSeverity::Error);
            let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
            assert_eq!(library.len(), 2);
            assert_eq!(library.diagnostics()[0].code, "cyclic_reference");
        }
    }

    #[test]
    fn abbreviation_cycles_return_errors_and_recover_locally() {
        let source = "@string{one=two,two=one}\n@book{bad,title=one}\n@book{good,title={Good}}";
        let report = parse_bibtex_report(source, RecoveryPolicy::Error);
        assert!(!report.ok);
        assert_eq!(report.diagnostics[0].code, "cyclic_abbreviation");
        let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
        assert_eq!(
            library.get_record("good").unwrap().title.as_deref(),
            Some("Good")
        );
        assert_eq!(
            library.diagnostics()[0].action,
            DiagnosticAction::Literalized
        );
    }

    #[test]
    fn shared_ancestry_is_not_a_reference_cycle() {
        let source = "@book{a,title={A},crossref={parent}}\n@book{b,title={B},crossref={parent}}\n@book{parent,year={2024}}";
        let library = Library::parse_biblatex(source, RecoveryPolicy::Error).unwrap();
        assert_eq!(
            library.get_record("a").unwrap().date.as_deref(),
            Some("2024")
        );
        assert_eq!(
            library.get_record("b").unwrap().date.as_deref(),
            Some("2024")
        );
    }

    #[test]
    fn deeply_nested_source_returns_a_resource_diagnostic() {
        let source = format!("@book{{a,title={}{}}}", "{".repeat(65), "}".repeat(65));
        for policy in [RecoveryPolicy::Error, RecoveryPolicy::Report] {
            let report = parse_bibtex_report(&source, policy);
            assert!(!report.ok);
            assert_eq!(report.diagnostics[0].code, "resource_limit");
        }
    }

    #[test]
    fn percent_comment_braces_do_not_consume_the_nesting_budget() {
        let source = format!("% {}\n@book{{a,title={{Visible}}}}", "{".repeat(100));
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap();
        assert_eq!(
            library.get_record("a").unwrap().title.as_deref(),
            Some("Visible")
        );
    }

    #[test]
    fn protected_reference_keys_use_normalized_identity_for_cycle_detection() {
        let source = "@book{a,title={A},crossref={{a}}}";
        let report = parse_bibtex_report(source, RecoveryPolicy::Error);
        assert!(!report.ok);
        assert_eq!(report.diagnostics[0].code, "cyclic_reference");
    }

    #[test]
    fn empty_macro_expansion_has_a_work_budget() {
        let mut source = "@string{a0={}}\n".to_string();
        for index in 1..19 {
            source.push_str(&format!(
                "@string{{a{index}=a{} # a{}}}\n",
                index - 1,
                index - 1
            ));
        }
        source.push_str("@book{a,title=a18}");
        let report = parse_bibtex_report(&source, RecoveryPolicy::Error);
        assert!(!report.ok);
        assert_eq!(report.diagnostics[0].code, "resource_limit");
    }

    #[test]
    fn fatal_report_preserves_each_recovery_diagnostic() {
        let source = "@book{one,title={One},month={invalid}}\n@broken{unfinished";
        let report = parse_bibtex_report(source, RecoveryPolicy::Report);
        assert!(report.ok);
        assert_eq!(report.diagnostics.len(), 2);
        assert_eq!(report.diagnostics[0].field.as_deref(), Some("month"));
        assert_eq!(report.diagnostics[1].code, "malformed_block");
        let empty = parse_bibtex_report("@bad{one\n@bad{two", RecoveryPolicy::Report);
        assert!(!empty.ok);
        assert_eq!(empty.diagnostics.len(), 2);
        assert!(
            empty
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == "malformed_block")
        );
    }
}
