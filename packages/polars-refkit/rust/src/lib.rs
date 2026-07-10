mod expressions;

use pyo3::prelude::*;
use pyo3_polars::PolarsAllocator;
pub use pyo3_polars::derive::{_polars_plugin_get_last_error_message, _polars_plugin_get_version};

#[pymodule]
fn _internal(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", python_version(env!("CARGO_PKG_VERSION")))?;
    Ok(())
}

fn python_version(cargo_version: &str) -> String {
    // Cargo and Python spell release candidates differently. Export the PEP 440 form.
    cargo_version.replace("-rc.", "rc")
}

#[global_allocator]
static ALLOC: PolarsAllocator = PolarsAllocator::new();

#[cfg(test)]
mod tests {
    use super::python_version;

    #[test]
    fn python_version_uses_pep_440_rc_form() {
        assert_eq!(python_version("1.2.3-rc.4"), "1.2.3rc4");
    }

    #[test]
    fn python_version_preserves_stable_form() {
        assert_eq!(python_version("1.2.3"), "1.2.3");
    }
}
