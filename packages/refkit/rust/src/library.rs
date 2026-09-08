use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::{PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyList, PyListMethods};

use refkit_core::Library as CoreLibrary;

use crate::conversion::{
    diagnostics_to_py, parse_project_fields_arg, parse_projection_keys, parse_recovery_policy,
    project_rows_to_py,
};
use crate::entry::Entry;
use crate::errors::{RefkitError, library_error_to_py};
use crate::filesystem::{LibraryFormat, read_library};

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Library {
    pub(crate) inner: Arc<CoreLibrary>,
    diagnostics: Arc<Vec<refkit_core::Diagnostic>>,
    py_keys: Arc<PyOnceLock<Py<PyList>>>,
}

impl Library {
    fn from_core(inner: CoreLibrary, diagnostic: Option<refkit_core::Diagnostic>) -> Self {
        let diagnostics = diagnostic
            .into_iter()
            .chain(inner.diagnostics().iter().cloned())
            .collect();
        Self {
            inner: Arc::new(inner),
            diagnostics: Arc::new(diagnostics),
            py_keys: Arc::new(PyOnceLock::new()),
        }
    }

    fn entry_for_key(&self, key: &str) -> Option<Entry> {
        self.inner.get_record(key).cloned().map(Entry::from_record)
    }
}

#[pymethods]
impl Library {
    #[staticmethod]
    #[pyo3(signature = (path, *, recovery = "error"))]
    fn read(py: Python<'_>, path: PathBuf, recovery: &str) -> PyResult<Self> {
        let recovery = parse_recovery_policy(recovery)?;
        let source = py
            .detach(move || read_library(&path))
            .map_err(RefkitError::new_err)?;
        let library = py
            .detach(|| match source.format {
                LibraryFormat::Biblatex => CoreLibrary::parse_biblatex(&source.text, recovery),
                LibraryFormat::HayagrivaYaml => CoreLibrary::parse_hayagriva_yaml(&source.text),
            })
            .map_err(|error| library_error_to_py(py, error))?;
        Ok(Self::from_core(library, source.diagnostic))
    }

    #[staticmethod]
    #[pyo3(signature = (source, *, recovery = "error"))]
    fn parse_bibtex(py: Python<'_>, source: String, recovery: &str) -> PyResult<Self> {
        let recovery = parse_recovery_policy(recovery)?;
        let library = py.detach(move || CoreLibrary::parse_biblatex(&source, recovery));
        library
            .map(|library| Self::from_core(library, None))
            .map_err(|error| library_error_to_py(py, error))
    }

    #[staticmethod]
    fn parse_yaml(py: Python<'_>, source: String) -> PyResult<Self> {
        let library = py.detach(move || CoreLibrary::parse_hayagriva_yaml(&source));
        library
            .map(|library| Self::from_core(library, None))
            .map_err(|error| library_error_to_py(py, error))
    }

    #[getter]
    fn diagnostics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        diagnostics_to_py(py, &self.diagnostics)
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
        let rows = PyList::empty(py);
        let iter = keys
            .try_iter()
            .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
        for key in iter {
            let key = key?;
            let key = key.extract::<&str>()?;
            let entry = self
                .entry_for_key(key)
                .ok_or_else(|| PyKeyError::new_err(key.to_string()))?;
            rows.append(Py::new(py, entry)?)?;
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
        match keys {
            Some(keys) => project_rows_to_py(
                py,
                &fields,
                keys.iter().filter_map(|key| self.inner.get_record(key)),
            ),
            None => project_rows_to_py(py, &fields, self.inner.records()),
        }
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
