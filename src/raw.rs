use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use indexmap::IndexMap;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_json::json;

use crate::{CitecoreError, json_to_py};

#[derive(Debug, Clone)]
struct RawFieldData {
    name: String,
    value: String,
    value_mode: RawValueMode,
    span: Range<usize>,
    patch_span: Range<usize>,
    changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawValueMode {
    Bare,
    Braced,
    Expression,
    Quoted,
}

#[derive(Debug, Clone)]
struct RawEntryData {
    key: String,
    kind: String,
    fields: IndexMap<String, RawFieldData>,
    span: Range<usize>,
    raw: String,
}

#[derive(Debug, Clone)]
enum RawBlock {
    Whitespace {
        raw: String,
        span: Range<usize>,
    },
    Comment {
        raw: String,
        span: Range<usize>,
    },
    Preamble {
        raw: String,
        value: String,
        span: Range<usize>,
    },
    StringDef {
        raw: String,
        key: String,
        value: String,
        span: Range<usize>,
    },
    Entry {
        id: usize,
        key: String,
        span: Range<usize>,
    },
    Failed {
        raw: String,
        error: String,
        span: Range<usize>,
    },
    Other {
        raw: String,
        span: Range<usize>,
    },
}

impl RawBlock {
    fn span(&self) -> &Range<usize> {
        match self {
            Self::Whitespace { span, .. }
            | Self::Comment { span, .. }
            | Self::Preamble { span, .. }
            | Self::StringDef { span, .. }
            | Self::Entry { span, .. }
            | Self::Failed { span, .. }
            | Self::Other { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone)]
struct RawDocumentData {
    path: Option<PathBuf>,
    blocks: Vec<RawBlock>,
    entries: IndexMap<String, usize>,
    entry_blocks: Vec<RawEntryData>,
}

type SharedDocument = Rc<RefCell<RawDocumentData>>;

#[pyclass(module = "citecore", unsendable)]
pub struct BibDocument {
    doc: SharedDocument,
}

#[pymethods]
impl BibDocument {
    #[staticmethod]
    fn read(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        let parsed: Result<RawDocumentData, String> = py.detach(move || {
            let source =
                fs::read_to_string(&path).map_err(|err| format!("failed to read BibTeX: {err}"))?;
            let mut data = parse_raw_document(&source);
            data.path = Some(path);
            Ok(data)
        });
        let data = parsed.map_err(CitecoreError::new_err)?;
        Ok(Self {
            doc: Rc::new(RefCell::new(data)),
        })
    }

    #[staticmethod]
    fn parse(py: Python<'_>, source: String) -> Self {
        let data = py.detach(move || parse_raw_document(&source));
        Self {
            doc: Rc::new(RefCell::new(data)),
        }
    }

    #[getter]
    fn entries(&self) -> BibEntryMap {
        BibEntryMap {
            doc: Rc::clone(&self.doc),
        }
    }

    #[getter]
    fn comments(&self) -> Vec<String> {
        self.doc
            .borrow()
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Comment { raw, .. } => Some(raw.clone()),
                _ => None,
            })
            .collect()
    }

    #[getter]
    fn preamble(&self) -> String {
        self.doc
            .borrow()
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Preamble { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" # ")
    }

    #[getter]
    fn strings(&self) -> BTreeMap<String, String> {
        self.doc
            .borrow()
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::StringDef { key, value, .. } => Some((key.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    #[getter]
    fn failed_blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self
            .doc
            .borrow()
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Failed { raw, error, span } => Some(json!({
                    "kind": "failed",
                    "raw": raw,
                    "error": error,
                    "span": [span.start, span.end],
                })),
                _ => None,
            })
            .collect::<Vec<_>>();
        let payload = serde_json::to_string(&blocks)
            .map_err(|err| CitecoreError::new_err(err.to_string()))?;
        json_to_py(py, &payload)
    }

    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let blocks = self
            .doc
            .borrow()
            .blocks
            .iter()
            .map(block_to_json)
            .collect::<Vec<_>>();
        let payload = serde_json::to_string(&blocks)
            .map_err(|err| CitecoreError::new_err(err.to_string()))?;
        json_to_py(py, &payload)
    }

    #[pyo3(signature = (path = None))]
    fn write(&self, path: Option<PathBuf>) -> PyResult<()> {
        let target = path
            .or_else(|| self.doc.borrow().path.clone())
            .ok_or_else(|| PyValueError::new_err("write path is required"))?;
        let rendered = render_document(&self.doc.borrow())?;
        fs::write(&target, rendered)
            .map_err(|err| CitecoreError::new_err(format!("failed to write BibTeX: {err}")))?;
        Ok(())
    }

    fn __repr__(&self) -> String {
        let doc = self.doc.borrow();
        format!(
            "BibDocument({} entries, {} blocks)",
            doc.entries.len(),
            doc.blocks.len()
        )
    }
}

#[pyclass(module = "citecore", unsendable)]
pub struct BibEntryMap {
    doc: SharedDocument,
}

#[pymethods]
impl BibEntryMap {
    fn keys(&self) -> Vec<String> {
        self.doc.borrow().entries.keys().cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.doc.borrow().entries.is_empty()
    }

    fn get(&self, key: &str) -> Option<BibEntry> {
        if self.doc.borrow().entries.contains_key(key) {
            Some(BibEntry {
                doc: Rc::clone(&self.doc),
                key: key.to_string(),
            })
        } else {
            None
        }
    }

    fn __len__(&self) -> usize {
        self.doc.borrow().entries.len()
    }

    fn __bool__(&self) -> bool {
        !self.doc.borrow().entries.is_empty()
    }

    fn __contains__(&self, key: &str) -> bool {
        self.doc.borrow().entries.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibEntry> {
        if self.doc.borrow().entries.contains_key(key) {
            Ok(BibEntry {
                doc: Rc::clone(&self.doc),
                key: key.to_string(),
            })
        } else {
            Err(PyKeyError::new_err(key.to_string()))
        }
    }
}

#[pyclass(module = "citecore", unsendable)]
pub struct BibEntry {
    doc: SharedDocument,
    key: String,
}

#[pymethods]
impl BibEntry {
    #[getter]
    fn key(&self) -> String {
        self.key.clone()
    }

    #[getter]
    fn kind(&self) -> PyResult<String> {
        self.with_entry(|entry| entry.kind.clone())
    }

    #[getter]
    fn fields(&self) -> BibFieldMap {
        BibFieldMap {
            doc: Rc::clone(&self.doc),
            entry_key: self.key.clone(),
        }
    }

    #[getter]
    fn span(&self) -> PyResult<(usize, usize)> {
        self.with_entry(|entry| (entry.span.start, entry.span.end))
    }

    fn __repr__(&self) -> PyResult<String> {
        self.with_entry(|entry| format!("BibEntry(key={:?}, kind={:?})", entry.key, entry.kind))
    }
}

impl BibEntry {
    fn with_entry<T>(&self, f: impl FnOnce(&RawEntryData) -> T) -> PyResult<T> {
        let doc = self.doc.borrow();
        entry_by_key(&doc, &self.key)
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.key.clone()))
    }
}

#[pyclass(module = "citecore", unsendable)]
pub struct BibFieldMap {
    doc: SharedDocument,
    entry_key: String,
}

#[pymethods]
impl BibFieldMap {
    fn keys(&self) -> PyResult<Vec<String>> {
        self.with_entry(|entry| entry.fields.keys().cloned().collect())
    }

    fn is_empty(&self) -> PyResult<bool> {
        self.with_entry(|entry| entry.fields.is_empty())
    }

    fn get(&self, key: &str) -> PyResult<Option<BibField>> {
        let key = key.to_ascii_lowercase();
        if self.with_entry(|entry| entry.fields.contains_key(&key))? {
            Ok(Some(BibField {
                doc: Rc::clone(&self.doc),
                entry_key: self.entry_key.clone(),
                field_key: key,
            }))
        } else {
            Ok(None)
        }
    }

    fn __len__(&self) -> PyResult<usize> {
        self.with_entry(|entry| entry.fields.len())
    }

    fn __bool__(&self) -> PyResult<bool> {
        self.with_entry(|entry| !entry.fields.is_empty())
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        self.with_entry(|entry| entry.fields.contains_key(&key.to_ascii_lowercase()))
    }

    fn __getitem__(&self, key: &str) -> PyResult<BibField> {
        let key = key.to_ascii_lowercase();
        if self.with_entry(|entry| entry.fields.contains_key(&key))? {
            Ok(BibField {
                doc: Rc::clone(&self.doc),
                entry_key: self.entry_key.clone(),
                field_key: key,
            })
        } else {
            Err(PyKeyError::new_err(key))
        }
    }
}

impl BibFieldMap {
    fn with_entry<T>(&self, f: impl FnOnce(&RawEntryData) -> T) -> PyResult<T> {
        let doc = self.doc.borrow();
        entry_by_key(&doc, &self.entry_key)
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.entry_key.clone()))
    }
}

#[pyclass(module = "citecore", unsendable)]
pub struct BibField {
    doc: SharedDocument,
    entry_key: String,
    field_key: String,
}

#[pymethods]
impl BibField {
    #[getter]
    fn name(&self) -> PyResult<String> {
        self.with_field(|field| field.name.clone())
    }

    #[getter]
    fn value(&self) -> PyResult<String> {
        self.with_field(|field| field.value.clone())
    }

    #[setter]
    fn set_value(&self, value: String) -> PyResult<()> {
        let mut doc = self.doc.borrow_mut();
        let Some(entry_id) = doc.entries.get(&self.entry_key).copied() else {
            return Err(PyKeyError::new_err(self.entry_key.clone()));
        };
        let field = doc
            .entry_blocks
            .get_mut(entry_id)
            .and_then(|entry| entry.fields.get_mut(&self.field_key))
            .ok_or_else(|| PyKeyError::new_err(self.field_key.clone()))?;
        validate_field_value(&value, field.value_mode)?;
        field.value = value;
        field.changed = true;
        Ok(())
    }

    #[getter]
    fn span(&self) -> PyResult<(usize, usize)> {
        self.with_field(|field| (field.span.start, field.span.end))
    }

    fn __repr__(&self) -> PyResult<String> {
        self.with_field(|field| format!("BibField(name={:?}, value={:?})", field.name, field.value))
    }
}

impl BibField {
    fn with_field<T>(&self, f: impl FnOnce(&RawFieldData) -> T) -> PyResult<T> {
        let doc = self.doc.borrow();
        entry_by_key(&doc, &self.entry_key)
            .and_then(|entry| entry.fields.get(&self.field_key))
            .map(f)
            .ok_or_else(|| PyKeyError::new_err(self.field_key.clone()))
    }
}

fn entry_by_key<'a>(doc: &'a RawDocumentData, key: &str) -> Option<&'a RawEntryData> {
    doc.entries
        .get(key)
        .and_then(|entry_id| doc.entry_blocks.get(*entry_id))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<BibDocument>()?;
    module.add_class::<BibEntryMap>()?;
    module.add_class::<BibEntry>()?;
    module.add_class::<BibFieldMap>()?;
    module.add_class::<BibField>()?;
    Ok(())
}

pub(crate) fn sanitize_biblatex_for_library(source: &str) -> (String, Vec<String>) {
    let data = parse_raw_document(source);
    let mut output = String::with_capacity(source.len());
    let mut diagnostics = Vec::new();
    for block in &data.blocks {
        match block {
            RawBlock::Whitespace { raw, .. }
            | RawBlock::Comment { raw, .. }
            | RawBlock::Preamble { raw, .. }
            | RawBlock::StringDef { raw, .. } => output.push_str(raw),
            RawBlock::Entry { id, .. } => {
                if let Some(entry) = data.entry_blocks.get(*id) {
                    output.push_str(&entry.raw);
                }
            }
            RawBlock::Failed { error, span, .. } => {
                diagnostics.push(format!(
                    "ignored malformed BibTeX block at {}..{}: {}",
                    span.start, span.end, error
                ));
            }
            RawBlock::Other { raw, span } => {
                if !raw.trim().is_empty() {
                    diagnostics.push(format!(
                        "ignored raw BibTeX text at {}..{}",
                        span.start, span.end
                    ));
                }
            }
        }
    }
    (output, diagnostics)
}

fn parse_raw_document(source: &str) -> RawDocumentData {
    let mut blocks = Vec::new();
    let mut entries = IndexMap::new();
    let mut entry_blocks = Vec::new();
    let mut pos = 0;
    while pos < source.len() {
        let Some(ch) = source[pos..].chars().next() else {
            break;
        };

        if ch.is_whitespace() {
            let end = take_while(source, pos, char::is_whitespace);
            blocks.push(RawBlock::Whitespace {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        if ch == '%' {
            let end = take_line(source, pos);
            blocks.push(RawBlock::Comment {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        if ch != '@' {
            let end = source[pos + ch.len_utf8()..]
                .find(['@', '%'])
                .map(|offset| pos + ch.len_utf8() + offset)
                .unwrap_or(source.len());
            blocks.push(RawBlock::Other {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        let (mut block, parsed_entry, end) = parse_at_block(source, pos);
        if let Some(entry) = parsed_entry {
            let id = entry_blocks.len();
            if let RawBlock::Entry { id: block_id, .. } = &mut block {
                *block_id = id;
            }
            entries.insert(entry.key.clone(), id);
            entry_blocks.push(entry);
        }
        blocks.push(block);
        pos = end;
    }

    RawDocumentData {
        path: None,
        blocks,
        entries,
        entry_blocks,
    }
}

fn parse_at_block(source: &str, start: usize) -> (RawBlock, Option<RawEntryData>, usize) {
    match find_at_block_end(source, start) {
        Ok(end) => parse_complete_at_block(source, start, end),
        Err((end, error)) => (
            RawBlock::Failed {
                raw: source[start..end].to_string(),
                error,
                span: start..end,
            },
            None,
            end,
        ),
    }
}

fn parse_complete_at_block(
    source: &str,
    start: usize,
    end: usize,
) -> (RawBlock, Option<RawEntryData>, usize) {
    let raw = &source[start..end];
    let body_start = raw.find(['{', '(']).unwrap_or(raw.len());
    let kind = raw[1..body_start].trim().to_ascii_lowercase();
    let absolute_body_start = start + body_start + 1;
    let absolute_body_end = end.saturating_sub(1);
    let body = &source[absolute_body_start..absolute_body_end];

    if !is_valid_identifier(&kind) {
        return (
            RawBlock::Failed {
                raw: raw.to_string(),
                error: format!("entry type {kind:?} is invalid"),
                span: start..end,
            },
            None,
            end,
        );
    }

    match kind.as_str() {
        "comment" => (
            RawBlock::Comment {
                raw: raw.to_string(),
                span: start..end,
            },
            None,
            end,
        ),
        "preamble" => (
            RawBlock::Preamble {
                raw: raw.to_string(),
                value: parse_preamble_value(body),
                span: start..end,
            },
            None,
            end,
        ),
        "string" => {
            if let Some((key, value)) = parse_assignment(body) {
                (
                    RawBlock::StringDef {
                        raw: raw.to_string(),
                        key,
                        value,
                        span: start..end,
                    },
                    None,
                    end,
                )
            } else {
                (
                    RawBlock::Failed {
                        raw: raw.to_string(),
                        error: "string definition is missing '='".to_string(),
                        span: start..end,
                    },
                    None,
                    end,
                )
            }
        }
        _ => match parse_entry(source, start, end, &kind, absolute_body_start, body) {
            Ok(entry) => {
                let key = entry.key.clone();
                (
                    RawBlock::Entry {
                        id: usize::MAX,
                        key,
                        span: start..end,
                    },
                    Some(entry),
                    end,
                )
            }
            Err(error) => (
                RawBlock::Failed {
                    raw: raw.to_string(),
                    error,
                    span: start..end,
                },
                None,
                end,
            ),
        },
    }
}

fn parse_entry(
    source: &str,
    start: usize,
    end: usize,
    kind: &str,
    body_start: usize,
    body: &str,
) -> Result<RawEntryData, String> {
    let comma = body
        .find(',')
        .ok_or_else(|| "entry key is missing".to_string())?;
    let key = body[..comma].trim().to_string();
    if key.is_empty() {
        return Err("entry key is empty".to_string());
    }
    if !is_valid_entry_key(&key) {
        return Err(format!("entry key {key:?} is invalid"));
    }

    let mut fields = IndexMap::new();
    let mut cursor = comma + 1;
    while cursor < body.len() {
        skip_field_gap(body, &mut cursor);
        if cursor >= body.len() {
            break;
        }

        let name_start = cursor;
        while cursor < body.len() {
            let ch = body[cursor..].chars().next().unwrap();
            if ch == '=' || ch.is_whitespace() {
                break;
            }
            cursor += ch.len_utf8();
        }
        let name = body[name_start..cursor].trim().to_ascii_lowercase();
        if name.is_empty() {
            return Err("field name is empty".to_string());
        }
        if !is_valid_identifier(&name) {
            return Err(format!("field name {name:?} is invalid"));
        }

        while cursor < body.len() && body[cursor..].chars().next().unwrap().is_whitespace() {
            cursor += body[cursor..].chars().next().unwrap().len_utf8();
        }
        if !body[cursor..].starts_with('=') {
            return Err(format!("field {name} is missing '='"));
        }
        cursor += 1;
        while cursor < body.len() && body[cursor..].chars().next().unwrap().is_whitespace() {
            cursor += body[cursor..].chars().next().unwrap().len_utf8();
        }

        let value_start = cursor;
        let (value, value_end, inner_span, value_mode) = parse_value(body, cursor, body_start)?;
        cursor = value_end;
        fields.insert(
            name.clone(),
            RawFieldData {
                name: name.clone(),
                value,
                value_mode,
                span: inner_span.unwrap_or((body_start + value_start)..(body_start + value_end)),
                patch_span: (body_start + value_start)..(body_start + value_end),
                changed: false,
            },
        );

        skip_field_trivia(body, &mut cursor);
        if cursor < body.len() {
            let ch = body[cursor..].chars().next().unwrap();
            if ch != ',' {
                return Err(format!("field {name} is missing a separator"));
            }
            cursor += ch.len_utf8();
        }
    }

    Ok(RawEntryData {
        key,
        kind: kind.to_string(),
        fields,
        span: start..end,
        raw: source[start..end].to_string(),
    })
}

fn find_at_block_end(source: &str, start: usize) -> Result<usize, (usize, String)> {
    let next_block = find_recovery_block_start(source, start);
    let Some(open_rel) = source[start..].find(['{', '(']) else {
        return Err((
            next_block.unwrap_or_else(|| take_line(source, start)),
            "entry opener is missing".to_string(),
        ));
    };
    let open = start + open_rel;
    if let Some(next) = next_block
        && next < open
    {
        return Err((next, "entry opener is missing".to_string()));
    }
    let opener = source[open..].chars().next().unwrap();
    let root_closer = if opener == '{' { '}' } else { ')' };
    let kind = source[start + 1..open].trim().to_ascii_lowercase();
    let raw_comment = kind == "comment";
    let mut in_entry_key =
        is_valid_identifier(&kind) && !matches!(kind.as_str(), "comment" | "preamble" | "string");
    let mut closers = vec![root_closer];
    let mut in_quote = false;
    let mut quote_brace_depth = 0usize;
    let mut escaped = false;
    let mut pos = open + opener.len_utf8();

    while pos < source.len() {
        let ch = source[pos..].chars().next().unwrap();
        if in_entry_key {
            if ch == ',' {
                in_entry_key = false;
            } else if ch == root_closer {
                return Ok(pos + ch.len_utf8());
            }
            pos += ch.len_utf8();
            continue;
        }
        if in_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '{' {
                quote_brace_depth += 1;
            } else if ch == '}' && quote_brace_depth > 0 {
                quote_brace_depth -= 1;
            } else if ch == '"' && quote_brace_depth == 0 {
                in_quote = false;
            }
        } else if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '%' {
            pos = take_line(source, pos);
            continue;
        } else if ch == '"' && closers.len() == 1 && !raw_comment {
            in_quote = true;
            quote_brace_depth = 0;
        } else if ch == '{' {
            closers.push('}');
        } else if ch == '(' && closers.last() == Some(&')') {
            closers.push(')');
        } else if closers.last() == Some(&ch) {
            closers.pop();
            if closers.is_empty() {
                return Ok(pos + ch.len_utf8());
            }
        }
        pos += ch.len_utf8();
    }

    Err((
        next_block.unwrap_or(source.len()),
        "entry ended before closing delimiter".to_string(),
    ))
}

fn find_recovery_block_start(source: &str, start: usize) -> Option<usize> {
    let mut cursor = take_line(source, start);
    while cursor < source.len() {
        let line_start = cursor;
        while cursor < source.len() {
            let ch = source[cursor..].chars().next().unwrap();
            if !ch.is_whitespace() || ch == '\n' {
                break;
            }
            cursor += ch.len_utf8();
        }
        if source[cursor..].starts_with('@') {
            return Some(cursor);
        }
        cursor = take_line(source, line_start);
    }
    None
}

fn parse_value(
    body: &str,
    start: usize,
    body_offset: usize,
) -> Result<(String, usize, Option<Range<usize>>, RawValueMode), String> {
    let first = parse_value_atom(body, start, body_offset)?;
    let mut cursor = first.1;
    let mut expression_cursor = cursor;
    skip_field_space(body, &mut expression_cursor);
    if !body[expression_cursor..].starts_with('#') {
        return Ok(first);
    }

    while expression_cursor < body.len() && body[expression_cursor..].starts_with('#') {
        expression_cursor += 1;
        skip_field_space(body, &mut expression_cursor);
        let (_, atom_end, _, _) = parse_value_atom(body, expression_cursor, body_offset)?;
        cursor = atom_end;
        expression_cursor = atom_end;
        skip_field_space(body, &mut expression_cursor);
    }

    Ok((
        body[start..cursor].trim().to_string(),
        cursor,
        Some((body_offset + start)..(body_offset + cursor)),
        RawValueMode::Expression,
    ))
}

fn parse_value_atom(
    body: &str,
    start: usize,
    body_offset: usize,
) -> Result<(String, usize, Option<Range<usize>>, RawValueMode), String> {
    let Some(ch) = body[start..].chars().next() else {
        return Err("field value is missing".to_string());
    };

    if ch == '{' {
        let end = find_balanced_in_body(body, start, '{', '}')?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        return Ok((
            body[start + 1..end - 1].trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Braced,
        ));
    }

    if ch == '"' {
        let end = find_quoted_end(body, start)?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        return Ok((
            body[start + 1..end - 1].trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Quoted,
        ));
    }

    let mut cursor = start;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if ch.is_whitespace() || ch == ',' || ch == '#' || ch == '%' {
            break;
        }
        cursor += ch.len_utf8();
    }
    if cursor == start {
        return Err("field value is missing".to_string());
    }
    let value = body[start..cursor].trim().to_string();
    if !is_safe_bare_value(&value) {
        return Err(format!("bare field value {value:?} is invalid"));
    }
    Ok((value, cursor, None, RawValueMode::Bare))
}

fn find_balanced_in_body(
    body: &str,
    start: usize,
    opener: char,
    closer: char,
) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut escaped = false;
    let mut cursor = start;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '%' {
            cursor = take_line(body, cursor);
            continue;
        } else if ch == opener {
            depth += 1;
        } else if ch == closer {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok(cursor + ch.len_utf8());
            }
        }
        cursor += ch.len_utf8();
    }
    Err("field value ended before closing brace".to_string())
}

fn find_quoted_end(body: &str, start: usize) -> Result<usize, String> {
    let mut cursor = start + 1;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '{' {
            brace_depth += 1;
        } else if ch == '}' && brace_depth > 0 {
            brace_depth -= 1;
        } else if ch == '"' && brace_depth == 0 {
            return Ok(cursor + 1);
        }
        cursor += ch.len_utf8();
    }
    Err("field value ended before closing quote".to_string())
}

fn skip_field_gap(body: &str, cursor: &mut usize) {
    while *cursor < body.len() {
        let ch = body[*cursor..].chars().next().unwrap();
        if ch.is_whitespace() || ch == ',' {
            *cursor += ch.len_utf8();
            continue;
        }
        if ch == '%' {
            *cursor = take_line(body, *cursor);
            continue;
        }
        break;
    }
}

fn skip_field_space(body: &str, cursor: &mut usize) {
    while *cursor < body.len() {
        let ch = body[*cursor..].chars().next().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        *cursor += ch.len_utf8();
    }
}

fn skip_field_trivia(body: &str, cursor: &mut usize) {
    loop {
        skip_field_space(body, cursor);
        if *cursor < body.len() && body[*cursor..].starts_with('%') {
            *cursor = take_line(body, *cursor);
            continue;
        }
        break;
    }
}

fn parse_assignment(body: &str) -> Option<(String, String)> {
    let equals = body.find('=')?;
    let key = body[..equals].trim().to_ascii_lowercase();
    if !is_valid_identifier(&key) {
        return None;
    }
    let mut cursor = equals + 1;
    skip_field_space(body, &mut cursor);
    let (value, end, _, _) = parse_value(body, cursor, 0).ok()?;
    cursor = end;
    skip_field_trivia(body, &mut cursor);
    if cursor != body.len() {
        return None;
    }
    Some((key, value))
}

fn parse_preamble_value(body: &str) -> String {
    let mut cursor = 0;
    skip_field_trivia(body, &mut cursor);
    if let Ok((value, end, _, RawValueMode::Braced | RawValueMode::Quoted)) =
        parse_value(body, cursor, 0)
    {
        cursor = end;
        skip_field_trivia(body, &mut cursor);
        if cursor == body.len() {
            return value;
        }
    }
    body.trim().to_string()
}

fn render_document(data: &RawDocumentData) -> PyResult<String> {
    let mut output = String::with_capacity(
        data.blocks
            .last()
            .map(|block| block.span().end)
            .unwrap_or_default(),
    );
    for block in &data.blocks {
        match block {
            RawBlock::Whitespace { raw, .. }
            | RawBlock::Comment { raw, .. }
            | RawBlock::Preamble { raw, .. }
            | RawBlock::StringDef { raw, .. }
            | RawBlock::Failed { raw, .. }
            | RawBlock::Other { raw, .. } => output.push_str(raw),
            RawBlock::Entry { id, key, .. } => {
                let entry = data
                    .entry_blocks
                    .get(*id)
                    .ok_or_else(|| PyKeyError::new_err(key.clone()))?;
                if entry.fields.values().any(|field| field.changed) {
                    output.push_str(&patch_entry(entry)?);
                } else {
                    output.push_str(&entry.raw);
                }
            }
        }
    }
    Ok(output)
}

fn patch_entry(entry: &RawEntryData) -> PyResult<String> {
    let mut fields = entry
        .fields
        .values()
        .filter(|field| field.changed)
        .collect::<Vec<_>>();
    fields.sort_by_key(|field| patch_field_value(field).0.start);

    let mut output = String::with_capacity(entry.raw.len());
    let mut cursor = entry.span.start;
    for field in fields {
        let (span, value) = patch_field_value(field);
        if span.start < entry.span.start || span.end > entry.span.end || span.start < cursor {
            return Err(CitecoreError::new_err(format!(
                "invalid source span for BibTeX field {}",
                field.name
            )));
        }

        output.push_str(&entry.raw[cursor - entry.span.start..span.start - entry.span.start]);
        output.push_str(&value);
        cursor = span.end;
    }
    output.push_str(&entry.raw[cursor - entry.span.start..]);
    Ok(output)
}

fn patch_field_value(field: &RawFieldData) -> (Range<usize>, String) {
    if field.value_mode == RawValueMode::Quoted && contains_unescaped(&field.value, '"') {
        return (field.patch_span.clone(), format!("{{{}}}", field.value));
    }
    (field.span.clone(), render_field_value(field))
}

fn render_field_value(field: &RawFieldData) -> String {
    match field.value_mode {
        RawValueMode::Bare if !is_safe_bare_value(&field.value) => {
            format!("{{{}}}", field.value)
        }
        RawValueMode::Expression => format!("{{{}}}", field.value),
        RawValueMode::Bare | RawValueMode::Braced | RawValueMode::Quoted => field.value.clone(),
    }
}

fn validate_field_value(value: &str, value_mode: RawValueMode) -> PyResult<()> {
    match value_mode {
        RawValueMode::Bare if is_safe_bare_value(value) => Ok(()),
        RawValueMode::Bare | RawValueMode::Braced | RawValueMode::Expression => {
            validate_braced_field_value(value)
        }
        RawValueMode::Quoted => validate_quoted_field_value(value),
    }
}

fn validate_braced_field_value(value: &str) -> PyResult<()> {
    if value.contains('\n')
        || contains_unescaped(value, '%')
        || ends_with_unescaped_backslash(value)
    {
        return Err(PyValueError::new_err(
            "BibTeX field value contains an unsafe braced delimiter",
        ));
    }
    if !has_balanced_unescaped_braces(value) {
        return Err(PyValueError::new_err(
            "BibTeX field value contains an unsafe braced delimiter",
        ));
    }
    Ok(())
}

fn validate_quoted_field_value(value: &str) -> PyResult<()> {
    if value.contains('\n')
        || contains_unprotected_unescaped_quote(value)
        || contains_unescaped(value, '%')
        || ends_with_unescaped_backslash(value)
    {
        return Err(PyValueError::new_err(
            "BibTeX field value contains an unsafe quoted delimiter",
        ));
    }
    if !has_balanced_unescaped_braces(value) {
        return Err(PyValueError::new_err(
            "BibTeX field value contains an unsafe quoted delimiter",
        ));
    }
    Ok(())
}

fn contains_unprotected_unescaped_quote(value: &str) -> bool {
    let mut depth = 0usize;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' && depth > 0 {
            depth -= 1;
        } else if ch == '"' && depth == 0 {
            return true;
        }
    }
    false
}

fn has_balanced_unescaped_braces(value: &str) -> bool {
    let mut depth = 0usize;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            let Some(next_depth) = depth.checked_sub(1) else {
                return false;
            };
            depth = next_depth;
        }
    }
    depth == 0
}

fn contains_unescaped(value: &str, target: char) -> bool {
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == target {
            return true;
        }
    }
    false
}

fn ends_with_unescaped_backslash(value: &str) -> bool {
    let mut count = 0usize;
    for ch in value.chars().rev() {
        if ch == '\\' {
            count += 1;
        } else {
            break;
        }
    }
    count % 2 == 1
}

fn is_valid_entry_key(value: &str) -> bool {
    value.chars().all(is_entry_key_char)
}

fn is_entry_key_char(ch: char) -> bool {
    !matches!(ch, ',' | '}') && !ch.is_control() && !ch.is_whitespace()
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(is_identifier_continue)
}

fn is_identifier_start(ch: char) -> bool {
    !matches!(ch, ':' | '<' | '-' | '>') && is_identifier_continue(ch)
}

fn is_identifier_continue(ch: char) -> bool {
    !matches!(
        ch,
        '@' | '{' | '}' | '"' | '#' | '\'' | '(' | ')' | ',' | '=' | '%' | '\\' | '~'
    ) && !ch.is_control()
        && !ch.is_whitespace()
}

fn is_safe_bare_value(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.' | '/'))
}

fn block_to_json(block: &RawBlock) -> serde_json::Value {
    match block {
        RawBlock::Whitespace { span, .. } => {
            json!({"kind": "whitespace", "span": [span.start, span.end]})
        }
        RawBlock::Comment { raw, span } => {
            json!({"kind": "comment", "raw": raw, "span": [span.start, span.end]})
        }
        RawBlock::Preamble { value, span, .. } => {
            json!({"kind": "preamble", "value": value, "span": [span.start, span.end]})
        }
        RawBlock::StringDef {
            key, value, span, ..
        } => {
            json!({"kind": "string", "key": key, "value": value, "span": [span.start, span.end]})
        }
        RawBlock::Entry { id, key, span } => {
            json!({"kind": "entry", "id": id, "key": key, "span": [span.start, span.end]})
        }
        RawBlock::Failed { raw, error, span } => {
            json!({"kind": "failed", "raw": raw, "error": error, "span": [span.start, span.end]})
        }
        RawBlock::Other { raw, span } => {
            json!({"kind": "other", "raw": raw, "span": [span.start, span.end]})
        }
    }
}

fn take_while(source: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut cursor = start;
    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if !predicate(ch) {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn take_line(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(source.len())
}
