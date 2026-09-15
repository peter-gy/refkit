use super::{RawDocument, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BibFieldValue {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub expression: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BibEdit {
    SetField {
        entry_id: RawEntryId,
        field_id: RawFieldId,
        value: String,
        #[serde(default)]
        expression: bool,
    },
    AddField {
        entry_id: RawEntryId,
        name: String,
        value: String,
        #[serde(default)]
        expression: bool,
    },
    RemoveField {
        entry_id: RawEntryId,
        field_id: RawFieldId,
    },
    AddEntry {
        key: String,
        entry_type: String,
        #[serde(default)]
        fields: Vec<BibFieldValue>,
        #[serde(default)]
        before: Option<RawEntryId>,
    },
    RemoveEntry {
        entry_id: RawEntryId,
    },
    RenameEntry {
        entry_id: RawEntryId,
        key: String,
    },
    SetEntryType {
        entry_id: RawEntryId,
        entry_type: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BibPatchKind {
    SetField,
    AddField,
    RemoveField,
    AddEntry,
    RemoveEntry,
    RenameEntry,
    SetEntryType,
    RewriteReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BibPatchErrorCode {
    InvalidTarget,
    InvalidValue,
    Overlap,
    InvalidResult,
    AmbiguousReference,
    ReferenceError,
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibPatchError {
    pub code: BibPatchErrorCode,
    pub operation: Option<usize>,
    pub message: String,
}

impl BibPatchError {
    pub(super) fn new(
        code: BibPatchErrorCode,
        operation: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            operation,
            message: message.into(),
        }
    }
}

impl fmt::Display for BibPatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for BibPatchError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibPatchChange {
    /// Input operation indices. Empty for automatic reference rewrites.
    pub operations: Vec<usize>,
    pub kind: BibPatchKind,
    pub before: Range<usize>,
    pub after: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibFieldMapping {
    pub before: Option<RawFieldInfo>,
    pub after: Option<RawFieldInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibEntryMapping {
    pub before: Option<RawEntryInfo>,
    pub after: Option<RawEntryInfo>,
    pub fields: Vec<BibFieldMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BibPatchWarning {
    pub code: &'static str,
    /// Occurrence indices in the result document.
    pub entry_id: usize,
    pub field_id: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BibPatchResult {
    pub document: RawDocument,
    pub changes: Vec<BibPatchChange>,
    pub entries: Vec<BibEntryMapping>,
    pub warnings: Vec<BibPatchWarning>,
}
