use super::{RawDocument, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A field to author in a raw snapshot.
pub struct BibFieldValue {
    /// BibTeX field name.
    pub name: String,
    /// Literal content or a complete expression according to `expression`.
    pub value: String,
    #[serde(default)]
    /// Interpret `value` as a complete BibTeX expression instead of literal content.
    pub expression: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
/// One atomic patch operation targeting occurrences in the original snapshot.
pub enum BibEdit {
    /// Replace the value of an existing field occurrence.
    SetField {
        /// Owning entry in the input snapshot.
        entry_id: RawEntryId,
        /// Field occurrence within that entry.
        field_id: RawFieldId,
        /// Replacement literal content or complete expression.
        value: String,
        #[serde(default)]
        /// Validate and retain a complete expression when true.
        expression: bool,
    },
    /// Append a field to an existing entry.
    AddField {
        /// Entry receiving the new field.
        entry_id: RawEntryId,
        /// New field's BibTeX name.
        name: String,
        /// New literal content or complete expression.
        value: String,
        #[serde(default)]
        /// Validate and retain a complete expression when true.
        expression: bool,
    },
    /// Remove one field occurrence and its assignment syntax.
    RemoveField {
        /// Owning entry in the input snapshot.
        entry_id: RawEntryId,
        /// Exact field occurrence to remove.
        field_id: RawFieldId,
    },
    /// Insert a complete entry without rewriting unrelated source blocks.
    AddEntry {
        /// Authored entry key.
        key: String,
        /// BibTeX entry kind without the leading at-sign.
        entry_type: String,
        #[serde(default)]
        /// Fields in their requested source order.
        fields: Vec<BibFieldValue>,
        #[serde(default)]
        /// Input-snapshot insertion anchor, or append when absent.
        before: Option<RawEntryId>,
    },
    /// Remove one complete entry block.
    RemoveEntry {
        /// Input-snapshot entry occurrence to remove.
        entry_id: RawEntryId,
    },
    /// Rename one entry and rewrite unambiguous references to its key.
    RenameEntry {
        /// Input-snapshot entry occurrence to rename.
        entry_id: RawEntryId,
        /// Authored replacement key.
        key: String,
    },
    /// Replace an entry kind while preserving its fields and delimiters.
    SetEntryType {
        /// Input-snapshot entry occurrence to change.
        entry_id: RawEntryId,
        /// Replacement BibTeX kind without the leading at-sign.
        entry_type: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Operation responsible for one reported byte change.
pub enum BibPatchKind {
    /// Existing field value replacement.
    SetField,
    /// New field insertion.
    AddField,
    /// Field occurrence removal.
    RemoveField,
    /// New entry insertion.
    AddEntry,
    /// Entry block removal.
    RemoveEntry,
    /// Entry key replacement.
    RenameEntry,
    /// Entry kind replacement.
    SetEntryType,
    /// Automatic rewrite of a reference affected by a key change.
    RewriteReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Why an atomic patch was refused.
pub enum BibPatchErrorCode {
    /// An occurrence does not exist in the input snapshot.
    InvalidTarget,
    /// Authored text is not valid for the requested syntax position.
    InvalidValue,
    /// Operations conflict or require overlapping byte replacements.
    Overlap,
    /// The proposed output fails post-application verification.
    InvalidResult,
    /// A referenced key cannot identify one final entry.
    AmbiguousReference,
    /// A reference expression cannot be resolved or safely rewritten.
    ReferenceError,
    /// Patch size or traversal exceeds the supported budget.
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Atomic patch refusal with an optional offending operation index.
pub struct BibPatchError {
    /// Machine-readable failure category.
    pub code: BibPatchErrorCode,
    /// Zero-based index in the submitted patch when attributable to one operation.
    pub operation: Option<usize>,
    /// Explanation of the conflict or rejected syntax.
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
/// Corresponding UTF-8 byte ranges replaced in the old and new snapshots.
pub struct BibPatchChange {
    /// Input operation indices. Empty for automatic reference rewrites.
    pub operations: Vec<usize>,
    /// Transformation responsible for the byte replacement.
    pub kind: BibPatchKind,
    /// Replaced range in the input snapshot.
    pub before: Range<usize>,
    /// Replacement range in the returned snapshot.
    pub after: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One field's identity and spans across an atomic patch.
pub struct BibFieldMapping {
    /// Input occurrence, absent for an added field.
    pub before: Option<RawFieldInfo>,
    /// Output occurrence, absent for a removed field.
    pub after: Option<RawFieldInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One entry's identity, spans, and field mappings across a patch.
pub struct BibEntryMapping {
    /// Input occurrence, absent for an added entry.
    pub before: Option<RawEntryInfo>,
    /// Output occurrence, absent for a removed entry.
    pub after: Option<RawEntryInfo>,
    /// Mappings for retained, removed, and added fields.
    pub fields: Vec<BibFieldMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Nonfatal condition in a successfully patched snapshot.
pub struct BibPatchWarning {
    /// Machine-readable warning code.
    pub code: &'static str,
    /// Occurrence indices in the result document.
    pub entry_id: usize,
    /// Result-snapshot field occurrence when the warning concerns a field.
    pub field_id: Option<usize>,
    /// Explanation of the condition requiring review.
    pub message: String,
}

#[derive(Debug, Clone)]
/// New immutable snapshot and complete source-change report.
pub struct BibPatchResult {
    /// Validated output snapshot. The original document remains unchanged.
    pub document: RawDocument,
    /// Replacements in source order, leaving intervening bytes untouched.
    pub changes: Vec<BibPatchChange>,
    /// Complete input-to-output entry and field occurrence mappings.
    pub entries: Vec<BibEntryMapping>,
    /// Reviewable conditions in the output snapshot.
    pub warnings: Vec<BibPatchWarning>,
}
