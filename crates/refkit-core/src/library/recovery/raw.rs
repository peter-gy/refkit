use std::collections::{HashMap, HashSet};

use biblatex::{Field, RawBibliography, RawChunk};

use crate::library::guard::month;
use crate::library::{Diagnostic, DiagnosticAction};

pub(super) fn has_duplicate_entries(raw: &RawBibliography<'_>) -> bool {
    let mut keys = HashSet::with_capacity(raw.entries.len());
    raw.entries.iter().any(|entry| !keys.insert(entry.v.key.v))
}

pub(super) fn repair_independent_values(
    raw: &mut RawBibliography<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let definitions: HashMap<_, _> = raw
        .abbreviations
        .iter()
        .map(|pair| (pair.key.v, &pair.value.v))
        .collect();
    let mut pending = raw
        .entries
        .iter()
        .flat_map(|entry| &entry.v.fields)
        .flat_map(|field| abbreviation_names(&field.value.v))
        .collect::<Vec<_>>();
    let mut reachable = HashSet::new();
    while let Some(name) = pending.pop() {
        if reachable.insert(name)
            && let Some(field) = definitions.get(name)
        {
            pending.extend(abbreviation_names(field));
        }
    }
    let names = definitions.keys().copied().collect::<HashSet<_>>();
    for pair in &mut raw.abbreviations {
        if reachable.contains(pair.key.v) {
            repair_unknowns(&mut pair.value.v, &names, None, None, diagnostics);
        }
    }
    for entry in &mut raw.entries {
        for field in &mut entry.v.fields {
            repair_unknowns(
                &mut field.value.v,
                &names,
                Some(entry.v.key.v),
                Some(field.key.v),
                diagnostics,
            );
        }
    }
}

fn abbreviation_names<'a>(field: &Field<'a>) -> impl Iterator<Item = &'a str> {
    field.iter().filter_map(|chunk| match chunk.v {
        RawChunk::Abbreviation(name) => Some(name),
        RawChunk::Normal(_) => None,
    })
}

fn repair_unknowns(
    field: &mut Field<'_>,
    definitions: &HashSet<&str>,
    entry: Option<&str>,
    field_name: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for chunk in field {
        let RawChunk::Abbreviation(name) = chunk.v else {
            continue;
        };
        if definitions.contains(name) || month(&name.to_lowercase()).is_some() {
            continue;
        }
        let mut diagnostic = Diagnostic::error(
            "unknown_abbreviation",
            Some(chunk.span.clone()),
            format!("unknown BibTeX abbreviation {name:?}"),
        )
        .recovered(DiagnosticAction::Literalized);
        diagnostic.entry = entry.map(str::to_string);
        diagnostic.field = field_name.map(str::to_ascii_lowercase);
        diagnostics.push(diagnostic);
        chunk.v = RawChunk::Normal(name);
    }
}

pub(super) fn apply_guard_repairs(
    raw: &mut RawBibliography<'_>,
    diagnostics: &[Diagnostic],
) -> bool {
    let spans = |code| {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .filter_map(|diagnostic| diagnostic.span.as_ref())
            .map(|span| (span.start, span.end))
            .collect::<HashSet<_>>()
    };
    let mut dropped = spans("invalid_field");
    let mut cleared = spans("cyclic_reference");
    let mut literalized = spans("cyclic_abbreviation");
    for pair in &mut raw.abbreviations {
        literalize_selected(&mut pair.value.v, &mut literalized);
    }
    for entry in &mut raw.entries {
        entry.v.fields.retain_mut(|field| {
            let span = (field.value.span.start, field.value.span.end);
            if dropped.remove(&span) {
                return false;
            }
            if cleared.remove(&span) {
                field.value.v.clear();
            }
            literalize_selected(&mut field.value.v, &mut literalized);
            true
        });
    }
    dropped.is_empty() && cleared.is_empty() && literalized.is_empty()
}

fn literalize_selected(field: &mut Field<'_>, selected: &mut HashSet<(usize, usize)>) {
    for chunk in field {
        if selected.contains(&(chunk.span.start, chunk.span.end))
            && let RawChunk::Abbreviation(name) = chunk.v
        {
            chunk.v = RawChunk::Normal(name);
            selected.remove(&(chunk.span.start, chunk.span.end));
        }
    }
}

pub(super) fn remove_duplicate_entries(
    raw: &mut RawBibliography<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut keys = HashSet::new();
    raw.entries.retain_mut(|entry| {
        if !keys.insert(entry.v.key.v) {
            let mut diagnostic = Diagnostic::error(
                "duplicate_key",
                Some(entry.span.clone()),
                format!("ignored duplicate BibTeX entry key {:?}", entry.v.key.v),
            )
            .recovered(DiagnosticAction::DroppedBlock);
            diagnostic.entry = Some(entry.v.key.v.to_string());
            diagnostics.push(diagnostic);
            return false;
        }
        true
    });
}
