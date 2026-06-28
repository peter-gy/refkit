use std::fmt;
use std::ops::Range;

use indexmap::IndexMap;

mod edit;
mod parse;
mod sanitize;
#[cfg(test)]
mod tests;

use self::edit::{render_raw_document, set_raw_field_value};
use self::parse::parse_raw_document;
pub(crate) use self::sanitize::{
    remove_block_containing_span, sanitize_biblatex_for_library,
    sanitize_biblatex_for_library_literals,
};
use crate::quoted;

#[derive(Debug, Clone)]
pub struct RawFieldData {
    pub name: String,
    pub value: String,
    pub value_mode: RawValueMode,
    pub value_atoms: Vec<RawValueAtom>,
    pub span: Range<usize>,
    pub patch_span: Range<usize>,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawValueMode {
    Bare,
    Braced,
    Expression,
    Missing,
    Quoted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawValueAtom {
    pub value: String,
    pub value_mode: RawValueMode,
}

#[derive(Debug, Clone)]
pub struct RawEntryData {
    pub key: String,
    pub kind: String,
    pub fields: IndexMap<String, Vec<usize>>,
    pub field_blocks: Vec<RawFieldData>,
    pub span: Range<usize>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub enum RawBlock {
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
    pub fn span(&self) -> &Range<usize> {
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
pub struct RawDocumentData {
    pub blocks: Vec<RawBlock>,
    pub entries: IndexMap<String, Vec<usize>>,
    pub entry_blocks: Vec<RawEntryData>,
}

#[derive(Debug, Clone)]
pub struct RawDocument {
    data: RawDocumentData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawEntryId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawFieldId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxDocument {
    pub blocks: Vec<RawSyntaxBlock>,
    pub entries: Vec<RawSyntaxEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawSyntaxBlock {
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
        id: RawEntryId,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxEntry {
    pub id: RawEntryId,
    pub key: String,
    pub kind: String,
    pub fields: Vec<RawSyntaxField>,
    pub span: Range<usize>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxField {
    pub id: RawFieldId,
    pub name: String,
    pub value: String,
    pub value_mode: RawValueMode,
    pub value_atoms: Vec<RawValueAtom>,
    pub span: Range<usize>,
    pub patch_span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntryInfo {
    pub id: RawEntryId,
    pub key: String,
    pub kind: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFieldInfo {
    pub id: RawFieldId,
    pub name: String,
    pub value: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawBlockInfo {
    Whitespace {
        span: Range<usize>,
    },
    Comment {
        raw: String,
        span: Range<usize>,
    },
    Preamble {
        value: String,
        span: Range<usize>,
    },
    StringDef {
        key: String,
        value: String,
        span: Range<usize>,
    },
    Entry {
        id: RawEntryId,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawEditError {
    MissingField { entry_id: usize, field_id: usize },
    InvalidValue(String),
}

impl fmt::Display for RawEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { entry_id, field_id } => write!(
                f,
                "raw BibTeX field {} in entry {} is no longer available",
                field_id, entry_id
            ),
            Self::InvalidValue(message) => f.write_str(message),
        }
    }
}

impl RawDocument {
    pub fn parse(source: &str) -> Self {
        Self {
            data: parse_raw_document(source),
        }
    }

    pub fn entry_count(&self) -> usize {
        self.data.entries.len()
    }

    pub fn block_count(&self) -> usize {
        self.data.blocks.len()
    }

    pub fn entry_keys(&self) -> Vec<String> {
        self.data.entries.keys().cloned().collect()
    }

    pub fn contains_entry(&self, key: &str) -> bool {
        self.data.entries.contains_key(key)
    }

    pub fn unique_entry(&self, key: &str) -> Result<Option<RawEntryId>, String> {
        unique_entry_id(&self.data, key).map(|entry_id| entry_id.map(RawEntryId))
    }

    pub fn entry_occurrences(&self) -> Vec<RawEntryInfo> {
        self.data
            .entry_blocks
            .iter()
            .enumerate()
            .map(|(entry_id, entry)| entry_info(RawEntryId(entry_id), entry))
            .collect()
    }

    pub fn entries_for_key(&self, key: &str) -> Vec<RawEntryInfo> {
        self.data
            .entries
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|entry_id| self.entry_info(RawEntryId(*entry_id)))
            .collect()
    }

    pub fn entry_info(&self, entry_id: RawEntryId) -> Option<RawEntryInfo> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .map(|entry| entry_info(entry_id, entry))
    }

    pub fn field_keys(&self, entry_id: RawEntryId) -> Option<Vec<String>> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .map(|entry| entry.fields.keys().cloned().collect())
    }

    pub fn contains_field(&self, entry_id: RawEntryId, key: &str) -> bool {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .is_some_and(|entry| entry.fields.contains_key(&key.to_ascii_lowercase()))
    }

    pub fn unique_field(
        &self,
        entry_id: RawEntryId,
        key: &str,
    ) -> Result<Option<RawFieldId>, String> {
        let Some(entry) = self.data.entry_blocks.get(entry_id.0) else {
            return Ok(None);
        };
        unique_field_id(entry, &entry.key, &key.to_ascii_lowercase())
            .map(|field_id| field_id.map(RawFieldId))
    }

    pub fn field_occurrences(&self, entry_id: RawEntryId) -> Option<Vec<RawFieldInfo>> {
        self.data.entry_blocks.get(entry_id.0).map(|entry| {
            entry
                .field_blocks
                .iter()
                .enumerate()
                .map(|(field_id, field)| field_info(RawFieldId(field_id), field))
                .collect()
        })
    }

    pub fn fields_for_key(&self, entry_id: RawEntryId, key: &str) -> Option<Vec<RawFieldInfo>> {
        let entry = self.data.entry_blocks.get(entry_id.0)?;
        Some(
            entry
                .fields
                .get(&key.to_ascii_lowercase())
                .into_iter()
                .flatten()
                .filter_map(|field_id| {
                    entry
                        .field_blocks
                        .get(*field_id)
                        .map(|field| field_info(RawFieldId(*field_id), field))
                })
                .collect(),
        )
    }

    pub fn field_info(&self, entry_id: RawEntryId, field_id: RawFieldId) -> Option<RawFieldInfo> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .and_then(|entry| entry.field_blocks.get(field_id.0))
            .map(|field| field_info(field_id, field))
    }

    pub fn set_field_value(
        &mut self,
        entry_id: RawEntryId,
        field_id: RawFieldId,
        value: String,
    ) -> Result<(), RawEditError> {
        set_raw_field_value(&mut self.data, entry_id.0, field_id.0, value)
    }

    pub fn comments(&self) -> Vec<String> {
        self.data
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Comment { raw, .. } => Some(raw.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn preamble(&self) -> String {
        self.data
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Preamble { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" # ")
    }

    pub fn strings(&self) -> IndexMap<String, String> {
        self.data
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::StringDef { key, value, .. } if !key.is_empty() => {
                    Some((key.clone(), value.clone()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn failed_blocks(&self) -> Vec<RawBlockInfo> {
        self.data
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawBlock::Failed { .. } => Some(raw_block_info(block)),
                _ => None,
            })
            .collect()
    }

    pub fn blocks(&self) -> Vec<RawBlockInfo> {
        self.data.blocks.iter().map(raw_block_info).collect()
    }

    pub fn syntax(&self) -> RawSyntaxDocument {
        RawSyntaxDocument {
            blocks: self.data.blocks.iter().map(raw_syntax_block).collect(),
            entries: self
                .data
                .entry_blocks
                .iter()
                .enumerate()
                .map(|(entry_id, entry)| raw_syntax_entry(RawEntryId(entry_id), entry))
                .collect(),
        }
    }

    pub fn render(&self) -> Result<String, String> {
        render_raw_document(&self.data)
    }
}

pub fn normalize_raw_at_command(raw: &str) -> String {
    let Some(rest) = raw.strip_prefix('@') else {
        return raw.to_string();
    };
    let command_end = rest
        .char_indices()
        .find_map(|(index, ch)| {
            if ch == '{' || ch == '(' || ch.is_whitespace() {
                Some(index)
            } else {
                None
            }
        })
        .unwrap_or(rest.len());
    let command = rest[..command_end].to_ascii_lowercase();
    let tail = &rest[command_end..];
    let tail = match tail.trim_start().chars().next() {
        Some('{' | '(') => tail.trim_start(),
        _ => tail,
    };
    format!("@{command}{tail}")
}

impl RawEntryId {
    pub fn index(self) -> usize {
        self.0
    }
}

impl RawFieldId {
    pub fn index(self) -> usize {
        self.0
    }
}

fn entry_info(id: RawEntryId, entry: &RawEntryData) -> RawEntryInfo {
    RawEntryInfo {
        id,
        key: entry.key.clone(),
        kind: entry.kind.clone(),
        span: entry.span.clone(),
    }
}

fn field_info(id: RawFieldId, field: &RawFieldData) -> RawFieldInfo {
    RawFieldInfo {
        id,
        name: field.name.clone(),
        value: field.value.clone(),
        span: field.span.clone(),
    }
}

fn raw_block_info(block: &RawBlock) -> RawBlockInfo {
    match block {
        RawBlock::Whitespace { span, .. } => RawBlockInfo::Whitespace { span: span.clone() },
        RawBlock::Comment { raw, span } => RawBlockInfo::Comment {
            raw: raw.clone(),
            span: span.clone(),
        },
        RawBlock::Preamble { value, span, .. } => RawBlockInfo::Preamble {
            value: value.clone(),
            span: span.clone(),
        },
        RawBlock::StringDef {
            key, value, span, ..
        } => RawBlockInfo::StringDef {
            key: key.clone(),
            value: value.clone(),
            span: span.clone(),
        },
        RawBlock::Entry { id, key, span } => RawBlockInfo::Entry {
            id: RawEntryId(*id),
            key: key.clone(),
            span: span.clone(),
        },
        RawBlock::Failed { raw, error, span } => RawBlockInfo::Failed {
            raw: raw.clone(),
            error: error.clone(),
            span: span.clone(),
        },
        RawBlock::Other { raw, span } => RawBlockInfo::Other {
            raw: raw.clone(),
            span: span.clone(),
        },
    }
}

fn raw_syntax_block(block: &RawBlock) -> RawSyntaxBlock {
    match block {
        RawBlock::Whitespace { raw, span } => RawSyntaxBlock::Whitespace {
            raw: raw.clone(),
            span: span.clone(),
        },
        RawBlock::Comment { raw, span } => RawSyntaxBlock::Comment {
            raw: raw.clone(),
            span: span.clone(),
        },
        RawBlock::Preamble { raw, value, span } => RawSyntaxBlock::Preamble {
            raw: raw.clone(),
            value: value.clone(),
            span: span.clone(),
        },
        RawBlock::StringDef {
            raw,
            key,
            value,
            span,
        } => RawSyntaxBlock::StringDef {
            raw: raw.clone(),
            key: key.clone(),
            value: value.clone(),
            span: span.clone(),
        },
        RawBlock::Entry { id, key, span } => RawSyntaxBlock::Entry {
            id: RawEntryId(*id),
            key: key.clone(),
            span: span.clone(),
        },
        RawBlock::Failed { raw, error, span } => RawSyntaxBlock::Failed {
            raw: raw.clone(),
            error: error.clone(),
            span: span.clone(),
        },
        RawBlock::Other { raw, span } => RawSyntaxBlock::Other {
            raw: raw.clone(),
            span: span.clone(),
        },
    }
}

fn raw_syntax_entry(id: RawEntryId, entry: &RawEntryData) -> RawSyntaxEntry {
    RawSyntaxEntry {
        id,
        key: entry.key.clone(),
        kind: entry.kind.clone(),
        fields: entry
            .field_blocks
            .iter()
            .enumerate()
            .map(|(field_id, field)| RawSyntaxField {
                id: RawFieldId(field_id),
                name: field.name.clone(),
                value: field.value.clone(),
                value_mode: field.value_mode,
                value_atoms: field.value_atoms.clone(),
                span: field.span.clone(),
                patch_span: field.patch_span.clone(),
            })
            .collect(),
        span: entry.span.clone(),
        raw: entry.raw.clone(),
    }
}

pub fn unique_entry_id(doc: &RawDocumentData, key: &str) -> Result<Option<usize>, String> {
    let Some(entry_ids) = doc.entries.get(key) else {
        return Ok(None);
    };
    if entry_ids.len() == 1 {
        return Ok(entry_ids.first().copied());
    }
    Err(format!(
        "BibTeX entry key {} is ambiguous across {} occurrences; use entries.get_all(key) or entries.occurrences()",
        quoted(key),
        entry_ids.len()
    ))
}

pub fn unique_field_id(
    entry: &RawEntryData,
    entry_key: &str,
    key: &str,
) -> Result<Option<usize>, String> {
    let Some(field_ids) = entry.fields.get(key) else {
        return Ok(None);
    };
    if field_ids.len() == 1 {
        return Ok(field_ids.first().copied());
    }
    Err(format!(
        "BibTeX field {} in entry {} is ambiguous across {} occurrences; use fields.get_all(key) or fields.occurrences()",
        quoted(key),
        quoted(entry_key),
        field_ids.len()
    ))
}

fn is_valid_entry_key(value: &str) -> bool {
    value.chars().all(is_entry_key_char)
}

fn is_entry_key_char(ch: char) -> bool {
    !matches!(ch, '#' | '%' | '{' | '}' | '~' | '$' | ',')
        && !ch.is_control()
        && !ch.is_whitespace()
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(is_identifier_continue)
}

fn is_valid_field_name_char(ch: char) -> bool {
    !matches!(ch, '=' | ',' | '{' | '}' | '(' | ')' | '[' | ']') && (!ch.is_control() || ch == '\t')
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
