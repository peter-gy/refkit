use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;

use refkit_core::{DocumentError, StyleError, quoted};

create_exception!(refkit_core, RefkitError, PyException);
create_exception!(refkit_core, MissingReferenceError, RefkitError);
create_exception!(refkit_core, TidyError, RefkitError);
create_exception!(refkit_core, TidySyntaxError, TidyError);

pub(crate) fn document_error_to_py(err: DocumentError) -> PyErr {
    match err {
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
