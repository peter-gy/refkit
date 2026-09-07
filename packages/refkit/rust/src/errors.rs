use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;

use refkit_core::{DocumentError, StyleError};

use crate::repr::quoted;

create_exception!(refkit, RefkitError, PyException);
create_exception!(refkit, ParseError, RefkitError);
create_exception!(refkit, MissingReferenceError, RefkitError);
create_exception!(refkit, TidyError, RefkitError);
create_exception!(refkit, TidySyntaxError, TidyError);

pub(crate) fn document_error_to_py(err: DocumentError) -> PyErr {
    match err {
        DocumentError::EmptyCitation => {
            PyValueError::new_err("citation requires at least one item")
        }
        DocumentError::InvalidNoteNumber => {
            PyValueError::new_err("note_number must be between 1 and 4294967295")
        }
        DocumentError::MissingReference(key) => {
            MissingReferenceError::new_err(format!("missing reference {key}"))
        }
        DocumentError::UnknownLocatorLabel(label) => {
            PyValueError::new_err(format!("unknown locator label {}", quoted(&label)))
        }
        DocumentError::Render(message) => RefkitError::new_err(message),
    }
}

pub(crate) fn style_error_to_py(err: StyleError) -> PyErr {
    match err {
        StyleError::InvalidMacro(message) => {
            PyValueError::new_err(format!("invalid CSL macro graph: {message}"))
        }
        StyleError::InvalidXml(message) => {
            PyValueError::new_err(format!("invalid CSL XML: {message}"))
        }
        StyleError::DependentStyle(_) => {
            PyValueError::new_err("dependent CSL styles need explicit parent resolution")
        }
        StyleError::UnknownBundledStyle(name) => {
            PyValueError::new_err(format!("unknown bundled style {}", quoted(&name)))
        }
        StyleError::CachePoisoned => RefkitError::new_err(err.to_string()),
    }
}

pub(crate) fn library_error_to_py(py: Python<'_>, error: refkit_core::LibraryError) -> PyErr {
    let exception = ParseError::new_err(error.to_string());
    let diagnostics = match &error {
        refkit_core::LibraryError::Biblatex(failure)
        | refkit_core::LibraryError::HayagrivaYaml(failure) => failure.diagnostics.as_slice(),
        _ => &[],
    };
    match crate::conversion::diagnostics_to_py(py, diagnostics)
        .and_then(|diagnostics| exception.value(py).setattr("diagnostics", diagnostics))
    {
        Ok(()) => exception,
        Err(error) => error,
    }
}
