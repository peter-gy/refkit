use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::{PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyDict, PyDictMethods, PyList, PyListMethods};

use refkit_core::Library as CoreLibrary;

use crate::conversion::{
    parse_project_fields_arg, parse_projection_keys, parse_recovery_policy, project_rows_to_py,
};
use crate::entry::Entry;
use crate::errors::RefkitError;
use crate::filesystem::{LibraryFormat, read_library};

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Library {
    pub(crate) inner: Arc<CoreLibrary>,
    diagnostics: Arc<Vec<String>>,
    py_keys: Arc<PyOnceLock<Py<PyList>>>,
    py_entries: Arc<PyOnceLock<Py<PyDict>>>,
}

impl Library {
    fn from_core(inner: CoreLibrary, diagnostic: Option<String>) -> Self {
        let diagnostics = diagnostic
            .into_iter()
            .chain(inner.diagnostics().iter().cloned())
            .collect();
        Self {
            inner: Arc::new(inner),
            diagnostics: Arc::new(diagnostics),
            py_keys: Arc::new(PyOnceLock::new()),
            py_entries: Arc::new(PyOnceLock::new()),
        }
    }

    fn entry_for_key(&self, key: &str) -> Option<Entry> {
        self.inner.get_record(key).cloned().map(Entry::from_record)
    }

    fn py_entry_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let entries = self.py_entries.get_or_try_init(py, || {
            let dict = PyDict::new(py);
            for record in self.inner.records() {
                dict.set_item(
                    &record.key,
                    Py::new(py, Entry::from_record(record.clone()))?,
                )?;
            }
            Ok::<_, PyErr>(dict.unbind())
        })?;
        Ok(entries.bind(py).clone())
    }
}

#[pymethods]
impl Library {
    #[staticmethod]
    #[pyo3(signature = (path, *, recovery = "error"))]
    fn read(py: Python<'_>, path: PathBuf, recovery: &str) -> PyResult<Self> {
        let recovery = parse_recovery_policy(recovery)?;
        let parsed: Result<(CoreLibrary, Option<String>), String> = py.detach(move || {
            let source = read_library(&path)?;
            let library = match source.format {
                LibraryFormat::Biblatex => CoreLibrary::parse_biblatex(&source.text, recovery),
                LibraryFormat::HayagrivaYaml => CoreLibrary::parse_hayagriva_yaml(&source.text),
            }
            .map_err(|error| error.to_string())?;
            Ok((library, source.diagnostic))
        });
        parsed
            .map(|(library, diagnostic)| Self::from_core(library, diagnostic))
            .map_err(RefkitError::new_err)
    }

    #[staticmethod]
    #[pyo3(signature = (source, *, recovery = "error"))]
    fn parse_bibtex(py: Python<'_>, source: String, recovery: &str) -> PyResult<Self> {
        let recovery = parse_recovery_policy(recovery)?;
        let library = py.detach(move || CoreLibrary::parse_biblatex(&source, recovery));
        library
            .map(|library| Self::from_core(library, None))
            .map_err(|error| RefkitError::new_err(error.to_string()))
    }

    #[staticmethod]
    fn parse_yaml(py: Python<'_>, source: String) -> PyResult<Self> {
        let library = py.detach(move || CoreLibrary::parse_hayagriva_yaml(&source));
        library
            .map(|library| Self::from_core(library, None))
            .map_err(|error| RefkitError::new_err(error.to_string()))
    }

    #[getter]
    fn diagnostics(&self) -> Vec<String> {
        self.diagnostics.as_ref().clone()
    }

    fn keys(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let keys = self.py_keys.get_or_try_init(py, || {
            PyList::new(py, self.inner.keys().iter().map(String::as_str)).map(Bound::unbind)
        })?;
        keys.call_method0(py, "copy")
    }

    fn get_many(&self, py: Python<'_>, keys: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if keys.extract::<String>().is_ok() {
            return Err(PyTypeError::new_err(
                "keys must be an iterable of entry keys",
            ));
        }
        let entries = self.py_entry_dict(py)?;
        let rows = PyList::empty(py);
        let iter = keys
            .try_iter()
            .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
        for key in iter {
            let key = key?;
            let Some(entry) = entries.get_item(&key)? else {
                return Err(PyKeyError::new_err(key.str()?.to_string()));
            };
            rows.append(entry)?;
        }
        Ok(rows.into_any().unbind())
    }

    fn values(&self) -> Vec<Entry> {
        self.inner
            .records()
            .iter()
            .cloned()
            .map(Entry::from_record)
            .collect()
    }

    fn get(&self, key: &str) -> Option<Entry> {
        self.entry_for_key(key)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn select(&self, selector: &str) -> PyResult<Vec<Entry>> {
        Ok(self
            .inner
            .select_records(selector)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
            .into_iter()
            .map(Entry::from_record)
            .collect())
    }

    #[pyo3(signature = (fields = None, *, keys = None))]
    fn project(
        &self,
        py: Python<'_>,
        fields: Option<&Bound<'_, PyAny>>,
        keys: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let fields = parse_project_fields_arg(fields)?;
        let keys = parse_projection_keys(self.inner.as_ref(), keys)?;
        let records = match keys {
            Some(keys) => keys
                .iter()
                .filter_map(|key| self.inner.get_record(key).cloned())
                .collect(),
            None => self.inner.records().to_vec(),
        };
        project_rows_to_py(py, &fields, &records)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<Entry> {
        self.entry_for_key(key)
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Library({} entries)", self.inner.len())
    }
}
