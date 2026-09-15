use refkit_core::{RawBlockInfo, RawDocument, RawEntryId, RawEntryInfo, RawFieldInfo};
use serde_json::{Value, json};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::errors::{error, parse_error};

#[wasm_bindgen]
#[derive(Clone)]
/// An immutable source-preserving snapshot with document-local occurrence handles.
pub struct NativeRawDocument {
    inner: Arc<RawDocument>,
}

impl NativeRawDocument {
    fn from_core(inner: RawDocument) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[wasm_bindgen]
/// A patched snapshot and its JSON occurrence-mapping report.
pub struct NativePatchResult {
    document: NativeRawDocument,
    report: String,
}

#[wasm_bindgen]
impl NativePatchResult {
    #[wasm_bindgen(getter)]
    /// Clone the handle to the patched snapshot.
    pub fn document(&self) -> NativeRawDocument {
        self.document.clone()
    }
    /// Return the patch changes, occurrence mappings, and warnings as JSON.
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

#[wasm_bindgen]
impl NativeRawDocument {
    #[must_use]
    /// Parse source into an inspectable snapshot, retaining malformed blocks.
    pub fn parse(source: &str) -> NativeRawDocument {
        Self::from_core(RawDocument::parse(source))
    }

    /// Return inspect-only duplicate groups for a JSON rule selection.
    ///
    /// # Errors
    /// Rejects invalid rules and source that cannot be reviewed within resource limits.
    pub fn find_duplicates(&self, source: &str) -> Result<String, JsValue> {
        let rules: Option<Vec<refkit_core::DuplicateRule>> =
            serde_json::from_str(source).map_err(|error| {
                crate::errors::merge_error(refkit_core::MergeError {
                    code: refkit_core::MergeErrorCode::InvalidSelection,
                    message: format!("Invalid duplicate rules: {error}"),
                })
            })?;
        let report = self
            .inner
            .find_duplicates(rules.as_deref())
            .map_err(crate::errors::merge_error)?;
        Ok(crate::conversion::raw_keys(json!(report), true)
            .map_err(|message| error("RefkitError", message))?
            .to_string())
    }

    /// Compile a JSON merge request into a reviewable patch plan.
    ///
    /// # Errors
    /// Rejects invalid selections, conflicting choices, ambiguous references, cycles, and resource limits.
    pub fn plan_merge(&self, source: &str) -> Result<String, JsValue> {
        let invalid = |message| {
            crate::errors::merge_error(refkit_core::MergeError {
                code: refkit_core::MergeErrorCode::InvalidSelection,
                message,
            })
        };
        let value: Value = serde_json::from_str(source)
            .map_err(|error| invalid(format!("Invalid merge request: {error}")))?;
        let value = crate::conversion::raw_keys(value, false).map_err(invalid)?;
        let request: refkit_core::MergeRequest = serde_json::from_value(value)
            .map_err(|error| invalid(format!("Invalid merge request: {error}")))?;
        let plan = self
            .inner
            .plan_merge(&request)
            .map_err(crate::errors::merge_error)?;
        Ok(crate::conversion::raw_keys(json!(plan), true)
            .map_err(|message| error("RefkitError", message))?
            .to_string())
    }

    /// Apply JSON edits atomically and return a new snapshot with its change report.
    ///
    /// # Errors
    /// Rejects malformed edits, missing targets, overlaps, invalid values, ambiguous references, and resource limits.
    pub fn apply_patch(&self, source: &str) -> Result<NativePatchResult, JsValue> {
        let invalid = |message: String| {
            crate::errors::patch_error(refkit_core::BibPatchError {
                code: refkit_core::BibPatchErrorCode::InvalidValue,
                operation: None,
                message,
            })
        };
        let value: Value = serde_json::from_str(source)
            .map_err(|error| invalid(format!("Invalid patch input: {error}")))?;
        let value = crate::conversion::raw_keys(value, false).map_err(invalid)?;
        let operations: Vec<refkit_core::BibEdit> = serde_json::from_value(value)
            .map_err(|error| invalid(format!("Invalid patch input: {error}")))?;
        let result = self
            .inner
            .apply_patch(&operations)
            .map_err(crate::errors::patch_error)?;
        let report = json!({
            "changes": result.changes.iter().map(|change| json!({"operations": change.operations, "kind": change.kind, "before": [change.before.start, change.before.end], "after": [change.after.start, change.after.end]})).collect::<Vec<_>>(),
            "entries": result.entries.iter().map(|mapping| json!({"before": mapping.before.as_ref().map(entry), "after": mapping.after.as_ref().map(entry), "fields": mapping.fields.iter().map(|mapping| json!({"before": mapping.before.as_ref().map(field), "after": mapping.after.as_ref().map(field)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "warnings": result.warnings.iter().map(|warning| json!({"code": warning.code, "entryId": warning.entry_id, "fieldId": warning.field_id, "message": warning.message})).collect::<Vec<_>>()
        }).to_string();
        Ok(NativePatchResult {
            document: Self::from_core(result.document),
            report,
        })
    }

    #[must_use]
    /// Return the number of parsed entry occurrences.
    pub fn entry_count(&self) -> usize {
        self.inner.entry_count()
    }

    #[must_use]
    /// Check whether the exact key occurs in this snapshot.
    pub fn contains_entry(&self, key: &str) -> bool {
        self.inner.contains_entry(key)
    }

    /// Check whether a case-insensitive field name occurs in an entry.
    ///
    /// # Errors
    /// Rejects an entry index absent from this snapshot.
    pub fn contains_field(&self, entry_id: usize, key: &str) -> Result<bool, JsValue> {
        Ok(self.inner.contains_field(self.entry_id(entry_id)?, key))
    }

    /// Return inspect-only source-profile validation findings as JSON.
    ///
    /// # Errors
    /// Returns parser diagnostics when source cannot be resolved or normalized safely.
    pub fn validate(&self) -> Result<String, JsValue> {
        let report = self.inner.validate().map_err(parse_error)?;
        Ok(crate::conversion::validation(&report).to_string())
    }

    /// Return the field occurrence count for a document-local entry index.
    ///
    /// # Errors
    /// Rejects an entry index absent from this snapshot.
    pub fn field_count(&self, entry_id: usize) -> Result<usize, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(self.inner.field_count(id).unwrap_or_default())
    }

    /// Return source-ordered entry occurrences as JSON.
    pub fn entries(&self) -> String {
        json!(
            self.inner
                .entry_occurrences()
                .iter()
                .map(entry)
                .collect::<Vec<_>>()
        )
        .to_string()
    }

    #[must_use]
    /// Return source-ordered entry keys, including duplicates, as JSON.
    pub fn entry_keys(&self) -> String {
        json!(self.inner.entry_keys()).to_string()
    }

    /// Return every occurrence of the requested key as JSON.
    pub fn entries_for_key(&self, key: &str) -> String {
        json!(
            self.inner
                .entries_for_key(key)
                .iter()
                .map(entry)
                .collect::<Vec<_>>()
        )
        .to_string()
    }

    /// Return the unique matching entry as JSON, or JSON null when absent.
    ///
    /// # Errors
    /// Rejects keys with multiple occurrences.
    pub fn unique_entry(&self, key: &str) -> Result<String, JsValue> {
        let id = self.inner.unique_entry(key).map_err(|value| {
            error(
                "RefkitError",
                value.replace(
                    "; use entries.get_all(key) or entries.occurrences()",
                    ". Use entries.getAll(key) or entries.occurrences()",
                ),
            )
        })?;
        Ok(json!(
            id.and_then(|id| self.inner.entry_info(id))
                .as_ref()
                .map(entry)
        )
        .to_string())
    }

    /// Return field names for a document-local entry index as JSON.
    ///
    /// # Errors
    /// Rejects an entry index absent from this snapshot.
    pub fn field_keys(&self, entry_id: usize) -> Result<String, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(json!(self.inner.field_keys(id).unwrap_or_default()).to_string())
    }

    /// Return matching field occurrences from one entry as JSON.
    ///
    /// # Errors
    /// Rejects an entry index absent from this snapshot.
    pub fn fields_for_key(&self, entry_id: usize, key: &str) -> Result<String, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(json!(
            self.inner
                .fields_for_key(id, key)
                .unwrap_or_default()
                .iter()
                .map(field)
                .collect::<Vec<_>>()
        )
        .to_string())
    }

    /// Return the unique matching field as JSON, or JSON null when absent.
    ///
    /// # Errors
    /// Rejects missing entry indices and duplicated field names.
    pub fn unique_field(&self, entry_id: usize, key: &str) -> Result<String, JsValue> {
        let entry_id = self.entry_id(entry_id)?;
        let field_id = self.inner.unique_field(entry_id, key).map_err(|value| {
            error(
                "RefkitError",
                value.replace(
                    "; use fields.get_all(key) or fields.occurrences()",
                    ". Use fields.getAll(key) or fields.occurrences()",
                ),
            )
        })?;
        Ok(json!(
            field_id
                .and_then(|field_id| self.inner.field_info(entry_id, field_id))
                .as_ref()
                .map(field)
        )
        .to_string())
    }

    /// Return all field occurrences in source order for one entry as JSON.
    ///
    /// # Errors
    /// Rejects an entry index absent from this snapshot.
    pub fn fields(&self, entry_id: usize) -> Result<String, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(json!(
            self.inner
                .field_occurrences(id)
                .unwrap_or_default()
                .iter()
                .map(field)
                .collect::<Vec<_>>()
        )
        .to_string())
    }

    #[must_use]
    /// Return source-ordered comment blocks as JSON.
    pub fn comments(&self) -> String {
        json!(self.inner.comments()).to_string()
    }

    #[must_use]
    /// Join source-ordered preamble values with BibTeX concatenation operators.
    pub fn preamble(&self) -> String {
        self.inner.preamble()
    }

    #[must_use]
    /// Return string definitions as JSON, retaining the last value for each name.
    pub fn strings(&self) -> String {
        let strings: serde_json::Map<String, Value> = self
            .inner
            .strings()
            .into_iter()
            .map(|(key, value)| (key, json!(value)))
            .collect();
        Value::Object(strings).to_string()
    }

    /// Return malformed source blocks as JSON.
    pub fn failed_blocks(&self) -> String {
        json!(
            self.inner
                .failed_blocks()
                .iter()
                .map(block)
                .collect::<Vec<_>>()
        )
        .to_string()
    }

    /// Return every source block in order as JSON.
    pub fn blocks(&self) -> String {
        json!(self.inner.blocks().iter().map(block).collect::<Vec<_>>()).to_string()
    }

    /// Render the source-preserving snapshot back to BibTeX.
    ///
    /// # Errors
    /// Returns a writeback error when source spans cannot be rendered.
    pub fn to_bibtex(&self) -> Result<String, JsValue> {
        self.inner
            .render()
            .map_err(|value| error("RefkitError", value))
    }

    /// Resolve macros into source field values and return entries as JSON.
    ///
    /// # Errors
    /// Returns diagnostics for malformed source, unresolved macros, cycles, and resource limits.
    pub fn resolve(&self) -> Result<String, JsValue> {
        let entries = self.inner.resolve().map_err(parse_error)?;
        Ok(json!(
            entries
                .iter()
                .map(|entry| json!({
                    "key": entry.key,
                    "entryType": entry.entry_type,
                    "fields": entry.fields,
                }))
                .collect::<Vec<_>>()
        )
        .to_string())
    }
}

impl NativeRawDocument {
    fn entry_id(&self, index: usize) -> Result<RawEntryId, JsValue> {
        self.inner
            .entry_id_at(index)
            .ok_or_else(|| error("MissingReferenceError", "raw entry is unavailable"))
    }
}

fn entry(value: &RawEntryInfo) -> Value {
    json!({"id": value.id.index(), "key": value.key, "kind": value.kind, "span": [value.span.start, value.span.end]})
}

fn field(value: &RawFieldInfo) -> Value {
    json!({"id": value.id.index(), "name": value.name, "value": value.value, "span": [value.span.start, value.span.end]})
}

#[expect(
    clippy::indexing_slicing,
    reason = "Every match arm constructs a JSON object before the common span property is inserted."
)]
fn block(value: &RawBlockInfo) -> Value {
    let (mut value, span) = match value {
        RawBlockInfo::Whitespace { span } => (json!({"kind": "whitespace"}), span),
        RawBlockInfo::Comment { raw, span } => (json!({"kind": "comment", "raw": raw}), span),
        RawBlockInfo::Preamble { value, span } => {
            (json!({"kind": "preamble", "value": value}), span)
        }
        RawBlockInfo::StringDef { key, value, span } => {
            (json!({"kind": "string", "key": key, "value": value}), span)
        }
        RawBlockInfo::Entry { id, key, span } => {
            (json!({"kind": "entry", "id": id.index(), "key": key}), span)
        }
        RawBlockInfo::Failed { raw, error, span } => {
            (json!({"kind": "failed", "raw": raw, "error": error}), span)
        }
        RawBlockInfo::Other { raw, span } => (json!({"kind": "other", "raw": raw}), span),
    };
    value["span"] = json!([span.start, span.end]);
    value
}
