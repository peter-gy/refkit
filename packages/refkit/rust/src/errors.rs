use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;

use refkit_core::{DocumentError, StyleError};

use crate::repr::quoted;

create_exception!(refkit, RefkitError, PyException);
create_exception!(refkit, ParseError, RefkitError);
create_exception!(refkit, ConversionError, RefkitError);
create_exception!(refkit, PatchError, RefkitError);
create_exception!(refkit, MergeError, RefkitError);

pub(crate) fn merge_error_to_py(py: Python<'_>, error: refkit_core::MergeError) -> PyErr {
    let exception = MergeError::new_err(error.message);
    match serde_json::to_string(&error.code)
        .map_err(|error| RefkitError::new_err(error.to_string()))
        .and_then(|code| crate::conversion::json_to_py(py, &code))
        .and_then(|code| exception.value(py).setattr("code", code))
    {
        Ok(()) => exception,
        Err(error) => error,
    }
}

pub(crate) fn patch_error_to_py(py: Python<'_>, error: refkit_core::BibPatchError) -> PyErr {
    let exception = PatchError::new_err(error.message);
    let populated = (|| -> PyResult<()> {
        exception.value(py).setattr(
            "code",
            crate::conversion::json_to_py(
                py,
                &serde_json::to_string(&error.code)
                    .map_err(|error| RefkitError::new_err(error.to_string()))?,
            )?,
        )?;
        exception.value(py).setattr("operation", error.operation)?;
        Ok(())
    })();
    populated.err().unwrap_or(exception)
}

pub(crate) fn codec_error_to_py(py: Python<'_>, error: refkit_core::CodecError) -> PyErr {
    let exception = ConversionError::new_err(error.message);
    let populated = (|| -> PyResult<()> {
        let issues = serde_json::to_string(&error.issues)
            .map_err(|error| RefkitError::new_err(error.to_string()))?;
        exception
            .value(py)
            .setattr("issues", crate::conversion::json_to_py(py, &issues)?)?;
        exception.value(py).setattr(
            "diagnostics",
            crate::conversion::diagnostics_to_py(py, &error.diagnostics)?,
        )?;
        Ok(())
    })();
    populated.err().unwrap_or(exception)
}
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
        DocumentError::UnknownCitationPurpose(purpose) => {
            PyValueError::new_err(format!("unknown citation purpose {}", quoted(&purpose)))
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
        StyleError::MissingParent(_) | StyleError::InvalidParent(_) => {
            PyValueError::new_err(err.to_string())
        }
        StyleError::UnknownBundledStyle(name) => {
            PyValueError::new_err(format!("unknown bundled style {}", quoted(&name)))
        }
        StyleError::CachePoisoned => RefkitError::new_err(err.to_string()),
    }
}

pub(crate) fn library_error_to_py(py: Python<'_>, error: refkit_core::LibraryError) -> PyErr {
    if let refkit_core::LibraryError::Record(error) = &error {
        return PyValueError::new_err(error.to_string());
    }
    let exception = ParseError::new_err(error.to_string());
    let diagnostics = match error {
        refkit_core::LibraryError::Biblatex(failure)
        | refkit_core::LibraryError::HayagrivaYaml(failure) => failure.diagnostics,
        _ => Vec::new(),
    };
    match crate::conversion::diagnostics_to_py(py, &diagnostics)
        .and_then(|diagnostics| exception.value(py).setattr("diagnostics", diagnostics))
    {
        Ok(()) => exception,
        Err(error) => error,
    }
}
