use std::sync::Arc;

use pyo3::prelude::*;

use refkit_core::EntryRecord;

use crate::repr::quoted;

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Entry {
    data: Arc<EntryRecord>,
}

impl Entry {
    pub(crate) fn from_record(record: EntryRecord) -> Self {
        Self {
            data: Arc::new(record),
        }
    }
}

#[pymethods]
impl Entry {
    #[getter]
    fn key(&self) -> &str {
        &self.data.key
    }

    #[getter]
    fn entry_type(&self) -> &str {
        &self.data.entry_type
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.data.title.as_deref()
    }

    #[getter]
    fn date(&self) -> Option<&str> {
        self.data.date.as_deref()
    }

    #[getter]
    fn parents(&self) -> Vec<Entry> {
        self.data
            .parents
            .iter()
            .cloned()
            .map(Entry::from_record)
            .collect()
    }

    #[getter]
    fn volume(&self) -> Option<&str> {
        self.data.volume.as_deref()
    }

    #[getter]
    fn doi(&self) -> Option<&str> {
        self.data.doi.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Entry(key={}, type={})",
            quoted(&self.data.key),
            quoted(&self.data.entry_type)
        )
    }
}
