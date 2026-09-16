use std::sync::Arc;

use refkit_core::{EntryField, EntryRecord, Library, RecoveryPolicy};
use serde_json::{Map, Value, json};
use wasm_bindgen::prelude::*;

use crate::conversion::{diagnostics, record, record_keys};
use crate::errors::{error, library_error};

#[wasm_bindgen]
/// An immutable normalized library shared by WebAssembly handles.
pub struct NativeLibrary {
    pub(crate) inner: Arc<Library>,
}

#[wasm_bindgen]
impl NativeLibrary {
    /// Construct a library from a JSON array of camelCase host records.
    ///
    /// # Errors
    /// Rejects invalid JSON, record shapes, duplicate keys, and resource-limit violations.
    pub fn from_records(source: &str) -> Result<NativeLibrary, JsValue> {
        refkit_core::validate_record_source(source).map_err(|value| error("RangeError", value))?;
        let mut decoder = serde_json::Deserializer::from_str(source);
        decoder.disable_recursion_limit();
        let input: Value = serde::Deserialize::deserialize(&mut decoder)
            .map_err(|value| error("RangeError", value))?;
        decoder.end().map_err(|value| error("RangeError", value))?;
        let records = record_keys(input, false)?.to_string();
        Library::from_records_json(&records)
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(library_error)
    }

    /// Restore a canonical versioned record snapshot.
    ///
    /// # Errors
    /// Rejects invalid JSON, unsupported schema versions, invalid records, and duplicate keys.
    pub fn from_json(source: &str) -> Result<NativeLibrary, JsValue> {
        Library::from_json(source)
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(library_error)
    }

    #[must_use]
    /// Serialize the library as a canonical versioned record snapshot.
    pub fn to_json(&self) -> String {
        self.inner.to_json()
    }

    #[must_use]
    /// Return inspect-only record validation findings as JSON.
    pub fn validate(&self) -> String {
        crate::conversion::validation(&self.inner.validate()).to_string()
    }

    /// Parse BibTeX or BibLaTeX using the selected recovery policy.
    ///
    /// # Errors
    /// Rejects unknown policies, unrecoverable input, resource limits, and invalid normalized records.
    pub fn parse_bibtex(source: &str, recovery: &str) -> Result<NativeLibrary, JsValue> {
        let policy = match recovery {
            "error" => RecoveryPolicy::Error,
            "report" => RecoveryPolicy::Report,
            _ => return Err(error("RangeError", "recovery must be 'error' or 'report'")),
        };
        Library::parse_biblatex(source, policy)
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(library_error)
    }

    /// Parse Hayagriva YAML into normalized records.
    ///
    /// # Errors
    /// Rejects invalid YAML, resource limits, and invalid normalized records.
    pub fn parse_yaml(source: &str) -> Result<NativeLibrary, JsValue> {
        Library::parse_hayagriva_yaml(source)
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(library_error)
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    /// Return the number of normalized records.
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    /// Check whether a record has this exact key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    #[must_use]
    /// Return record keys in library order as a JSON array.
    pub fn keys(&self) -> String {
        json!(self.inner.keys()).to_string()
    }

    /// Return complete camelCase records in library order as a JSON array.
    pub fn records(&self) -> String {
        json!(self.inner.records().iter().map(record).collect::<Vec<_>>()).to_string()
    }

    /// Return one camelCase record as JSON, or JSON null when the key is absent.
    pub fn get_record(&self, key: &str) -> String {
        json!(self.inner.get_record(key).map(record)).to_string()
    }

    /// Return records matching a key or supported selector as JSON.
    ///
    /// # Errors
    /// Rejects invalid selectors and missing selected references.
    pub fn select_records(&self, selector: &str) -> Result<String, JsValue> {
        let records = self.inner.select_records(selector).map_err(library_error)?;
        Ok(json!(records.iter().map(record).collect::<Vec<_>>()).to_string())
    }

    #[must_use]
    /// Return parser and recovery diagnostics as JSON.
    pub fn diagnostics(&self) -> String {
        diagnostics(self.inner.diagnostics()).to_string()
    }

    /// Return records in the order of a JSON array of keys.
    ///
    /// # Errors
    /// Rejects invalid key arrays and missing references.
    pub fn get_many(&self, keys: &str) -> Result<String, JsValue> {
        let keys: Vec<String> =
            serde_json::from_str(keys).map_err(|value| error("TypeError", value))?;
        Ok(json!(
            self.records_for_keys(&keys)?
                .into_iter()
                .map(record)
                .collect::<Vec<_>>()
        )
        .to_string())
    }

    /// Return scalar record projections as JSON, with optional JSON field and key arrays.
    ///
    /// # Errors
    /// Rejects malformed arrays, unsupported projection fields, and missing references.
    pub fn project(&self, fields: Option<String>, keys: Option<String>) -> Result<String, JsValue> {
        let fields: Vec<String> = match fields {
            Some(fields) => serde_json::from_str::<Option<Vec<String>>>(&fields)
                .map_err(|value| error("TypeError", value))?,
            None => None,
        }
        .unwrap_or_else(|| {
            ["key", "title", "doi", "volume"]
                .map(str::to_string)
                .to_vec()
        });
        let fields = fields
            .into_iter()
            .map(|name| {
                let field: EntryField = (if name == "entryType" {
                    "entry_type"
                } else {
                    &name
                })
                .parse()
                .map_err(|value| error("RangeError", value))?;
                Ok((name, field))
            })
            .collect::<Result<Vec<_>, JsValue>>()?;
        let keys: Option<Vec<String>> = match keys {
            Some(keys) => serde_json::from_str(&keys).map_err(|value| error("TypeError", value))?,
            None => None,
        };
        let records = match keys {
            Some(keys) => self.records_for_keys(&keys)?,
            None => self.inner.records().iter().collect(),
        };
        let rows = records
            .into_iter()
            .map(|record| {
                fields
                    .iter()
                    .map(|(name, field)| (name.clone(), json!(record.field(*field))))
                    .collect::<Map<String, Value>>()
            })
            .collect::<Vec<_>>();
        Ok(json!(rows).to_string())
    }
}

impl NativeLibrary {
    fn records_for_keys(&self, keys: &[String]) -> Result<Vec<&EntryRecord>, JsValue> {
        keys.iter()
            .map(|key| {
                self.inner.get_record(key).ok_or_else(|| {
                    error(
                        "MissingReferenceError",
                        format!("missing reference {key:?}"),
                    )
                })
            })
            .collect()
    }
}
