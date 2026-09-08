use std::sync::Arc;

use refkit_core::{EntryField, EntryRecord, Library, RecoveryPolicy};
use serde_json::{Map, Value, json};
use wasm_bindgen::prelude::*;

use crate::conversion::{diagnostics, record};
use crate::errors::{error, library_error};

#[wasm_bindgen]
pub struct NativeLibrary {
    pub(crate) inner: Arc<Library>,
}

#[wasm_bindgen]
impl NativeLibrary {
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

    pub fn parse_yaml(source: &str) -> Result<NativeLibrary, JsValue> {
        Library::parse_hayagriva_yaml(source)
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(library_error)
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn keys(&self) -> String {
        json!(self.inner.keys()).to_string()
    }

    pub fn records(&self) -> String {
        json!(self.inner.records().iter().map(record).collect::<Vec<_>>()).to_string()
    }

    pub fn get_record(&self, key: &str) -> String {
        json!(self.inner.get_record(key).map(record)).to_string()
    }

    pub fn select_records(&self, selector: &str) -> Result<String, JsValue> {
        let records = self.inner.select_records(selector).map_err(library_error)?;
        Ok(json!(records.iter().map(record).collect::<Vec<_>>()).to_string())
    }

    pub fn diagnostics(&self) -> String {
        diagnostics(self.inner.diagnostics()).to_string()
    }

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
