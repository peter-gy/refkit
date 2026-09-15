use super::*;

impl Plan<'_> {
    pub(super) fn rewrite_references(&mut self) -> Result<(), BibPatchError> {
        let mut targets = HashMap::new();
        for (id, key) in &self.renamed {
            let old = &self.document.data.entry_blocks[*id].key;
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
            for (field_id, field) in entry.field_blocks.iter().enumerate() {
                if self.touched_fields.contains(&(entry_id, field_id)) {
                    continue;
                }
                let is_list = field.name.eq_ignore_ascii_case("xdata")
                    || field.name.eq_ignore_ascii_case("xref");
                if !is_list && !field.name.eq_ignore_ascii_case("crossref") {
                    continue;
                }
                let keys = crate::library::normalize_reference(
                    &crate::references::field_chunks(&field.value_atoms, &field.span),
                    &macros.abbreviations,
                    is_list,
                )
                .map_err(|error| {
                    BibPatchError::new(BibPatchErrorCode::ReferenceError, None, error.message)
                })?;
                let mut changed = false;
                let mut rewritten = Vec::new();
                for key in &keys {
                    if let Some(target) = targets.get(key.trim()) {
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
                    } else {
                        rewritten.push(key.as_str());
                    }
                }
                if changed {
                    let value =
                        crate::references::encode_keys(&rewritten, is_list).map_err(|message| {
                            BibPatchError::new(BibPatchErrorCode::ReferenceError, None, message)
                        })?;
                    let text =
                        super::super::edit::prepare_new_value(&value).map_err(|message| {
                            BibPatchError::new(BibPatchErrorCode::InvalidValue, None, message)
                        })?;
                    edits.push((field.patch_span.clone(), text));
                }
            }
        }
        for (span, text) in edits {
            self.push(span, text, Vec::new(), BibPatchKind::RewriteReference);
        }
        Ok(())
    }
}
