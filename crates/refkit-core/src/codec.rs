mod biblatex;
mod csl;
mod yaml;

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Diagnostic, EntryRecord, Library, LibraryError, RecoveryPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Supported normalized bibliography interchange formats.
pub enum BibliographyFormat {
    #[serde(rename = "biblatex")]
    /// BibTeX and BibLaTeX fields, distinct from source-preserving raw snapshots.
    Biblatex,
    #[serde(rename = "hayagriva")]
    /// Hayagriva's YAML record format.
    Hayagriva,
    #[serde(rename = "csl-json")]
    /// The documented RefKit support profile for CSL-JSON.
    CslJson,
}

impl BibliographyFormat {
    #[must_use]
    /// Return the format identifier accepted by the host APIs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Biblatex => "biblatex",
            Self::Hayagriva => "hayagriva",
            Self::CslJson => "csl-json",
        }
    }
}

impl FromStr for BibliographyFormat {
    type Err = CodecError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "biblatex" => Ok(Self::Biblatex),
            "hayagriva" => Ok(Self::Hayagriva),
            "csl-json" => Ok(Self::CslJson),
            _ => Err(CodecError::new(format!(
                "unknown bibliography format {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether an interchange operation may return a lossy result.
pub enum LossPolicy {
    /// Refuse a result when any conversion issue is lossy.
    Error,
    /// Return the result together with its conversion issues.
    Report,
}

impl FromStr for LossPolicy {
    type Err = CodecError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "error" => Ok(Self::Error),
            "report" => Ok(Self::Report),
            _ => Err(CodecError::new("loss must be 'error' or 'report'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// One retained, approximated, unsupported, or discarded part of a conversion.
pub struct ConversionIssue {
    /// Machine-readable conversion issue code.
    pub code: String,
    /// `decode` or `encode`, identifying where the issue arose.
    pub stage: String,
    /// Affected entry key when attributable to one record.
    pub entry: Option<String>,
    /// Canonical record field path.
    pub path: String,
    /// Whether strict loss policy rejects this issue.
    pub lossy: bool,
    /// Explanation of the mapping and its consequence.
    pub message: String,
}

/// Decoded records and issues discovered before source information is lost.
pub struct DecodeReport {
    /// Constructed normalized library.
    pub library: Library,
    /// Format used to interpret the input.
    pub format: BibliographyFormat,
    /// Conversion findings in deterministic traversal order.
    pub issues: Vec<ConversionIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Normalized source text and its representational limitations.
pub struct EncodeReport {
    /// Target format of the encoded text.
    pub format: BibliographyFormat,
    /// Newly serialized bibliography text.
    pub text: String,
    /// Mapping losses and readback differences.
    pub issues: Vec<ConversionIssue>,
}

#[derive(Debug, Clone)]
/// Combined decode and encode result for a format-to-format conversion.
pub struct ConversionReport {
    /// Input bibliography format.
    pub source_format: BibliographyFormat,
    /// Output bibliography format.
    pub target_format: BibliographyFormat,
    /// Normalized target-format text.
    pub text: String,
    /// Decode findings followed by encode findings.
    pub issues: Vec<ConversionIssue>,
    /// Parser recovery diagnostics from the source library.
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
/// Invalid interchange input or a conversion refused by strict loss policy.
pub struct CodecError {
    /// Summary of the failed operation.
    pub message: String,
    /// Field-level conversion findings collected before refusal.
    pub issues: Vec<ConversionIssue>,
    /// Associated parser diagnostics when source interpretation failed.
    pub diagnostics: Vec<Diagnostic>,
}

impl CodecError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            issues: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
    pub(crate) fn at(path: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            issues: vec![ConversionIssue {
                code: "invalid_value".into(),
                stage: String::new(),
                entry: None,
                path: path.into(),
                lossy: false,
                message: message.clone(),
            }],
            message,
            diagnostics: Vec::new(),
        }
    }
    fn context(mut self, stage: &str) -> Self {
        if self.issues.is_empty() {
            self.issues.push(ConversionIssue {
                code: if stage == "decode" {
                    "invalid_source"
                } else {
                    "invalid_target"
                }
                .into(),
                stage: stage.into(),
                entry: self
                    .diagnostics
                    .first()
                    .and_then(|diagnostic| diagnostic.entry.clone()),
                path: self
                    .diagnostics
                    .first()
                    .and_then(|diagnostic| diagnostic.field.clone())
                    .unwrap_or_default(),
                lossy: false,
                message: self.message.clone(),
            });
        }
        for issue in &mut self.issues {
            if issue.stage.is_empty() {
                issue.stage = stage.into();
            }
        }
        self
    }
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CodecError {}
impl From<LibraryError> for CodecError {
    fn from(error: LibraryError) -> Self {
        if let LibraryError::Record(error) = error {
            return Self::at(&error.path, &error.message);
        }
        let diagnostics = match &error {
            LibraryError::Biblatex(failure) | LibraryError::HayagrivaYaml(failure) => {
                failure.diagnostics.clone()
            }
            _ => Vec::new(),
        };
        Self {
            message: error.to_string(),
            issues: Vec::new(),
            diagnostics,
        }
    }
}

/// Decode normalized records while retaining source-specific extensions.
///
/// # Errors
/// Rejects invalid or oversized input, invalid record shapes, unsupported recovery
/// policies, and lossy mappings when `loss` is [`LossPolicy::Error`].
pub fn decode(
    source: &str,
    format: BibliographyFormat,
    loss: LossPolicy,
    recovery: RecoveryPolicy,
) -> Result<DecodeReport, CodecError> {
    decode_inner(source, format, loss, recovery).map_err(|error| error.context("decode"))
}

fn decode_inner(
    source: &str,
    format: BibliographyFormat,
    loss: LossPolicy,
    recovery: RecoveryPolicy,
) -> Result<DecodeReport, CodecError> {
    crate::library::validate_source_size(source.len()).map_err(|error| CodecError {
        message: error.message.clone(),
        diagnostics: vec![error],
        issues: Vec::new(),
    })?;
    if format != BibliographyFormat::Biblatex && recovery != RecoveryPolicy::Error {
        return Err(CodecError::new("recovery applies to BibLaTeX input"));
    }
    let mut issues = Vec::new();
    let library = match format {
        BibliographyFormat::Biblatex => Library::parse_biblatex(source, recovery)?,
        BibliographyFormat::Hayagriva => Library::parse_hayagriva_yaml(source)?,
        BibliographyFormat::CslJson => Library::from_records(csl::decode(source, &mut issues)?)?,
    };
    for diagnostic in library.diagnostics() {
        if matches!(
            diagnostic.action,
            crate::DiagnosticAction::DroppedBlock
                | crate::DiagnosticAction::DroppedField
                | crate::DiagnosticAction::Literalized
        ) {
            issues.push(ConversionIssue {
                code: "recovered_input".into(),
                stage: "decode".into(),
                entry: diagnostic.entry.clone(),
                path: diagnostic.field.clone().unwrap_or_default(),
                lossy: true,
                message: diagnostic.message.clone(),
            });
        }
    }
    if format == BibliographyFormat::Biblatex {
        biblatex::decode_issues(library.records(), &mut issues);
    }
    if let Err(mut error) = check_loss(&issues, loss) {
        error.diagnostics = library.diagnostics().to_vec();
        return Err(error);
    }
    Ok(DecodeReport {
        library,
        format,
        issues,
    })
}

/// Encode records and compare the decoded output with the retained input data.
///
/// # Errors
/// Rejects unrepresentable output, failed readback, and lossy mappings when
/// `loss` is [`LossPolicy::Error`]. Source-preserving writeback uses raw snapshots.
pub fn encode(
    library: &Library,
    format: BibliographyFormat,
    loss: LossPolicy,
) -> Result<EncodeReport, CodecError> {
    encode_inner(library, format, loss).map_err(|error| error.context("encode"))
}

fn encode_inner(
    library: &Library,
    format: BibliographyFormat,
    loss: LossPolicy,
) -> Result<EncodeReport, CodecError> {
    let mut issues = Vec::new();
    let text = match format {
        BibliographyFormat::Biblatex => biblatex::encode(library.records(), &mut issues)?,
        BibliographyFormat::Hayagriva => yaml::encode(library.records(), &mut issues)?,
        BibliographyFormat::CslJson => csl::encode(library.records(), &mut issues)?,
    };
    let decoded = decode(&text, format, LossPolicy::Report, RecoveryPolicy::Error)?;
    for record in library.records() {
        match decoded.library.get_record(&record.key) {
            Some(actual) => differences(
                &semantic_record(record),
                &semantic_record(actual),
                "",
                &record.key,
                &mut issues,
            ),
            None => issue(
                &mut issues,
                "record_removed",
                "encode",
                &record.key,
                "",
                true,
                "target format did not retain this record",
            ),
        }
    }
    deduplicate_issues(&mut issues);
    check_loss(&issues, loss)?;
    Ok(EncodeReport {
        format,
        text,
        issues,
    })
}

/// Decode one bibliography format and encode another with combined loss reporting.
///
/// # Errors
/// Returns the first decode or encode failure, including strict loss refusal.
pub fn convert(
    source: &str,
    source_format: BibliographyFormat,
    target_format: BibliographyFormat,
    loss: LossPolicy,
    recovery: RecoveryPolicy,
) -> Result<ConversionReport, CodecError> {
    let decoded = decode(source, source_format, loss, recovery)?;
    let mut encoded = encode(&decoded.library, target_format, loss)?;
    encoded.issues.splice(0..0, decoded.issues);
    Ok(ConversionReport {
        source_format,
        target_format,
        text: encoded.text,
        issues: encoded.issues,
        diagnostics: decoded.library.diagnostics().to_vec(),
    })
}

fn check_loss(issues: &[ConversionIssue], policy: LossPolicy) -> Result<(), CodecError> {
    if policy == LossPolicy::Error && issues.iter().any(|issue| issue.lossy) {
        return Err(CodecError {
            message: "conversion would lose bibliography data".into(),
            issues: issues.to_vec(),
            diagnostics: Vec::new(),
        });
    }
    Ok(())
}

pub(super) fn issue(
    issues: &mut Vec<ConversionIssue>,
    code: &str,
    stage: &str,
    entry: &str,
    path: &str,
    lossy: bool,
    message: impl Into<String>,
) {
    issues.push(ConversionIssue {
        code: code.into(),
        stage: stage.into(),
        entry: Some(entry.into()),
        path: path.into(),
        lossy,
        message: message.into(),
    });
}

#[expect(
    clippy::expect_used,
    reason = "Codec inputs are immutable Library records whose finite extension numbers and JSON serialization were validated during construction."
)]
fn semantic_record(record: &EntryRecord) -> Value {
    let mut value = serde_json::to_value(record).expect("validated record serializes");
    normalize_record_value(&mut value, record);
    value
}

fn normalize_record_value(value: &mut Value, record: &EntryRecord) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    normalize_extensions(object.get_mut("extensions"), record);
    if let Some(Value::Array(parents)) = object.get_mut("parents") {
        for (parent, record) in parents.iter_mut().zip(&record.parents) {
            normalize_record_value(parent, record);
        }
    }
    for chunks in object
        .values_mut()
        .filter_map(|field| field.get_mut("chunks").and_then(Value::as_array_mut))
    {
        merge_text_chunks(chunks);
    }
}

fn normalize_extensions(value: Option<&mut Value>, record: &EntryRecord) {
    let Some(namespaces) = value.and_then(Value::as_object_mut) else {
        return;
    };
    for (namespace, fields) in namespaces.iter_mut() {
        let Some(fields) = fields.as_object_mut() else {
            continue;
        };
        fields.retain(|field, _| {
            !(field.starts_with('@')
                || namespace == "biblatex"
                    && record
                        .extensions
                        .get(namespace)
                        .and_then(|values| values.get(field))
                        .is_some_and(|value| biblatex::redundant_extension(record, field, value)))
        });
    }
    namespaces.retain(|_, fields| fields.as_object().is_some_and(|fields| !fields.is_empty()));
}

fn merge_text_chunks(chunks: &mut Vec<Value>) {
    let mut merged: Vec<Value> = Vec::new();
    for chunk in std::mem::take(chunks) {
        let previous = merged
            .last_mut()
            .filter(|previous| previous.get("kind") == chunk.get("kind"))
            .and_then(Value::as_object_mut);
        if let Some(previous) = previous {
            let text = format!(
                "{}{}",
                previous
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                chunk
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            );
            previous.insert("text".into(), Value::String(text));
        } else {
            merged.push(chunk);
        }
    }
    *chunks = merged;
}

fn differences(
    expected: &Value,
    actual: &Value,
    path: &str,
    key: &str,
    issues: &mut Vec<ConversionIssue>,
) {
    if expected == actual {
        return;
    }
    match (expected, actual) {
        (Value::Object(left), Value::Null) => {
            for (field, value) in left {
                let path = if path.is_empty() {
                    field.clone()
                } else {
                    format!("{path}.{field}")
                };
                differences(value, &Value::Null, &path, key, issues);
            }
        }
        (Value::Object(left), Value::Object(right)) => {
            let fields: BTreeSet<_> = left.keys().chain(right.keys()).collect();
            for field in fields {
                let path = if path.is_empty() {
                    field.clone()
                } else {
                    format!("{path}.{field}")
                };
                differences(
                    left.get(field).unwrap_or(&Value::Null),
                    right.get(field).unwrap_or(&Value::Null),
                    &path,
                    key,
                    issues,
                );
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                differences(left, right, &format!("{path}[{index}]"), key, issues);
            }
        }
        _ => issue(
            issues,
            if actual.is_null() {
                "field_removed"
            } else {
                "field_changed"
            },
            "encode",
            key,
            path,
            true,
            "field is not retained exactly when the target bibliography is decoded",
        ),
    }
}

fn deduplicate_issues(issues: &mut Vec<ConversionIssue>) {
    let mut seen = BTreeSet::new();
    issues.retain(|issue| {
        seen.insert((
            issue.stage.clone(),
            issue.entry.clone(),
            issue.path.clone(),
            issue.code.clone(),
        ))
    });
}
