mod expressions;

use pyo3::prelude::*;
use pyo3_polars::PolarsAllocator;

#[pymodule]
fn _internal(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[global_allocator]
static ALLOC: PolarsAllocator = PolarsAllocator::new();
