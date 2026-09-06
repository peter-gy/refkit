use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyAny;

use refkit_core::{
    PreparedStyle, StyleError, is_bundled_locale, load_prepared_style, prepare_style_from_xml,
};

use crate::errors::{RefkitError, style_error_to_py};
use crate::filesystem::read_style;
use crate::repr::quoted;

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Style {
    id: String,
    pub(crate) data: Arc<PreparedStyle>,
}

#[pymethods]
impl Style {
    #[staticmethod]
    fn load(py: Python<'_>, name: String) -> PyResult<Self> {
        let id = name.clone();
        py.detach(move || load_prepared_style(&name))
            .map(|data| Self { id, data })
            .map_err(style_error_to_py)
    }

    #[staticmethod]
    fn from_xml(py: Python<'_>, xml: String) -> PyResult<Self> {
        py.detach(move || prepare_style_from_xml(&xml))
            .map(|style| Self {
                id: "xml".to_string(),
                data: Arc::new(style),
            })
            .map_err(style_error_to_py)
    }

    #[staticmethod]
    fn from_path(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        let id = path.display().to_string();
        let xml = py
            .detach(move || read_style(&path))
            .map_err(RefkitError::new_err)?;
        py.detach(move || prepare_style_from_xml(&xml))
            .map(|style| Self {
                id,
                data: Arc::new(style),
            })
            .map_err(style_error_to_py)
    }

    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    #[getter]
    fn title(&self) -> String {
        self.data.title().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "Style(id={}, title={})",
            quoted(&self.id),
            quoted(&self.title())
        )
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Locale {
    code: String,
}

#[pymethods]
impl Locale {
    #[staticmethod]
    fn load(py: Python<'_>, code: String) -> PyResult<Self> {
        let exists = py.detach({
            let code = code.clone();
            move || is_bundled_locale(&code)
        });
        exists.then(|| Self { code: code.clone() }).ok_or_else(|| {
            PyValueError::new_err(format!("unknown bundled locale {}", quoted(&code)))
        })
    }

    #[getter]
    fn code(&self) -> String {
        self.code.clone()
    }

    fn __repr__(&self) -> String {
        format!("Locale(code={})", quoted(&self.code))
    }
}

pub(crate) fn extract_locale(locale: Option<&Bound<'_, PyAny>>) -> PyResult<Option<String>> {
    let Some(locale) = locale else {
        return Ok(None);
    };
    if locale.is_none() {
        return Ok(None);
    }
    if let Ok(code) = locale.extract::<String>() {
        return Ok(Some(code));
    }
    if let Ok(locale) = locale.extract::<PyRef<'_, Locale>>() {
        return Ok(Some(locale.code.clone()));
    }
    Err(PyTypeError::new_err(
        "locale must be a string, Locale, or None",
    ))
}

#[allow(dead_code)]
fn _style_error_type(_: StyleError) {}
