use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_json::{Value, json};

use crate::conversion::json_to_py;
use crate::errors::RefkitError;
use crate::tidy::{TidyOptions, TidyResult, tidy_error_to_py};
use refkit_core::{
    RawBlockInfo, RawDocument, RawEditError, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo,
    quoted, read_bibliography_text, tidy_bibtex as core_tidy_bibtex,
};

type SharedDocument = Rc<RefCell<RawDocument>>;

#[pyclass(module = "refkit_core", unsendable)]
pub struct BibDocument {
    doc: SharedDocument,
}

#[pymethods]
impl BibDocument {
    #[staticmethod]
    fn read(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        let read_path = path.clone();
        let parsed: Result<RawDocument, String> = py.detach(move || {
            let text = read_bibliography_text(&read_path)?;
            Ok(RawDocument::parse(&text.source))
        });
        let data = parsed.map_err(RefkitError::new_err)?;
        Ok(Self {
            doc: Rc::new(RefCell::new(data)),
        })
    }

    #[staticmethod]
    fn parse(py: Python<'_>, source: String) -> Self {
        let data = py.detach(move || RawDocument::parse(&source));
        Self {
            doc: Rc::new(RefCell::new(data)),
        }
    }

    #[getter]
    fn entries(&self) -> BibEntryMap {
        BibEntryMap {
            doc: Rc::clone(&self.doc),
        }
    }

    #[getter]
    fn comments(&self) -> Vec<String> {
        self.doc.borrow().comments()
    }

    #[getter]
    fn preamble(&self) -> String {
        self.doc.borrow().preamble()
    }

    #[getter]
    fn strings(&self) -> BTreeMap<String, String> {
        self.doc.borrow().strings().into_iter().collect()
    }

    #[getter]
    fn failed_blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self.doc.borrow().failed_blocks();
        let blocks = blocks.iter().map(raw_block_to_json).collect::<Vec<_>>();
        let payload =
            serde_json::to_string(&blocks).map_err(|err| RefkitError::new_err(err.to_string()))?;
        json_to_py(py, &payload)
    }

    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self.doc.borrow().blocks();
        let blocks = blocks.iter().map(raw_block_to_json).collect::<Vec<_>>();
        let payload =
            serde_json::to_string(&blocks).map_err(|err| RefkitError::new_err(err.to_string()))?;
        json_to_py(py, &payload)
    }

    fn write(&self, py: Python<'_>, path: PathBuf) -> PyResult<()> {
        let data = self.doc.borrow().clone();

        py.detach(move || {
            let rendered = render_document(&data)?;
            fs::write(&path, rendered)
                .map_err(|err| RefkitError::new_err(format!("failed to write BibTeX: {err}")))?;
            Ok(())
        })
    }

    fn to_bibtex(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.doc.borrow().clone();
        py.detach(move || render_document(&data))
    }

    #[pyo3(signature = (*, options = None))]
    fn tidy(
        &self,
        py: Python<'_>,
        options: Option<PyRef<'_, TidyOptions>>,
    ) -> PyResult<TidyResult> {
        let data = self.doc.borrow().clone();
        let options = options
            .as_ref()
            .map(|options| options.inner())
            .unwrap_or_default();
        let rendered = py
            .detach(move || render_document_text(&data))
            .map_err(RefkitError::new_err)?;
        let result = py
            .detach(move || core_tidy_bibtex(&rendered, options))
            .map_err(|err| tidy_error_to_py(py, err))?;
        Ok(TidyResult::from_core(result))
    }

    fn __repr__(&self) -> String {
        let doc = self.doc.borrow();
        format!(
            "BibDocument({} entries, {} blocks)",
            doc.entry_count(),
            doc.block_count()
        )
    }
}

#[pyclass(module = "refkit_core", unsendable)]
pub struct BibEntryMap {
    doc: SharedDocument,
}

#[pymethods]
impl BibEntryMap {
    fn unique_keys(&self) -> Vec<String> {
        self.doc.borrow().entry_keys()
    }

    fn occurrence_keys(&self) -> Vec<String> {
        self.doc
            .borrow()
            .entry_occurrences()
            .into_iter()
            .map(|entry| entry.key)
            .collect()
    }

    fn occurrences(&self) -> Vec<BibEntry> {
        self.doc
            .borrow()
            .entry_occurrences()
            .into_iter()
            .map(|entry| BibEntry {
                doc: Rc::clone(&self.doc),
                entry_id: entry.id,
                key: entry.key.clone(),
            })
            .collect()
    }

    fn get_all(&self, key: &str) -> Vec<BibEntry> {
        let doc = self.doc.borrow();
        doc.entries_for_key(key)
            .into_iter()
            .map(|entry| BibEntry {
                doc: Rc::clone(&self.doc),
                entry_id: entry.id,
                key: entry.key,
            })
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.doc.borrow().entry_count() == 0
    }

    fn get_unique(&self, key: &str) -> PyResult<Option<BibEntry>> {
        let entry_id = {
            let doc = self.doc.borrow();
            unique_entry_id(&doc, key)?
        };
        Ok(entry_id.map(|entry_id| BibEntry {
            doc: Rc::clone(&self.doc),
            entry_id,
            key: key.to_string(),
        }))
    }

    fn __len__(&self) -> usize {
        self.doc.borrow().entry_count()
    }

    fn __bool__(&self) -> bool {
        self.doc.borrow().entry_count() != 0
    }

    fn __contains__(&self, key: &str) -> bool {
        self.doc.borrow().contains_entry(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibEntry> {
        let entry_id = {
            let doc = self.doc.borrow();
            unique_entry_id(&doc, key)?
        };
        if let Some(entry_id) = entry_id {
            Ok(BibEntry {
                doc: Rc::clone(&self.doc),
                entry_id,
                key: key.to_string(),
            })
        } else {
            Err(PyKeyError::new_err(key.to_string()))
        }
    }
}

#[pyclass(module = "refkit_core", unsendable)]
pub struct BibEntry {
    doc: SharedDocument,
    entry_id: RawEntryId,
    key: String,
}

#[pymethods]
impl BibEntry {
    #[getter]
    fn key(&self) -> String {
        self.key.clone()
    }

    #[getter]
    fn kind(&self) -> PyResult<String> {
        self.with_entry(|entry| entry.kind.clone())
    }

    #[getter]
    fn fields(&self) -> BibFieldMap {
        BibFieldMap {
            doc: Rc::clone(&self.doc),
            entry_id: self.entry_id,
            entry_key: self.key.clone(),
        }
    }

    #[getter]
    fn span(&self) -> PyResult<(usize, usize)> {
        self.with_entry(|entry| (entry.span.start, entry.span.end))
    }

    fn __repr__(&self) -> PyResult<String> {
        self.with_entry(|entry| {
            format!(
                "BibEntry(key={}, kind={})",
                quoted(&entry.key),
                quoted(&entry.kind)
            )
        })
    }
}

impl BibEntry {
    fn with_entry<T>(&self, f: impl FnOnce(RawEntryInfo) -> T) -> PyResult<T> {
        let doc = self.doc.borrow();
        doc.entry_info(self.entry_id)
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.key.clone()))
    }
}

#[pyclass(module = "refkit_core", unsendable)]
pub struct BibFieldMap {
    doc: SharedDocument,
    entry_id: RawEntryId,
    entry_key: String,
}

#[pymethods]
impl BibFieldMap {
    fn unique_keys(&self) -> PyResult<Vec<String>> {
        self.with_fields(|doc| doc.field_keys(self.entry_id))
    }

    fn occurrence_keys(&self) -> PyResult<Vec<String>> {
        self.with_fields(|doc| doc.field_occurrences(self.entry_id))
            .map(|fields| fields.into_iter().map(|field| field.name).collect())
    }

    fn occurrences(&self) -> PyResult<Vec<BibField>> {
        self.with_fields(|doc| doc.field_occurrences(self.entry_id))
            .map(|fields| {
                fields
                    .into_iter()
                    .map(|field| BibField {
                        doc: Rc::clone(&self.doc),
                        entry_id: self.entry_id,
                        field_id: field.id,
                        field_key: field.name.clone(),
                    })
                    .collect()
            })
    }

    fn get_all(&self, key: &str) -> PyResult<Vec<BibField>> {
        let key = key.to_ascii_lowercase();
        self.with_fields(|doc| doc.fields_for_key(self.entry_id, &key))
            .map(|fields| {
                fields
                    .into_iter()
                    .map(|field| BibField {
                        doc: Rc::clone(&self.doc),
                        entry_id: self.entry_id,
                        field_id: field.id,
                        field_key: field.name.clone(),
                    })
                    .collect()
            })
    }

    fn is_empty(&self) -> PyResult<bool> {
        self.with_fields(|doc| doc.field_keys(self.entry_id))
            .map(|fields| fields.is_empty())
    }

    fn get_unique(&self, key: &str) -> PyResult<Option<BibField>> {
        let key = key.to_ascii_lowercase();
        let field_id = self.unique_field(&key)?;
        Ok(field_id.map(|field_id| BibField {
            doc: Rc::clone(&self.doc),
            entry_id: self.entry_id,
            field_id,
            field_key: key,
        }))
    }

    fn __len__(&self) -> PyResult<usize> {
        self.with_fields(|doc| doc.field_keys(self.entry_id))
            .map(|fields| fields.len())
    }

    fn __bool__(&self) -> PyResult<bool> {
        self.with_fields(|doc| doc.field_keys(self.entry_id))
            .map(|fields| !fields.is_empty())
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        self.with_fields(|doc| Some(doc.contains_field(self.entry_id, key)))
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibField> {
        let key = key.to_ascii_lowercase();
        let field_id = self.unique_field(&key)?;
        if let Some(field_id) = field_id {
            Ok(BibField {
                doc: Rc::clone(&self.doc),
                entry_id: self.entry_id,
                field_id,
                field_key: key,
            })
        } else {
            Err(PyKeyError::new_err(key))
        }
    }
}

impl BibFieldMap {
    fn with_fields<T>(&self, f: impl FnOnce(&RawDocument) -> Option<T>) -> PyResult<T> {
        let doc = self.doc.borrow();
        f(&doc).ok_or_else(|| PyKeyError::new_err(self.entry_key.clone()))
    }

    fn unique_field(&self, key: &str) -> PyResult<Option<RawFieldId>> {
        let doc = self.doc.borrow();
        if doc.entry_info(self.entry_id).is_none() {
            return Err(PyKeyError::new_err(self.entry_key.clone()));
        }
        doc.unique_field(self.entry_id, key)
            .map_err(RefkitError::new_err)
    }
}

#[pyclass(module = "refkit_core", unsendable)]
pub struct BibField {
    doc: SharedDocument,
    entry_id: RawEntryId,
    field_id: RawFieldId,
    field_key: String,
}

#[pymethods]
impl BibField {
    #[getter]
    fn name(&self) -> PyResult<String> {
        self.with_field(|field| field.name.clone())
    }

    #[getter]
    fn value(&self) -> PyResult<String> {
        self.with_field(|field| field.value.clone())
    }

    #[setter]
    fn set_value(&self, value: String) -> PyResult<()> {
        let mut doc = self.doc.borrow_mut();
        doc.set_field_value(self.entry_id, self.field_id, value)
            .map_err(|err| raw_edit_error_to_py(err, &self.field_key))
    }

    #[getter]
    fn span(&self) -> PyResult<(usize, usize)> {
        self.with_field(|field| (field.span.start, field.span.end))
    }

    fn __repr__(&self) -> PyResult<String> {
        self.with_field(|field| {
            format!(
                "BibField(name={}, value={})",
                quoted(&field.name),
                quoted(&field.value)
            )
        })
    }
}

impl BibField {
    fn with_field<T>(&self, f: impl FnOnce(RawFieldInfo) -> T) -> PyResult<T> {
        let doc = self.doc.borrow();
        doc.field_info(self.entry_id, self.field_id)
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.field_key.clone()))
    }
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<BibDocument>()?;
    module.add_class::<BibEntryMap>()?;
    module.add_class::<BibEntry>()?;
    module.add_class::<BibFieldMap>()?;
    module.add_class::<BibField>()?;
    Ok(())
}

fn unique_entry_id(doc: &RawDocument, key: &str) -> PyResult<Option<RawEntryId>> {
    doc.unique_entry(key).map_err(RefkitError::new_err)
}

fn render_document(data: &RawDocument) -> PyResult<String> {
    render_document_text(data).map_err(RefkitError::new_err)
}

fn render_document_text(data: &RawDocument) -> Result<String, String> {
    data.render()
}

fn raw_edit_error_to_py(err: RawEditError, field_key: &str) -> PyErr {
    match err {
        RawEditError::MissingField { .. } => PyKeyError::new_err(field_key.to_string()),
        RawEditError::InvalidValue(message) => PyValueError::new_err(message),
    }
}

fn raw_block_to_json(block: &RawBlockInfo) -> Value {
    match block {
        RawBlockInfo::Whitespace { span } => {
            json!({"kind": "whitespace", "span": [span.start, span.end]})
        }
        RawBlockInfo::Comment { raw, span } => {
            json!({"kind": "comment", "raw": raw, "span": [span.start, span.end]})
        }
        RawBlockInfo::Preamble { value, span } => {
            json!({"kind": "preamble", "value": value, "span": [span.start, span.end]})
        }
        RawBlockInfo::StringDef { key, value, span } => {
            json!({"kind": "string", "key": key, "value": value, "span": [span.start, span.end]})
        }
        RawBlockInfo::Entry { id, key, span } => {
            json!({"kind": "entry", "id": id.index(), "key": key, "span": [span.start, span.end]})
        }
        RawBlockInfo::Failed { raw, error, span } => {
            json!({"kind": "failed", "raw": raw, "error": error, "span": [span.start, span.end]})
        }
        RawBlockInfo::Other { raw, span } => {
            json!({"kind": "other", "raw": raw, "span": [span.start, span.end]})
        }
    }
}
