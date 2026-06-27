use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyAny;

use refkit_core::{
    PreparedStyle, StyleError, bundled_locales, load_prepared_style, prepare_style_from_xml, quoted,
};

use crate::errors::{RefkitError, style_error_to_py};

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Style {
    id: String,
    pub(crate) data: Arc<PreparedStyle>,
}

#[pymethods]
impl Style {
    #[staticmethod]
    fn load(name: &str) -> PyResult<Self> {
        cached_bundled_style(name)
    }

    #[staticmethod]
    fn from_xml(xml: &str) -> PyResult<Self> {
        prepare_style_from_xml(xml)
            .map(|style| Self {
                id: "xml".to_string(),
                data: Arc::new(style),
            })
            .map_err(style_error_to_py)
    }

    #[staticmethod]
    fn from_path(path: PathBuf) -> PyResult<Self> {
        let xml = fs::read_to_string(&path)
            .map_err(|err| RefkitError::new_err(format!("failed to read style: {err}")))?;
        prepare_style_from_xml(&xml)
            .map(|style| Self {
                id: path.display().to_string(),
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

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Locale {
    code: String,
}

#[pymethods]
impl Locale {
    #[staticmethod]
    fn load(code: &str) -> PyResult<Self> {
        bundled_locales()
            .iter()
            .find(|locale| locale.lang.as_ref().is_some_and(|lang| lang.0 == code))
            .map(|inner| Self {
                code: inner
                    .lang
                    .as_ref()
                    .map(|lang| lang.0.clone())
                    .unwrap_or_else(|| code.to_string()),
            })
            .ok_or_else(|| {
                PyValueError::new_err(format!("unknown bundled locale {}", quoted(code)))
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

pub(crate) fn cached_bundled_style(name: &str) -> PyResult<Style> {
    load_prepared_style(name)
        .map(|style| Style {
            id: name.to_string(),
            data: style,
        })
        .map_err(style_error_to_py)
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
