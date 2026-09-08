use pyo3::exceptions::{PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule};

use refkit_core::{EntryField, EntryRecord, Library as CoreLibrary, RecoveryPolicy};

pub(crate) struct ProjectionField {
    name: &'static str,
    field: EntryField,
}

pub(crate) fn parse_recovery_policy(recovery: &str) -> PyResult<RecoveryPolicy> {
    match recovery {
        "error" => Ok(RecoveryPolicy::Error),
        "report" => Ok(RecoveryPolicy::Report),
        _ => Err(PyValueError::new_err(
            "recovery must be 'error' or 'report'",
        )),
    }
}

pub(crate) fn parse_project_fields_arg(
    fields: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<ProjectionField>> {
    let Some(fields) = fields.filter(|fields| !fields.is_none()) else {
        return ["key", "title", "doi", "volume"]
            .into_iter()
            .map(parse_projection_field)
            .collect();
    };
    if fields.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "fields must be an iterable of field names",
        ));
    }
    let iter = fields
        .try_iter()
        .map_err(|_| PyTypeError::new_err("fields must be an iterable of field names"))?;
    let mut parsed = Vec::new();
    for field in iter {
        let field = field?;
        let field = field.extract::<&str>()?;
        parsed.push(parse_projection_field(field)?);
    }
    Ok(parsed)
}

fn parse_projection_field(field: &str) -> PyResult<ProjectionField> {
    let parsed = field
        .parse()
        .map_err(|error: refkit_core::EntryFieldError| PyValueError::new_err(error.to_string()))?;
    let name = match field {
        "key" => "key",
        "entry_type" => "entry_type",
        "type" => "type",
        "title" => "title",
        "date" => "date",
        "doi" => "doi",
        "volume" => "volume",
        _ => unreachable!("EntryField accepted an unknown name"),
    };
    Ok(ProjectionField {
        name,
        field: parsed,
    })
}

pub(crate) fn parse_projection_keys(
    library: &CoreLibrary,
    keys: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<String>>> {
    let Some(keys) = keys.filter(|keys| !keys.is_none()) else {
        return Ok(None);
    };
    if keys.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "keys must be an iterable of entry keys",
        ));
    }
    let iter = keys
        .try_iter()
        .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
    let mut parsed = Vec::new();
    for key in iter {
        let key = key?;
        let key = key.extract::<&str>()?;
        if !library.contains_key(key) {
            return Err(PyKeyError::new_err(key.to_string()));
        }
        parsed.push(key.to_string());
    }
    Ok(Some(parsed))
}

pub(crate) fn project_rows_to_py<'a>(
    py: Python<'_>,
    fields: &[ProjectionField],
    records: impl IntoIterator<Item = &'a EntryRecord>,
) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for record in records {
        rows.append(project_row_to_py(py, fields, record)?)?;
    }
    Ok(rows.into_any().unbind())
}

fn project_row_to_py(
    py: Python<'_>,
    fields: &[ProjectionField],
    record: &EntryRecord,
) -> PyResult<Py<PyAny>> {
    let row = PyDict::new(py);
    for field in fields {
        row.set_item(field.name, record.field(field.field))?;
    }
    Ok(row.into_any().unbind())
}

pub(crate) fn json_to_py(py: Python<'_>, value: &str) -> PyResult<Py<PyAny>> {
    let json = PyModule::import(py, "json")?;
    Ok(json.call_method1("loads", (value,))?.unbind())
}

pub(crate) fn diagnostics_to_py(
    py: Python<'_>,
    diagnostics: &[refkit_core::Diagnostic],
) -> PyResult<Py<PyAny>> {
    let values = PyList::empty(py);
    for diagnostic in diagnostics {
        let value = PyDict::new(py);
        value.set_item("code", diagnostic.code)?;
        value.set_item("severity", diagnostic.severity.as_str())?;
        value.set_item("action", diagnostic.action.as_str())?;
        value.set_item(
            "span",
            diagnostic.span.as_ref().map(|span| (span.start, span.end)),
        )?;
        value.set_item("entry", &diagnostic.entry)?;
        value.set_item("field", &diagnostic.field)?;
        value.set_item("message", &diagnostic.message)?;
        values.append(value)?;
    }
    Ok(values.into_any().unbind())
}
