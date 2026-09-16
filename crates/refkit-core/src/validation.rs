mod biblatex;
mod identifiers;
pub(crate) use identifiers::canonical as canonical_identifier;

use std::collections::BTreeMap;
use std::ops::Range;

use serde::Serialize;

use crate::{Date, DateParts, DateValue, EntryRecord, Library, Name};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Rules used to interpret a validation report.
pub enum ValidationProfile {
    /// Format-neutral quality and identifier checks on normalized records.
    Records,
    /// Source-field requirements from the pinned BibLaTeX engine.
    Biblatex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Whether a finding makes its validation report invalid.
pub enum ValidationSeverity {
    /// A malformed or incoherent value that invalidates the report.
    Error,
    /// A condition requiring review without invalidating the report.
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Deterministic source or bibliographic quality finding.
pub enum ValidationCode {
    /// The source profile requires an absent field.
    MissingRequiredField,
    /// The source profile does not expect this field for the entry kind.
    SuperfluousField,
    /// A source field cannot be interpreted according to its field type.
    MalformedField,
    /// A recognized identifier fails syntax or checksum validation.
    InvalidIdentifier,
    /// An identifier has a valid but noncanonical representation.
    IdentifierForm,
    /// Multiple top-level records share a canonical identifier.
    SharedIdentifier,
    /// URL text is not a supported absolute URL.
    InvalidUrl,
    /// An explicitly supplied title contains no text.
    EmptyTitle,
    /// A supplied creator has no identifying name content.
    EmptyName,
    /// A date interval's lower bound follows its upper bound.
    ReversedDateRange,
    /// A source reference does not resolve to an entry.
    UnresolvedReference,
    /// A nested container lacks descriptive metadata.
    IncompleteContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Record or source occurrence to which a finding applies.
pub struct ValidationTarget {
    /// Top-level record or source entry key.
    pub entry: String,
    /// Record-relative canonical field path, or a BibLaTeX source field name.
    pub path: String,
    /// Source entry occurrence index when validating a raw snapshot.
    pub entry_id: Option<usize>,
    /// Source field occurrence index when available.
    pub field_id: Option<usize>,
    /// UTF-8 byte offsets in the validated source snapshot.
    pub span: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An inspect-only finding with optional related records and advice.
pub struct ValidationIssue {
    /// Machine-readable finding category.
    pub code: ValidationCode,
    /// Whether this finding invalidates the report.
    pub severity: ValidationSeverity,
    /// Primary affected record or source occurrence.
    pub target: ValidationTarget,
    /// Other occurrences contributing to the same finding.
    pub related: Vec<ValidationTarget>,
    /// Description of the bibliographic or source inconsistency.
    pub message: String,
    /// Inspect-only advice. This is never applied automatically.
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Deterministically ordered findings from one inspect-only validation pass.
pub struct ValidationReport {
    /// Vocabulary and requirements used for this report.
    pub profile: ValidationProfile,
    /// Findings in traversal order.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    #[must_use]
    /// Return true when no finding has error severity.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}

impl ValidationTarget {
    fn record(entry: &str, path: impl Into<String>) -> Self {
        Self {
            entry: entry.into(),
            path: path.into(),
            entry_id: None,
            field_id: None,
            span: None,
        }
    }
    fn at(&self, path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..self.clone()
        }
    }
}

fn issue(
    code: ValidationCode,
    target: ValidationTarget,
    message: impl Into<String>,
) -> ValidationIssue {
    let severity = match code {
        ValidationCode::MissingRequiredField
        | ValidationCode::MalformedField
        | ValidationCode::InvalidIdentifier
        | ValidationCode::InvalidUrl
        | ValidationCode::ReversedDateRange
        | ValidationCode::UnresolvedReference => ValidationSeverity::Error,
        _ => ValidationSeverity::Warning,
    };
    ValidationIssue {
        code,
        severity,
        target,
        related: Vec::new(),
        message: message.into(),
        suggestion: None,
    }
}

type Identifiers = BTreeMap<(String, String), Vec<ValidationTarget>>;

fn check_identifier(
    kind: &str,
    value: &str,
    target: ValidationTarget,
    issues: &mut Vec<ValidationIssue>,
) -> Option<String> {
    match identifiers::canonical(kind, value) {
        Some(Ok(canonical)) => {
            if canonical != value {
                let mut finding = issue(
                    ValidationCode::IdentifierForm,
                    target,
                    format!("{kind} has a noncanonical representation"),
                );
                finding.suggestion = Some(canonical.clone());
                issues.push(finding);
            }
            Some(canonical)
        }
        Some(Err(())) => {
            issues.push(issue(
                ValidationCode::InvalidIdentifier,
                target,
                format!("{kind} fails the supported syntax or checksum check"),
            ));
            None
        }
        None => None,
    }
}

fn shared_identifiers(identifiers: Identifiers, issues: &mut Vec<ValidationIssue>) {
    for ((kind, _), mut targets) in identifiers {
        targets.dedup_by(|left, right| left.entry == right.entry);
        if targets.len() > 1 {
            let mut finding = issue(
                ValidationCode::SharedIdentifier,
                targets.remove(0),
                format!("Multiple entries share {kind}. Review the records before merging"),
            );
            finding.related = targets;
            issues.push(finding);
        }
    }
}

fn check_url(value: &str, target: ValidationTarget, issues: &mut Vec<ValidationIssue>) {
    if value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.parse::<hayagriva::types::QualifiedUrl>().is_err()
    {
        issues.push(issue(
            ValidationCode::InvalidUrl,
            target,
            "URL must be an absolute URL without whitespace or control characters",
        ));
    }
}

impl Library {
    /// Inspect record quality without changing records or parser diagnostics.
    pub fn validate(&self) -> ValidationReport {
        let mut issues = Vec::new();
        let mut identifiers = Identifiers::new();
        for record in self.records() {
            check_record(
                record,
                &ValidationTarget::record(&record.key, ""),
                &mut issues,
            );
            collect_identifiers(record, &mut identifiers);
        }
        shared_identifiers(identifiers, &mut issues);
        ValidationReport {
            profile: ValidationProfile::Records,
            issues,
        }
    }
}

fn check_record(
    record: &EntryRecord,
    target: &ValidationTarget,
    issues: &mut Vec<ValidationIssue>,
) {
    let path = |field: &str| {
        if target.path.is_empty() {
            field.into()
        } else {
            format!("{}.{field}", target.path)
        }
    };
    if record
        .title
        .as_ref()
        .is_some_and(|title| title.plain_text().trim().is_empty())
    {
        issues.push(issue(
            ValidationCode::EmptyTitle,
            target.at(path("title")),
            "Title is present but empty",
        ));
    }
    for (field, names) in [("authors", &record.authors), ("editors", &record.editors)] {
        check_names(names, target, &path(field), issues);
    }
    for (i, contributors) in record.affiliated.iter().enumerate() {
        check_names(
            &contributors.names,
            target,
            &path(&format!("affiliated[{i}].names")),
            issues,
        );
    }
    for (field, date) in [
        ("date", &record.date),
        ("event_date", &record.event_date),
        ("original_date", &record.original_date),
    ] {
        if let Some(date) = date {
            check_date(date, target.at(path(field)), issues);
        }
    }
    if let Some(url) = &record.url {
        check_url(&url.value, target.at(path("url.value")), issues);
        if let Some(date) = &url.accessed {
            check_date(date, target.at(path("url.accessed")), issues);
        }
    }
    for (kind, value) in &record.identifiers {
        check_identifier(
            kind,
            value,
            target.at(path(&format!("identifiers.{kind}"))),
            issues,
        );
    }
    for (i, parent) in record.parents.iter().enumerate() {
        let parent_target = target.at(path(&format!("parents[{i}]")));
        if parent.title.is_none()
            && parent.identifiers.is_empty()
            && !["Original", "Conference"].contains(&parent.entry_type.as_str())
        {
            issues.push(issue(
                ValidationCode::IncompleteContainer,
                parent_target.clone(),
                "Container has neither a title nor an identifier",
            ));
        }
        check_record(parent, &parent_target, issues);
    }
}

fn check_names(
    names: &[Name],
    target: &ValidationTarget,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for (i, name) in names.iter().enumerate() {
        let path = format!("{path}[{i}]");
        let empty = match name {
            Name::Organization { name } => name.trim().is_empty(),
            Name::Person { family, given, .. } => {
                family.trim().is_empty()
                    && given.as_ref().is_none_or(|given| given.trim().is_empty())
            }
        };
        if empty {
            issues.push(issue(
                ValidationCode::EmptyName,
                target.at(&path),
                "Creator name is empty",
            ));
        }
        if let Name::Person { id: Some(id), .. } = name
            && identifiers::is_orcid(id)
        {
            check_identifier("orcid", id, target.at(format!("{path}.id")), issues);
        }
    }
}

fn check_date(date: &Date, target: ValidationTarget, issues: &mut Vec<ValidationIssue>) {
    if let DateValue::Range {
        start: Some(start),
        end: Some(end),
    } = &date.value
    {
        // Compare disjoint calendar bounds, never invent ordering within a partial year/month.
        fn bounds(date: &DateParts, latest: bool) -> (i32, u8, u8) {
            (
                date.year,
                date.month.unwrap_or(if latest { 12 } else { 1 }),
                date.day.unwrap_or(if latest { 31 } else { 1 }),
            )
        }
        if bounds(start, false) > bounds(end, true) {
            issues.push(issue(
                ValidationCode::ReversedDateRange,
                target,
                "Date range ends before it starts",
            ));
        }
    }
}

fn collect_identifiers(record: &EntryRecord, identifiers: &mut Identifiers) {
    for (kind, value) in &record.identifiers {
        let Some(Ok(canonical)) = identifiers::canonical(kind, value) else {
            continue;
        };
        identifiers
            .entry((kind.to_ascii_lowercase(), canonical))
            .or_default()
            .push(ValidationTarget::record(
                &record.key,
                format!("identifiers.{kind}"),
            ));
    }
}
