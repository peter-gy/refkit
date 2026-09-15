use super::{
    BTreeMap, BibEdit, BibFieldValue, BibPatchError, BibPatchErrorCode, BibPatchKind, HashSet,
    Plan, Range, Replacement,
};
use crate::raw::{RawDocument, RawEntryData, RawEntryId, RawFieldData, RawFieldId};

impl Plan<'_> {
    pub(super) fn check_targets(&mut self, operations: &[BibEdit]) -> Result<(), BibPatchError> {
        let mut slots = HashSet::new();
        let mut touched_entries: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (index, operation) in operations.iter().enumerate() {
            let (entry_id, slot) = match operation {
                BibEdit::SetField {
                    entry_id, field_id, ..
                }
                | BibEdit::RemoveField { entry_id, field_id } => {
                    source_field(self.document, *entry_id, *field_id, index)?;
                    self.touched_fields
                        .insert((entry_id.index(), field_id.index()));
                    (*entry_id, Some((0, field_id.index())))
                }
                BibEdit::AddField { entry_id, .. } => (*entry_id, None),
                BibEdit::RemoveEntry { entry_id } => {
                    self.removed_entries.insert(entry_id.index());
                    (*entry_id, Some((1, 0)))
                }
                BibEdit::RenameEntry { entry_id, .. } => (*entry_id, Some((2, 0))),
                BibEdit::SetEntryType { entry_id, .. } => (*entry_id, Some((3, 0))),
                BibEdit::AddEntry { before, .. } => {
                    check_anchor(self.document, *before, index)?;
                    continue;
                }
            };
            if entry_id.index() >= self.document.entry_count() {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::InvalidTarget,
                    Some(index),
                    "Patch entry occurrence does not exist in the input snapshot",
                ));
            }
            if let Some(slot) = slot
                && !slots.insert((entry_id.index(), slot))
            {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::Overlap,
                    Some(index),
                    "Multiple patch operations target the same field, key, type, or removal",
                ));
            }
            touched_entries
                .entry(entry_id.index())
                .or_default()
                .push(index);
        }
        for indices in touched_entries
            .into_iter()
            .filter_map(|(id, indices)| self.removed_entries.contains(&id).then_some(indices))
        {
            if indices.len() > 1 {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::Overlap,
                    indices.last().copied(),
                    "Removing an entry conflicts with other edits inside that entry",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn push(
        &mut self,
        span: Range<usize>,
        text: String,
        operations: Vec<usize>,
        kind: BibPatchKind,
    ) {
        if self.source[span.clone()] != text {
            self.replacements.push(Replacement {
                span,
                text,
                operations,
                kind,
            });
        }
    }

    pub(super) fn operation(
        &mut self,
        index: usize,
        operation: &BibEdit,
    ) -> Result<(), BibPatchError> {
        let invalid =
            |message| BibPatchError::new(BibPatchErrorCode::InvalidValue, Some(index), message);
        match operation {
            BibEdit::SetField {
                entry_id,
                field_id,
                value,
                expression,
            } => {
                self.set_field(index, *entry_id, *field_id, value, *expression)?;
            }
            BibEdit::RemoveField { entry_id, field_id } => {
                self.remove_field(index, *entry_id, *field_id)?;
            }
            BibEdit::AddField {
                entry_id,
                name,
                value,
                expression,
            } => {
                check_name(name).map_err(invalid)?;
                if *expression {
                    super::super::edit::prepare_expression(value).map_err(invalid)?;
                } else {
                    super::super::edit::prepare_new_value(value).map_err(invalid)?;
                }
                self.added_fields
                    .entry(entry_id.index())
                    .or_default()
                    .push((
                        index,
                        BibFieldValue {
                            name: name.clone(),
                            value: value.clone(),
                            expression: *expression,
                        },
                    ));
            }
            BibEdit::RemoveEntry { entry_id } => {
                let span = source_entry(self.document, *entry_id, index)?.span.clone();
                self.push(span, String::new(), vec![index], BibPatchKind::RemoveEntry);
            }
            BibEdit::RenameEntry { entry_id, key } => {
                check_key(key).map_err(invalid)?;
                let entry = source_entry(self.document, *entry_id, index)?;
                let separator = entry.key.is_empty()
                    && !entry.field_blocks.is_empty()
                    && !self.source[entry.key_span.end..entry.span.end - 1].starts_with(',');
                let text = if separator {
                    format!("{key},")
                } else {
                    key.clone()
                };
                self.renamed.insert(entry_id.index(), key.clone());
                self.push(
                    entry.key_span.clone(),
                    text,
                    vec![index],
                    BibPatchKind::RenameEntry,
                );
            }
            BibEdit::SetEntryType {
                entry_id,
                entry_type,
            } => {
                check_type(entry_type).map_err(invalid)?;
                let span = source_entry(self.document, *entry_id, index)?
                    .kind_span
                    .clone();
                self.types.insert(entry_id.index(), entry_type.clone());
                self.push(
                    span,
                    entry_type.clone(),
                    vec![index],
                    BibPatchKind::SetEntryType,
                );
            }
            BibEdit::AddEntry {
                key,
                entry_type,
                fields,
                before,
            } => {
                let text = new_entry_source(key, entry_type, fields, index)?;
                let position = before
                    .map(|id| source_entry(self.document, id, index).map(|entry| entry.span.start))
                    .transpose()?
                    .unwrap_or(self.source.len());
                self.push(
                    position..position,
                    text,
                    vec![index],
                    BibPatchKind::AddEntry,
                );
                self.added_keys.push(key.clone());
            }
        }
        Ok(())
    }

    fn remove_field(
        &mut self,
        index: usize,
        entry_id: RawEntryId,
        field_id: RawFieldId,
    ) -> Result<(), BibPatchError> {
        self.removed_fields
            .insert((entry_id.index(), field_id.index()));
        let field = source_field(self.document, entry_id, field_id, index)?;
        let comma = field.comma;
        self.push(
            field.assignment_span.clone(),
            String::new(),
            vec![index],
            BibPatchKind::RemoveField,
        );
        if let Some(comma) = comma {
            self.push(
                comma..comma + 1,
                String::new(),
                vec![index],
                BibPatchKind::RemoveField,
            );
        }
        Ok(())
    }

    fn set_field(
        &mut self,
        index: usize,
        entry_id: RawEntryId,
        field_id: RawFieldId,
        value: &str,
        expression: bool,
    ) -> Result<(), BibPatchError> {
        let invalid =
            |message| BibPatchError::new(BibPatchErrorCode::InvalidValue, Some(index), message);
        let field = source_field(self.document, entry_id, field_id, index)?;
        let (span, text, expected) = if expression {
            let (text, expected) =
                super::super::edit::prepare_expression(value).map_err(invalid)?;
            (field.patch_span.clone(), text, expected)
        } else {
            let (span, text) =
                super::super::edit::prepare_field_edit(field, value).map_err(invalid)?;
            (span, text, value.trim().to_string())
        };
        self.values
            .insert((entry_id.index(), field_id.index()), expected);
        self.push(span, text, vec![index], BibPatchKind::SetField);
        Ok(())
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "Added-field keys are populated only from entry IDs accepted by check_targets, and the borrowed input snapshot cannot change during planning."
    )]
    pub(super) fn append_fields(&mut self) {
        let mut replacements = Vec::new();
        for (entry_id, fields) in &self.added_fields {
            let entry = &self.document.data.entry_blocks[*entry_id];
            let last = entry
                .field_blocks
                .iter()
                .enumerate()
                .rev()
                .find(|(id, _)| !self.removed_fields.contains(&(*entry_id, *id)));
            let key = self.renamed.get(entry_id).unwrap_or(&entry.key);
            let header_comma = self.source[entry.key_span.end..entry.span.end - 1]
                .trim_start()
                .starts_with(',')
                || (entry.key.is_empty()
                    && self.renamed.contains_key(entry_id)
                    && !entry.field_blocks.is_empty());
            let needs_comma = last.map_or_else(
                || !key.is_empty() && !header_comma,
                |(_, field)| field.comma.is_none(),
            );
            let text = appended_fields(fields, needs_comma);
            let position = entry.span.end - 1;
            replacements.push((
                position..position,
                text,
                fields.iter().map(|(op, _)| *op).collect(),
            ));
        }
        for (span, text, operations) in replacements {
            self.push(span, text, operations, BibPatchKind::AddField);
        }
    }
}

fn source_entry(
    document: &RawDocument,
    entry_id: RawEntryId,
    operation: usize,
) -> Result<&RawEntryData, BibPatchError> {
    document
        .data
        .entry_blocks
        .get(entry_id.index())
        .ok_or_else(|| {
            BibPatchError::new(
                BibPatchErrorCode::InvalidTarget,
                Some(operation),
                "Patch entry occurrence does not exist in the input snapshot",
            )
        })
}

fn source_field(
    document: &RawDocument,
    entry_id: RawEntryId,
    field_id: RawFieldId,
    operation: usize,
) -> Result<&RawFieldData, BibPatchError> {
    document
        .data
        .entry_blocks
        .get(entry_id.index())
        .and_then(|entry| entry.field_blocks.get(field_id.index()))
        .ok_or_else(|| {
            BibPatchError::new(
                BibPatchErrorCode::InvalidTarget,
                Some(operation),
                "Patch field occurrence does not exist in the input snapshot",
            )
        })
}

fn new_entry_source(
    key: &str,
    entry_type: &str,
    fields: &[BibFieldValue],
    index: usize,
) -> Result<String, BibPatchError> {
    let invalid =
        |message| BibPatchError::new(BibPatchErrorCode::InvalidValue, Some(index), message);
    check_key(key).map_err(invalid)?;
    check_type(entry_type).map_err(invalid)?;
    let mut text = format!("\n@{entry_type}{{{key}");
    for field in fields {
        check_name(&field.name).map_err(invalid)?;
        let value = if field.expression {
            super::super::edit::prepare_expression(&field.value)
                .map_err(invalid)?
                .0
        } else {
            super::super::edit::prepare_new_value(&field.value).map_err(invalid)?
        };
        text.push_str(",\n  ");
        text.push_str(&field.name);
        text.push_str(" = ");
        text.push_str(&value);
    }
    text.push_str("\n}\n");
    Ok(text)
}

fn check_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || !super::super::is_valid_entry_key(key)
        || key.contains(['(', ')', '@', '='])
    {
        Err(format!("Invalid authored BibTeX key {key:?}"))
    } else {
        Ok(())
    }
}

fn check_name(name: &str) -> Result<(), String> {
    if super::super::is_valid_identifier(name) {
        Ok(())
    } else {
        Err(format!("Invalid authored BibTeX field name {name:?}"))
    }
}

fn check_type(kind: &str) -> Result<(), String> {
    if super::super::is_valid_identifier(kind)
        && !["comment", "preamble", "string"].contains(&kind.to_ascii_lowercase().as_str())
    {
        Ok(())
    } else {
        Err(format!("Invalid authored BibTeX entry type {kind:?}"))
    }
}

fn check_anchor(
    document: &RawDocument,
    before: Option<RawEntryId>,
    index: usize,
) -> Result<(), BibPatchError> {
    if before.is_some_and(|id| id.index() >= document.entry_count()) {
        return Err(BibPatchError::new(
            BibPatchErrorCode::InvalidTarget,
            Some(index),
            "Insertion anchor does not exist in the input snapshot",
        ));
    }
    Ok(())
}

fn appended_fields(fields: &[(usize, BibFieldValue)], needs_comma: bool) -> String {
    let mut text = if needs_comma {
        ",".into()
    } else {
        String::new()
    };
    for (i, (_, field)) in fields.iter().enumerate() {
        if i > 0 {
            text.push(',');
        }
        let value = if field.expression {
            field.value.trim().to_string()
        } else {
            format!("{{{}}}", field.value)
        };
        text.push_str("\n  ");
        text.push_str(&field.name);
        text.push_str(" = ");
        text.push_str(&value);
    }
    text.push('\n');
    text
}
