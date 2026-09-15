use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::conversion::diagnostics_to_py;
use crate::errors::{RefkitError, library_error_to_py};
use crate::filesystem::{read_bibtex, write_bibtex};
use crate::repr::quoted;
use crate::tidy::{TidyOptions, TidyResult, tidy_error_to_py};
use refkit_core::{
    RawBlockInfo, RawDocument, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo,
    tidy_bibtex as core_tidy_bibtex,
};

type SharedDocument = Arc<RawDocument>;

#[pyclass(module = "refkit")]
pub struct BibDocument {
    doc: SharedDocument,
    diagnostics: Vec<refkit_core::Diagnostic>,
}

#[pymethods]
impl BibDocument {
    #[pyo3(signature = (*, rules = None))]
    fn find_duplicates(
        &self,
        py: Python<'_>,
        rules: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let rules = match rules.filter(|value| !value.is_none()) {
            None => None,
            Some(value) => {
                if value.extract::<String>().is_ok() {
                    return Err(PyTypeError::new_err(
                        "rules must be an iterable of rule names",
                    ));
                }
                let list = PyList::new(py, value.try_iter()?.collect::<PyResult<Vec<_>>>()?)?;
                let json = PyModule::import(py, "json")?
                    .call_method1("dumps", (list,))?
                    .extract::<String>()?;
                Some(
                    serde_json::from_str::<Vec<refkit_core::DuplicateRule>>(&json).map_err(
                        |error| {
                            crate::errors::merge_error_to_py(
                                py,
                                refkit_core::MergeError {
                                    code: refkit_core::MergeErrorCode::InvalidSelection,
                                    message: format!("Invalid duplicate rules: {error}"),
                                },
                            )
                        },
                    )?,
                )
            }
        };
        let document = Arc::clone(&self.doc);
        let report = py
            .detach(move || document.find_duplicates(rules.as_deref()))
            .map_err(|error| crate::errors::merge_error_to_py(py, error))?;
        crate::conversion::json_to_py(
            py,
            &serde_json::to_string(&report)
                .map_err(|error| crate::errors::RefkitError::new_err(error.to_string()))?,
        )
    }

    #[pyo3(signature = (entries, *, retain, fields = None, entry_type = None))]
    fn plan_merge(
        &self,
        py: Python<'_>,
        entries: &Bound<'_, PyAny>,
        retain: &Bound<'_, PyAny>,
        fields: Option<&Bound<'_, PyAny>>,
        entry_type: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        if entries.extract::<String>().is_ok() {
            return Err(PyTypeError::new_err(
                "entries must be an iterable of occurrence IDs",
            ));
        }
        let request = PyDict::new(py);
        request.set_item(
            "entries",
            PyList::new(py, entries.try_iter()?.collect::<PyResult<Vec<_>>>()?)?,
        )?;
        request.set_item("retain", retain)?;
        request.set_item("entry_type", entry_type)?;
        let fields = match fields.filter(|value| !value.is_none()) {
            None => PyList::empty(py),
            Some(fields) => {
                if fields.extract::<String>().is_ok() {
                    return Err(PyTypeError::new_err(
                        "fields must be an iterable of merge choices",
                    ));
                }
                PyList::new(py, fields.try_iter()?.collect::<PyResult<Vec<_>>>()?)?
            }
        };
        request.set_item("fields", fields)?;
        let json = PyModule::import(py, "json")?
            .call_method1("dumps", (request,))?
            .extract::<String>()?;
        let request: refkit_core::MergeRequest = serde_json::from_str(&json).map_err(|error| {
            crate::errors::merge_error_to_py(
                py,
                refkit_core::MergeError {
                    code: refkit_core::MergeErrorCode::InvalidSelection,
                    message: format!("Invalid merge request: {error}"),
                },
            )
        })?;
        let document = Arc::clone(&self.doc);
        let plan = py
            .detach(move || document.plan_merge(&request))
            .map_err(|error| crate::errors::merge_error_to_py(py, error))?;
        crate::conversion::json_to_py(
            py,
            &serde_json::to_string(&plan)
                .map_err(|error| crate::errors::RefkitError::new_err(error.to_string()))?,
        )
    }

    fn apply_patch(&self, py: Python<'_>, patch: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if patch.extract::<String>().is_ok() {
            return Err(PyTypeError::new_err(
                "patch must be an iterable of edit records",
            ));
        }
        let patch = PyList::new(py, patch.try_iter()?.collect::<PyResult<Vec<_>>>()?)?;
        let source = PyModule::import(py, "json")?
            .call_method1("dumps", (patch,))?
            .extract::<String>()?;
        let operations: Vec<refkit_core::BibEdit> =
            serde_json::from_str(&source).map_err(|error| {
                crate::errors::patch_error_to_py(
                    py,
                    refkit_core::BibPatchError {
                        code: refkit_core::BibPatchErrorCode::InvalidValue,
                        operation: None,
                        message: format!("Invalid patch input: {error}"),
                    },
                )
            })?;
        let doc = Arc::clone(&self.doc);
        let result = py
            .detach(move || doc.apply_patch(&operations))
            .map_err(|error| crate::errors::patch_error_to_py(py, error))?;
        patch_result_to_py(py, result)
    }

    #[staticmethod]
    fn read(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        let (data, diagnostic) = py
            .detach(move || {
                read_bibtex(&path)
                    .map(|(source, diagnostic)| (RawDocument::parse(&source), diagnostic))
            })
            .map_err(RefkitError::new_err)?;
        Ok(Self {
            doc: Arc::new(data),
            diagnostics: diagnostic.into_iter().collect(),
        })
    }

    #[staticmethod]
    fn parse(py: Python<'_>, source: String) -> Self {
        let data = py.detach(move || RawDocument::parse(&source));
        Self {
            doc: Arc::new(data),
            diagnostics: Vec::new(),
        }
    }

    #[getter]
    fn diagnostics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        diagnostics_to_py(py, &self.diagnostics)
    }

    #[getter]
    fn entries(&self) -> BibEntryMap {
        BibEntryMap {
            doc: Arc::clone(&self.doc),
        }
    }

    #[getter]
    fn comments(&self) -> Vec<String> {
        self.doc.as_ref().comments()
    }

    #[getter]
    fn preamble(&self) -> String {
        self.doc.as_ref().preamble()
    }

    #[getter]
    fn strings(&self) -> BTreeMap<String, String> {
        self.doc.as_ref().strings().into_iter().collect()
    }

    #[getter]
    fn failed_blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self.doc.as_ref().failed_blocks();
        raw_blocks_to_py(py, &blocks)
    }

    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self.doc.as_ref().blocks();
        raw_blocks_to_py(py, &blocks)
    }

    fn write(&self, py: Python<'_>, path: PathBuf) -> PyResult<()> {
        let data = Arc::clone(&self.doc);

        py.detach(move || {
            let rendered = render_document(&data)?;
            write_bibtex(&path, &rendered).map_err(RefkitError::new_err)
        })
    }

    fn to_bibtex(&self, py: Python<'_>) -> PyResult<String> {
        let data = Arc::clone(&self.doc);
        py.detach(move || render_document(&data))
    }

    fn validate(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data = Arc::clone(&self.doc);
        let report = py
            .detach(move || data.validate())
            .map_err(|error| library_error_to_py(py, refkit_core::LibraryError::Biblatex(error)))?;
        crate::conversion::validation_to_py(py, &report)
    }

    fn resolve(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data = Arc::clone(&self.doc);
        let entries = py
            .detach(move || data.resolve())
            .map_err(|error| library_error_to_py(py, refkit_core::LibraryError::Biblatex(error)))?;
        let values = PyList::empty(py);
        for entry in entries {
            let value = PyDict::new(py);
            value.set_item("key", entry.key)?;
            value.set_item("entry_type", entry.entry_type)?;
            value.set_item("fields", entry.fields)?;
            values.append(value)?;
        }
        Ok(values.into_any().unbind())
    }

    #[pyo3(signature = (*, options = None))]
    fn tidy(
        &self,
        py: Python<'_>,
        options: Option<PyRef<'_, TidyOptions>>,
    ) -> PyResult<TidyResult> {
        let data = Arc::clone(&self.doc);
        let options = options.map(|options| options.inner()).unwrap_or_default();
        let rendered = py
            .detach(move || render_document_text(&data))
            .map_err(RefkitError::new_err)?;
        let result = py
            .detach(move || core_tidy_bibtex(&rendered, options))
            .map_err(|err| tidy_error_to_py(py, err))?;
        Ok(TidyResult::from_core(result))
    }

    fn __repr__(&self) -> String {
        let doc = self.doc.as_ref();
        format!(
            "BibDocument({} entries, {} blocks)",
            doc.entry_count(),
            doc.block_count()
        )
    }
}

#[pyclass(module = "refkit")]
pub struct BibEntryMap {
    doc: SharedDocument,
}

#[pymethods]
impl BibEntryMap {
    fn unique_keys(&self) -> Vec<String> {
        self.doc.as_ref().entry_keys()
    }

    fn occurrence_keys(&self) -> Vec<String> {
        self.doc
            .as_ref()
            .entry_occurrences()
            .into_iter()
            .map(|entry| entry.key)
            .collect()
    }

    fn occurrences(&self) -> Vec<BibEntry> {
        self.doc
            .as_ref()
            .entry_occurrences()
            .into_iter()
            .map(|entry| BibEntry {
                doc: Arc::clone(&self.doc),
                entry_id: entry.id,
                key: entry.key,
            })
            .collect()
    }

    fn get_all(&self, key: &str) -> Vec<BibEntry> {
        let doc = self.doc.as_ref();
        doc.entries_for_key(key)
            .into_iter()
            .map(|entry| BibEntry {
                doc: Arc::clone(&self.doc),
                entry_id: entry.id,
                key: entry.key,
            })
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.doc.as_ref().entry_count() == 0
    }

    fn get_unique(&self, key: &str) -> PyResult<Option<BibEntry>> {
        let entry_id = {
            let doc = self.doc.as_ref();
            unique_entry_id(doc, key)?
        };
        Ok(entry_id.map(|entry_id| BibEntry {
            doc: Arc::clone(&self.doc),
            entry_id,
            key: key.to_string(),
        }))
    }

    fn __len__(&self) -> usize {
        self.doc.as_ref().entry_count()
    }

    fn __bool__(&self) -> bool {
        self.doc.as_ref().entry_count() != 0
    }

    fn __contains__(&self, key: &str) -> bool {
        self.doc.as_ref().contains_entry(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibEntry> {
        let entry_id = {
            let doc = self.doc.as_ref();
            unique_entry_id(doc, key)?
        };
        if let Some(entry_id) = entry_id {
            Ok(BibEntry {
                doc: Arc::clone(&self.doc),
                entry_id,
                key: key.to_string(),
            })
        } else {
            Err(PyKeyError::new_err(key.to_string()))
        }
    }
}

#[pyclass(module = "refkit")]
pub struct BibEntry {
    doc: SharedDocument,
    entry_id: RawEntryId,
    key: String,
}

#[pymethods]
impl BibEntry {
    #[getter]
    fn id(&self) -> usize {
        self.entry_id.index()
    }
    #[getter]
    fn key(&self) -> String {
        self.key.clone()
    }

    #[getter]
    fn kind(&self) -> PyResult<String> {
        self.with_entry(|entry| entry.kind)
    }

    #[getter]
    fn fields(&self) -> BibFieldMap {
        BibFieldMap {
            doc: Arc::clone(&self.doc),
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
        let doc = self.doc.as_ref();
        doc.entry_info(self.entry_id)
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.key.clone()))
    }
}

#[pyclass(module = "refkit")]
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
                        doc: Arc::clone(&self.doc),
                        entry_id: self.entry_id,
                        field_id: field.id,
                        field_key: field.name,
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
                        doc: Arc::clone(&self.doc),
                        entry_id: self.entry_id,
                        field_id: field.id,
                        field_key: field.name,
                    })
                    .collect()
            })
    }

    fn is_empty(&self) -> PyResult<bool> {
        self.with_fields(|doc| doc.field_count(self.entry_id))
            .map(|count| count == 0)
    }

    fn get_unique(&self, key: &str) -> PyResult<Option<BibField>> {
        let key = key.to_ascii_lowercase();
        let field_id = self.unique_field(&key)?;
        Ok(field_id.map(|field_id| BibField {
            doc: Arc::clone(&self.doc),
            entry_id: self.entry_id,
            field_id,
            field_key: key,
        }))
    }

    fn __len__(&self) -> PyResult<usize> {
        self.with_fields(|doc| doc.field_count(self.entry_id))
    }

    fn __bool__(&self) -> PyResult<bool> {
        self.with_fields(|doc| doc.field_count(self.entry_id))
            .map(|count| count != 0)
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        self.with_fields(|doc| Some(doc.contains_field(self.entry_id, key)))
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibField> {
        let key = key.to_ascii_lowercase();
        let field_id = self.unique_field(&key)?;
        if let Some(field_id) = field_id {
            Ok(BibField {
                doc: Arc::clone(&self.doc),
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
        let doc = self.doc.as_ref();
        f(doc).ok_or_else(|| PyKeyError::new_err(self.entry_key.clone()))
    }

    fn unique_field(&self, key: &str) -> PyResult<Option<RawFieldId>> {
        let doc = self.doc.as_ref();
        if doc.entry_info(self.entry_id).is_none() {
            return Err(PyKeyError::new_err(self.entry_key.clone()));
        }
        doc.unique_field(self.entry_id, key)
            .map_err(RefkitError::new_err)
    }
}

#[pyclass(module = "refkit")]
pub struct BibField {
    doc: SharedDocument,
    entry_id: RawEntryId,
    field_id: RawFieldId,
    field_key: String,
}

#[pymethods]
impl BibField {
    #[getter]
    fn id(&self) -> usize {
        self.field_id.index()
    }

    #[getter]
    fn entry_id(&self) -> usize {
        self.entry_id.index()
    }
    #[getter]
    fn name(&self) -> PyResult<String> {
        self.with_field(|field| field.name)
    }

    #[getter]
    fn value(&self) -> PyResult<String> {
        self.with_field(|field| field.value)
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
        let doc = self.doc.as_ref();
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

fn patch_result_to_py(py: Python<'_>, result: refkit_core::BibPatchResult) -> PyResult<Py<PyAny>> {
    fn entry(py: Python<'_>, info: Option<RawEntryInfo>) -> PyResult<Py<PyAny>> {
        let Some(info) = info else {
            return Ok(py.None());
        };
        let value = PyDict::new(py);
        value.set_item("id", info.id.index())?;
        value.set_item("key", info.key)?;
        value.set_item("kind", info.kind)?;
        value.set_item("span", (info.span.start, info.span.end))?;
        Ok(value.into_any().unbind())
    }
    fn field(py: Python<'_>, info: Option<RawFieldInfo>) -> PyResult<Py<PyAny>> {
        let Some(info) = info else {
            return Ok(py.None());
        };
        let value = PyDict::new(py);
        value.set_item("id", info.id.index())?;
        value.set_item("name", info.name)?;
        value.set_item("value", info.value)?;
        value.set_item("span", (info.span.start, info.span.end))?;
        Ok(value.into_any().unbind())
    }
    let output = PyDict::new(py);
    output.set_item(
        "document",
        Py::new(
            py,
            BibDocument {
                doc: Arc::new(result.document),
                diagnostics: Vec::new(),
            },
        )?,
    )?;
    let changes = PyList::empty(py);
    for change in result.changes {
        let value = PyDict::new(py);
        value.set_item("operations", change.operations)?;
        value.set_item(
            "kind",
            crate::conversion::json_to_py(
                py,
                &serde_json::to_string(&change.kind)
                    .map_err(|error| crate::errors::RefkitError::new_err(error.to_string()))?,
            )?,
        )?;
        value.set_item("before", (change.before.start, change.before.end))?;
        value.set_item("after", (change.after.start, change.after.end))?;
        changes.append(value)?;
    }
    output.set_item("changes", changes)?;
    let entries = PyList::empty(py);
    for mapping in result.entries {
        let value = PyDict::new(py);
        value.set_item("before", entry(py, mapping.before)?)?;
        value.set_item("after", entry(py, mapping.after)?)?;
        let fields = PyList::empty(py);
        for mapping in mapping.fields {
            let value = PyDict::new(py);
            value.set_item("before", field(py, mapping.before)?)?;
            value.set_item("after", field(py, mapping.after)?)?;
            fields.append(value)?;
        }
        value.set_item("fields", fields)?;
        entries.append(value)?;
    }
    output.set_item("entries", entries)?;
    output.set_item(
        "warnings",
        crate::conversion::json_to_py(
            py,
            &serde_json::to_string(&result.warnings)
                .map_err(|error| crate::errors::RefkitError::new_err(error.to_string()))?,
        )?,
    )?;
    Ok(output.into_any().unbind())
}

fn raw_blocks_to_py(py: Python<'_>, blocks: &[RawBlockInfo]) -> PyResult<Py<PyAny>> {
    let values = PyList::empty(py);
    for block in blocks {
        let value = PyDict::new(py);
        let (kind, span) = match block {
            RawBlockInfo::Whitespace { span } => ("whitespace", span),
            RawBlockInfo::Comment { raw, span } => {
                value.set_item("raw", raw)?;
                ("comment", span)
            }
            RawBlockInfo::Preamble {
                value: preamble,
                span,
            } => {
                value.set_item("value", preamble)?;
                ("preamble", span)
            }
            RawBlockInfo::StringDef {
                key,
                value: definition,
                span,
            } => {
                value.set_item("key", key)?;
                value.set_item("value", definition)?;
                ("string", span)
            }
            RawBlockInfo::Entry { id, key, span } => {
                value.set_item("id", id.index())?;
                value.set_item("key", key)?;
                ("entry", span)
            }
            RawBlockInfo::Failed { raw, error, span } => {
                value.set_item("raw", raw)?;
                value.set_item("error", error)?;
                ("failed", span)
            }
            RawBlockInfo::Other { raw, span } => {
                value.set_item("raw", raw)?;
                ("other", span)
            }
        };
        value.set_item("kind", kind)?;
        value.set_item("span", (span.start, span.end))?;
        values.append(value)?;
    }
    Ok(values.into_any().unbind())
}
