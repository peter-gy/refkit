use std::collections::HashMap;
use std::ops::Range;

use biblatex::{Field, Pair, RawBibliography, RawChunk, RawEntry, Spanned};

use super::Diagnostic;

const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_EXPANDED_BYTES: usize = 16 * 1024 * 1024;
const MAX_STEPS: usize = 100_000;

pub(crate) fn validate_source(source: &str) -> Result<(), Diagnostic> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(limit(None, "bibliography source exceeds 16 MiB"));
    }
    Ok(())
}

pub(crate) fn validate_literal(value: &str, source_offset: usize) -> Result<(), Diagnostic> {
    let mut delimiters = Vec::new();
    let mut escaped = false;
    for (offset, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '{' | '[') {
            delimiters.push(character);
            if delimiters.len() >= MAX_DEPTH {
                let offset = source_offset + offset;
                return Err(limit(
                    Some(offset..offset + 1),
                    "bibliography value nesting exceeds 64 levels",
                ));
            }
        } else if matches!(character, '}' | ']')
            && delimiters
                .last()
                .is_some_and(|open| matches!((*open, character), ('{', '}') | ('[', ']')))
        {
            delimiters.pop();
        }
    }
    Ok(())
}

fn validate_value(field: &Field<'_>) -> Result<(), Diagnostic> {
    for chunk in field {
        if let RawChunk::Normal(value) = chunk.v {
            validate_literal(value, chunk.span.start)?;
        }
    }
    Ok(())
}

pub(crate) fn normalize_reference<'a>(
    field: &Field<'a>,
    abbreviations: &[Pair<'a>],
    is_list: bool,
) -> Result<Vec<String>, Diagnostic> {
    let lookup = abbreviations
        .iter()
        .map(|pair| (pair.key.v, &pair.value.v))
        .collect();
    resolve_field(field, &lookup)?;
    let span = field.first().map_or(0, |chunk| chunk.span.start)
        ..field.last().map_or(0, |chunk| chunk.span.end);
    let raw = RawBibliography {
        preamble: String::new(),
        abbreviations: abbreviations.to_vec(),
        entries: vec![Spanned::new(
            RawEntry {
                key: Spanned::new("reference", span.clone()),
                kind: Spanned::new("misc", span.clone()),
                fields: vec![Pair::new(
                    Spanned::new("value", span.clone()),
                    Spanned::new(field.clone(), span.clone()),
                )],
            },
            span.clone(),
        )],
    };
    let normalized =
        biblatex::Bibliography::from_raw(raw).map_err(|error| super::parse::parse_error(&error))?;
    let entry = normalized
        .get("reference")
        .expect("reference entry was constructed");
    let keys = if is_list {
        entry.get_as::<Vec<String>>("value")
    } else {
        entry.get_as::<String>("value").map(|key| vec![key])
    };
    keys.map_err(|error| Diagnostic::error("invalid_reference", Some(span), error.to_string()))
}

pub(crate) fn resolve_field(
    field: &Field<'_>,
    abbreviations: &HashMap<&str, &Field<'_>>,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    let mut ancestors = Vec::new();
    let mut steps = 0;
    expand(
        field,
        abbreviations,
        &mut ancestors,
        &mut steps,
        &mut output,
    )?;
    Ok(output)
}

fn expand<'a>(
    field: &Field<'a>,
    abbreviations: &HashMap<&str, &Field<'a>>,
    ancestors: &mut Vec<String>,
    steps: &mut usize,
    output: &mut String,
) -> Result<(), Diagnostic> {
    validate_value(field)?;
    for chunk in field {
        *steps += 1;
        if *steps > MAX_STEPS || ancestors.len() >= MAX_DEPTH {
            return Err(limit(
                Some(chunk.span.clone()),
                "bibliography expansion exceeds its dependency budget",
            ));
        }
        match chunk.v {
            RawChunk::Normal(text) => output.push_str(text),
            RawChunk::Abbreviation(name) => {
                if ancestors.iter().any(|ancestor| ancestor == name) {
                    return Err(Diagnostic::error(
                        "cyclic_abbreviation",
                        Some(chunk.span.clone()),
                        format!("cyclic BibTeX abbreviation {name:?}"),
                    ));
                }
                if let Some(value) = abbreviations.get(name) {
                    ancestors.push(name.to_string());
                    expand(value, abbreviations, ancestors, steps, output)?;
                    ancestors.pop();
                } else {
                    output.push_str(name);
                }
            }
        }
        if output.len() > MAX_EXPANDED_BYTES {
            return Err(limit(
                Some(chunk.span.clone()),
                "bibliography expansion exceeds 16 MiB",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_raw(raw: &RawBibliography<'_>) -> Result<(), Diagnostic> {
    for abbreviation in &raw.abbreviations {
        validate_value(&abbreviation.value.v)?;
    }
    let abbreviations: HashMap<_, _> = raw
        .abbreviations
        .iter()
        .map(|pair| (pair.key.v, &pair.value.v))
        .collect();
    let has_references = raw
        .entries
        .iter()
        .any(|entry| entry.v.fields.iter().any(|field| is_reference(field.key.v)));
    let mut weights = vec![1usize; raw.entries.len()];
    let mut bytes = 0usize;
    let mut expansion_steps = 0usize;
    let mut value = String::new();
    let mut ancestors = Vec::new();
    for (index, entry) in raw.entries.iter().enumerate() {
        for field in &entry.v.fields {
            value.clear();
            expand(
                &field.value.v,
                &abbreviations,
                &mut ancestors,
                &mut expansion_steps,
                &mut value,
            )
            .map_err(|mut diagnostic| {
                diagnostic.entry = Some(entry.v.key.v.to_string());
                diagnostic.field = Some(field.key.v.to_ascii_lowercase());
                diagnostic
            })?;
            bytes = bytes.saturating_add(value.len());
            if bytes > MAX_EXPANDED_BYTES {
                return Err(limit(
                    Some(field.value.span.clone()),
                    "bibliography expansion exceeds 16 MiB",
                ));
            }
            weights[index] = weights[index].saturating_add(value.len());
        }
    }
    if !has_references {
        return Ok(());
    }
    let entry_index: HashMap<_, _> = raw
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.v.key.v, index))
        .collect();
    let mut graph = vec![Vec::new(); raw.entries.len()];
    let mut detached = raw.clone();
    for entry in &mut detached.entries {
        entry.v.fields.retain(|field| {
            !field.key.v.eq_ignore_ascii_case("__refkit_guard_crossref")
                && !field.key.v.eq_ignore_ascii_case("__refkit_guard_xdata")
        });
        for field in &mut entry.v.fields {
            if field.key.v.eq_ignore_ascii_case("crossref") {
                field.key.v = "__refkit_guard_crossref";
            } else if field.key.v.eq_ignore_ascii_case("xdata") {
                field.key.v = "__refkit_guard_xdata";
            }
        }
    }
    let Ok(normalized) = biblatex::Bibliography::from_raw(detached) else {
        return Ok(());
    };
    for (index, entry) in raw.entries.iter().enumerate() {
        let Some(normalized_entry) = normalized.get(entry.v.key.v) else {
            continue;
        };
        for field in &entry.v.fields {
            let keys = if field.key.v.eq_ignore_ascii_case("crossref") {
                normalized_entry
                    .get_as::<String>("__refkit_guard_crossref")
                    .map(|key| vec![key])
            } else if field.key.v.eq_ignore_ascii_case("xdata") {
                normalized_entry.get_as::<Vec<String>>("__refkit_guard_xdata")
            } else {
                continue;
            };
            if let Ok(keys) = keys {
                for key in keys {
                    if let Some(target) = entry_index.get(key.as_str()).copied() {
                        graph[index].push((
                            target,
                            field.value.span.clone(),
                            field.key.v.to_string(),
                        ));
                    }
                }
            }
        }
    }
    let mut ancestors = Vec::new();
    let mut steps = 0;
    let mut inherited_bytes = 0;
    for index in 0..graph.len() {
        visit(
            index,
            &graph,
            &weights,
            raw,
            &mut ancestors,
            &mut steps,
            &mut inherited_bytes,
        )?;
    }
    Ok(())
}

fn is_reference(name: &str) -> bool {
    name.eq_ignore_ascii_case("crossref") || name.eq_ignore_ascii_case("xdata")
}

type ReferenceGraph = Vec<Vec<(usize, Range<usize>, String)>>;

fn visit(
    index: usize,
    graph: &ReferenceGraph,
    weights: &[usize],
    raw: &RawBibliography<'_>,
    ancestors: &mut Vec<usize>,
    steps: &mut usize,
    inherited_bytes: &mut usize,
) -> Result<(), Diagnostic> {
    *inherited_bytes = inherited_bytes.saturating_add(weights[index]);
    if *inherited_bytes > MAX_EXPANDED_BYTES {
        return Err(limit(
            Some(raw.entries[index].span.clone()),
            "bibliography inheritance exceeds 16 MiB",
        ));
    }
    ancestors.push(index);
    for (target, span, field) in &graph[index] {
        *steps += 1;
        let mut diagnostic = if ancestors.contains(target) {
            Some(Diagnostic::error(
                "cyclic_reference",
                Some(span.clone()),
                format!(
                    "cyclic BibTeX reference to {:?}",
                    raw.entries[*target].v.key.v
                ),
            ))
        } else if ancestors.len() >= MAX_DEPTH || *steps > MAX_STEPS {
            Some(limit(
                Some(span.clone()),
                "bibliography inheritance exceeds its dependency budget",
            ))
        } else {
            None
        };
        if let Some(ref mut diagnostic) = diagnostic {
            diagnostic.entry = Some(raw.entries[index].v.key.v.to_string());
            diagnostic.field = Some(field.to_ascii_lowercase());
        }
        if let Some(diagnostic) = diagnostic {
            return Err(diagnostic);
        }
        visit(
            *target,
            graph,
            weights,
            raw,
            ancestors,
            steps,
            inherited_bytes,
        )?;
    }
    ancestors.pop();
    Ok(())
}

fn limit(span: Option<Range<usize>>, message: &str) -> Diagnostic {
    Diagnostic::error("resource_limit", span, message.to_string())
}
