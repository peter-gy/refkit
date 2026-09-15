use refkit_core::{RawBlockInfo, RawDocument, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo};
use serde_json::{Value, json};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::errors::{error, parse_error};

#[wasm_bindgen]
#[derive(Clone)]
pub struct NativeRawDocument {
    inner: Arc<RawDocument>,
    ids: Arc<Vec<(RawEntryId, Vec<RawFieldId>)>>,
}

impl NativeRawDocument {
    fn from_core(inner: RawDocument) -> Self {
        let ids = inner
            .entry_occurrences()
            .iter()
            .map(|entry| {
                (
                    entry.id,
                    inner
                        .field_occurrences(entry.id)
                        .unwrap_or_default()
                        .iter()
                        .map(|field| field.id)
                        .collect(),
                )
            })
            .collect();
        Self {
            inner: Arc::new(inner),
            ids: Arc::new(ids),
        }
    }
}

#[wasm_bindgen]
pub struct NativePatchResult {
    document: NativeRawDocument,
    report: String,
}

#[wasm_bindgen]
impl NativePatchResult {
    #[wasm_bindgen(getter)]
    pub fn document(&self) -> NativeRawDocument {
        self.document.clone()
    }
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

#[wasm_bindgen]
impl NativeRawDocument {
    pub fn parse(source: &str) -> NativeRawDocument {
        Self::from_core(RawDocument::parse(source))
    }

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
            .expect("owned report keys")
            .to_string())
    }

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
            .expect("owned plan keys")
            .to_string())
    }

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

    pub fn entry_count(&self) -> usize {
        self.inner.entry_count()
    }

    pub fn validate(&self) -> Result<String, JsValue> {
        let report = self.inner.validate().map_err(parse_error)?;
        Ok(crate::conversion::validation(&report).to_string())
    }

    pub fn field_count(&self, entry_id: usize) -> Result<usize, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(self.inner.field_count(id).unwrap_or_default())
    }

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

    pub fn entry_keys(&self) -> String {
        json!(self.inner.entry_keys()).to_string()
    }

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

    pub fn field_keys(&self, entry_id: usize) -> Result<String, JsValue> {
        let id = self.entry_id(entry_id)?;
        Ok(json!(self.inner.field_keys(id).unwrap_or_default()).to_string())
    }

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

    pub fn field(&self, entry_id: usize, field_id: usize) -> Result<String, JsValue> {
        let (entry_id, field_id) = self.field_ids(entry_id, field_id)?;
        self.inner
            .field_info(entry_id, field_id)
            .map(|value| field(&value).to_string())
            .ok_or_else(|| error("MissingReferenceError", "raw field is unavailable"))
    }

    pub fn metadata(&self) -> String {
        let strings: serde_json::Map<String, Value> = self
            .inner
            .strings()
            .into_iter()
            .map(|(key, value)| (key, json!(value)))
            .collect();
        json!({"comments": self.inner.comments(), "preamble": self.inner.preamble(), "strings": strings,
            "failedBlocks": self.inner.failed_blocks().iter().map(block).collect::<Vec<_>>(),
            "blocks": self.inner.blocks().iter().map(block).collect::<Vec<_>>(), "diagnostics": []}).to_string()
    }

    pub fn to_bibtex(&self) -> Result<String, JsValue> {
        self.inner
            .render()
            .map_err(|value| error("RefkitError", value))
    }

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
        self.ids
            .get(index)
            .map(|(id, _)| *id)
            .ok_or_else(|| error("MissingReferenceError", "raw entry is unavailable"))
    }

    fn field_ids(&self, entry: usize, field: usize) -> Result<(RawEntryId, RawFieldId), JsValue> {
        let (entry_id, fields) = self
            .ids
            .get(entry)
            .ok_or_else(|| error("MissingReferenceError", "raw entry is unavailable"))?;
        fields
            .get(field)
            .map(|field_id| (*entry_id, *field_id))
            .ok_or_else(|| error("MissingReferenceError", "raw field is unavailable"))
    }
}

fn entry(value: &RawEntryInfo) -> Value {
    json!({"id": value.id.index(), "key": value.key, "kind": value.kind, "span": [value.span.start, value.span.end]})
}

fn field(value: &RawFieldInfo) -> Value {
    json!({"id": value.id.index(), "name": value.name, "value": value.value, "span": [value.span.start, value.span.end]})
}

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
