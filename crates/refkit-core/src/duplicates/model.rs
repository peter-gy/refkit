use crate::{BibEdit, DuplicateRule, RawEntryId, RawFieldId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Entry occurrence participating in a duplicate candidate group.
pub struct DuplicateMember {
    /// Identity in the reviewed source snapshot.
    pub entry_id: RawEntryId,
    /// Entry key, which may be duplicated elsewhere in the snapshot.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Matching evidence from one deterministic duplicate rule.
pub struct DuplicateEvidence {
    /// Rule that produced the shared signature.
    pub rule: DuplicateRule,
    /// Rule-specific normalized match value, not a confidence score.
    pub signature: String,
    /// Occurrences sharing this signature.
    pub members: Vec<RawEntryId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Source value contributing to a reviewable conflict.
pub struct DuplicateValue {
    /// Owning entry occurrence in the reviewed snapshot.
    pub entry_id: RawEntryId,
    /// Field occurrence, or `None` for an entry-type value.
    pub field_id: Option<RawFieldId>,
    /// Value displayed for review.
    pub value: String,
    /// Complete source expression used when copying this choice into a patch.
    pub expression: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// The bibliographic disagreement requiring an explicit merge choice.
pub enum DuplicateConflictKind {
    /// Different expressions for the same source field.
    Field,
    /// Different canonical values for a recognized identifier scheme.
    Identifier,
    /// Different entry kinds.
    EntryType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Conflicting values within a selected group of occurrences.
pub struct DuplicateConflict {
    /// Category of disagreement.
    pub kind: DuplicateConflictKind,
    /// Source field name, or the entry-type conflict marker.
    pub field: String,
    /// Candidate values with their source identities.
    pub values: Vec<DuplicateValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Connected candidate entries, their match evidence, and unresolved differences.
pub struct DuplicateGroup {
    /// Representative occurrence used as a snapshot-local group identity.
    pub id: RawEntryId,
    /// Candidate entries in source order.
    pub members: Vec<DuplicateMember>,
    /// Rules and signatures connecting group members.
    pub evidence: Vec<DuplicateEvidence>,
    /// Differences to inspect before merging.
    pub conflicts: Vec<DuplicateConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Inspect-only duplicate review, independent of tidy's merge strategy.
pub struct DuplicateReport {
    /// Applied rules after duplicate rule selections are removed.
    pub rules: Vec<DuplicateRule>,
    /// Deterministic candidate groups for the reviewed snapshot.
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
/// An explicit decision for a field in a merge selection.
pub enum MergeFieldChoice {
    /// Retain the complete expression from one source field occurrence.
    Take {
        /// Source field name, matched case-insensitively.
        name: String,
        /// Selected entry owning the retained expression.
        entry_id: RawEntryId,
        /// Field occurrence to retain.
        field_id: RawFieldId,
    },
    /// Omit this field from the retained entry.
    Drop {
        /// Source field name, matched case-insensitively.
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Snapshot-relative selection and decisions used to compile a merge patch.
pub struct MergeRequest {
    /// At least two distinct existing entry occurrences.
    pub entries: Vec<RawEntryId>,
    /// Member of `entries` that survives the merge.
    pub retain: RawEntryId,
    #[serde(default)]
    /// At most one explicit decision per existing field name.
    pub fields: Vec<MergeFieldChoice>,
    /// Explicit retained kind when entry types disagree.
    pub entry_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Inspected merge outcome, with a patch only after all conflicts are resolved.
pub struct MergePlan {
    /// Entry occurrence that will survive.
    pub retained_id: RawEntryId,
    /// Entry occurrences that the accepted patch removes.
    pub removed_ids: Vec<RawEntryId>,
    /// Validated atomic edits, or `None` while choices are still required.
    pub patch: Option<Vec<BibEdit>>,
    /// Decisions still required before a patch can be produced.
    pub conflicts: Vec<DuplicateConflict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Why a merge request could not produce a valid plan.
pub enum MergeErrorCode {
    /// Missing, repeated, or insufficient entry occurrences.
    InvalidSelection,
    /// A field choice does not identify a unique eligible source value.
    InvalidChoice,
    /// A key reference has more than one possible target after merging.
    AmbiguousReference,
    /// A reference expression could not be resolved or represented safely.
    ReferenceError,
    /// Merging would create an inheritance cycle.
    ReferenceCycle,
    /// Input or traversal exceeds a resource budget.
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Structured merge refusal that leaves the reviewed source unchanged.
pub struct MergeError {
    /// Machine-readable refusal category.
    pub code: MergeErrorCode,
    /// Explanation of the invalid selection, choice, or reference graph.
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
