use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::conversion::{json_to_py, parse_recovery_policy};
use crate::errors::{RefkitError, codec_error_to_py};
use crate::library::Library;

#[pyfunction]
#[pyo3(signature = (source, *, format, loss = "report", recovery = "error"))]
fn decode(
    py: Python<'_>,
    source: String,
    format: &str,
    loss: &str,
    recovery: &str,
) -> PyResult<Py<PyAny>> {
    let format = format
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let loss = loss
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let recovery = parse_recovery_policy(recovery)?;
    let report = py
        .detach(move || refkit_core::decode(&source, format, loss, recovery))
        .map_err(|error| codec_error_to_py(py, error))?;
    let result = PyDict::new(py);
    result.set_item("format", report.format.as_str())?;
    result.set_item(
        "issues",
        json_to_py(
            py,
            &serde_json::to_string(&report.issues)
                .map_err(|error| RefkitError::new_err(error.to_string()))?,
        )?,
    )?;
    result.set_item(
        "library",
        Py::new(py, Library::from_core(report.library, None))?,
    )?;
    Ok(result.into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (library, *, format, loss = "report"))]
fn encode(
    py: Python<'_>,
    library: PyRef<'_, Library>,
    format: &str,
    loss: &str,
) -> PyResult<Py<PyAny>> {
    let format = format
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let loss = loss
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let library = library.inner.clone();
    let report = py
        .detach(move || refkit_core::encode(&library, format, loss))
        .map_err(|error| codec_error_to_py(py, error))?;
    json_to_py(
        py,
        &serde_json::to_string(&report).map_err(|error| RefkitError::new_err(error.to_string()))?,
    )
}

#[pyfunction]
#[pyo3(signature = (source, *, source_format, target_format, loss = "report", recovery = "error"))]
fn convert(
    py: Python<'_>,
    source: String,
    source_format: &str,
    target_format: &str,
    loss: &str,
    recovery: &str,
) -> PyResult<Py<PyAny>> {
    let source_format = source_format
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let target_format = target_format
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let loss = loss
        .parse()
        .map_err(|error: refkit_core::CodecError| PyValueError::new_err(error.to_string()))?;
    let recovery = parse_recovery_policy(recovery)?;
    let report = py
        .detach(move || refkit_core::convert(&source, source_format, target_format, loss, recovery))
        .map_err(|error| codec_error_to_py(py, error))?;
    let result = PyDict::new(py);
    result.set_item("source_format", report.source_format.as_str())?;
    result.set_item("target_format", report.target_format.as_str())?;
    result.set_item("text", report.text)?;
    result.set_item(
        "issues",
        json_to_py(
            py,
            &serde_json::to_string(&report.issues)
                .map_err(|error| RefkitError::new_err(error.to_string()))?,
        )?,
    )?;
    result.set_item(
        "diagnostics",
        crate::conversion::diagnostics_to_py(py, &report.diagnostics)?,
    )?;
    Ok(result.into_any().unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(decode, module)?)?;
    module.add_function(wrap_pyfunction!(encode, module)?)?;
    module.add_function(wrap_pyfunction!(convert, module)?)?;
    Ok(())
}
