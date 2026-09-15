use super::{
    DuplicateConflictKind, DuplicateValue, MergeError, MergeErrorCode, MergeFieldChoice, MergePlan,
    MergeRequest,
};
use crate::{
    BibEdit, RawDocument,
    raw::{RawSyntaxBlock, RawSyntaxEntry},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

impl RawDocument {
    /// Compile explicit merge choices into atomic source edits without applying them.
    ///
    /// # Errors
    /// Rejects invalid selections or choices, ambiguous references, inheritance cycles,
    /// invalid expressions, and resource-budget violations.
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
        let retained = syntax.entries.get(request.retain.index()).ok_or_else(|| {
            MergeError::new(
                MergeErrorCode::InvalidSelection,
                "Retained entry is absent from the selection",
            )
        })?;
        if retained.key.is_empty() {
            return Err(MergeError::new(
                MergeErrorCode::InvalidSelection,
                "Retained entry requires a nonempty key",
            ));
        }
        let entries = syntax
            .entries
            .iter()
            .filter(|entry| selected.contains(&entry.id.index()))
            .collect::<Vec<_>>();
        let values = super::review::values(&entries, &source);
        let choices = validate_choices(request, &values)?;
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
        let mut merged = merge_fields(request, &values, &choices)?;
        let mut patch = rewrite_references(
            &syntax.entries,
            &syntax.blocks,
            retained,
            &result.removed_ids,
            &mut merged,
            &source,
        )?;
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

fn validate_choices<'a>(
    request: &'a MergeRequest,
    values: &BTreeMap<String, Vec<DuplicateValue>>,
) -> Result<BTreeMap<String, &'a MergeFieldChoice>, MergeError> {
    let mut choices = BTreeMap::new();
    for choice in &request.fields {
        let name = match choice {
            MergeFieldChoice::Take { name, .. } | MergeFieldChoice::Drop { name } => {
                name.to_ascii_lowercase()
            }
        };
        let candidates = values.get(&name).ok_or_else(|| {
            MergeError::new(
                MergeErrorCode::InvalidChoice,
                format!("Merge field {name:?} must exist and have at most one choice"),
            )
        })?;
        if choices.insert(name.clone(), choice).is_some() {
            return Err(MergeError::new(
                MergeErrorCode::InvalidChoice,
                format!("Merge field {name:?} must exist and have at most one choice"),
            ));
        }
        if let MergeFieldChoice::Take {
            entry_id, field_id, ..
        } = choice
            && !candidates
                .iter()
                .any(|value| value.entry_id == *entry_id && value.field_id == Some(*field_id))
        {
            return Err(MergeError::new(
                MergeErrorCode::InvalidChoice,
                format!("Selected occurrence is not a {name:?} field in the merge selection"),
            ));
        }
    }
    Ok(choices)
}

fn merge_fields(
    request: &MergeRequest,
    values: &BTreeMap<String, Vec<DuplicateValue>>,
    choices: &BTreeMap<String, &MergeFieldChoice>,
) -> Result<BTreeMap<String, String>, MergeError> {
    let mut merged = BTreeMap::new();
    for (name, candidates) in values {
        let Some(first) = candidates.first() else {
            continue;
        };
        let chosen = match choices.get(name) {
            Some(MergeFieldChoice::Drop { .. }) => continue,
            Some(MergeFieldChoice::Take {
                entry_id, field_id, ..
            }) => candidates
                .iter()
                .find(|value| value.entry_id == *entry_id && value.field_id == Some(*field_id))
                .ok_or_else(|| {
                    MergeError::new(
                        MergeErrorCode::InvalidChoice,
                        format!("Chosen field occurrence for {name:?} is absent"),
                    )
                })?,
            None => candidates
                .iter()
                .find(|value| value.entry_id == request.retain)
                .unwrap_or(first),
        };
        merged.insert(name.clone(), chosen.expression.clone());
    }
    Ok(merged)
}

fn rewrite_references(
    entries: &[RawSyntaxEntry],
    blocks: &[RawSyntaxBlock],
    retained: &RawSyntaxEntry,
    removed_ids: &[crate::RawEntryId],
    merged: &mut BTreeMap<String, String>,
    source: &str,
) -> Result<Vec<BibEdit>, MergeError> {
    let removed: HashSet<_> = removed_ids.iter().map(|id| id.index()).collect();
    let mut final_keys: HashMap<&str, Option<usize>> = HashMap::new();
    for entry in entries {
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
    let targets = entries
        .iter()
        .filter(|entry| removed.contains(&entry.id.index()))
        .map(|entry| entry.key.as_str())
        .filter(|key| !key.is_empty() && *key != retained.key)
        .collect::<HashSet<_>>();
    let definitions = blocks
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
    let mut graph = vec![Vec::new(); entries.len()];
    let mut references = MergeReferences {
        retained_key: &retained.key,
        targets,
        final_keys,
        abbreviations: &macros.abbreviations,
        edge_count: 0,
    };
    for (entry, entry_edges) in entries.iter().zip(&mut graph) {
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
            let Some(expression) = references.rewrite(&name, &expression, entry_edges)? else {
                continue;
            };
            if entry.id == retained.id {
                merged.insert(name, expression);
                continue;
            }
            patch.push(BibEdit::SetField {
                entry_id: entry.id,
                field_id: field_id.ok_or_else(|| {
                    MergeError::new(
                        MergeErrorCode::ReferenceError,
                        "Reference rewrite requires an existing field occurrence",
                    )
                })?,
                value: expression,
                expression: true,
            });
        }
    }
    crate::references::validate_graph(
        &entries
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        &graph,
    )
    .map_err(|message| MergeError::new(MergeErrorCode::ReferenceCycle, message))?;
    Ok(patch)
}

struct MergeReferences<'a> {
    retained_key: &'a str,
    targets: HashSet<&'a str>,
    final_keys: HashMap<&'a str, Option<usize>>,
    abbreviations: &'a [biblatex::Pair<'a>],
    edge_count: usize,
}

impl MergeReferences<'_> {
    fn rewrite(
        &mut self,
        name: &str,
        expression: &str,
        entry_edges: &mut Vec<usize>,
    ) -> Result<Option<String>, MergeError> {
        let is_list = name == "xdata" || name == "xref";
        if !is_list && name != "crossref" {
            return Ok(None);
        }
        let keys = crate::references::decode_expression(expression, self.abbreviations, is_list)
            .map_err(|message| MergeError::new(MergeErrorCode::ReferenceError, message))?;
        let mut changed = false;
        let mut rewritten = Vec::new();
        for key in &keys {
            let removed = self.targets.contains(key.as_str());
            if removed && self.final_keys.contains_key(key.as_str()) {
                return Err(MergeError::new(
                    MergeErrorCode::AmbiguousReference,
                    format!(
                        "Reference {key:?} also identifies an entry outside the merge selection"
                    ),
                ));
            }
            let target = if removed {
                changed = true;
                self.retained_key
            } else {
                key.as_str()
            };
            if self.final_keys.get(target) == Some(&None) {
                return Err(MergeError::new(
                    MergeErrorCode::AmbiguousReference,
                    format!("Reference target {target:?} would remain ambiguous after merging"),
                ));
            }
            rewritten.push(target);
            let Some(Some(id)) = self.final_keys.get(target) else {
                continue;
            };
            self.edge_count += 1;
            if self.edge_count > 100_000 {
                return Err(MergeError::new(
                    MergeErrorCode::ResourceLimit,
                    "Merge reference graph exceeds 100,000 edges",
                ));
            }
            if name != "xref" {
                entry_edges.push(*id);
            }
        }
        if changed {
            return Ok(Some(format!(
                "{{{}}}",
                crate::references::encode_keys(&rewritten, is_list).map_err(|message| {
                    MergeError::new(MergeErrorCode::ReferenceError, message)
                })?
            )));
        }
        Ok(None)
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
        let chosen = merged.get(name).and_then(|expression| {
            fields
                .iter()
                .find(|field| source[field.patch_span.clone()] == *expression)
                .or_else(|| fields.first())
                .copied()
        });
        for field in fields {
            if chosen.is_none_or(|chosen| chosen.id != field.id) {
                patch.push(BibEdit::RemoveField {
                    entry_id: retained.id,
                    field_id: field.id,
                });
            }
        }
        if let Some(chosen) = chosen
            && let Some(expression) = merged.get(name)
            && source[chosen.patch_span.clone()] != *expression
        {
            patch.push(BibEdit::SetField {
                entry_id: retained.id,
                field_id: chosen.id,
                value: expression.clone(),
                expression: true,
            });
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
