mod raw;
mod rendered;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use pyo3::IntoPyObjectExt;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyDict, PyDictMethods, PyList, PyListMethods, PyModule};

use refkit_core::{
    CoreCite, CoreDocument, CoreLibrary, CoreRenderedDocument, DocumentError, EntryRecord,
    NormalizedEntry, NormalizedValue, PreparedStyle, ProjectField, StyleError, bundled_locales,
    load_prepared_style, option_quoted, parse_project_field, prepare_style_from_xml, quoted,
};
use rendered::Rendered;

create_exception!(refkit_core, RefkitError, PyException);
create_exception!(refkit_core, MissingReferenceError, RefkitError);

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Library {
    inner: Arc<CoreLibrary>,
    py_keys: Arc<PyOnceLock<Py<PyList>>>,
    py_entries: Arc<PyOnceLock<Py<PyDict>>>,
}

struct EntryData {
    record: EntryRecord,
}

impl EntryData {
    fn new(record: EntryRecord) -> Self {
        Self { record }
    }
}

impl Library {
    fn from_core(inner: CoreLibrary) -> Self {
        Self {
            inner: Arc::new(inner),
            py_keys: Arc::new(PyOnceLock::new()),
            py_entries: Arc::new(PyOnceLock::new()),
        }
    }

    fn entry_for_key(&self, key: &str) -> Option<Entry> {
        self.inner.get_record(key).cloned().map(Entry::from_record)
    }

    fn py_entry_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let entries = self.py_entries.get_or_try_init(py, || {
            let dict = PyDict::new(py);
            for record in self.inner.records() {
                dict.set_item(
                    &record.key,
                    Py::new(py, Entry::from_record(record.clone()))?,
                )?;
            }
            Ok::<_, PyErr>(dict.unbind())
        })?;
        Ok(entries.bind(py).clone())
    }
}

#[pymethods]
impl Library {
    #[staticmethod]
    #[pyo3(signature = (path, *, recovery = "error"))]
    fn read(py: Python<'_>, path: PathBuf, recovery: &str) -> PyResult<Self> {
        let (strict, diagnostics) = parse_recovery_policy(recovery)?;
        let library = py.detach(move || CoreLibrary::read_path(path, strict, diagnostics));
        library.map(Self::from_core).map_err(RefkitError::new_err)
    }

    #[staticmethod]
    #[pyo3(signature = (source, *, recovery = "error"))]
    fn parse_bibtex(py: Python<'_>, source: String, recovery: &str) -> PyResult<Self> {
        let (strict, diagnostics) = parse_recovery_policy(recovery)?;
        let library =
            py.detach(move || CoreLibrary::parse_source(&source, "bibtex", strict, diagnostics));
        library.map(Self::from_core).map_err(RefkitError::new_err)
    }

    #[staticmethod]
    fn parse_yaml(py: Python<'_>, source: String) -> PyResult<Self> {
        let library = py.detach(move || CoreLibrary::parse_source(&source, "yaml", false, false));
        library.map(Self::from_core).map_err(RefkitError::new_err)
    }

    #[getter]
    fn diagnostics(&self) -> Vec<String> {
        self.inner.diagnostics().to_vec()
    }

    fn keys(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let keys = self.py_keys.get_or_try_init(py, || {
            PyList::new(py, self.inner.keys().iter().map(String::as_str)).map(Bound::unbind)
        })?;
        keys.call_method0(py, "copy")
    }

    fn get_many(&self, py: Python<'_>, keys: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if keys.extract::<String>().is_ok() {
            return Err(PyTypeError::new_err(
                "keys must be an iterable of entry keys",
            ));
        }
        let entries = self.py_entry_dict(py)?;
        let rows = PyList::empty(py);
        let iter = keys
            .try_iter()
            .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
        for key in iter {
            let key = key?;
            let Some(entry) = entries.get_item(&key)? else {
                return Err(PyKeyError::new_err(key.str()?.to_string()));
            };
            rows.append(entry)?;
        }
        Ok(rows.into_any().unbind())
    }

    fn values(&self) -> Vec<Entry> {
        self.inner
            .records()
            .iter()
            .cloned()
            .map(Entry::from_record)
            .collect()
    }

    fn get(&self, key: &str) -> Option<Entry> {
        self.entry_for_key(key)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn select(&self, selector: &str) -> PyResult<Vec<Entry>> {
        Ok(self
            .inner
            .select_records(selector)
            .map_err(PyValueError::new_err)?
            .into_iter()
            .map(Entry::from_record)
            .collect())
    }

    fn to_dicts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let entries = py
            .detach(|| self.inner.normalized_entries())
            .map_err(RefkitError::new_err)?;
        normalized_entries_to_py(py, &entries)
    }

    #[pyo3(signature = (fields = None, *, keys = None))]
    fn project(
        &self,
        py: Python<'_>,
        fields: Option<&Bound<'_, PyAny>>,
        keys: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let fields = parse_project_fields_arg(fields)?;
        let keys = parse_projection_keys(self.inner.as_ref(), keys)?;
        let records = self
            .inner
            .project_records(&fields, keys.as_deref())
            .map_err(RefkitError::new_err)?;
        project_rows_to_py(py, &fields, records)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<Entry> {
        self.entry_for_key(key)
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Library({} entries)", self.inner.len())
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Entry {
    data: Arc<EntryData>,
}

impl Entry {
    fn from_record(record: EntryRecord) -> Self {
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

fn parse_recovery_policy(recovery: &str) -> PyResult<(bool, bool)> {
    match recovery {
        "error" => Ok((true, true)),
        "report" => Ok((false, true)),
        _ => Err(PyValueError::new_err(
            "recovery must be 'error' or 'report'",
        )),
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Style {
    id: String,
    data: Arc<PreparedStyle>,
}

#[pymethods]
impl Style {
    #[staticmethod]
    fn load(name: &str) -> PyResult<Self> {
        cached_bundled_style(name)
    }

    #[staticmethod]
    fn from_xml(xml: &str) -> PyResult<Self> {
        prepare_style_from_xml(xml)
            .map(|style| Self {
                id: "xml".to_string(),
                data: Arc::new(style),
            })
            .map_err(style_error_to_py)
    }

    #[staticmethod]
    fn from_path(path: PathBuf) -> PyResult<Self> {
        let xml = fs::read_to_string(&path)
            .map_err(|err| RefkitError::new_err(format!("failed to read style: {err}")))?;
        prepare_style_from_xml(&xml)
            .map(|style| Self {
                id: path.display().to_string(),
                data: Arc::new(style),
            })
            .map_err(style_error_to_py)
    }

    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    #[getter]
    fn title(&self) -> String {
        self.data.title().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "Style(id={}, title={})",
            quoted(&self.id),
            quoted(&self.title())
        )
    }
}

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct Locale {
    code: String,
}

#[pymethods]
impl Locale {
    #[staticmethod]
    fn load(code: &str) -> PyResult<Self> {
        bundled_locales()
            .iter()
            .find(|locale| locale.lang.as_ref().is_some_and(|lang| lang.0 == code))
            .map(|inner| Self {
                code: inner
                    .lang
                    .as_ref()
                    .map(|lang| lang.0.clone())
                    .unwrap_or_else(|| code.to_string()),
            })
            .ok_or_else(|| {
                PyValueError::new_err(format!("unknown bundled locale {}", quoted(code)))
            })
    }

    #[getter]
    fn code(&self) -> String {
        self.code.clone()
    }

    fn __repr__(&self) -> String {
        format!("Locale(code={})", quoted(&self.code))
    }
}

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

fn default_project_fields() -> Vec<String> {
    ["key", "title", "doi", "volume"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn parse_project_fields_arg(fields: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<ProjectField>> {
    let Some(fields) = fields.filter(|fields| !fields.is_none()) else {
        return parse_project_fields(default_project_fields().iter().map(String::as_str));
    };
    if fields.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "fields must be an iterable of field names",
        ));
    }
    let iter = fields
        .try_iter()
        .map_err(|_| PyTypeError::new_err("fields must be an iterable of field names"))?;
    let mut parsed = Vec::new();
    for field in iter {
        let field = field?;
        let field = field.extract::<&str>()?;
        parsed.push(
            parse_project_field(field).map_err(|err| PyValueError::new_err(err.to_string()))?,
        );
    }
    Ok(parsed)
}

fn parse_project_fields<'a>(fields: impl Iterator<Item = &'a str>) -> PyResult<Vec<ProjectField>> {
    let mut parsed = Vec::new();
    for field in fields {
        parsed.push(
            parse_project_field(field).map_err(|err| PyValueError::new_err(err.to_string()))?,
        );
    }
    Ok(parsed)
}

fn parse_projection_keys(
    library: &CoreLibrary,
    keys: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<String>>> {
    let Some(keys) = keys.filter(|keys| !keys.is_none()) else {
        return Ok(None);
    };
    if keys.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "keys must be an iterable of entry keys",
        ));
    }
    let iter = keys
        .try_iter()
        .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
    let mut parsed = Vec::new();
    for key in iter {
        let key = key?;
        let key = key.extract::<&str>()?;
        if !library.contains_key(key) {
            return Err(PyKeyError::new_err(key.to_string()));
        }
        parsed.push(key.to_string());
    }
    Ok(Some(parsed))
}

fn project_rows_to_py(
    py: Python<'_>,
    fields: &[ProjectField],
    records: Vec<Vec<Option<String>>>,
) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for record in records {
        rows.append(project_row_to_py(py, fields, &record)?)?;
    }
    Ok(rows.into_any().unbind())
}

fn project_row_to_py(
    py: Python<'_>,
    fields: &[ProjectField],
    record: &[Option<String>],
) -> PyResult<Py<PyAny>> {
    let row = PyDict::new(py);
    for (field, value) in fields.iter().zip(record) {
        row.set_item(project_field_name(*field), value.as_deref())?;
    }
    Ok(row.into_any().unbind())
}

fn project_field_name(field: ProjectField) -> &'static str {
    match field {
        ProjectField::Key => "key",
        ProjectField::EntryType => "entry_type",
        ProjectField::Type => "type",
        ProjectField::Title => "title",
        ProjectField::Doi => "doi",
        ProjectField::Volume => "volume",
    }
}

fn normalized_entries_to_py(py: Python<'_>, entries: &[NormalizedEntry]) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for entry in entries {
        rows.append(normalized_value_to_py(py, &entry.value)?)?;
    }
    Ok(rows.into_any().unbind())
}

fn normalized_value_to_py(py: Python<'_>, value: &NormalizedValue) -> PyResult<Py<PyAny>> {
    match value {
        NormalizedValue::Null => Ok(py.None()),
        NormalizedValue::Bool(value) => value.into_py_any(py),
        NormalizedValue::Number(value) => normalized_number_to_py(py, value),
        NormalizedValue::String(value) => value.clone().into_py_any(py),
        NormalizedValue::Array(values) => {
            let items = PyList::empty(py);
            for value in values {
                items.append(normalized_value_to_py(py, value)?)?;
            }
            Ok(items.into_any().unbind())
        }
        NormalizedValue::Object(values) => {
            let object = PyDict::new(py);
            for (key, value) in values {
                object.set_item(key, normalized_value_to_py(py, value)?)?;
            }
            Ok(object.into_any().unbind())
        }
    }
}

fn normalized_number_to_py(py: Python<'_>, value: &serde_json::Number) -> PyResult<Py<PyAny>> {
    if let Some(value) = value.as_i64() {
        value.into_py_any(py)
    } else if let Some(value) = value.as_u64() {
        value.into_py_any(py)
    } else if let Some(value) = value.as_f64() {
        value.into_py_any(py)
    } else {
        value.to_string().into_py_any(py)
    }
}

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
            .map(|citation| citation.id.clone())
            .collect::<Vec<_>>();
        let groups = parsed
            .into_iter()
            .map(|citation| {
                citation
                    .group
                    .into_iter()
                    .map(|cite| cite.to_core())
                    .collect()
            })
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
            .map(|citation| {
                citation
                    .group
                    .into_iter()
                    .map(|cite| cite.to_core())
                    .collect()
            })
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

fn document_error_to_py(err: DocumentError) -> PyErr {
    match err {
        DocumentError::MissingReference(key) => {
            MissingReferenceError::new_err(format!("missing reference {key}"))
        }
        DocumentError::UnknownLocatorLabel(label) => {
            PyValueError::new_err(format!("unknown locator label {}", quoted(&label)))
        }
        DocumentError::Render(message) => RefkitError::new_err(message),
    }
}

fn cached_bundled_style(name: &str) -> PyResult<Style> {
    load_prepared_style(name)
        .map(|style| Style {
            id: name.to_string(),
            data: style,
        })
        .map_err(style_error_to_py)
}

fn style_error_to_py(err: StyleError) -> PyErr {
    match err {
        StyleError::InvalidXml(message) => {
            PyValueError::new_err(format!("invalid CSL XML: {message}"))
        }
        StyleError::DependentStyle(_) => {
            PyValueError::new_err("dependent CSL styles need explicit parent resolution")
        }
        StyleError::UnknownBundledStyle(name) => {
            PyValueError::new_err(format!("unknown bundled style {}", quoted(&name)))
        }
        StyleError::CachePoisoned => RefkitError::new_err(err.to_string()),
    }
}

fn extract_locale(locale: Option<&Bound<'_, PyAny>>) -> PyResult<Option<String>> {
    let Some(locale) = locale else {
        return Ok(None);
    };
    if locale.is_none() {
        return Ok(None);
    }
    if let Ok(code) = locale.extract::<String>() {
        return Ok(Some(code));
    }
    if let Ok(locale) = locale.extract::<PyRef<'_, Locale>>() {
        return Ok(Some(locale.code.clone()));
    }
    Err(PyTypeError::new_err(
        "locale must be a string, Locale, or None",
    ))
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

fn parse_document_citations(citations: &Bound<'_, PyAny>) -> PyResult<Vec<Citation>> {
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

fn json_to_py(py: Python<'_>, value: &str) -> PyResult<Py<PyAny>> {
    let json = PyModule::import(py, "json")?;
    Ok(json.call_method1("loads", (value,))?.unbind())
}

#[pymodule(gil_used = true)]
fn _refkit_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("build_info", build_info())?;
    m.add("build_mode", build_mode())?;
    m.add("RefkitError", py.get_type::<RefkitError>())?;
    m.add(
        "MissingReferenceError",
        py.get_type::<MissingReferenceError>(),
    )?;
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
