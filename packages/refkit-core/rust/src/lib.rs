mod citation;
mod conversion;
mod document;
mod entry;
mod errors;
mod library;
mod module;
mod raw;
mod rendered;
mod style;

use pyo3::prelude::*;
use pyo3::types::PyModule;

#[pymodule(gil_used = true)]
fn _refkit_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    module::register(m)
}
