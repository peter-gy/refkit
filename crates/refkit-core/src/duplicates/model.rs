use crate::{BibEdit, DuplicateRule, RawEntryId, RawFieldId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateMember {
    pub entry_id: RawEntryId,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateEvidence {
    pub rule: DuplicateRule,
    pub signature: String,
    pub members: Vec<RawEntryId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateValue {
    pub entry_id: RawEntryId,
    pub field_id: Option<RawFieldId>,
    pub value: String,
    pub expression: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateConflictKind {
    Field,
    Identifier,
    EntryType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateConflict {
    pub kind: DuplicateConflictKind,
    pub field: String,
    pub values: Vec<DuplicateValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateGroup {
    pub id: RawEntryId,
    pub members: Vec<DuplicateMember>,
    pub evidence: Vec<DuplicateEvidence>,
    pub conflicts: Vec<DuplicateConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateReport {
    pub rules: Vec<DuplicateRule>,
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MergeFieldChoice {
    Take {
        name: String,
        entry_id: RawEntryId,
        field_id: RawFieldId,
    },
    Drop {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergeRequest {
    pub entries: Vec<RawEntryId>,
    pub retain: RawEntryId,
    #[serde(default)]
    pub fields: Vec<MergeFieldChoice>,
    pub entry_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergePlan {
    pub retained_id: RawEntryId,
    pub removed_ids: Vec<RawEntryId>,
    pub patch: Option<Vec<BibEdit>>,
    pub conflicts: Vec<DuplicateConflict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeErrorCode {
    InvalidSelection,
    InvalidChoice,
    AmbiguousReference,
    ReferenceError,
    ReferenceCycle,
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeError {
    pub code: MergeErrorCode,
    pub message: String,
}
impl MergeError {
    pub(super) fn new(code: MergeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for MergeError {}
