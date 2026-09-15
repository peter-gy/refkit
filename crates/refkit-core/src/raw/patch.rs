use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

use super::{RawDocument, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo};

mod apply;
mod model;
mod plan;
mod references;

pub use model::*;

struct Replacement {
    span: Range<usize>,
    text: String,
    operations: Vec<usize>,
    kind: BibPatchKind,
}

struct Plan<'a> {
    document: &'a RawDocument,
    source: &'a str,
    replacements: Vec<Replacement>,
    removed_entries: HashSet<usize>,
    removed_fields: HashSet<(usize, usize)>,
    touched_fields: HashSet<(usize, usize)>,
    renamed: HashMap<usize, String>,
    types: HashMap<usize, String>,
    values: HashMap<(usize, usize), String>,
    added_fields: BTreeMap<usize, Vec<(usize, BibFieldValue)>>,
    added_keys: Vec<String>,
}

impl RawDocument {
    /// Apply snapshot-relative edits atomically and return a new document.
    ///
    /// # Errors
    /// Rejects missing targets, overlapping edits, invalid authored values, ambiguous
    /// references, resource-budget violations, and failed output verification.
    pub fn apply_patch(&self, operations: &[BibEdit]) -> Result<BibPatchResult, BibPatchError> {
        if operations.len() > 100_000 {
            return Err(BibPatchError::new(
                BibPatchErrorCode::ResourceLimit,
                None,
                "BibTeX patch exceeds 100,000 operations",
            ));
        }
        let mut input_bytes = 0usize;
        for (index, operation) in operations.iter().enumerate() {
            let bytes = authored_bytes(operation);
            input_bytes = input_bytes.saturating_add(bytes);
            if input_bytes > 16 * 1024 * 1024 {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::ResourceLimit,
                    Some(index),
                    "Patch authored values exceed 16 MiB",
                ));
            }
        }
        let source = self.render().map_err(|message| {
            BibPatchError::new(BibPatchErrorCode::InvalidResult, None, message)
        })?;
        let mut plan = Plan {
            document: self,
            source: &source,
            replacements: Vec::new(),
            removed_entries: HashSet::new(),
            removed_fields: HashSet::new(),
            touched_fields: HashSet::new(),
            renamed: HashMap::new(),
            types: HashMap::new(),
            values: HashMap::new(),
            added_fields: BTreeMap::new(),
            added_keys: Vec::new(),
        };
        plan.check_targets(operations)?;
        for (index, operation) in operations.iter().enumerate() {
            plan.operation(index, operation)?;
        }
        plan.append_fields();
        plan.rewrite_references()?;
        plan.finish()
    }
}

fn authored_bytes(operation: &BibEdit) -> usize {
    match operation {
        BibEdit::SetField { value, .. } => value.len(),
        BibEdit::AddField { name, value, .. } => name.len().saturating_add(value.len()),
        BibEdit::AddEntry {
            key,
            entry_type,
            fields,
            ..
        } => fields
            .iter()
            .fold(key.len().saturating_add(entry_type.len()), |size, field| {
                size.saturating_add(field.name.len())
                    .saturating_add(field.value.len())
            }),
        BibEdit::RenameEntry { key, .. } => key.len(),
        BibEdit::SetEntryType { entry_type, .. } => entry_type.len(),
        _ => 0,
    }
}
