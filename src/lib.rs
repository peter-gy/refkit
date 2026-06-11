mod public_strings;
mod raw;
mod rendered;
mod style_analysis;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use biblatex::{
    Bibliography as BiblatexBibliography, ChunksExt, Entry as BiblatexEntry,
    TypeError as BiblatexTypeError,
};
use hayagriva::citationberg::taxonomy::Locator as CslLocator;
use hayagriva::citationberg::{
    IndependentStyle, Locale as CslLocale, LocaleCode, Style as CslStyle,
};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, BufWriteFormat, CitationItem, CitationRequest,
    Entry as HayEntry, Library as HayLibrary, LocatorPayload, Selector, SpecificLocator, archive,
    standalone_citation,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyDict, PyDictMethods, PyList, PyListMethods, PyModule};
use pyo3::{IntoPyObjectExt, intern};
use serde_json::{Value, json};

use public_strings::{entry_type_name, option_quoted, quoted};
use rendered::{
    Rendered, RenderedTree, elem_children_to_html, elem_children_to_string,
    rendered_from_bibliography, rendered_from_citation,
};
use style_analysis::{
    can_fast_render_single_citations, citation_depends_on_subsequent_names, citation_only_style,
    full_history_citation_style,
};

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

pub(crate) struct SourceText {
    pub(crate) source: String,
    pub(crate) diagnostic: Option<String>,
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
        let entry_type = entry_type_name(inner.entry_type()).to_string();
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

pub(crate) fn read_bibliography_text(path: &PathBuf) -> Result<SourceText, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(source) => Ok(SourceText {
            source,
            diagnostic: None,
        }),
        Err(err) => Ok(SourceText {
            source: decode_windows_1252(&err.into_bytes()),
            diagnostic: Some(format!(
                "decoded {} as Windows-1252-compatible text because it is not valid UTF-8",
                path.display()
            )),
        }),
    }
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x80 => '\u{20ac}',
            0x82 => '\u{201a}',
            0x83 => '\u{0192}',
            0x84 => '\u{201e}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02c6}',
            0x89 => '\u{2030}',
            0x8a => '\u{0160}',
            0x8b => '\u{2039}',
            0x8c => '\u{0152}',
            0x8e => '\u{017d}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201c}',
            0x94 => '\u{201d}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02dc}',
            0x99 => '\u{2122}',
            0x9a => '\u{0161}',
            0x9b => '\u{203a}',
            0x9c => '\u{0153}',
            0x9e => '\u{017e}',
            0x9f => '\u{0178}',
            0x81 | 0x8d | 0x8f | 0x90 | 0x9d => '\u{fffd}',
            _ => char::from(*byte),
        })
        .collect()
}

#[pymethods]
impl Library {
    #[staticmethod]
    #[pyo3(signature = (path, strict = false, diagnostics = false))]
    fn read(py: Python<'_>, path: PathBuf, strict: bool, diagnostics: bool) -> PyResult<Self> {
        let parsed = py.detach(move || parse_library_path(path, strict, diagnostics));
        parsed.map(Self::from_parsed).map_err(RefkitError::new_err)
    }

    #[staticmethod]
    #[pyo3(signature = (source, format = "bibtex", strict = false, diagnostics = false))]
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
            "Entry(key={}, type={})",
            quoted(&self.data.key),
            quoted(&self.data.entry_type)
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
        let archived =
            archive::ArchivedStyle::by_name(&name.to_ascii_lowercase()).ok_or_else(|| {
                PyValueError::new_err(format!("unknown bundled style {}", quoted(name)))
            })?;
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
        format!(
            "Style(id={}, title={})",
            quoted(&self.id),
            quoted(&self.title())
        )
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
            "Cite(key={}, locator={}, label={})",
            quoted(&self.key),
            option_quoted(self.locator.as_deref()),
            option_quoted(self.label.as_deref())
        )
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
            "unsupported projection field {}",
            quoted(field)
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
    citation_style: Arc<IndependentStyle>,
    standalone_style: Arc<IndependentStyle>,
    locale: Option<String>,
    citations: Vec<Vec<Cite>>,
    fast_cite: FastCitationState,
}

#[derive(Clone)]
struct FastCitationState {
    enabled: bool,
    key_by_text: HashMap<String, String>,
    seen_keys: HashSet<String>,
    subsequent_name_rules: bool,
}

impl FastCitationState {
    fn new(style: &IndependentStyle) -> Self {
        Self {
            enabled: can_fast_render_single_citations(style),
            key_by_text: HashMap::new(),
            seen_keys: HashSet::new(),
            subsequent_name_rules: citation_depends_on_subsequent_names(style),
        }
    }
}

#[pymethods]
impl Document {
    #[new]
    #[pyo3(signature = (library, style, locale = None))]
    fn new(library: &Library, style: &Style, locale: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let citation_style = full_history_citation_style(style.inner.as_ref())
            .map(Arc::new)
            .unwrap_or_else(|| Arc::clone(&style.inner));
        let standalone_style = Arc::new(citation_only_style(style.inner.as_ref()));
        Ok(Self {
            library: Arc::clone(&library.inner),
            style: Arc::clone(&style.inner),
            citation_style,
            standalone_style,
            locale: extract_locale(locale)?,
            citations: Vec::new(),
            fast_cite: FastCitationState::new(style.inner.as_ref()),
        })
    }

    fn cite(&mut self, py: Python<'_>, items: &Bound<'_, PyAny>) -> PyResult<Rendered> {
        let group = parse_cite_group(items)?;
        py.detach(|| self.cite_group(group))
    }

    #[pyo3(signature = (all = false))]
    fn bibliography(&self, py: Python<'_>, all: bool) -> PyResult<Rendered> {
        py.detach(|| self.render_bibliography(all))
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
    fn cite_group(&mut self, group: Vec<Cite>) -> PyResult<Rendered> {
        let fast_cite = self.fast_cite.clone();
        self.citations.push(group);
        match self.render_appended_citation() {
            Ok(rendered) => Ok(rendered),
            Err(err) => {
                self.citations.pop();
                self.fast_cite = fast_cite;
                Err(err)
            }
        }
    }

    fn render_appended_citation(&mut self) -> PyResult<Rendered> {
        if let Some(rendered) = self.try_render_fast_citation()? {
            return Ok(rendered);
        }
        self.render_latest_citation()
    }

    fn try_render_fast_citation(&mut self) -> PyResult<Option<Rendered>> {
        if !self.fast_cite.enabled {
            return Ok(None);
        }

        let Some(group) = self.citations.last() else {
            return Ok(None);
        };
        let [cite] = group.as_slice() else {
            self.fast_cite.enabled = false;
            return Ok(None);
        };
        if cite.locator.is_some() {
            self.fast_cite.enabled = false;
            return Ok(None);
        }

        let entry = self.library.get(&cite.key).ok_or_else(|| {
            MissingReferenceError::new_err(format!("missing reference {}", cite.key))
        })?;
        if self.fast_cite.subsequent_name_rules && self.fast_cite.seen_keys.contains(&cite.key) {
            return Ok(None);
        }

        let locale = self.locale.as_ref().map(|code| LocaleCode(code.clone()));
        let children = standalone_citation(CitationRequest::new(
            vec![citation_item(entry, cite)?],
            self.standalone_style.as_ref(),
            locale,
            bundled_locales(),
            None,
        ));
        let text = elem_children_to_string(&children, BufWriteFormat::Plain)?;

        match self.fast_cite.key_by_text.get(&text) {
            Some(existing_key) if existing_key != &cite.key => {
                self.fast_cite.enabled = false;
                Ok(None)
            }
            _ => {
                self.fast_cite
                    .key_by_text
                    .insert(text.clone(), cite.key.clone());
                self.fast_cite.seen_keys.insert(cite.key.clone());
                let html = elem_children_to_html(&children)?;
                Ok(Some(Rendered::new(
                    text,
                    html,
                    RenderedTree::Citation(children),
                )))
            }
        }
    }

    fn render_latest_citation(&self) -> PyResult<Rendered> {
        let rendered = match self.render_with_style(false, self.citation_style.as_ref()) {
            Ok(rendered) => rendered,
            Err(err) => {
                return Err(err);
            }
        };
        let Some(citation) = rendered.citations.last() else {
            return Err(RefkitError::new_err(
                "citation renderer returned no citations",
            ));
        };
        rendered_from_citation(citation)
    }

    fn render_bibliography(&self, all: bool) -> PyResult<Rendered> {
        let rendered = self.render_all(all)?;
        rendered_from_bibliography(rendered.bibliography)
    }
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
    let text = read_bibliography_text(&path)?;
    let mut parsed = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bib") => parse_library_source(&text.source, "bibtex", strict, diagnostics),
        Some("yaml" | "yml") => parse_library_source(&text.source, "yaml", strict, diagnostics),
        Some(ext) => Err(format!(
            "unsupported bibliography extension {}",
            quoted(ext)
        )),
        None => Err("bibliography path has no extension".to_string()),
    }?;
    if diagnostics && let Some(diagnostic) = text.diagnostic {
        parsed.diagnostics.insert(0, diagnostic);
    }
    Ok(parsed)
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
        other => Err(format!("unsupported bibliography format {}", quoted(other))),
    }
}

fn parse_biblatex_library(
    source: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    if !strict {
        return recover_biblatex_library(source, diagnostics);
    }

    match hayagriva::io::from_biblatex_str(source) {
        Ok(inner) => Ok(ParsedLibrary {
            inner,
            diagnostics: Vec::new(),
        }),
        Err(errors) => Err(format_biblatex_errors(&errors)),
    }
}

fn recover_biblatex_library(source: &str, diagnostics: bool) -> Result<ParsedLibrary, String> {
    let (sanitized, mut recovery_diagnostics) =
        raw::sanitize_biblatex_for_library(source, false, diagnostics);
    let mut bibliography =
        recover_biblatex_syntax(&sanitized, &mut recovery_diagnostics, diagnostics)
            .map_err(|err| format_recovery_failure(&err))?;
    sanitize_biblatex_typed_fields(&mut bibliography, &mut recovery_diagnostics, diagnostics);
    let inner =
        convert_biblatex_with_recovery(&bibliography, &mut recovery_diagnostics, diagnostics)
            .map_err(|err| format_recovery_failure(&err))?;

    Ok(ParsedLibrary {
        inner,
        diagnostics: if diagnostics {
            recovery_diagnostics
        } else {
            Vec::new()
        },
    })
}

fn format_recovery_failure(recovery_error: &str) -> String {
    format!("non-strict recovery failed:\n{recovery_error}")
}

fn sanitize_biblatex_typed_fields(
    bibliography: &mut BiblatexBibliography,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) {
    for entry in bibliography.iter_mut() {
        remove_field_if(entry, diagnostics, collect_diagnostics, "month", |value| {
            is_valid_month_field(value)
        });
        remove_field_if(entry, diagnostics, collect_diagnostics, "year", |value| {
            is_valid_year_field(value)
        });
        remove_field_if(entry, diagnostics, collect_diagnostics, "day", |value| {
            is_valid_day_field(value)
        });
        for field in ["endyear", "endmonth", "endday"] {
            remove_field_if(entry, diagnostics, collect_diagnostics, field, |value| {
                field.ends_with("year") && is_valid_year_field(value)
                    || field.ends_with("month") && is_valid_month_field(value)
                    || field.ends_with("day") && is_valid_day_field(value)
            });
        }
    }
}

fn remove_field_if(
    entry: &mut BiblatexEntry,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
    field: &str,
    is_valid: impl FnOnce(&str) -> bool,
) {
    let Some(value) = entry
        .fields
        .get(field)
        .map(|chunks| chunks.format_verbatim())
    else {
        return;
    };
    if is_valid(value.trim()) {
        return;
    }
    entry.fields.remove(field);
    if collect_diagnostics {
        diagnostics.push(format!(
            "ignored BibTeX field {} in entry {} because value {} is not valid for normalization",
            quoted(field),
            quoted(&entry.key),
            quoted(value.trim())
        ));
    }
}

fn is_valid_year_field(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    (1..=4).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_valid_month_field(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "jan"
            | "january"
            | "feb"
            | "february"
            | "mar"
            | "march"
            | "apr"
            | "april"
            | "may"
            | "jun"
            | "june"
            | "jul"
            | "july"
            | "aug"
            | "august"
            | "sep"
            | "september"
            | "oct"
            | "october"
            | "nov"
            | "november"
            | "dec"
            | "december"
    ) || normalized
        .parse::<u8>()
        .is_ok_and(|month| (1..=12).contains(&month))
}

fn is_valid_day_field(value: &str) -> bool {
    value.parse::<u8>().is_ok_and(|day| (1..=31).contains(&day))
}

fn recover_biblatex_syntax(
    source: &str,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<BiblatexBibliography, String> {
    const MAX_SYNTAX_RECOVERY_PASSES: usize = 16;
    let mut candidate = source.to_string();
    match BiblatexBibliography::parse(&candidate) {
        Ok(bibliography) => return Ok(bibliography),
        Err(first_err) => {
            let (validated, validation_diagnostics) =
                raw::sanitize_biblatex_for_library(&candidate, true, collect_diagnostics);
            if validated != candidate {
                diagnostics.extend(validation_diagnostics);
                candidate = validated;
                if let Ok(bibliography) = BiblatexBibliography::parse(&candidate) {
                    return Ok(bibliography);
                }
            } else if collect_diagnostics {
                diagnostics.push(format!(
                    "syntax recovery could not pre-filter BibTeX entries after parse error: {first_err}"
                ));
            }

            let (literal, literal_diagnostics) =
                raw::sanitize_biblatex_for_library_literals(&candidate, collect_diagnostics);
            if literal != candidate {
                diagnostics.extend(literal_diagnostics);
                candidate = literal;
                if let Ok(bibliography) = BiblatexBibliography::parse(&candidate) {
                    return Ok(bibliography);
                }
            }
        }
    }

    for _ in 0..MAX_SYNTAX_RECOVERY_PASSES {
        match BiblatexBibliography::parse(&candidate) {
            Ok(bibliography) => return Ok(bibliography),
            Err(err) => {
                let Some((next, diagnostic)) =
                    raw::remove_block_containing_span(&candidate, err.span.clone())
                else {
                    return Err(format!("biblatex parse error: {err}"));
                };
                if collect_diagnostics {
                    diagnostics.push(format!(
                        "ignored BibTeX block during syntax recovery because {err}: {diagnostic}"
                    ));
                }
                if next.len() >= candidate.len() {
                    return Err(format!("biblatex parse error did not make progress: {err}"));
                }
                candidate = next;
            }
        }
    }

    Err(format!(
        "biblatex syntax recovery exceeded {MAX_SYNTAX_RECOVERY_PASSES} passes"
    ))
}

fn convert_biblatex_with_recovery(
    bibliography: &BiblatexBibliography,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<HayLibrary, String> {
    if let Ok(inner) = hayagriva::io::from_biblatex(bibliography) {
        return Ok(inner);
    }

    let mut converted = Vec::with_capacity(bibliography.len());
    for entry in bibliography.iter() {
        if let Some(entry) =
            convert_biblatex_entry_with_recovery(entry, diagnostics, collect_diagnostics)?
        {
            converted.push(entry);
        }
    }
    Ok(converted.into_iter().collect())
}

fn convert_biblatex_entry_with_recovery(
    source_entry: &BiblatexEntry,
    diagnostics: &mut Vec<String>,
    collect_diagnostics: bool,
) -> Result<Option<HayEntry>, String> {
    const MAX_FIELD_RECOVERY_PASSES: usize = 64;
    let mut entry = source_entry.clone();
    let mut removed_fields = HashSet::new();

    for _ in 0..MAX_FIELD_RECOVERY_PASSES {
        match HayEntry::try_from(&entry) {
            Ok(entry) => return Ok(Some(entry)),
            Err(err) => {
                let Some(field) = field_for_type_error(&entry, &err, &removed_fields) else {
                    if collect_diagnostics {
                        diagnostics.push(format!(
                            "ignored BibTeX entry {} because type recovery failed: {err}",
                            quoted(&entry.key)
                        ));
                    }
                    return Ok(None);
                };
                if collect_diagnostics {
                    diagnostics.push(format!(
                        "ignored BibTeX field {} in entry {} because type conversion failed: {err}",
                        quoted(&field),
                        quoted(&entry.key)
                    ));
                }
                entry.fields.remove(&field);
                removed_fields.insert(field);
            }
        }
    }

    Err(format!(
        "biblatex type recovery exceeded {MAX_FIELD_RECOVERY_PASSES} passes for entry {}",
        quoted(&source_entry.key)
    ))
}

fn field_for_type_error(
    entry: &BiblatexEntry,
    err: &BiblatexTypeError,
    removed_fields: &HashSet<String>,
) -> Option<String> {
    field_containing_span(entry, err.span.clone(), removed_fields).or_else(|| {
        TYPED_RECOVERY_FIELDS
            .iter()
            .find(|field| entry.fields.contains_key(**field) && !removed_fields.contains(**field))
            .map(|field| (*field).to_string())
    })
}

fn field_containing_span(
    entry: &BiblatexEntry,
    span: Range<usize>,
    removed_fields: &HashSet<String>,
) -> Option<String> {
    entry
        .fields
        .iter()
        .filter(|(field, _)| !removed_fields.contains(*field))
        .find_map(|(field, chunks)| {
            chunks
                .iter()
                .any(|chunk| range_contains(&chunk.span, &span))
                .then(|| field.clone())
        })
}

fn range_contains(container: &Range<usize>, inner: &Range<usize>) -> bool {
    container.start <= inner.start && inner.end <= container.end
}

const TYPED_RECOVERY_FIELDS: &[&str] = &[
    "date",
    "year",
    "month",
    "day",
    "endyear",
    "endmonth",
    "endday",
    "origdate",
    "urldate",
    "eventdate",
    "edition",
    "volume",
    "volumes",
    "number",
    "issue",
    "pages",
    "pagetotal",
    "pagination",
    "language",
    "langid",
    "gender",
    "editortype",
    "editoratype",
    "editorbtype",
    "editorctype",
];

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
            let locator = CslLocator::from_str(label).map_err(|_| {
                PyValueError::new_err(format!("unknown locator label {}", quoted(label)))
            })?;
            Some(SpecificLocator(locator, LocatorPayload::Str(value)))
        }
        None => None,
    };
    Ok(CitationItem::with_locator(entry, locator))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_group_citation_restores_fast_cite_state() {
        let mut document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        assert!(document.fast_cite.enabled);
        assert!(
            document
                .cite_group(vec![test_cite("valid"), test_cite("missing")])
                .is_err()
        );

        assert!(document.citations.is_empty());
        assert!(document.fast_cite.enabled);

        let rendered = document.cite_group(vec![test_cite("valid")]).unwrap();
        assert!(rendered.text.contains("Doe"));
    }

    fn archived_independent_style(name: &str) -> IndependentStyle {
        let style = archive::ArchivedStyle::by_name(name)
            .unwrap_or_else(|| panic!("missing archived style {name}"))
            .get();
        match style {
            CslStyle::Independent(style) => style,
            CslStyle::Dependent(_) => panic!("expected independent style {name}"),
        }
    }

    fn test_document(style_name: &str, source: &str) -> Document {
        let parsed = parse_library_source(source, "bibtex", true, false).unwrap();
        let style = Arc::new(archived_independent_style(style_name));
        let citation_style = full_history_citation_style(style.as_ref())
            .map(Arc::new)
            .unwrap_or_else(|| Arc::clone(&style));
        Document {
            library: Arc::new(parsed.inner),
            style: Arc::clone(&style),
            citation_style,
            standalone_style: Arc::new(citation_only_style(style.as_ref())),
            locale: Some("en-US".to_string()),
            citations: Vec::new(),
            fast_cite: FastCitationState::new(style.as_ref()),
        }
    }

    fn test_cite(key: &str) -> Cite {
        Cite {
            key: key.to_string(),
            locator: None,
            label: None,
        }
    }
}
