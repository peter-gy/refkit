use std::collections::{HashMap, HashSet};

use biblatex::RawBibliography;

use crate::raw::{
    RawEntryId, RawSyntaxBlock, RawSyntaxDocument, RawSyntaxField, RawValueAtom, RawValueMode,
};

use super::{TidyError, TidyOptions, duplicates::DuplicatePlan};

pub(super) fn rewrite(
    doc: &mut RawSyntaxDocument,
    original_keys: &[(RawEntryId, String)],
    duplicates: &DuplicatePlan,
    source: &str,
    options: &TidyOptions,
) -> Result<(), TidyError> {
    let mut targets: HashMap<&str, Option<String>> = HashMap::new();
    for (id, old_key) in original_keys {
        let new_key = &doc
            .entries
            .get(duplicates.retained_id(*id).index())
            .ok_or_else(|| {
                TidyError::Reference(
                    "reference rewrite refers to a missing retained entry".to_string(),
                )
            })?
            .key;
        targets
            .entry(old_key)
            .and_modify(|target| {
                if target.as_ref() != Some(new_key) {
                    *target = None;
                }
            })
            .or_insert_with(|| Some(new_key.clone()));
    }
    let definitions = doc
        .blocks
        .iter()
        .filter_map(|block| match block {
            RawSyntaxBlock::StringDef { span, .. } => Some(&source[span.clone()]),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let needs_expansion = doc
        .entries
        .iter()
        .filter(|entry| !duplicates.should_skip(entry.id))
        .flat_map(|entry| &entry.fields)
        .any(|field| {
            (field.name.eq_ignore_ascii_case("crossref")
                || field.name.eq_ignore_ascii_case("xdata")
                || field.name.eq_ignore_ascii_case("xref"))
                && matches!(
                    field.value_mode,
                    RawValueMode::Bare | RawValueMode::Expression
                )
        });
    let macros = if needs_expansion {
        Some(
            RawBibliography::parse(&definitions)
                .map_err(|error| TidyError::Reference(error.to_string()))?,
        )
    } else {
        None
    };
    let abbreviations = macros
        .as_ref()
        .map(|raw| raw.abbreviations.as_slice())
        .unwrap_or_default();
    let mut final_keys = HashMap::new();
    for entry in &doc.entries {
        if !duplicates.should_skip(entry.id) {
            final_keys
                .entry(entry.key.clone())
                .and_modify(|target| *target = None)
                .or_insert(Some(entry.id.index()));
        }
    }
    let mut graph = vec![Vec::new(); doc.entries.len()];
    for (entry, edges) in doc.entries.iter_mut().zip(&mut graph) {
        if duplicates.should_skip(entry.id) {
            continue;
        }
        let emitted = super::render::field_indices(entry, options)
            .into_iter()
            .collect::<HashSet<_>>();
        let effective = entry
            .fields
            .iter()
            .enumerate()
            .filter(|(index, _)| emitted.contains(index))
            .map(|(index, field)| (field.name.to_ascii_lowercase(), index))
            .collect::<HashMap<_, _>>();
        for (index, field) in entry.fields.iter_mut().enumerate() {
            if !emitted.contains(&index) {
                continue;
            }
            let inheritance = !field.name.eq_ignore_ascii_case("xref")
                && effective.get(&field.name.to_ascii_lowercase()) == Some(&index);
            rewrite_field(
                &entry.key,
                field,
                inheritance,
                &targets,
                &final_keys,
                abbreviations,
                edges,
            )?;
        }
    }
    validate_graph(doc, &graph)
}

fn rewrite_field(
    entry_key: &str,
    field: &mut RawSyntaxField,
    inheritance: bool,
    targets: &HashMap<&str, Option<String>>,
    final_keys: &HashMap<String, Option<usize>>,
    abbreviations: &[biblatex::Pair<'_>],
    edges: &mut Vec<usize>,
) -> Result<(), TidyError> {
    let is_list =
        field.name.eq_ignore_ascii_case("xdata") || field.name.eq_ignore_ascii_case("xref");
    if !is_list && !field.name.eq_ignore_ascii_case("crossref") {
        return Ok(());
    }
    let keys = crate::library::normalize_reference(&field_chunks(field), abbreviations, is_list)
        .map_err(|diagnostic| TidyError::Reference(diagnostic.message))?;
    let mut changed = false;
    let rewritten = keys
        .iter()
        .map(|key| {
            let target = match targets.get(key.trim()) {
                Some(Some(target)) => {
                    changed |= target != key.trim();
                    target.as_str()
                }
                Some(None) => {
                    return Err(TidyError::Reference(format!(
                        "entry {:?} field {:?} refers to ambiguous citation key {:?}",
                        entry_key,
                        field.name,
                        key.trim(),
                    )));
                }
                None => key.as_str(),
            };
            match final_keys.get(target) {
                Some(Some(id)) if inheritance => {
                    edges.push(*id);
                }
                Some(Some(_)) | None => {}
                Some(None) => {
                    return Err(TidyError::Reference(format!(
                        "entry {:?} field {:?} refers to ambiguous final citation key {:?}",
                        entry_key, field.name, target,
                    )));
                }
            }
            Ok(target)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if changed {
        field.value =
            crate::references::encode_keys(&rewritten, is_list).map_err(TidyError::Reference)?;
        field.value_mode = RawValueMode::Braced;
        field.value_atoms = vec![RawValueAtom {
            value: field.value.clone(),
            value_mode: RawValueMode::Braced,
        }];
    }
    Ok(())
}

pub(super) fn field_chunks(field: &RawSyntaxField) -> biblatex::Field<'_> {
    crate::references::field_chunks(&field.value_atoms, &field.span)
}

fn validate_graph(doc: &RawSyntaxDocument, graph: &[Vec<usize>]) -> Result<(), TidyError> {
    crate::references::validate_graph(
        &doc.entries
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        graph,
    )
    .map_err(TidyError::Reference)
}
