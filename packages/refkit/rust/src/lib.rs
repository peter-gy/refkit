mod citation;
mod conversion;
mod document;
mod entry;
mod errors;
mod filesystem;
mod library;
mod module;
mod raw;
mod rendered;
mod repr;
mod style;
mod tidy;

use pyo3::prelude::*;
use pyo3::types::PyModule;

#[pymodule(gil_used = true)]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    module::register(m)
}
