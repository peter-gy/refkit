use super::{
    BTreeMap, BibEntryMapping, BibFieldMapping, BibPatchChange, BibPatchError, BibPatchErrorCode,
    BibPatchKind, BibPatchResult, BibPatchWarning, HashSet, Plan, RawDocument, RawEntryId,
    RawFieldId,
};

impl Plan<'_> {
    pub(super) fn finish(mut self) -> Result<BibPatchResult, BibPatchError> {
        let applied = self.apply_replacements()?;
        let entries = self.map_entries(&applied.document, &applied.offsets)?;
        let warnings = warnings(&applied.document);
        Ok(BibPatchResult {
            document: applied.document,
            changes: applied.changes,
            entries,
            warnings,
        })
    }

    fn output_size(&self) -> Result<usize, BibPatchError> {
        let mut cursor = 0;
        let mut size = self.source.len();
        for edit in &self.replacements {
            if edit.span.start < cursor {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::Overlap,
                    edit.operations.first().copied(),
                    "Patch byte ranges overlap",
                ));
            }
            cursor = edit.span.end;
            size = size
                .checked_sub(edit.span.len())
                .and_then(|size| size.checked_add(edit.text.len()))
                .ok_or_else(|| {
                    BibPatchError::new(
                        BibPatchErrorCode::ResourceLimit,
                        None,
                        "Patch output size exceeds the supported range",
                    )
                })?;
        }
        if size > 16 * 1024 * 1024 {
            return Err(BibPatchError::new(
                BibPatchErrorCode::ResourceLimit,
                None,
                "Patched bibliography exceeds 16 MiB",
            ));
        }
        Ok(size)
    }

    fn apply_replacements(&mut self) -> Result<AppliedReplacements, BibPatchError> {
        self.replacements.sort_by_key(|edit| {
            (
                edit.span.start,
                !edit.span.is_empty(),
                edit.kind != BibPatchKind::RenameEntry,
                edit.operations.first().copied(),
            )
        });
        let size = self.output_size()?;
        let mut output = String::with_capacity(size);
        let mut changes = Vec::new();
        let mut offsets = Vec::new();
        let mut cursor = 0;
        let mut delta = 0isize;
        for edit in std::mem::take(&mut self.replacements) {
            output.push_str(&self.source[cursor..edit.span.start]);
            let start = output.len();
            output.push_str(&edit.text);
            changes.push(BibPatchChange {
                operations: edit.operations,
                kind: edit.kind,
                before: edit.span.clone(),
                after: start..output.len(),
            });
            cursor = edit.span.end;
            delta += isize::try_from(edit.text.len()).map_err(|_| invalid_mapping())?
                - isize::try_from(edit.span.len()).map_err(|_| invalid_mapping())?;
            offsets.push((edit.span.end, delta));
        }
        output.push_str(&self.source[cursor..]);
        let document = RawDocument::parse(&output);
        if document.entry_count()
            != self.document.entry_count() - self.removed_entries.len() + self.added_keys.len()
        {
            return Err(BibPatchError::new(
                BibPatchErrorCode::InvalidResult,
                None,
                "Patch changed entry boundaries unexpectedly",
            ));
        }
        Ok(AppliedReplacements {
            document,
            changes,
            offsets,
        })
    }

    fn map_entries(
        &self,
        document: &RawDocument,
        offsets: &[(usize, isize)],
    ) -> Result<Vec<BibEntryMapping>, BibPatchError> {
        #[expect(
            clippy::indexing_slicing,
            clippy::expect_used,
            reason = "partition_point bounds the predecessor lookup. Validated nonoverlapping edits cannot remove more bytes than precede a surviving position, and the output-size check bounds every translated offset."
        )]
        let shift = |position: usize| {
            let index = offsets.partition_point(|(end, _)| *end <= position);
            position
                .checked_add_signed(if index == 0 { 0 } else { offsets[index - 1].1 })
                .expect("validated patch offsets")
        };
        let positions: BTreeMap<_, _> = document
            .data
            .entry_blocks
            .iter()
            .enumerate()
            .map(|(id, entry)| (entry.span.start, id))
            .collect();
        let mut entries = Vec::new();
        let mut mapped_entries = HashSet::new();
        for (id, entry) in self.document.data.entry_blocks.iter().enumerate() {
            let before = self.document.entry_info(RawEntryId(id));
            if self.removed_entries.contains(&id) {
                entries.push(BibEntryMapping {
                    before,
                    after: None,
                    fields: self
                        .document
                        .field_occurrences(RawEntryId(id))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|field| BibFieldMapping {
                            before: Some(field),
                            after: None,
                        })
                        .collect(),
                });
                continue;
            }
            let new_id = positions
                .get(&shift(entry.span.start))
                .copied()
                .ok_or_else(invalid_mapping)?;
            mapped_entries.insert(new_id);
            let after = document
                .data
                .entry_blocks
                .get(new_id)
                .ok_or_else(invalid_mapping)?;
            if after.key != *self.renamed.get(&id).unwrap_or(&entry.key)
                || after.kind != *self.types.get(&id).unwrap_or(&entry.kind)
            {
                return Err(invalid_mapping());
            }
            let fields = self.map_fields(id, new_id, document, &shift)?;
            entries.push(BibEntryMapping {
                before,
                after: document.entry_info(RawEntryId(new_id)),
                fields,
            });
        }
        for id in 0..document.entry_count() {
            if !mapped_entries.contains(&id) {
                entries.push(BibEntryMapping {
                    before: None,
                    after: document.entry_info(RawEntryId(id)),
                    fields: document
                        .field_occurrences(RawEntryId(id))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|field| BibFieldMapping {
                            before: None,
                            after: Some(field),
                        })
                        .collect(),
                });
            }
        }
        Ok(entries)
    }

    fn map_fields(
        &self,
        id: usize,
        new_id: usize,
        document: &RawDocument,
        shift: &impl Fn(usize) -> usize,
    ) -> Result<Vec<BibFieldMapping>, BibPatchError> {
        let entry = self
            .document
            .data
            .entry_blocks
            .get(id)
            .ok_or_else(invalid_mapping)?;
        let after = document
            .data
            .entry_blocks
            .get(new_id)
            .ok_or_else(invalid_mapping)?;
        let field_positions: BTreeMap<_, _> = after
            .field_blocks
            .iter()
            .enumerate()
            .map(|(id, field)| (field.assignment_span.start, id))
            .collect();
        let mut fields = Vec::new();
        let mut mapped_fields = HashSet::new();
        for (field_id, field) in entry.field_blocks.iter().enumerate() {
            let before = self
                .document
                .field_info(RawEntryId(id), RawFieldId(field_id));
            if self.removed_fields.contains(&(id, field_id)) {
                fields.push(BibFieldMapping {
                    before,
                    after: None,
                });
                continue;
            }
            let new_field_id = field_positions
                .get(&shift(field.assignment_span.start))
                .copied()
                .ok_or_else(invalid_mapping)?;
            mapped_fields.insert(new_field_id);
            let new_field = after
                .field_blocks
                .get(new_field_id)
                .ok_or_else(invalid_mapping)?;
            if field.name != new_field.name
                || self
                    .values
                    .get(&(id, field_id))
                    .is_some_and(|value| *value != new_field.value)
            {
                return Err(invalid_mapping());
            }
            fields.push(BibFieldMapping {
                before,
                after: document.field_info(RawEntryId(new_id), RawFieldId(new_field_id)),
            });
        }
        let added = after.field_blocks.len() - mapped_fields.len();
        if added != self.added_fields.get(&id).map_or(0, Vec::len) {
            return Err(invalid_mapping());
        }
        for field_id in 0..after.field_blocks.len() {
            if !mapped_fields.contains(&field_id) {
                fields.push(BibFieldMapping {
                    before: None,
                    after: document.field_info(RawEntryId(new_id), RawFieldId(field_id)),
                });
            }
        }
        Ok(fields)
    }
}

struct AppliedReplacements {
    document: RawDocument,
    changes: Vec<BibPatchChange>,
    offsets: Vec<(usize, isize)>,
}

fn invalid_mapping() -> BibPatchError {
    BibPatchError::new(
        BibPatchErrorCode::InvalidResult,
        None,
        "Patch changed occurrence structure unexpectedly",
    )
}

fn warnings(document: &RawDocument) -> Vec<BibPatchWarning> {
    let mut warnings = Vec::new();
    let mut keys = HashSet::new();
    for (entry_id, entry) in document.data.entry_blocks.iter().enumerate() {
        if !keys.insert(&entry.key) {
            warnings.push(BibPatchWarning {
                code: "duplicate_entry",
                entry_id,
                field_id: None,
                message: format!("Result contains another entry with key {:?}", entry.key),
            });
        }
        let mut names = HashSet::new();
        for (field_id, field) in entry.field_blocks.iter().enumerate() {
            if !names.insert(field.name.to_ascii_lowercase()) {
                warnings.push(BibPatchWarning {
                    code: "duplicate_field",
                    entry_id,
                    field_id: Some(field_id),
                    message: format!(
                        "Result entry {:?} contains another {:?} field",
                        entry.key, field.name
                    ),
                });
            }
        }
    }
    warnings
}
