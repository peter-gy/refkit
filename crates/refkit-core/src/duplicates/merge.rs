use super::*;
use crate::{
    BibEdit, RawDocument,
    raw::{RawSyntaxBlock, RawSyntaxEntry},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

impl RawDocument {
    pub fn plan_merge(&self, request: &MergeRequest) -> Result<MergePlan, MergeError> {
        let source = self
            .render()
            .map_err(|message| MergeError::new(MergeErrorCode::InvalidSelection, message))?;
        crate::library::validate_source(&source)
            .map_err(|error| MergeError::new(MergeErrorCode::ResourceLimit, error.message))?;
        let syntax = self.clone().into_syntax();
        let selected: BTreeSet<_> = request.entries.iter().map(|id| id.index()).collect();
        if selected.len() < 2
            || selected.len() != request.entries.len()
            || !selected.contains(&request.retain.index())
            || selected.iter().any(|id| *id >= syntax.entries.len())
        {
            return Err(MergeError::new(
                MergeErrorCode::InvalidSelection,
                "Merge requires distinct existing entries and a retained member of that selection",
            ));
        }
        let retained = &syntax.entries[request.retain.index()];
        if retained.key.is_empty() {
            return Err(MergeError::new(
                MergeErrorCode::InvalidSelection,
                "Retained entry requires a nonempty key",
            ));
        }
        let entries = selected
            .iter()
            .map(|id| &syntax.entries[*id])
            .collect::<Vec<_>>();
        let values = super::review::values(&entries, &source);
        let mut choices = BTreeMap::new();
        for choice in &request.fields {
            let name = match choice {
                MergeFieldChoice::Take { name, .. } | MergeFieldChoice::Drop { name } => {
                    name.to_ascii_lowercase()
                }
            };
            if !values.contains_key(&name) || choices.insert(name.clone(), choice).is_some() {
                return Err(MergeError::new(
                    MergeErrorCode::InvalidChoice,
                    format!("Merge field {name:?} must exist and have at most one choice"),
                ));
            }
            if let MergeFieldChoice::Take {
                entry_id, field_id, ..
            } = choice
                && !values[&name]
                    .iter()
                    .any(|value| value.entry_id == *entry_id && value.field_id == Some(*field_id))
            {
                return Err(MergeError::new(
                    MergeErrorCode::InvalidChoice,
                    format!("Selected occurrence is not a {name:?} field in the merge selection"),
                ));
            }
        }
        let conflicts = super::review::conflicts(&entries, &source)
            .into_iter()
            .filter(|conflict| {
                if conflict.kind == DuplicateConflictKind::EntryType {
                    request.entry_type.is_none()
                } else {
                    !choices.contains_key(&conflict.field)
                }
            })
            .collect::<Vec<_>>();
        let removed_ids = entries
            .iter()
            .filter(|entry| entry.id != request.retain)
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        let mut result = MergePlan {
            retained_id: request.retain,
            removed_ids,
            patch: None,
            conflicts,
        };
        if !result.conflicts.is_empty() {
            return Ok(result);
        }
        let mut merged = BTreeMap::new();
        for (name, candidates) in &values {
            let chosen = match choices.get(name) {
                Some(MergeFieldChoice::Drop { .. }) => continue,
                Some(MergeFieldChoice::Take {
                    entry_id, field_id, ..
                }) => candidates
                    .iter()
                    .find(|value| value.entry_id == *entry_id && value.field_id == Some(*field_id))
                    .expect("choice validated"),
                None => candidates
                    .iter()
                    .find(|value| value.entry_id == request.retain)
                    .unwrap_or(&candidates[0]),
            };
            merged.insert(name.clone(), chosen.expression.clone());
        }
        let removed: HashSet<_> = result.removed_ids.iter().map(|id| id.index()).collect();
        let mut final_keys: HashMap<&str, Option<usize>> = HashMap::new();
        for entry in &syntax.entries {
            if removed.contains(&entry.id.index()) {
                continue;
            }
            final_keys
                .entry(&entry.key)
                .and_modify(|id| *id = None)
                .or_insert(Some(entry.id.index()));
        }
        if final_keys.get(retained.key.as_str()) != Some(&Some(retained.id.index())) {
            return Err(MergeError::new(
                MergeErrorCode::AmbiguousReference,
                "Retained key would remain ambiguous after merging",
            ));
        }
        let targets = result
            .removed_ids
            .iter()
            .map(|id| syntax.entries[id.index()].key.as_str())
            .filter(|key| !key.is_empty() && *key != retained.key)
            .collect::<HashSet<_>>();
        let definitions = syntax
            .blocks
            .iter()
            .filter_map(|block| match block {
                RawSyntaxBlock::StringDef { raw, .. } => Some(raw.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let macros = biblatex::RawBibliography::parse(&definitions)
            .map_err(|error| MergeError::new(MergeErrorCode::ReferenceError, error.to_string()))?;
        let mut patch = Vec::new();
        let mut graph = vec![Vec::new(); syntax.entries.len()];
        let mut edges = 0usize;
        for entry in &syntax.entries {
            if removed.contains(&entry.id.index()) {
                continue;
            }
            let fields = if entry.id == retained.id {
                merged
                    .iter()
                    .map(|(name, expression)| (None, name.clone(), expression.clone()))
                    .collect::<Vec<_>>()
            } else {
                entry
                    .fields
                    .iter()
                    .map(|field| {
                        (
                            Some(field.id),
                            field.name.to_ascii_lowercase(),
                            source[field.patch_span.clone()].to_string(),
                        )
                    })
                    .collect()
            };
            for (field_id, name, expression) in fields {
                let is_list = name == "xdata" || name == "xref";
                if !is_list && name != "crossref" {
                    continue;
                }
                let keys = crate::references::decode_expression(
                    &expression,
                    &macros.abbreviations,
                    is_list,
                )
                .map_err(|message| MergeError::new(MergeErrorCode::ReferenceError, message))?;
                let mut changed = false;
                let mut rewritten = Vec::new();
                for key in &keys {
                    let target = if targets.contains(key.as_str()) {
                        if final_keys.contains_key(key.as_str()) {
                            return Err(MergeError::new(
                                MergeErrorCode::AmbiguousReference,
                                format!(
                                    "Reference {key:?} also identifies an entry outside the merge selection"
                                ),
                            ));
                        }
                        changed = true;
                        retained.key.as_str()
                    } else {
                        key.as_str()
                    };
                    if final_keys.get(target) == Some(&None) {
                        return Err(MergeError::new(
                            MergeErrorCode::AmbiguousReference,
                            format!(
                                "Reference target {target:?} would remain ambiguous after merging"
                            ),
                        ));
                    }
                    if let Some(Some(id)) = final_keys.get(target) {
                        edges += 1;
                        if edges > 100_000 {
                            return Err(MergeError::new(
                                MergeErrorCode::ResourceLimit,
                                "Merge reference graph exceeds 100,000 edges",
                            ));
                        }
                        if name != "xref" {
                            graph[entry.id.index()].push(*id);
                        }
                    }
                    rewritten.push(target);
                }
                if changed {
                    let expression = format!(
                        "{{{}}}",
                        crate::references::encode_keys(&rewritten, is_list).map_err(|message| {
                            MergeError::new(MergeErrorCode::ReferenceError, message)
                        })?
                    );
                    if entry.id == retained.id {
                        merged.insert(name, expression);
                    } else {
                        patch.push(BibEdit::SetField {
                            entry_id: entry.id,
                            field_id: field_id.expect("existing field"),
                            value: expression,
                            expression: true,
                        });
                    }
                }
            }
        }
        crate::references::validate_graph(
            &syntax
                .entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            &graph,
        )
        .map_err(|message| MergeError::new(MergeErrorCode::ReferenceCycle, message))?;
        append_field_edits(retained, &merged, &source, &mut patch);
        if let Some(entry_type) = &request.entry_type
            && *entry_type != retained.kind
        {
            patch.push(BibEdit::SetEntryType {
                entry_id: retained.id,
                entry_type: entry_type.clone(),
            });
        }
        patch.extend(
            result
                .removed_ids
                .iter()
                .map(|entry_id| BibEdit::RemoveEntry {
                    entry_id: *entry_id,
                }),
        );
        self.apply_patch(&patch).map_err(|error| {
            MergeError::new(
                if error.code == crate::BibPatchErrorCode::ResourceLimit {
                    MergeErrorCode::ResourceLimit
                } else {
                    MergeErrorCode::InvalidChoice
                },
                error.message,
            )
        })?;
        result.patch = Some(patch);
        Ok(result)
    }
}

fn append_field_edits(
    retained: &RawSyntaxEntry,
    merged: &BTreeMap<String, String>,
    source: &str,
    patch: &mut Vec<BibEdit>,
) {
    let mut original: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for field in &retained.fields {
        original
            .entry(field.name.to_ascii_lowercase())
            .or_default()
            .push(field);
    }
    for (name, fields) in &original {
        let chosen = merged.get(name).map(|expression| {
            fields
                .iter()
                .find(|field| source[field.patch_span.clone()] == *expression)
                .copied()
                .unwrap_or(fields[0])
        });
        for field in fields {
            if chosen.is_none_or(|chosen| chosen.id != field.id) {
                patch.push(BibEdit::RemoveField {
                    entry_id: retained.id,
                    field_id: field.id,
                });
            }
        }
        if let Some(chosen) = chosen {
            let expression = &merged[name];
            if source[chosen.patch_span.clone()] != *expression {
                patch.push(BibEdit::SetField {
                    entry_id: retained.id,
                    field_id: chosen.id,
                    value: expression.clone(),
                    expression: true,
                });
            }
        }
    }
    for (name, expression) in merged {
        if !original.contains_key(name) {
            patch.push(BibEdit::AddField {
                entry_id: retained.id,
                name: name.clone(),
                value: expression.clone(),
                expression: true,
            });
        }
    }
}
