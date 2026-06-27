use std::collections::HashSet;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyAny;

use refkit_core::{CoreCite, option_quoted, quoted};

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Cite {
    #[pyo3(get)]
    key: String,
    #[pyo3(get)]
    locator: Option<String>,
    #[pyo3(get)]
    label: Option<String>,
}

#[pymethods]
impl Cite {
    #[new]
    #[pyo3(signature = (key, *, locator = None, label = None))]
    fn new(key: String, locator: Option<String>, label: Option<String>) -> Self {
        Self {
            key,
            locator,
            label,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Cite(key={}, locator={}, label={})",
            quoted(&self.key),
            option_quoted(self.locator.as_deref()),
            option_quoted(self.label.as_deref())
        )
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct CitationGroup {
    cites: Vec<Cite>,
}

#[pymethods]
impl CitationGroup {
    #[new]
    #[pyo3(signature = (items))]
    fn new(items: &Bound<'_, PyAny>) -> PyResult<Self> {
        let cites = parse_citation_group_items(items)?;
        if cites.is_empty() {
            return Err(PyValueError::new_err(
                "CitationGroup requires at least one citation",
            ));
        }
        Ok(Self { cites })
    }

    #[getter]
    fn items(&self) -> Vec<Cite> {
        self.cites.clone()
    }

    fn __len__(&self) -> usize {
        self.cites.len()
    }

    fn __repr__(&self) -> String {
        format!("CitationGroup({} citations)", self.cites.len())
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Citation {
    #[pyo3(get)]
    id: String,
    group: Vec<Cite>,
}

#[pymethods]
impl Citation {
    #[new]
    #[pyo3(signature = (id, citation))]
    fn new(id: String, citation: &Bound<'_, PyAny>) -> PyResult<Self> {
        let group = parse_citation_arg(citation)?;
        Ok(Self { id, group })
    }

    #[getter]
    fn group(&self) -> CitationGroup {
        CitationGroup {
            cites: self.group.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Citation(id={}, {} items)",
            quoted(&self.id),
            self.group.len()
        )
    }
}

impl Cite {
    fn to_core(&self) -> CoreCite {
        CoreCite::new(self.key.clone(), self.locator.clone(), self.label.clone())
    }
}

impl Citation {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn into_core_group(self) -> Vec<CoreCite> {
        self.group.into_iter().map(|cite| cite.to_core()).collect()
    }
}

fn parse_citation_arg(citation: &Bound<'_, PyAny>) -> PyResult<Vec<Cite>> {
    if let Ok(group) = citation.extract::<PyRef<'_, CitationGroup>>() {
        return Ok(group.cites.clone());
    }

    if let Ok(cite) = parse_single_cite(citation) {
        return Ok(vec![cite]);
    }

    Err(PyTypeError::new_err(
        "citation must be a key string, Cite, or CitationGroup",
    ))
}

pub(crate) fn parse_document_citations(citations: &Bound<'_, PyAny>) -> PyResult<Vec<Citation>> {
    if citations.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "citations must be an iterable of Citation objects",
        ));
    }
    let iter = citations
        .try_iter()
        .map_err(|_| PyTypeError::new_err("citations must be an iterable of Citation objects"))?;
    let mut parsed = Vec::new();
    let mut ids = HashSet::new();
    for citation in iter {
        let citation = citation?;
        let citation = citation.extract::<PyRef<'_, Citation>>()?;
        if !ids.insert(citation.id.clone()) {
            return Err(PyValueError::new_err(format!(
                "duplicate citation id {}",
                quoted(&citation.id)
            )));
        }
        parsed.push(citation.clone());
    }
    Ok(parsed)
}

fn parse_citation_group_items(items: &Bound<'_, PyAny>) -> PyResult<Vec<Cite>> {
    if items.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "CitationGroup items must be an iterable of key strings or Cite objects",
        ));
    }

    if let Ok(iter) = items.try_iter() {
        return iter.map(|item| parse_single_cite(&item?)).collect();
    }

    Err(PyTypeError::new_err(
        "CitationGroup items must be an iterable of key strings or Cite objects",
    ))
}

fn parse_single_cite(item: &Bound<'_, PyAny>) -> PyResult<Cite> {
    if let Ok(key) = item.extract::<String>() {
        return Ok(Cite {
            key,
            locator: None,
            label: None,
        });
    }
    if let Ok(cite) = item.extract::<PyRef<'_, Cite>>() {
        return Ok(cite.clone());
    }
    Err(PyTypeError::new_err(
        "citation items must be strings or Cite objects",
    ))
}
