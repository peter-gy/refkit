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
pub enum BibliographyFormat {
    #[serde(rename = "biblatex")]
    Biblatex,
    #[serde(rename = "hayagriva")]
    Hayagriva,
    #[serde(rename = "csl-json")]
    CslJson,
}

impl BibliographyFormat {
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
pub enum LossPolicy {
    Error,
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
pub struct ConversionIssue {
    pub code: String,
    pub stage: String,
    pub entry: Option<String>,
    pub path: String,
    pub lossy: bool,
    pub message: String,
}

pub struct DecodeReport {
    pub library: Library,
    pub format: BibliographyFormat,
    pub issues: Vec<ConversionIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeReport {
    pub format: BibliographyFormat,
    pub text: String,
    pub issues: Vec<ConversionIssue>,
}

#[derive(Debug, Clone)]
pub struct ConversionReport {
    pub source_format: BibliographyFormat,
    pub target_format: BibliographyFormat,
    pub text: String,
    pub issues: Vec<ConversionIssue>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct CodecError {
    pub message: String,
    pub issues: Vec<ConversionIssue>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CodecError {
    pub(crate) fn new(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            issues: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
    pub(crate) fn at(path: &str, message: impl ToString) -> Self {
        let message = message.to_string();
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

fn semantic_record(record: &EntryRecord) -> Value {
    let mut value = serde_json::to_value(record).expect("validated record serializes");
    fn normalize(value: &mut Value, record: &EntryRecord) {
        let Some(object) = value.as_object_mut() else {
            return;
        };
        if let Some(Value::Object(namespaces)) = object.get_mut("extensions") {
            for (namespace, fields) in namespaces.iter_mut() {
                if let Some(fields) = fields.as_object_mut() {
                    fields.retain(|field, _| {
                        !field.starts_with('@')
                            && !(namespace == "biblatex"
                                && record
                                    .extensions
                                    .get(namespace)
                                    .and_then(|values| values.get(field))
                                    .is_some_and(|value| {
                                        biblatex::redundant_extension(record, field, value)
                                    }))
                    });
                }
            }
            namespaces
                .retain(|_, fields| fields.as_object().is_some_and(|fields| !fields.is_empty()));
        }
        if let Some(Value::Array(parents)) = object.get_mut("parents") {
            for (parent, record) in parents.iter_mut().zip(&record.parents) {
                normalize(parent, record);
            }
        }
        for field in object.values_mut() {
            if let Some(chunks) = field.get_mut("chunks").and_then(Value::as_array_mut) {
                let mut merged: Vec<Value> = Vec::new();
                for chunk in std::mem::take(chunks) {
                    if let Some(previous) = merged
                        .last_mut()
                        .filter(|previous| previous["kind"] == chunk["kind"])
                    {
                        previous["text"] = Value::String(format!(
                            "{}{}",
                            previous["text"].as_str().unwrap_or_default(),
                            chunk["text"].as_str().unwrap_or_default()
                        ));
                    } else {
                        merged.push(chunk);
                    }
                }
                *chunks = merged;
            }
        }
    }
    normalize(&mut value, record);
    value
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
