use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule};

use refkit_core::{
    CoreLibrary, NormalizedEntry, NormalizedValue, ProjectField, parse_project_field,
};

pub(crate) fn parse_recovery_policy(recovery: &str) -> PyResult<(bool, bool)> {
    match recovery {
        "error" => Ok((true, true)),
        "report" => Ok((false, true)),
        _ => Err(PyValueError::new_err(
            "recovery must be 'error' or 'report'",
        )),
    }
}

pub(crate) fn default_project_fields() -> Vec<String> {
    ["key", "title", "doi", "volume"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn parse_project_fields_arg(
    fields: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<ProjectField>> {
    let Some(fields) = fields.filter(|fields| !fields.is_none()) else {
        return parse_project_fields(default_project_fields().iter().map(String::as_str));
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
        parsed.push(
            parse_project_field(field).map_err(|err| PyValueError::new_err(err.to_string()))?,
        );
    }
    Ok(parsed)
}

fn parse_project_fields<'a>(fields: impl Iterator<Item = &'a str>) -> PyResult<Vec<ProjectField>> {
    let mut parsed = Vec::new();
    for field in fields {
        parsed.push(
            parse_project_field(field).map_err(|err| PyValueError::new_err(err.to_string()))?,
        );
    }
    Ok(parsed)
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

pub(crate) fn project_rows_to_py(
    py: Python<'_>,
    fields: &[ProjectField],
    records: Vec<Vec<Option<String>>>,
) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for record in records {
        rows.append(project_row_to_py(py, fields, &record)?)?;
    }
    Ok(rows.into_any().unbind())
}

fn project_row_to_py(
    py: Python<'_>,
    fields: &[ProjectField],
    record: &[Option<String>],
) -> PyResult<Py<PyAny>> {
    let row = PyDict::new(py);
    for (field, value) in fields.iter().zip(record) {
        row.set_item(project_field_name(*field), value.as_deref())?;
    }
    Ok(row.into_any().unbind())
}

fn project_field_name(field: ProjectField) -> &'static str {
    match field {
        ProjectField::Key => "key",
        ProjectField::EntryType => "entry_type",
        ProjectField::Type => "type",
        ProjectField::Title => "title",
        ProjectField::Doi => "doi",
        ProjectField::Volume => "volume",
    }
}

pub(crate) fn normalized_entries_to_py(
    py: Python<'_>,
    entries: &[NormalizedEntry],
) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for entry in entries {
        rows.append(normalized_value_to_py(py, &entry.value)?)?;
    }
    Ok(rows.into_any().unbind())
}

fn normalized_value_to_py(py: Python<'_>, value: &NormalizedValue) -> PyResult<Py<PyAny>> {
    match value {
        NormalizedValue::Null => Ok(py.None()),
        NormalizedValue::Bool(value) => value.into_py_any(py),
        NormalizedValue::Number(value) => normalized_number_to_py(py, value),
        NormalizedValue::String(value) => value.clone().into_py_any(py),
        NormalizedValue::Array(values) => {
            let items = PyList::empty(py);
            for value in values {
                items.append(normalized_value_to_py(py, value)?)?;
            }
            Ok(items.into_any().unbind())
        }
        NormalizedValue::Object(values) => {
            let object = PyDict::new(py);
            for (key, value) in values {
                object.set_item(key, normalized_value_to_py(py, value)?)?;
            }
            Ok(object.into_any().unbind())
        }
    }
}

fn normalized_number_to_py(py: Python<'_>, value: &serde_json::Number) -> PyResult<Py<PyAny>> {
    if let Some(value) = value.as_i64() {
        value.into_py_any(py)
    } else if let Some(value) = value.as_u64() {
        value.into_py_any(py)
    } else if let Some(value) = value.as_f64() {
        value.into_py_any(py)
    } else {
        value.to_string().into_py_any(py)
    }
}

pub(crate) fn json_to_py(py: Python<'_>, value: &str) -> PyResult<Py<PyAny>> {
    let json = PyModule::import(py, "json")?;
    Ok(json.call_method1("loads", (value,))?.unbind())
}
