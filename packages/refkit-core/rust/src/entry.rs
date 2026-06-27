use std::sync::Arc;

use pyo3::prelude::*;

use refkit_core::{EntryRecord, quoted};

struct EntryData {
    record: EntryRecord,
}

impl EntryData {
    fn new(record: EntryRecord) -> Self {
        Self { record }
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Entry {
    data: Arc<EntryData>,
}

impl Entry {
    pub(crate) fn from_record(record: EntryRecord) -> Self {
        Self {
            data: Arc::new(EntryData::new(record)),
        }
    }
}

#[pymethods]
impl Entry {
    #[getter]
    fn key(&self) -> String {
        self.data.record.key.clone()
    }

    #[getter]
    fn entry_type(&self) -> String {
        self.data.record.entry_type.clone()
    }

    #[getter]
    fn title(&self) -> Option<String> {
        self.data.record.title.clone()
    }

    #[getter]
    fn parents(&self) -> Vec<Entry> {
        self.data
            .record
            .parents
            .iter()
            .cloned()
            .map(Entry::from_record)
            .collect()
    }

    #[getter]
    fn volume(&self) -> Option<String> {
        self.data.record.volume.clone()
    }

    #[getter]
    fn doi(&self) -> Option<String> {
        self.data.record.doi.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Entry(key={}, type={})",
            quoted(&self.data.record.key),
            quoted(&self.data.record.entry_type)
        )
    }
}
