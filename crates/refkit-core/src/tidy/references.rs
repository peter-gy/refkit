use std::collections::{HashMap, HashSet};

use biblatex::{RawBibliography, RawChunk, Spanned};

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
        let new_key = &doc.entries[duplicates.retained_id(*id).index()].key;
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
                || field.name.eq_ignore_ascii_case("xdata"))
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
    for entry in &mut doc.entries {
        if duplicates.should_skip(entry.id) {
            continue;
        }
        let emitted = super::render::field_indices(entry, options);
        let effective = emitted
            .iter()
            .map(|index| (entry.fields[*index].name.to_ascii_lowercase(), *index))
            .collect::<HashMap<_, _>>();
        let emitted = emitted.into_iter().collect::<HashSet<_>>();
        for (index, field) in entry.fields.iter_mut().enumerate() {
            if !emitted.contains(&index) {
                continue;
            }
            let is_list = field.name.eq_ignore_ascii_case("xdata");
            if !is_list && !field.name.eq_ignore_ascii_case("crossref") {
                continue;
            }
            let keys =
                crate::library::normalize_reference(&field_chunks(field), abbreviations, is_list)
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
                                entry.key,
                                field.name,
                                key.trim(),
                            )));
                        }
                        None => key.as_str(),
                    };
                    match final_keys.get(target) {
                        Some(Some(id))
                            if effective.get(&field.name.to_ascii_lowercase()) == Some(&index) =>
                        {
                            graph[entry.id.index()].push(*id)
                        }
                        Some(Some(_)) => {}
                        Some(None) => {
                            return Err(TidyError::Reference(format!(
                                "entry {:?} field {:?} refers to ambiguous final citation key {:?}",
                                entry.key, field.name, target,
                            )));
                        }
                        None => {}
                    }
                    Ok(target)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if changed {
                field.value = encode_keys(&rewritten, is_list)?;
                field.value_mode = RawValueMode::Braced;
                field.value_atoms = vec![RawValueAtom {
                    value: field.value.clone(),
                    value_mode: RawValueMode::Braced,
                }];
            }
        }
    }
    validate_graph(doc, &graph)
}

pub(super) fn field_chunks(field: &RawSyntaxField) -> biblatex::Field<'_> {
    field
        .value_atoms
        .iter()
        .map(|atom| {
            let value = if atom.value_mode == RawValueMode::Bare
                && !atom
                    .value
                    .chars()
                    .all(|character| character.is_ascii_digit())
            {
                RawChunk::Abbreviation(atom.value.as_str())
            } else {
                RawChunk::Normal(atom.value.as_str())
            };
            Spanned::new(value, field.span.clone())
        })
        .collect()
}

fn validate_graph(doc: &RawSyntaxDocument, graph: &[Vec<usize>]) -> Result<(), TidyError> {
    let mut state = vec![0u8; graph.len()];
    for start in 0..graph.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut pending = vec![(start, 0)];
        while let Some((current, next)) = pending.last_mut() {
            let Some(&target) = graph[*current].get(*next) else {
                state[*current] = 2;
                pending.pop();
                continue;
            };
            *next += 1;
            match state[target] {
                1 => {
                    return Err(TidyError::Reference(format!(
                        "transformation creates a reference cycle involving citation key {:?}",
                        doc.entries[target].key,
                    )));
                }
                0 => {
                    state[target] = 1;
                    pending.push((target, 0));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn encode_keys(keys: &[&str], is_list: bool) -> Result<String, TidyError> {
    let plain = keys.join(", ");
    let round_trips = |value: &str| {
        let field = vec![Spanned::new(RawChunk::Normal(value), 0..value.len())];
        crate::library::normalize_reference(&field, &[], is_list)
            .is_ok_and(|actual| actual.iter().map(String::as_str).eq(keys.iter().copied()))
    };
    if round_trips(&plain) {
        return Ok(plain);
    }
    let encoded = keys
        .iter()
        .map(|key| {
            biblatex::Chunk::Verbatim((*key).to_string())
                .to_biblatex_string(false)
                .replace('-', "{-}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    if round_trips(&encoded) {
        Ok(encoded)
    } else {
        Err(TidyError::Reference(format!(
            "citation keys {keys:?} cannot be represented faithfully in a reference field"
        )))
    }
}
