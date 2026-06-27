use std::sync::Arc;

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use refkit_core::{CoreDocument, CoreRenderedDocument};

use crate::citation::parse_document_citations;
use crate::errors::document_error_to_py;
use crate::library::Library;
use crate::rendered::Rendered;
use crate::style::{Style, extract_locale};

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Document {
    inner: CoreDocument,
}

#[pymethods]
impl Document {
    #[new]
    #[pyo3(signature = (library, style, *, locale = None))]
    fn new(library: &Library, style: &Style, locale: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let inner = CoreDocument::new(
            Arc::clone(&library.inner),
            Arc::clone(&style.data),
            extract_locale(locale)?,
        );
        Ok(Self { inner })
    }

    fn render(&self, py: Python<'_>, citations: &Bound<'_, PyAny>) -> PyResult<RenderedDocument> {
        let parsed = parse_document_citations(citations)?;
        let ids = parsed
            .iter()
            .map(|citation| citation.id().to_string())
            .collect::<Vec<_>>();
        let groups = parsed
            .into_iter()
            .map(|citation| citation.into_core_group())
            .collect();
        let rendered = py.detach(|| self.inner.render(groups));
        rendered
            .map(|document| RenderedDocument::from_core(ids, document))
            .map_err(document_error_to_py)
    }

    fn cited_bibliography(
        &self,
        py: Python<'_>,
        citations: &Bound<'_, PyAny>,
    ) -> PyResult<Rendered> {
        let groups = parse_document_citations(citations)?
            .into_iter()
            .map(|citation| citation.into_core_group())
            .collect();
        let rendered = py.detach(|| self.inner.cited_bibliography(groups));
        rendered
            .map(Rendered::from_record)
            .map_err(document_error_to_py)
    }

    fn full_bibliography(&self, py: Python<'_>) -> PyResult<Rendered> {
        let rendered = py.detach(|| self.inner.full_bibliography());
        rendered
            .map(Rendered::from_record)
            .map_err(document_error_to_py)
    }

    fn __repr__(&self) -> String {
        format!("Document({} entries)", self.inner.entry_count())
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
pub struct RenderedDocument {
    citation_ids: Vec<String>,
    citations: Vec<Rendered>,
    bibliography: Rendered,
}

#[pymethods]
impl RenderedDocument {
    #[getter]
    fn citation_order(&self) -> Vec<String> {
        self.citation_ids.clone()
    }

    #[getter]
    fn citations(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (id, rendered) in self.citation_ids.iter().zip(self.citations.iter()) {
            dict.set_item(id, rendered.clone())?;
        }
        Ok(dict.into_any().unbind())
    }

    #[getter]
    fn bibliography(&self) -> Rendered {
        self.bibliography.clone()
    }

    fn __getitem__(&self, id: &str) -> PyResult<Rendered> {
        self.citation_ids
            .iter()
            .position(|candidate| candidate == id)
            .map(|index| self.citations[index].clone())
            .ok_or_else(|| PyKeyError::new_err(id.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("RenderedDocument({} citations)", self.citations.len())
    }
}

impl RenderedDocument {
    fn from_core(ids: Vec<String>, document: CoreRenderedDocument) -> Self {
        Self {
            citation_ids: ids,
            citations: document
                .citations
                .into_iter()
                .map(Rendered::from_record)
                .collect(),
            bibliography: Rendered::from_record(document.bibliography),
        }
    }
}
