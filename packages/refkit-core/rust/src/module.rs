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
    m.add("__version__", python_version(env!("CARGO_PKG_VERSION")))?;
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
        python_version(env!("CARGO_PKG_VERSION")),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn python_version(cargo_version: &str) -> String {
    // Cargo and Python spell release candidates differently. Export the PEP 440 form.
    cargo_version.replace("-rc.", "rc")
}

fn build_mode() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[cfg(test)]
mod tests {
    use super::{build_info, python_version};

    #[test]
    fn build_info_uses_python_version() {
        let version = python_version(env!("CARGO_PKG_VERSION"));

        assert!(build_info().starts_with(&format!("refkit-core {version} (")));
        assert!(!build_info().contains("-rc."));
    }

    #[test]
    fn python_version_uses_pep_440_rc_form() {
        assert_eq!(python_version("1.2.3-rc.4"), "1.2.3rc4");
    }

    #[test]
    fn python_version_preserves_stable_form() {
        assert_eq!(python_version("1.2.3"), "1.2.3");
    }
}
