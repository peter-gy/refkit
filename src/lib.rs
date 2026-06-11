mod raw;

use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use hayagriva::citationberg::taxonomy::Locator as CslLocator;
use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, IndependentStyle, Locale as CslLocale, LocaleCode,
    Style as CslStyle, TextDecoration, VerticalAlign,
};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, BufWriteFormat, CitationItem, CitationRequest,
    ElemChild, ElemChildren, Entry as HayEntry, Library as HayLibrary, LocatorPayload,
    RenderedCitation, Selector, SpecificLocator, archive,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyDict, PyDictMethods, PyList, PyListMethods, PyModule};
use pyo3::{IntoPyObjectExt, intern};
use serde::Serialize;
use serde_json::{Value, json};

create_exception!(refkit, RefkitError, PyException);
create_exception!(refkit, MissingReferenceError, RefkitError);

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Library {
    inner: Arc<HayLibrary>,
    diagnostics: Vec<String>,
    cache: Arc<OnceLock<LibraryCache>>,
    py_keys: Arc<PyOnceLock<Py<PyList>>>,
    py_entries: Arc<PyOnceLock<Py<PyDict>>>,
}

struct ParsedLibrary {
    inner: HayLibrary,
    diagnostics: Vec<String>,
}

struct LibraryCache {
    keys: Vec<String>,
    entries: Vec<Arc<EntryData>>,
    index: HashMap<String, usize>,
}

struct EntryData {
    inner: Arc<HayEntry>,
    key: String,
    entry_type: String,
    title: Option<String>,
    volume: Option<String>,
    doi: Option<String>,
}

impl EntryData {
    fn new(inner: HayEntry) -> Self {
        let key = inner.key().to_string();
        let entry_type = format!("{:?}", inner.entry_type());
        let title = inner.title().map(ToString::to_string);
        let volume = inner
            .volume()
            .or_else(|| inner.parents().first().and_then(|parent| parent.volume()))
            .map(|value| value.to_string());
        let doi = inner
            .serial_number()
            .and_then(|serial| serial.0.get("doi").cloned());

        Self {
            inner: Arc::new(inner),
            key,
            entry_type,
            title,
            volume,
            doi,
        }
    }
}

impl LibraryCache {
    fn from_library(library: &HayLibrary) -> Self {
        let mut keys = Vec::with_capacity(library.len());
        let mut entries = Vec::with_capacity(library.len());
        let mut index = HashMap::with_capacity(library.len());

        for entry in library.iter() {
            let key = entry.key().to_string();
            keys.push(key.clone());
            index.insert(key, entries.len());
            entries.push(Arc::new(EntryData::new(entry.clone())));
        }

        Self {
            keys,
            entries,
            index,
        }
    }
}

impl Library {
    fn from_parsed(parsed: ParsedLibrary) -> Self {
        Self {
            inner: Arc::new(parsed.inner),
            diagnostics: parsed.diagnostics,
            cache: Arc::new(OnceLock::new()),
            py_keys: Arc::new(PyOnceLock::new()),
            py_entries: Arc::new(PyOnceLock::new()),
        }
    }

    fn cache(&self) -> &LibraryCache {
        self.cache
            .get_or_init(|| LibraryCache::from_library(self.inner.as_ref()))
    }

    fn entry_for_key(&self, key: &str) -> Option<Entry> {
        let cache = self.cache();
        cache
            .index
            .get(key)
            .map(|index| Entry::from_data(Arc::clone(&cache.entries[*index])))
    }

    fn entry_for_hay_entry(&self, entry: &HayEntry) -> Entry {
        self.entry_for_key(entry.key())
            .unwrap_or_else(|| Entry::from_owned(entry.clone()))
    }

    fn py_entry_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let entries = self.py_entries.get_or_try_init(py, || {
            let dict = PyDict::new(py);
            for entry in &self.cache().entries {
                dict.set_item(
                    &entry.key,
                    Py::new(py, Entry::from_data(Arc::clone(entry)))?,
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
    #[pyo3(signature = (path, strict = true, diagnostics = false))]
    fn read(py: Python<'_>, path: PathBuf, strict: bool, diagnostics: bool) -> PyResult<Self> {
        let parsed = py.detach(move || parse_library_path(path, strict, diagnostics));
        parsed.map(Self::from_parsed).map_err(RefkitError::new_err)
    }

    #[staticmethod]
    #[pyo3(signature = (source, format = "bibtex", strict = true, diagnostics = false))]
    fn parse(
        py: Python<'_>,
        source: String,
        format: &str,
        strict: bool,
        diagnostics: bool,
    ) -> PyResult<Self> {
        let format = format.to_string();
        let parsed = py.detach(move || parse_library_source(&source, &format, strict, diagnostics));
        parsed.map(Self::from_parsed).map_err(RefkitError::new_err)
    }

    #[getter]
    fn diagnostics(&self) -> Vec<String> {
        self.diagnostics.clone()
    }

    fn keys(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let keys = self.py_keys.get_or_try_init(py, || {
            PyList::new(py, self.cache().keys.iter().map(String::as_str)).map(Bound::unbind)
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
        self.cache()
            .entries
            .iter()
            .map(|entry| Entry::from_data(Arc::clone(entry)))
            .collect()
    }

    fn get(&self, key: &str) -> Option<Entry> {
        self.entry_for_key(key)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn select(&self, selector: &str) -> PyResult<Vec<Entry>> {
        let selector = Selector::parse(selector)
            .map_err(|err| PyValueError::new_err(format!("invalid selector: {err}")))?;
        Ok(self
            .inner
            .iter()
            .filter(|entry| selector.matches(entry))
            .map(|entry| self.entry_for_hay_entry(entry))
            .collect())
    }

    fn to_dicts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let values = PyList::empty(py);
        for entry in self.inner.iter() {
            let mut value =
                serde_json::to_value(entry).map_err(|err| RefkitError::new_err(err.to_string()))?;
            if let Some(map) = value.as_object_mut() {
                map.insert("key".to_string(), json!(entry.key()));
            }
            values.append(json_value_to_py(py, &value)?)?;
        }
        Ok(values.into_any().unbind())
    }

    #[pyo3(signature = (fields = None, keys = None))]
    fn project(
        &self,
        py: Python<'_>,
        fields: Option<&Bound<'_, PyAny>>,
        keys: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let fields = parse_project_fields_arg(fields)?;
        let cache = self.cache();
        let rows = PyList::empty(py);

        match keys {
            Some(keys) if !keys.is_none() => {
                append_projected_keyed_entries(py, &rows, cache, keys, &fields)?
            }
            _ => {
                for entry in &cache.entries {
                    rows.append(project_entry(py, entry, &fields)?)?;
                }
            }
        }

        Ok(rows.into_any().unbind())
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    fn __getitem__(&self, key: &str) -> PyResult<Entry> {
        self.entry_for_key(key)
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("Library({} entries)", self.inner.len())
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Entry {
    data: Arc<EntryData>,
}

impl Entry {
    fn from_owned(inner: HayEntry) -> Self {
        Self {
            data: Arc::new(EntryData::new(inner)),
        }
    }

    fn from_data(data: Arc<EntryData>) -> Self {
        Self { data }
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
    fn parent(&self) -> Option<Entry> {
        self.data
            .inner
            .parents()
            .first()
            .cloned()
            .map(Entry::from_owned)
    }

    #[getter]
    fn parents(&self) -> Vec<Entry> {
        self.data
            .inner
            .parents()
            .iter()
            .cloned()
            .map(Entry::from_owned)
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
            "Entry(key={:?}, type={:?})",
            self.data.key, self.data.entry_type
        )
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Style {
    id: String,
    inner: Arc<IndependentStyle>,
}

#[pymethods]
impl Style {
    #[staticmethod]
    fn load(name: &str) -> PyResult<Self> {
        let archived = archive::ArchivedStyle::by_name(&name.to_ascii_lowercase())
            .ok_or_else(|| PyValueError::new_err(format!("unknown bundled style {name:?}")))?;
        let style = archived.get();
        independent_style(name.to_string(), style)
    }

    #[staticmethod]
    fn from_xml(xml: &str) -> PyResult<Self> {
        let style = CslStyle::from_xml(xml)
            .map_err(|err| PyValueError::new_err(format!("invalid CSL XML: {err}")))?;
        independent_style("xml".to_string(), style)
    }

    #[staticmethod]
    fn from_path(path: PathBuf) -> PyResult<Self> {
        let xml = fs::read_to_string(&path)
            .map_err(|err| RefkitError::new_err(format!("failed to read style: {err}")))?;
        let style = CslStyle::from_xml(&xml)
            .map_err(|err| PyValueError::new_err(format!("invalid CSL XML: {err}")))?;
        independent_style(path.display().to_string(), style)
    }

    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    #[getter]
    fn title(&self) -> String {
        self.inner.info.title.value.clone()
    }

    fn __repr__(&self) -> String {
        format!("Style(id={:?}, title={:?})", self.id, self.title())
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
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
            .ok_or_else(|| PyValueError::new_err(format!("unknown bundled locale {code:?}")))
    }

    #[getter]
    fn code(&self) -> String {
        self.code.clone()
    }

    fn __repr__(&self) -> String {
        format!("Locale(code={:?})", self.code)
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
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
    #[pyo3(signature = (key, locator = None, label = None))]
    fn new(key: String, locator: Option<String>, label: Option<String>) -> Self {
        Self {
            key,
            locator,
            label,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Cite(key={:?}, locator={:?}, label={:?})",
            self.key, self.locator, self.label
        )
    }
}

#[pyclass(module = "refkit", skip_from_py_object)]
pub struct Rendered {
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    html: String,
    tree: RenderedTree,
    tree_json: OnceLock<String>,
}

enum RenderedTree {
    Empty,
    Citation(ElemChildren),
    Bibliography(Vec<hayagriva::BibliographyItem>),
}

#[pymethods]
impl Rendered {
    #[getter]
    fn tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_to_py(py, self.tree_json())
    }

    fn to_text(&self) -> String {
        self.text.clone()
    }

    fn to_html(&self) -> String {
        self.html.clone()
    }

    fn to_tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.tree(py)
    }

    fn __repr__(&self) -> String {
        format!("Rendered(text={:?})", preview(&self.text))
    }
}

impl Rendered {
    fn new(text: String, html: String, tree: RenderedTree) -> Self {
        Self {
            text,
            html,
            tree,
            tree_json: OnceLock::new(),
        }
    }

    fn tree_json(&self) -> &str {
        self.tree_json
            .get_or_init(|| rendered_tree_to_json(&self.tree))
            .as_str()
    }
}

fn rendered_tree_to_json(tree: &RenderedTree) -> String {
    match tree {
        RenderedTree::Empty => "[]".to_string(),
        RenderedTree::Citation(children) => serde_json::to_string(&children_to_tree(children))
            .expect("rendered citation tree is JSON serializable"),
        RenderedTree::Bibliography(items) => {
            let tree_items = items
                .iter()
                .map(bibliography_item_to_tree)
                .collect::<Vec<_>>();
            serde_json::to_string(&tree_items)
                .expect("rendered bibliography tree is JSON serializable")
        }
    }
}

fn default_project_fields() -> Vec<String> {
    ["key", "title", "doi", "volume"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProjectField {
    Key,
    EntryType,
    Type,
    Title,
    Doi,
    Volume,
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
        parsed.push(parse_project_field(field.extract::<&str>()?)?);
    }
    Ok(parsed)
}

fn parse_project_fields<'a>(fields: impl Iterator<Item = &'a str>) -> PyResult<Vec<ProjectField>> {
    let mut parsed = Vec::new();
    for field in fields {
        parsed.push(parse_project_field(field)?);
    }
    Ok(parsed)
}

fn parse_project_field(field: &str) -> PyResult<ProjectField> {
    match field {
        "key" => Ok(ProjectField::Key),
        "entry_type" => Ok(ProjectField::EntryType),
        "type" => Ok(ProjectField::Type),
        "title" => Ok(ProjectField::Title),
        "doi" => Ok(ProjectField::Doi),
        "volume" => Ok(ProjectField::Volume),
        _ => Err(PyValueError::new_err(format!(
            "unsupported projection field {field:?}"
        ))),
    }
}

fn append_projected_keyed_entries(
    py: Python<'_>,
    rows: &Bound<'_, PyList>,
    cache: &LibraryCache,
    keys: &Bound<'_, PyAny>,
    fields: &[ProjectField],
) -> PyResult<()> {
    if keys.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(
            "keys must be an iterable of entry keys",
        ));
    }
    let iter = keys
        .try_iter()
        .map_err(|_| PyTypeError::new_err("keys must be an iterable of entry keys"))?;
    for key in iter {
        let key = key?;
        let key = key.extract::<&str>()?;
        let Some(index) = cache.index.get(key) else {
            return Err(PyKeyError::new_err(key.to_string()));
        };
        rows.append(project_entry(py, &cache.entries[*index], fields)?)?;
    }
    Ok(())
}

fn project_entry(
    py: Python<'_>,
    entry: &EntryData,
    fields: &[ProjectField],
) -> PyResult<Py<PyAny>> {
    if fields == [ProjectField::Key, ProjectField::Title] {
        return project_key_title_entry(py, entry);
    }
    if fields
        == [
            ProjectField::Key,
            ProjectField::Title,
            ProjectField::Doi,
            ProjectField::Volume,
        ]
    {
        return project_common_entry(py, entry);
    }

    let row = PyDict::new(py);
    for field in fields {
        match field {
            ProjectField::Key => row.set_item(intern!(py, "key"), &entry.key)?,
            ProjectField::EntryType => {
                row.set_item(intern!(py, "entry_type"), &entry.entry_type)?
            }
            ProjectField::Type => row.set_item(intern!(py, "type"), &entry.entry_type)?,
            ProjectField::Title => row.set_item(intern!(py, "title"), entry.title.as_deref())?,
            ProjectField::Doi => row.set_item(intern!(py, "doi"), entry.doi.as_deref())?,
            ProjectField::Volume => row.set_item(intern!(py, "volume"), entry.volume.as_deref())?,
        }
    }
    Ok(row.into_any().unbind())
}

fn project_key_title_entry(py: Python<'_>, entry: &EntryData) -> PyResult<Py<PyAny>> {
    let row = PyDict::new(py);
    row.set_item(intern!(py, "key"), &entry.key)?;
    row.set_item(intern!(py, "title"), entry.title.as_deref())?;
    Ok(row.into_any().unbind())
}

fn project_common_entry(py: Python<'_>, entry: &EntryData) -> PyResult<Py<PyAny>> {
    let row = PyDict::new(py);
    row.set_item(intern!(py, "key"), &entry.key)?;
    row.set_item(intern!(py, "title"), entry.title.as_deref())?;
    row.set_item(intern!(py, "doi"), entry.doi.as_deref())?;
    row.set_item(intern!(py, "volume"), entry.volume.as_deref())?;
    Ok(row.into_any().unbind())
}

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Document {
    library: Arc<HayLibrary>,
    style: Arc<IndependentStyle>,
    locale: Option<String>,
    citations: Vec<Vec<Cite>>,
}

#[pymethods]
impl Document {
    #[new]
    #[pyo3(signature = (library, style, locale = None))]
    fn new(library: &Library, style: &Style, locale: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self {
            library: Arc::clone(&library.inner),
            style: Arc::clone(&style.inner),
            locale: extract_locale(locale)?,
            citations: Vec::new(),
        })
    }

    fn cite(&mut self, items: &Bound<'_, PyAny>) -> PyResult<Rendered> {
        let group = parse_cite_group(items)?;
        self.citations.push(group);
        let rendered = match self.render_all(false) {
            Ok(rendered) => rendered,
            Err(err) => {
                self.citations.pop();
                return Err(err);
            }
        };
        let Some(citation) = rendered.citations.last() else {
            self.citations.pop();
            return Err(RefkitError::new_err(
                "citation renderer returned no citations",
            ));
        };
        rendered_from_citation(citation)
    }

    #[pyo3(signature = (all = false))]
    fn bibliography(&self, all: bool) -> PyResult<Rendered> {
        let rendered = self.render_all(all)?;
        rendered_from_bibliography(rendered.bibliography)
    }

    fn __repr__(&self) -> String {
        format!(
            "Document({} entries, {} citations)",
            self.library.len(),
            self.citations.len()
        )
    }
}

impl Document {
    fn render_all(&self, all: bool) -> PyResult<hayagriva::Rendered> {
        self.render_with_style(all, self.style.as_ref())
    }

    fn render_with_style(
        &self,
        all: bool,
        style: &IndependentStyle,
    ) -> PyResult<hayagriva::Rendered> {
        let locales = bundled_locales();
        let locale = self.locale.as_ref().map(|code| LocaleCode(code.clone()));
        let mut driver = BibliographyDriver::new();

        for group in &self.citations {
            let mut items = Vec::with_capacity(group.len());
            for cite in group {
                let entry = self.library.get(&cite.key).ok_or_else(|| {
                    MissingReferenceError::new_err(format!("missing reference {}", cite.key))
                })?;
                items.push(citation_item(entry, cite)?);
            }

            driver.citation(CitationRequest::new(
                items,
                style,
                locale.clone(),
                locales,
                None,
            ));
        }

        if all {
            for entry in self.library.iter() {
                driver.citation(CitationRequest::new(
                    vec![CitationItem::with_entry(entry)],
                    style,
                    locale.clone(),
                    locales,
                    None,
                ));
            }
        }

        Ok(driver.finish(BibliographyRequest::new(style, locale, locales)))
    }
}

fn bundled_locales() -> &'static [CslLocale] {
    static LOCALES: OnceLock<Vec<CslLocale>> = OnceLock::new();
    LOCALES.get_or_init(archive::locales).as_slice()
}

fn parse_library_path(
    path: PathBuf,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bib") => parse_library_source(&source, "bibtex", strict, diagnostics),
        Some("yaml" | "yml") => parse_library_source(&source, "yaml", strict, diagnostics),
        Some(ext) => Err(format!("unsupported bibliography extension {ext:?}")),
        None => Err("bibliography path has no extension".to_string()),
    }
}

fn parse_library_source(
    source: &str,
    format: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    match format.to_ascii_lowercase().as_str() {
        "bib" | "bibtex" | "biblatex" => parse_biblatex_library(source, strict, diagnostics),
        "yaml" | "yml" => hayagriva::io::from_yaml_str(source)
            .map(|inner| ParsedLibrary {
                inner,
                diagnostics: Vec::new(),
            })
            .map_err(|err| format!("yaml parse error: {err}")),
        other => Err(format!("unsupported bibliography format {other:?}")),
    }
}

fn parse_biblatex_library(
    source: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    match hayagriva::io::from_biblatex_str(source) {
        Ok(inner) => Ok(ParsedLibrary {
            inner,
            diagnostics: Vec::new(),
        }),
        Err(errors) if strict => Err(format_biblatex_errors(&errors)),
        Err(errors) => {
            let (sanitized, raw_diagnostics) = raw::sanitize_biblatex_for_library(source);
            let inner =
                hayagriva::io::from_biblatex_str(&sanitized).map_err(|fallback_errors| {
                    format!(
                        "{}\nnon-strict recovery failed:\n{}",
                        format_biblatex_errors(&errors),
                        format_biblatex_errors(&fallback_errors)
                    )
                })?;
            Ok(ParsedLibrary {
                inner,
                diagnostics: if diagnostics {
                    raw_diagnostics
                } else {
                    Vec::new()
                },
            })
        }
    }
}

fn format_biblatex_errors(errors: &[hayagriva::io::BibLaTeXError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn independent_style(id: String, style: CslStyle) -> PyResult<Style> {
    match style {
        CslStyle::Independent(inner) => Ok(Style {
            id,
            inner: Arc::new(inner),
        }),
        CslStyle::Dependent(_) => Err(PyValueError::new_err(
            "dependent CSL styles need explicit parent resolution",
        )),
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

fn parse_cite_group(items: &Bound<'_, PyAny>) -> PyResult<Vec<Cite>> {
    if let Ok(cite) = parse_single_cite(items) {
        return Ok(vec![cite]);
    }

    if let Ok(iter) = items.try_iter() {
        return iter.map(|item| parse_single_cite(&item?)).collect();
    }

    Err(PyTypeError::new_err(
        "citation items must be strings, Cite objects, or iterables of them",
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

fn citation_item<'a>(entry: &'a HayEntry, cite: &'a Cite) -> PyResult<CitationItem<'a, HayEntry>> {
    let locator = match cite.locator.as_deref() {
        Some(value) => {
            let label = cite.label.as_deref().unwrap_or("page");
            let locator = CslLocator::from_str(label)
                .map_err(|_| PyValueError::new_err(format!("unknown locator label {label:?}")))?;
            Some(SpecificLocator(locator, LocatorPayload::Str(value)))
        }
        None => None,
    };
    Ok(CitationItem::with_locator(entry, locator))
}

fn rendered_from_citation(citation: &RenderedCitation) -> PyResult<Rendered> {
    let text = elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?;
    let html = elem_children_to_html(&citation.citation)?;
    Ok(Rendered::new(
        text,
        html,
        RenderedTree::Citation(citation.citation.clone()),
    ))
}

fn rendered_from_bibliography(
    bibliography: Option<hayagriva::RenderedBibliography>,
) -> PyResult<Rendered> {
    let Some(bibliography) = bibliography else {
        return Ok(Rendered::new(
            String::new(),
            String::new(),
            RenderedTree::Empty,
        ));
    };

    let items = bibliography.items;
    let mut text = String::with_capacity(items.len() * 224);
    let mut html = String::with_capacity(items.len() * 384);
    for item in &items {
        if !text.is_empty() {
            text.push('\n');
        }
        write_bibliography_item_text(item, &mut text)?;

        render_bibliography_item_html(item, &mut html)
            .map_err(|err| RefkitError::new_err(err.to_string()))?;
    }

    Ok(Rendered::new(text, html, RenderedTree::Bibliography(items)))
}

fn elem_children_to_string(children: &ElemChildren, format: BufWriteFormat) -> PyResult<String> {
    let mut output = String::new();
    children
        .write_buf(&mut output, format)
        .map_err(|err| RefkitError::new_err(err.to_string()))?;
    Ok(output)
}

fn write_bibliography_item_text(
    item: &hayagriva::BibliographyItem,
    output: &mut String,
) -> PyResult<()> {
    let item_start = output.len();
    if let Some(first_field) = &item.first_field {
        first_field
            .write_buf(output, BufWriteFormat::Plain)
            .map_err(|err| RefkitError::new_err(err.to_string()))?;
    }

    let content = elem_children_to_string(&item.content, BufWriteFormat::Plain)?;
    if output.len() > item_start && !content.is_empty() {
        output.push(' ');
    }
    output.push_str(&content);
    Ok(())
}

fn elem_children_to_html(children: &ElemChildren) -> PyResult<String> {
    let mut output = String::new();
    render_children_html(children, &mut output)
        .map_err(|err| RefkitError::new_err(err.to_string()))?;
    Ok(output)
}

fn render_bibliography_item_html(
    item: &hayagriva::BibliographyItem,
    output: &mut String,
) -> fmt::Result {
    output.push_str("<div class=\"csl-entry\" data-key=\"");
    write_html_escaped(output, &item.key);
    output.push_str("\">");
    if let Some(first_field) = &item.first_field {
        output.push_str("<div class=\"csl-left-margin\">");
        render_child_html(first_field, output)?;
        output.push_str("</div><div class=\"csl-right-inline\">");
        render_children_html(&item.content, output)?;
        output.push_str("</div>");
    } else {
        render_children_html(&item.content, output)?;
    }
    output.push_str("</div>");
    Ok(())
}

fn render_children_html(children: &ElemChildren, output: &mut String) -> fmt::Result {
    for child in &children.0 {
        render_child_html(child, output)?;
    }
    Ok(())
}

fn render_child_html(child: &ElemChild, output: &mut String) -> fmt::Result {
    match child {
        ElemChild::Text(text) => render_formatted_html(text, output),
        ElemChild::Elem(elem) => render_elem_html(elem, output),
        ElemChild::Markup(value) => {
            write_html_escaped(output, value);
            Ok(())
        }
        ElemChild::Link { text, url } => {
            if let Some(href) = safe_href(url) {
                output.push_str("<a href=\"");
                write_html_escaped(output, href);
                output.push_str("\">");
                render_formatted_html(text, output)?;
                output.push_str("</a>");
            } else {
                render_formatted_html(text, output)?;
            }
            Ok(())
        }
        ElemChild::Transparent { .. } => Ok(()),
    }
}

fn render_elem_html(elem: &hayagriva::Elem, output: &mut String) -> fmt::Result {
    if let Some(display) = elem.display {
        let class_name = match display {
            Display::Block => "csl-block",
            Display::LeftMargin => "csl-left-margin",
            Display::RightInline => "csl-right-inline",
            Display::Indent => "csl-indent",
        };
        write!(output, "<div class=\"{class_name}\">")?;
    }

    render_children_html(&elem.children, output)?;

    if elem.display.is_some() {
        output.push_str("</div>");
    }
    Ok(())
}

fn render_formatted_html(text: &hayagriva::Formatted, output: &mut String) -> fmt::Result {
    let formatting = text.formatting;
    if formatting == hayagriva::Formatting::default() {
        write_html_escaped(output, &text.text);
        return Ok(());
    }

    let mut css = String::new();
    let mut suffix = String::new();

    match formatting.vertical_align {
        VerticalAlign::Sub => push_html_wrapper(output, &mut suffix, "<sub>", "</sub>"),
        VerticalAlign::Sup => push_html_wrapper(output, &mut suffix, "<sup>", "</sup>"),
        VerticalAlign::Baseline => {
            css.push_str("vertical-align: baseline;");
        }
        VerticalAlign::None => {}
    }

    match formatting.font_weight {
        FontWeight::Bold => {
            if text.text.chars().any(|c| !c.is_whitespace()) {
                push_html_wrapper(output, &mut suffix, "<b>", "</b>");
            }
        }
        FontWeight::Light => css.push_str("font-weight: lighter;"),
        FontWeight::Normal => {}
    }

    if formatting.font_style == FontStyle::Italic {
        push_html_wrapper(output, &mut suffix, "<i>", "</i>");
    }

    if formatting.font_variant == FontVariant::SmallCaps {
        css.push_str("font-variant: small-caps;");
    }

    if formatting.text_decoration == TextDecoration::Underline {
        push_html_wrapper(output, &mut suffix, "<u>", "</u>");
    }

    if !css.is_empty() {
        push_html_wrapper(
            output,
            &mut suffix,
            &format!("<span style=\"{css}\">"),
            "</span>",
        );
    }

    write_html_escaped(output, &text.text);
    output.push_str(&suffix);
    Ok(())
}

fn push_html_wrapper(output: &mut String, suffix: &mut String, start: &str, end: &str) {
    output.push_str(start);
    suffix.insert_str(0, end);
}

fn safe_href(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let scheme_end = trimmed.find(':')?;
    let first_path_marker = trimmed.find(['/', '?', '#']).unwrap_or(usize::MAX);
    if scheme_end > first_path_marker {
        return None;
    }

    let scheme = trimmed[..scheme_end].to_ascii_lowercase();
    match scheme.as_str() {
        "http" | "https" | "mailto" => Some(trimmed),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
enum TreeNode {
    Text {
        text: String,
        formatting: TreeFormatting,
    },
    Element {
        display: Option<String>,
        meta: Option<String>,
        children: Vec<TreeNode>,
    },
    Markup {
        value: String,
    },
    Link {
        text: String,
        url: String,
        formatting: TreeFormatting,
    },
    Transparent {
        cite_idx: usize,
        format: String,
    },
}

#[derive(Debug, Serialize)]
struct TreeFormatting {
    font_style: String,
    font_variant: String,
    font_weight: String,
    text_decoration: String,
    vertical_align: String,
}

fn children_to_tree(children: &ElemChildren) -> Vec<TreeNode> {
    children.0.iter().map(child_to_tree).collect()
}

fn bibliography_item_children_to_tree(item: &hayagriva::BibliographyItem) -> Vec<TreeNode> {
    let mut children = Vec::new();
    if let Some(first_field) = &item.first_field {
        children.push(child_to_tree(first_field));
    }
    children.extend(children_to_tree(&item.content));
    children
}

fn bibliography_item_to_tree(item: &hayagriva::BibliographyItem) -> serde_json::Value {
    let first_field = item.first_field.as_ref().map(child_to_tree);
    json!({
        "kind": "bibliography-entry",
        "key": &item.key,
        "first_field": first_field,
        "children": bibliography_item_children_to_tree(item),
    })
}

fn child_to_tree(child: &ElemChild) -> TreeNode {
    match child {
        ElemChild::Text(text) => TreeNode::Text {
            text: text.text.clone(),
            formatting: formatting_to_tree(text.formatting),
        },
        ElemChild::Elem(elem) => TreeNode::Element {
            display: elem.display.map(|display| format!("{display:?}")),
            meta: elem.meta.as_ref().map(|meta| format!("{meta:?}")),
            children: children_to_tree(&elem.children),
        },
        ElemChild::Markup(value) => TreeNode::Markup {
            value: value.clone(),
        },
        ElemChild::Link { text, url } => match safe_href(url) {
            Some(href) => TreeNode::Link {
                text: text.text.clone(),
                url: href.to_string(),
                formatting: formatting_to_tree(text.formatting),
            },
            None => TreeNode::Text {
                text: text.text.clone(),
                formatting: formatting_to_tree(text.formatting),
            },
        },
        ElemChild::Transparent { cite_idx, format } => TreeNode::Transparent {
            cite_idx: *cite_idx,
            format: format!("{format:?}"),
        },
    }
}

fn formatting_to_tree(formatting: hayagriva::Formatting) -> TreeFormatting {
    TreeFormatting {
        font_style: format!("{:?}", formatting.font_style),
        font_variant: format!("{:?}", formatting.font_variant),
        font_weight: format!("{:?}", formatting.font_weight),
        text_decoration: format!("{:?}", formatting.text_decoration),
        vertical_align: format!("{:?}", formatting.vertical_align),
    }
}

fn json_to_py(py: Python<'_>, value: &str) -> PyResult<Py<PyAny>> {
    let json = PyModule::import(py, "json")?;
    Ok(json.call_method1("loads", (value,))?.unbind())
}

fn json_value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(value) => value.into_py_any(py),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                value.into_py_any(py)
            } else if let Some(value) = value.as_u64() {
                value.into_py_any(py)
            } else if let Some(value) = value.as_f64() {
                value.into_py_any(py)
            } else {
                Ok(py.None())
            }
        }
        Value::String(value) => value.into_py_any(py),
        Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(json_value_to_py(py, value)?)?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, json_value_to_py(py, value)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

fn preview(value: &str) -> String {
    const LIMIT: usize = 60;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    let mut output: String = value.chars().take(LIMIT).collect();
    output.push_str("...");
    output
}

fn write_html_escaped(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
}

#[pymodule(gil_used = true)]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
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
    m.add_class::<Document>()?;
    m.add_class::<Rendered>()?;
    raw::register(m)?;
    Ok(())
}
