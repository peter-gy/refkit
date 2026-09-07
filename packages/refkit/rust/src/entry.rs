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
    fn key(&self) -> String {
        self.data.key.clone()
    }

    #[getter]
    fn entry_type(&self) -> String {
        self.data.entry_type.clone()
    }

    #[getter]
    fn title(&self) -> Option<String> {
        self.data.title.clone()
    }

    #[getter]
    fn date(&self) -> Option<String> {
        self.data.date.clone()
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
    fn volume(&self) -> Option<String> {
        self.data.volume.clone()
    }

    #[getter]
    fn doi(&self) -> Option<String> {
        self.data.doi.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Entry(key={}, type={})",
            quoted(&self.data.key),
            quoted(&self.data.entry_type)
        )
    }
}
