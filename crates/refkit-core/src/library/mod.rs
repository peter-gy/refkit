mod diagnostic;
mod guard;
mod parse;
mod recovery;
mod source;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::str::FromStr;
use std::sync::OnceLock;

use hayagriva::{Library as HayLibrary, Selector};

use crate::quoted;
use crate::{EntryRecord, RecordError};

pub use self::diagnostic::{Diagnostic, DiagnosticAction, DiagnosticSeverity, ParseFailure};
pub(crate) use self::guard::{
    FieldResolver, normalize_reference, validate_date_parser_input, validate_literal, validate_raw,
    validate_source, validate_source_size,
};
pub use self::parse::parse_bibtex_report;
pub(crate) use self::parse::parse_error;
use self::parse::{parse_biblatex_library, parse_hayagriva_yaml};

pub(crate) struct ParsedLibrary {
    pub(crate) records: Vec<EntryRecord>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// Immutable normalized records and their derived rendering/selection state.
pub struct Library {
    inner: HayLibrary,
    diagnostics: Vec<Diagnostic>,
    keys: OnceLock<Vec<String>>,
    records: Vec<EntryRecord>,
    index: HashMap<String, usize>,
}

#[derive(serde::Serialize)]
struct RecordArchive<R> {
    schema_version: u32,
    records: R,
}

struct RecordSize(usize);

impl Write for RecordSize {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > crate::record::MAX_RECORD_JSON_BYTES {
            return Err(io::Error::other("record serialization exceeds 128 MiB"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Structured parse outcome for consumers that do not need a library instance.
pub struct ParseReport {
    /// Whether normalized records were produced.
    pub ok: bool,
    /// Normalized top-level entry count on success.
    pub entry_count: Option<usize>,
    /// Normalized keys in source order on success.
    pub keys: Option<Vec<String>>,
    /// Parser failures and recovery actions with original source coordinates.
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsing, structured construction, or selector failure.
pub enum LibraryError {
    /// BibTeX/BibLaTeX could not be normalized under the requested recovery policy.
    Biblatex(ParseFailure),
    /// Hayagriva YAML could not be normalized.
    HayagrivaYaml(ParseFailure),
    /// Selector syntax is invalid.
    Selector(String),
    /// A structured record violates the owned record contract.
    Record(RecordError),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Biblatex(failure) | Self::HayagrivaYaml(failure) => failure.fmt(f),
            Self::Selector(message) => write!(f, "invalid selector: {message}"),
            Self::Record(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LibraryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Treatment of recoverable BibTeX source errors.
pub enum RecoveryPolicy {
    /// Reject source errors instead of changing affected content.
    Error,
    /// Recover locally where supported and retain structured diagnostics.
    Report,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Scalar projection of a complete structured record.
pub enum EntryField {
    /// Top-level record key.
    Key,
    /// Normalized entry-type name.
    EntryType,
    /// Flattened full title text.
    Title,
    /// Retained date formatted with its precision and qualifiers.
    Date,
    /// DOI identifier value.
    Doi,
    /// Volume of the entry or its first container.
    Volume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Unsupported scalar projection field name.
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
            "entry_type" => Ok(Self::EntryType),
            "title" => Ok(Self::Title),
            "date" => Ok(Self::Date),
            "doi" => Ok(Self::Doi),
            "volume" => Ok(Self::Volume),
            _ => Err(EntryFieldError(value.to_string())),
        }
    }
}

impl EntryRecord {
    #[must_use]
    /// Project one scalar field, borrowing retained text when possible.
    pub fn field(&self, field: EntryField) -> Option<Cow<'_, str>> {
        match field {
            EntryField::Key => Some(Cow::Borrowed(&self.key)),
            EntryField::EntryType => Some(Cow::Borrowed(&self.entry_type)),
            EntryField::Title => self
                .title
                .as_ref()
                .map(|title| Cow::Owned(title.plain_text())),
            EntryField::Date => self.date.as_ref().map(|date| Cow::Owned(date.display())),
            EntryField::Doi => self
                .identifiers
                .get("doi")
                .map(|doi| Cow::Borrowed(doi.as_str())),
            EntryField::Volume => self
                .volume
                .as_ref()
                .or_else(|| {
                    self.parents
                        .first()
                        .and_then(|parent| parent.volume.as_ref())
                })
                .map(|volume| Cow::Borrowed(volume.value())),
        }
    }
}

impl Library {
    /// Validate and own complete records, then derive the private engine view.
    ///
    /// # Errors
    /// Rejects duplicate keys, invalid field shapes, resource-budget violations,
    /// or values that cannot prepare the supported rendering representation.
    pub fn from_records(records: Vec<EntryRecord>) -> Result<Self, LibraryError> {
        if records.len() > 100_000 {
            return Err(LibraryError::Record(RecordError::new(
                "records",
                "record count exceeds 100000",
            )));
        }
        let mut visited = 0;
        for record in &records {
            record
                .validate_limits(0, &mut visited)
                .map_err(LibraryError::Record)?;
        }
        serde_json::to_writer(
            RecordSize(0),
            &RecordArchive {
                schema_version: 1,
                records: &records,
            },
        )
        .map_err(|error| LibraryError::Record(RecordError::new("records", error.to_string())))?;
        let mut index = HashMap::with_capacity(records.len());
        let engine = records
            .iter()
            .enumerate()
            .map(|(position, record)| {
                if index.insert(record.key.clone(), position).is_some() {
                    return Err(LibraryError::Record(RecordError::new(
                        &record.key,
                        "duplicate entry key",
                    )));
                }
                record.to_engine().map_err(LibraryError::Record)
            })
            .collect::<Result<HayLibrary, _>>()?;
        Ok(Self {
            inner: engine,
            diagnostics: Vec::new(),
            keys: OnceLock::new(),
            records,
            index,
        })
    }

    /// Reconstruct a library from a versioned canonical record snapshot.
    ///
    /// # Errors
    /// Rejects invalid JSON, duplicate properties, unknown versions or fields,
    /// excessive size/nesting, and record-construction failures.
    pub fn from_json(source: &str) -> Result<Self, LibraryError> {
        Self::from_records(
            crate::record::decode_records(source, true).map_err(LibraryError::Record)?,
        )
    }

    /// Construct a library from a canonical JSON array of complete records.
    ///
    /// # Errors
    /// Rejects invalid JSON, duplicate properties, excessive size/nesting,
    /// invalid record shapes, and record-construction failures.
    pub fn from_records_json(source: &str) -> Result<Self, LibraryError> {
        Self::from_records(
            crate::record::decode_records(source, false).map_err(LibraryError::Record)?,
        )
    }

    /// Serialize the retained records as a versioned canonical snapshot.
    ///
    /// # Panics
    /// Panics if serialization violates the finite-value and size invariants
    /// checked during construction. Callers cannot mutate the retained records.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "Construction pre-serializes and bounds these immutable records, including every finite extension number."
    )]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&RecordArchive {
            schema_version: 1,
            records: self.records(),
        })
        .expect("validated records serialize")
    }

    /// Parse BibTeX/BibLaTeX while capturing data before engine normalization.
    ///
    /// # Errors
    /// Returns structured source failures or record-contract errors after recovery.
    pub fn parse_biblatex(source: &str, recovery: RecoveryPolicy) -> Result<Self, LibraryError> {
        parse_biblatex_library(source, recovery)
            .map_err(LibraryError::Biblatex)
            .and_then(Self::from_parsed)
    }

    /// Parse Hayagriva YAML and retain namespaced source extensions.
    ///
    /// # Errors
    /// Rejects invalid YAML, oversized source, or invalid normalized records.
    pub fn parse_hayagriva_yaml(source: &str) -> Result<Self, LibraryError> {
        parse_hayagriva_yaml(source)
            .map_err(LibraryError::HayagrivaYaml)
            .and_then(Self::from_parsed)
    }

    pub(crate) fn from_parsed(parsed: ParsedLibrary) -> Result<Self, LibraryError> {
        let mut library = Self::from_records(parsed.records)?;
        library.diagnostics = parsed.diagnostics;
        Ok(library)
    }

    pub(crate) fn inner(&self) -> &HayLibrary {
        &self.inner
    }

    /// Borrow the parser and recovery findings retained with this library.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Return the number of top-level records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Return whether the library contains no top-level records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Test an exact, case-sensitive record key.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// Borrow unique top-level keys in retained source order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        self.keys.get_or_init(|| {
            self.records
                .iter()
                .map(|record| record.key.clone())
                .collect()
        })
    }

    /// Borrow the complete immutable records in source order.
    #[must_use]
    pub fn records(&self) -> &[EntryRecord] {
        &self.records
    }

    /// Borrow one complete record by exact key, or return `None` when absent.
    #[must_use]
    pub fn get_record(&self, key: &str) -> Option<&EntryRecord> {
        self.index
            .get(key)
            .and_then(|index| self.records.get(*index))
    }

    /// Select complete records using the derived engine's selector semantics.
    ///
    /// # Errors
    /// Rejects invalid selectors or an inconsistent prepared-record lookup.
    pub fn select_records(&self, selector: &str) -> Result<Vec<EntryRecord>, LibraryError> {
        let selector =
            Selector::parse(selector).map_err(|err| LibraryError::Selector(err.to_string()))?;
        self.inner()
            .iter()
            .filter(|entry| selector.matches(entry))
            .map(|entry| {
                self.get_record(entry.key()).cloned().ok_or_else(|| {
                    LibraryError::Record(RecordError::new(
                        entry.key(),
                        "prepared entry is absent from retained records",
                    ))
                })
            })
            .collect()
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

        let library = Library::from_parsed(parsed).unwrap();
        let record = &library.records()[0];

        assert_eq!(
            [
                record.field(EntryField::Key).as_deref(),
                record.field(EntryField::Title).as_deref(),
                record.field(EntryField::Doi).as_deref(),
                record.field(EntryField::Volume).as_deref(),
            ],
            [Some("doe2024"), Some("Core"), Some("10.1/test"), None]
        );
    }

    #[test]
    fn bibtex_value_parse_treats_recovered_empty_source_as_failure() {
        let Err(err) = parse_biblatex_library("@broken{missing", RecoveryPolicy::Report) else {
            panic!("expected recovered empty source to fail");
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
    use std::fmt::Write as _;

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
            library
                .get_record("good")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
            Some("Correct title")
        );
        assert_eq!(
            library
                .get_record("first")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
            Some("missing suffix")
        );
        assert_eq!(
            library
                .get_record("second")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
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
        let Err(error) = Library::parse_hayagriva_yaml(source) else {
            panic!("expected a missing entry type error");
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
    fn typed_recovery_distinguishes_macro_and_field_source_spans() {
        let source = "@string{badyear=unknown}\n@book{a,title={A},year=badyear}";
        let report = parse_bibtex_report(source, RecoveryPolicy::Report);
        assert!(report.ok);
        assert_eq!(report.diagnostics.len(), 2);
        let start = source.find("unknown").unwrap();
        assert_eq!(
            report.diagnostics[0].span,
            Some(start..start + "unknown".len())
        );
        let diagnostic = &report.diagnostics[1];
        let field_start = source.rfind("badyear").unwrap();
        assert_eq!(
            diagnostic.span,
            Some(field_start..field_start + "badyear".len())
        );
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
                library
                    .get_record("a")
                    .unwrap()
                    .field(crate::EntryField::Title)
                    .as_deref(),
                Some("Coffee")
            );
            assert!(library.diagnostics().is_empty());
        }
        let tidy = crate::tidy_bibtex(source, crate::TidyOptions::default()).unwrap();
        let library = Library::parse_biblatex(&tidy.bibtex, RecoveryPolicy::Error).unwrap();
        assert_eq!(
            library
                .get_record("a")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
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
            library
                .get_record("same")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
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
            library
                .get_record("good")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
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
            library
                .get_record("a")
                .unwrap()
                .field(crate::EntryField::Date)
                .as_deref(),
            Some("2024")
        );
        assert_eq!(
            library
                .get_record("b")
                .unwrap()
                .field(crate::EntryField::Date)
                .as_deref(),
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
            library
                .get_record("a")
                .unwrap()
                .field(crate::EntryField::Title)
                .as_deref(),
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
            writeln!(
                source,
                "@string{{a{index}=a{} # a{}}}",
                index - 1,
                index - 1
            )
            .unwrap();
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
