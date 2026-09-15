use super::{BibPatchError, BibPatchErrorCode, BibPatchKind, HashMap, Plan};

impl Plan<'_> {
    pub(super) fn rewrite_references(&mut self) -> Result<(), BibPatchError> {
        let mut targets = HashMap::new();
        for (id, key) in &self.renamed {
            let old = &self
                .document
                .data
                .entry_blocks
                .get(*id)
                .ok_or_else(|| {
                    BibPatchError::new(
                        BibPatchErrorCode::InvalidResult,
                        None,
                        "Renamed entry is absent from the input snapshot",
                    )
                })?
                .key;
            if old != key && !old.is_empty() {
                targets.insert(old.as_str(), key.as_str());
            }
        }
        if targets.is_empty() {
            return Ok(());
        }
        let mut final_counts: HashMap<String, usize> = HashMap::new();
        for (id, entry) in self.document.data.entry_blocks.iter().enumerate() {
            if !self.removed_entries.contains(&id) {
                *final_counts
                    .entry(self.renamed.get(&id).unwrap_or(&entry.key).clone())
                    .or_default() += 1;
            }
        }
        for key in &self.added_keys {
            *final_counts.entry(key.clone()).or_default() += 1;
        }
        let definitions = self
            .document
            .data
            .blocks
            .iter()
            .filter_map(|block| match block {
                super::super::RawBlock::StringDef { raw, .. } => Some(raw.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let macros = biblatex::RawBibliography::parse(&definitions).map_err(|error| {
            BibPatchError::new(BibPatchErrorCode::ReferenceError, None, error.to_string())
        })?;
        let mut edits = Vec::new();
        for (entry_id, entry) in self.document.data.entry_blocks.iter().enumerate() {
            if self.removed_entries.contains(&entry_id) {
                continue;
            }
            self.rewrite_entry(
                entry_id,
                entry,
                &targets,
                &final_counts,
                &macros.abbreviations,
                &mut edits,
            )?;
        }
        for (span, text) in edits {
            self.push(span, text, Vec::new(), BibPatchKind::RewriteReference);
        }
        Ok(())
    }

    fn rewrite_entry(
        &self,
        entry_id: usize,
        entry: &super::super::RawEntryData,
        targets: &HashMap<&str, &str>,
        final_counts: &HashMap<String, usize>,
        abbreviations: &[biblatex::Pair<'_>],
        edits: &mut Vec<(std::ops::Range<usize>, String)>,
    ) -> Result<(), BibPatchError> {
        for (_, field) in entry
            .field_blocks
            .iter()
            .enumerate()
            .filter(|(field_id, _)| !self.touched_fields.contains(&(entry_id, *field_id)))
        {
            let Some(text) =
                self.rewrite_field(entry, field, targets, final_counts, abbreviations)?
            else {
                continue;
            };
            edits.push((field.patch_span.clone(), text));
        }
        Ok(())
    }
    fn rewrite_field(
        &self,
        entry: &super::super::RawEntryData,
        field: &super::super::RawFieldData,
        targets: &HashMap<&str, &str>,
        final_counts: &HashMap<String, usize>,
        abbreviations: &[biblatex::Pair<'_>],
    ) -> Result<Option<String>, BibPatchError> {
        let is_list =
            field.name.eq_ignore_ascii_case("xdata") || field.name.eq_ignore_ascii_case("xref");
        if !is_list && !field.name.eq_ignore_ascii_case("crossref") {
            return Ok(None);
        }
        let keys = crate::library::normalize_reference(
            &crate::references::field_chunks(&field.value_atoms, &field.span),
            abbreviations,
            is_list,
        )
        .map_err(|error| {
            BibPatchError::new(BibPatchErrorCode::ReferenceError, None, error.message)
        })?;
        let mut changed = false;
        let mut rewritten = Vec::new();
        for key in &keys {
            let Some(target) = targets.get(key.trim()) else {
                rewritten.push(key.as_str());
                continue;
            };
            if self
                .document
                .data
                .entries
                .get(key.trim())
                .is_some_and(|ids| ids.len() != 1)
                || final_counts.get(*target).copied() != Some(1)
            {
                return Err(BibPatchError::new(
                    BibPatchErrorCode::AmbiguousReference,
                    None,
                    format!(
                        "Entry {:?} field {:?} refers to an ambiguous renamed key {key:?}",
                        entry.key, field.name
                    ),
                ));
            }
            changed = true;
            rewritten.push(*target);
        }
        if changed {
            let value = crate::references::encode_keys(&rewritten, is_list).map_err(|message| {
                BibPatchError::new(BibPatchErrorCode::ReferenceError, None, message)
            })?;
            let text = super::super::edit::prepare_new_value(&value).map_err(|message| {
                BibPatchError::new(BibPatchErrorCode::InvalidValue, None, message)
            })?;
            return Ok(Some(text));
        }
        Ok(None)
    }
}
