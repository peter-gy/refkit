use std::collections::BTreeMap;
use std::ops::Range;

use indexmap::IndexMap;

mod edit;
mod parse;
mod patch;
mod resolve;
mod sanitize;
#[cfg(test)]
mod tests;

use self::edit::render_raw_document;
use self::parse::parse_raw_document;
pub(crate) use self::sanitize::sanitize_biblatex_for_library;
use crate::quoted;
pub use patch::{
    BibEdit, BibEntryMapping, BibFieldMapping, BibFieldValue, BibPatchChange, BibPatchError,
    BibPatchErrorCode, BibPatchKind, BibPatchResult, BibPatchWarning,
};

#[derive(Debug, Clone)]
pub struct RawFieldData {
    pub name: String,
    pub value: String,
    pub value_mode: RawValueMode,
    pub value_atoms: Vec<RawValueAtom>,
    pub span: Range<usize>,
    pub patch_span: Range<usize>,
    pub assignment_span: Range<usize>,
    pub comma: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawValueMode {
    Bare,
    Braced,
    Expression,
    Missing,
    Quoted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawValueAtom {
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
    pub key_span: Range<usize>,
    pub kind_span: Range<usize>,
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
/// Immutable, occurrence-preserving BibTeX syntax snapshot.
pub struct RawDocument {
    data: RawDocumentData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Effective source fields after bounded macro and inheritance resolution.
pub struct ResolvedBibEntry {
    /// Original entry key.
    pub key: String,
    /// Source entry kind.
    pub entry_type: String,
    /// Effective field names and resolved TeX text, ordered by field name.
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
/// Source-order entry occurrence identity, meaningful only within its snapshot.
pub struct RawEntryId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
/// Field occurrence identity relative to one entry in one source snapshot.
pub struct RawFieldId(usize);

impl<'de> serde::Deserialize<'de> for RawEntryId {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        <u32 as serde::Deserialize>::deserialize(decoder).map(|id| Self(id as usize))
    }
}

impl<'de> serde::Deserialize<'de> for RawFieldId {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        <u32 as serde::Deserialize>::deserialize(decoder).map(|id| Self(id as usize))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSyntaxDocument {
    pub blocks: Vec<RawSyntaxBlock>,
    pub entries: Vec<RawSyntaxEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawSyntaxBlock {
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
pub(crate) struct RawSyntaxEntry {
    pub id: RawEntryId,
    pub key: String,
    pub kind: String,
    pub fields: Vec<RawSyntaxField>,
    pub span: Range<usize>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSyntaxField {
    pub id: RawFieldId,
    pub name: String,
    pub value: String,
    pub value_mode: RawValueMode,
    pub value_atoms: Vec<RawValueAtom>,
    pub span: Range<usize>,
    pub patch_span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Detached entry identity and coordinates from an immutable snapshot.
pub struct RawEntryInfo {
    /// Snapshot-relative occurrence identity.
    pub id: RawEntryId,
    /// Entry key, which may be shared by other occurrences.
    pub key: String,
    /// Source entry kind.
    pub kind: String,
    /// UTF-8 byte range of the entry in its source snapshot.
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Detached field identity, inspected value, and source coordinates.
pub struct RawFieldInfo {
    /// Occurrence identity relative to its owning entry.
    pub id: RawFieldId,
    /// Source field name.
    pub name: String,
    /// Inspected value with outer syntax delimiters removed.
    pub value: String,
    /// UTF-8 byte range of the field value in its source snapshot.
    pub span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Field identity and contents borrowed from an immutable source snapshot.
pub struct RawFieldView<'a> {
    /// Occurrence identity relative to its owning entry.
    pub id: RawFieldId,
    /// Source field name.
    pub name: &'a str,
    /// Inspected value with outer syntax delimiters removed.
    pub value: &'a str,
    /// UTF-8 byte range of the field value in its source snapshot.
    pub span: &'a Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Source-order block inspection. Every span uses UTF-8 byte offsets.
pub enum RawBlockInfo {
    /// Preserved whitespace between source blocks.
    Whitespace {
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// A retained comment block.
    Comment {
        /// Complete original block text.
        raw: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// A BibTeX preamble declaration.
    Preamble {
        /// Inspected preamble expression.
        value: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// A BibTeX string macro definition.
    StringDef {
        /// Defined macro name.
        key: String,
        /// Inspected definition value.
        value: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// One bibliography entry occurrence.
    Entry {
        /// Identity in this snapshot's source-order entry sequence.
        id: RawEntryId,
        /// Entry key, potentially shared with another occurrence.
        key: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// A malformed block retained for inspection and writeback.
    Failed {
        /// Complete original block text.
        raw: String,
        /// Parse failure associated with this block.
        error: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
    /// Source text outside the recognized block categories.
    Other {
        /// Complete retained text.
        raw: String,
        /// Byte range in the source snapshot.
        span: Range<usize>,
    },
}

impl RawDocument {
    #[must_use]
    /// Capture a source snapshot, retaining malformed blocks rather than rejecting them.
    pub fn parse(source: &str) -> Self {
        Self {
            data: parse_raw_document(source),
        }
    }

    /// Resolve macros and inherited fields without changing the source snapshot.
    ///
    /// # Errors
    /// Returns structured syntax, macro, inheritance, or resource-budget failures.
    pub fn resolve(&self) -> Result<Vec<ResolvedBibEntry>, crate::ParseFailure> {
        resolve::resolve_document(self)
    }

    #[must_use]
    /// Count entry occurrences, including duplicate keys.
    pub fn entry_count(&self) -> usize {
        self.data.entry_blocks.len()
    }

    #[must_use]
    /// Resolve a source-order occurrence index, returning `None` beyond this snapshot.
    pub fn entry_id_at(&self, index: usize) -> Option<RawEntryId> {
        self.data.entry_blocks.get(index).map(|_| RawEntryId(index))
    }

    #[must_use]
    /// Count all source blocks, including whitespace, comments, and failed blocks.
    pub fn block_count(&self) -> usize {
        self.data.blocks.len()
    }

    #[must_use]
    /// List distinct keys in first-occurrence source order.
    pub fn entry_keys(&self) -> Vec<String> {
        self.data.entries.keys().cloned().collect()
    }

    #[must_use]
    /// Check whether the exact key occurs at least once.
    pub fn contains_entry(&self, key: &str) -> bool {
        self.data.entries.contains_key(key)
    }

    /// Find an exact key only when it identifies one occurrence.
    ///
    /// # Errors
    /// Returns an ambiguity error when more than one entry has the key.
    pub fn unique_entry(&self, key: &str) -> Result<Option<RawEntryId>, String> {
        unique_entry_id(&self.data, key).map(|entry_id| entry_id.map(RawEntryId))
    }

    #[must_use]
    /// Inspect every entry occurrence in source order.
    pub fn entry_occurrences(&self) -> Vec<RawEntryInfo> {
        self.data
            .entry_blocks
            .iter()
            .enumerate()
            .map(|(entry_id, entry)| entry_info(RawEntryId(entry_id), entry))
            .collect()
    }

    #[must_use]
    /// Inspect all occurrences of an exact key in source order.
    pub fn entries_for_key(&self, key: &str) -> Vec<RawEntryInfo> {
        self.data
            .entries
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|entry_id| self.entry_info(RawEntryId(*entry_id)))
            .collect()
    }

    #[must_use]
    /// Inspect an entry occurrence, returning `None` for an invalid identity.
    pub fn entry_info(&self, entry_id: RawEntryId) -> Option<RawEntryInfo> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .map(|entry| entry_info(entry_id, entry))
    }

    #[must_use]
    /// Count field occurrences, or return `None` if the entry does not exist.
    pub fn field_count(&self, entry_id: RawEntryId) -> Option<usize> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .map(|entry| entry.field_blocks.len())
    }

    #[must_use]
    /// List distinct field names in first-occurrence order for an existing entry.
    pub fn field_keys(&self, entry_id: RawEntryId) -> Option<Vec<String>> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .map(|entry| entry.fields.keys().cloned().collect())
    }

    #[must_use]
    /// Check for a case-insensitive field name in an existing entry.
    pub fn contains_field(&self, entry_id: RawEntryId, key: &str) -> bool {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .is_some_and(|entry| entry.fields.contains_key(&key.to_ascii_lowercase()))
    }

    /// Find one field by case-insensitive name in an existing entry.
    ///
    /// # Errors
    /// Returns an ambiguity error for duplicate fields. Missing entries or fields yield `None`.
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

    #[must_use]
    /// Inspect fields in source order, or return `None` for a missing entry.
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

    #[must_use]
    /// Inspect all occurrences of a case-insensitive field name in an existing entry.
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

    #[must_use]
    /// Inspect one field occurrence, returning `None` for invalid entry or field identities.
    pub fn field_info(&self, entry_id: RawEntryId, field_id: RawFieldId) -> Option<RawFieldInfo> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .and_then(|entry| entry.field_blocks.get(field_id.0))
            .map(|field| field_info(field_id, field))
    }

    #[must_use]
    /// Borrow one field occurrence, returning `None` for invalid entry or field identities.
    pub fn field_view(
        &self,
        entry_id: RawEntryId,
        field_id: RawFieldId,
    ) -> Option<RawFieldView<'_>> {
        self.data
            .entry_blocks
            .get(entry_id.0)
            .and_then(|entry| entry.field_blocks.get(field_id.0))
            .map(|field| RawFieldView {
                id: field_id,
                name: &field.name,
                value: &field.value,
                span: &field.span,
            })
    }

    #[must_use]
    /// Collect complete comment blocks in source order.
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

    #[must_use]
    /// Join preamble values in source order with BibTeX concatenation operators.
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

    #[must_use]
    /// Inspect string definitions, with later values replacing repeated names.
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

    #[must_use]
    /// Inspect malformed blocks in source order.
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

    /// Inspect all retained source blocks in order.
    #[must_use]
    pub fn blocks(&self) -> Vec<RawBlockInfo> {
        self.data.blocks.iter().map(raw_block_info).collect()
    }

    #[cfg(test)]
    fn syntax(&self) -> RawSyntaxDocument {
        self.clone().into_syntax()
    }

    pub(crate) fn into_syntax(self) -> RawSyntaxDocument {
        RawSyntaxDocument {
            blocks: self.data.blocks.into_iter().map(raw_syntax_block).collect(),
            entries: self
                .data
                .entry_blocks
                .into_iter()
                .enumerate()
                .map(|(entry_id, entry)| raw_syntax_entry(RawEntryId(entry_id), entry))
                .collect(),
        }
    }

    pub(crate) fn syntax_data(&self) -> &RawDocumentData {
        &self.data
    }

    /// Write back the immutable source snapshot with unrelated syntax preserved.
    ///
    /// # Errors
    /// Returns an error if recorded syntax or span invariants cannot be rendered.
    pub fn render(&self) -> Result<String, String> {
        render_raw_document(&self.data)
    }
}

pub(crate) fn normalize_raw_at_command(raw: &str) -> String {
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
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    /// Return the zero-based index within the snapshot's entry sequence.
    pub fn index(self) -> usize {
        self.0
    }
}

impl RawFieldId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    /// Return the zero-based index within the owning entry's field sequence.
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

fn raw_syntax_block(block: RawBlock) -> RawSyntaxBlock {
    match block {
        RawBlock::Whitespace { raw, span } => RawSyntaxBlock::Whitespace { raw, span },
        RawBlock::Comment { raw, span } => RawSyntaxBlock::Comment { raw, span },
        RawBlock::Preamble { raw, value, span } => RawSyntaxBlock::Preamble { raw, value, span },
        RawBlock::StringDef {
            raw,
            key,
            value,
            span,
        } => RawSyntaxBlock::StringDef {
            raw,
            key,
            value,
            span,
        },
        RawBlock::Entry { id, key, span } => RawSyntaxBlock::Entry {
            id: RawEntryId(id),
            key,
            span,
        },
        RawBlock::Failed { raw, error, span } => RawSyntaxBlock::Failed { raw, error, span },
        RawBlock::Other { raw, span } => RawSyntaxBlock::Other { raw, span },
    }
}

fn raw_syntax_entry(id: RawEntryId, entry: RawEntryData) -> RawSyntaxEntry {
    RawSyntaxEntry {
        id,
        key: entry.key,
        kind: entry.kind,
        fields: entry
            .field_blocks
            .into_iter()
            .enumerate()
            .map(|(field_id, field)| RawSyntaxField {
                id: RawFieldId(field_id),
                name: field.name,
                value: field.value,
                value_mode: field.value_mode,
                value_atoms: field.value_atoms,
                span: field.span,
                patch_span: field.patch_span,
            })
            .collect(),
        span: entry.span,
        raw: entry.raw,
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
