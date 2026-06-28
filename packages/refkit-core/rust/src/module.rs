use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::citation::{Citation, CitationGroup, Cite};
use crate::document::{Document, RenderedDocument};
use crate::entry::Entry;
use crate::errors::{MissingReferenceError, RefkitError, TidyError, TidySyntaxError};
use crate::library::Library;
use crate::raw;
use crate::rendered::Rendered;
use crate::style::{Locale, Style};
use crate::tidy;

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("build_info", build_info())?;
    m.add("build_mode", build_mode())?;
    m.add("RefkitError", py.get_type::<RefkitError>())?;
    m.add(
        "MissingReferenceError",
        py.get_type::<MissingReferenceError>(),
    )?;
    m.add("TidyError", py.get_type::<TidyError>())?;
    m.add("TidySyntaxError", py.get_type::<TidySyntaxError>())?;
    m.add_class::<Library>()?;
    m.add_class::<Entry>()?;
    m.add_class::<Style>()?;
    m.add_class::<Locale>()?;
    m.add_class::<Cite>()?;
    m.add_class::<CitationGroup>()?;
    m.add_class::<Citation>()?;
    m.add_class::<Document>()?;
    m.add_class::<RenderedDocument>()?;
    m.add_class::<Rendered>()?;
    tidy::register(m)?;
    raw::register(m)?;
    Ok(())
}

fn build_info() -> String {
    format!(
        "refkit-core {} ({}, {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn build_mode() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
