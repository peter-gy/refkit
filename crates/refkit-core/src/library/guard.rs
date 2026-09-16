use std::collections::{HashMap, HashSet};
use std::ops::Range;

use biblatex::{ChunksExt, Field, Pair, RawBibliography, RawChunk, RawEntry, Spanned};

use super::Diagnostic;

mod cycles;
mod dates;

pub(crate) use dates::validate_date_parser_input;
use dates::{is_date_parser_field, is_plain_date_text};

const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_EXPANDED_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_STEPS: usize = 100_000;

pub(crate) fn validate_source(source: &str) -> Result<(), Diagnostic> {
    validate_source_size(source.len())
}

pub(crate) fn validate_source_size(bytes: usize) -> Result<(), Diagnostic> {
    if bytes > MAX_SOURCE_BYTES {
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
        .map(|pair| (pair.key.v.to_string(), &pair.value.v))
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
    let entry = normalized.get("reference").ok_or_else(|| {
        Diagnostic::error(
            "invalid_reference",
            Some(span.clone()),
            "reference normalization produced no entry".to_string(),
        )
    })?;
    let keys = if is_list {
        entry.get_as::<Vec<String>>("value")
    } else {
        entry.get_as::<String>("value").map(|key| vec![key])
    };
    keys.map_err(|error| Diagnostic::error("invalid_reference", Some(span), error.to_string()))
}

pub(crate) fn resolve_field(
    field: &Field<'_>,
    abbreviations: &HashMap<String, &Field<'_>>,
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
        false,
    )?;
    Ok(output)
}

fn expand<'a>(
    field: &Field<'a>,
    abbreviations: &HashMap<String, &Field<'a>>,
    ancestors: &mut Vec<String>,
    steps: &mut usize,
    output: &mut String,
    strict: bool,
) -> Result<(), Diagnostic> {
    validate_value(field).map_err(|mut diagnostic| {
        if strict {
            diagnostic.span = field.first().map(|chunk| chunk.span.clone());
        }
        diagnostic
    })?;
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
                expand_abbreviation(
                    name,
                    &chunk.span,
                    abbreviations,
                    ancestors,
                    steps,
                    output,
                    strict,
                )?;
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

fn expand_abbreviation<'a>(
    name: &'a str,
    span: &Range<usize>,
    abbreviations: &HashMap<String, &Field<'a>>,
    ancestors: &mut Vec<String>,
    steps: &mut usize,
    output: &mut String,
    strict: bool,
) -> Result<(), Diagnostic> {
    let key = if strict {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    };
    if ancestors.contains(&key) {
        return Err(Diagnostic::error(
            "cyclic_abbreviation",
            Some(span.clone()),
            format!("cyclic BibTeX abbreviation {name:?}"),
        ));
    }
    if let Some(value) = abbreviations.get(&key) {
        ancestors.push(key);
        expand(value, abbreviations, ancestors, steps, output, strict)?;
        ancestors.pop();
    } else if strict {
        output.push_str(month(&key).ok_or_else(|| {
            Diagnostic::error(
                "unknown_abbreviation",
                Some(span.clone()),
                format!("unknown BibTeX abbreviation {name:?}"),
            )
        })?);
    } else {
        output.push_str(name);
    }
    Ok(())
}

pub(crate) struct FieldResolver<'a> {
    abbreviations: HashMap<String, &'a Field<'a>>,
    steps: usize,
    bytes: usize,
}

impl<'a> FieldResolver<'a> {
    pub(crate) fn new(abbreviations: HashMap<String, &'a Field<'a>>) -> Self {
        Self {
            abbreviations,
            steps: 0,
            bytes: 0,
        }
    }

    pub(crate) fn resolve(&mut self, field: &Field<'a>) -> Result<String, Diagnostic> {
        let mut output = String::new();
        expand(
            field,
            &self.abbreviations,
            &mut Vec::new(),
            &mut self.steps,
            &mut output,
            true,
        )?;
        self.bytes = self.bytes.saturating_add(output.len());
        if self.bytes > MAX_EXPANDED_BYTES {
            return Err(limit(
                field.first().map(|chunk| chunk.span.clone()),
                "bibliography expansion exceeds 16 MiB",
            ));
        }
        Ok(output)
    }
}

pub(super) fn month(name: &str) -> Option<&'static str> {
    Some(match name {
        "jan" => "January",
        "feb" => "February",
        "mar" => "March",
        "apr" => "April",
        "may" => "May",
        "jun" => "June",
        "jul" => "July",
        "aug" => "August",
        "sep" => "September",
        "oct" => "October",
        "nov" => "November",
        "dec" => "December",
        _ => return None,
    })
}

pub(crate) fn validate_raw(raw: &RawBibliography<'_>) -> Result<(), Diagnostic> {
    validate_raw_report(raw)?
        .into_iter()
        .next()
        .map_or(Ok(()), Err)
}

pub(super) fn validate_raw_report(
    raw: &RawBibliography<'_>,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    for abbreviation in &raw.abbreviations {
        validate_value(&abbreviation.value.v)?;
    }
    let abbreviation_cycles = cycles::abbreviations(raw)?;
    if !abbreviation_cycles.is_empty() {
        return Ok(abbreviation_cycles);
    }
    let abbreviations: HashMap<_, _> = raw
        .abbreviations
        .iter()
        .map(|pair| (pair.key.v.to_string(), &pair.value.v))
        .collect();
    let has_references = raw
        .entries
        .iter()
        .any(|entry| entry.v.fields.iter().any(|field| is_reference(field.key.v)));
    let (weights, needs_date_check, mut diagnostics) = expansion_weights(raw, &abbreviations)?;
    if !has_references && !needs_date_check {
        return Ok(diagnostics);
    }
    let entry_index: HashMap<_, _> = raw
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.v.key.v, index))
        .collect();
    let mut graph = vec![Vec::new(); raw.entries.len()];
    let detached = detach_references(raw);
    let Ok(normalized) = biblatex::Bibliography::from_raw(detached) else {
        return Ok(diagnostics);
    };
    if needs_date_check {
        diagnostics = validate_normalized_dates(raw, &normalized);
    }
    if !has_references {
        return Ok(diagnostics);
    }
    for (edges, entry) in graph.iter_mut().zip(&raw.entries) {
        let Some(normalized_entry) = normalized.get(entry.v.key.v) else {
            continue;
        };
        for field in effective_references(&entry.v) {
            let keys = if field.key.v.eq_ignore_ascii_case("crossref") {
                normalized_entry
                    .get_as::<String>("__refkit_guard_crossref")
                    .map(|key| vec![key])
            } else if field.key.v.eq_ignore_ascii_case("xdata") {
                normalized_entry.get_as::<Vec<String>>("__refkit_guard_xdata")
            } else {
                continue;
            };
            let Ok(keys) = keys else {
                continue;
            };
            for target in keys
                .iter()
                .filter_map(|key| entry_index.get(key.as_str()).copied())
            {
                edges.push((target, field.value.span.clone(), field.key.v.to_string()));
            }
        }
    }
    let mut ancestors = Vec::new();
    let reference_cycles = cycles::references(&graph, raw)?;
    if !reference_cycles.is_empty() {
        diagnostics.extend(reference_cycles);
        return Ok(diagnostics);
    }
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
    Ok(diagnostics)
}

fn expansion_weights(
    raw: &RawBibliography<'_>,
    abbreviations: &HashMap<String, &Field<'_>>,
) -> Result<(Vec<usize>, bool, Vec<Diagnostic>), Diagnostic> {
    let mut weights = vec![1usize; raw.entries.len()];
    let mut bytes = 0usize;
    let mut expansion_steps = 0usize;
    let mut value = String::new();
    let mut ancestors = Vec::new();
    let mut needs_date_check = false;
    let mut diagnostics = Vec::new();
    for (weight, entry) in weights.iter_mut().zip(&raw.entries) {
        let mut date_fields = HashSet::new();
        for field in &entry.v.fields {
            value.clear();
            expand(
                &field.value.v,
                abbreviations,
                &mut ancestors,
                &mut expansion_steps,
                &mut value,
                false,
            )
            .map_err(|mut diagnostic| {
                diagnostic.entry = Some(entry.v.key.v.to_string());
                diagnostic.field = Some(field.key.v.to_ascii_lowercase());
                diagnostic
            })?;
            needs_date_check |= check_expanded_date(
                field,
                entry.v.key.v,
                &value,
                &mut date_fields,
                &mut diagnostics,
            );
            bytes = bytes.saturating_add(value.len());
            if bytes > MAX_EXPANDED_BYTES {
                return Err(limit(
                    Some(field.value.span.clone()),
                    "bibliography expansion exceeds 16 MiB",
                ));
            }
            *weight = weight.saturating_add(value.len());
        }
    }
    Ok((weights, needs_date_check, diagnostics))
}

fn check_expanded_date(
    field: &Pair<'_>,
    entry_key: &str,
    value: &str,
    date_fields: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let name = field.key.v.to_ascii_lowercase();
    if !is_date_parser_field(&name) {
        return false;
    }
    let duplicate = !date_fields.insert(name.clone());
    if !is_plain_date_text(value) {
        return true;
    }
    if let Err(message) = validate_date_parser_input(&name, value) {
        diagnostics.push(invalid_date(
            entry_key,
            &name,
            field.value.span.clone(),
            message,
        ));
    }
    duplicate
}

fn effective_references<'a>(entry: &'a RawEntry<'a>) -> impl Iterator<Item = &'a Pair<'a>> {
    let positions = ["crossref", "xdata"].map(|name| {
        entry
            .fields
            .iter()
            .rposition(|field| field.key.v.eq_ignore_ascii_case(name))
    });
    entry
        .fields
        .iter()
        .enumerate()
        .filter(move |(index, _)| positions.contains(&Some(*index)))
        .map(|(_, field)| field)
}

fn detach_references<'a>(raw: &RawBibliography<'a>) -> RawBibliography<'a> {
    RawBibliography {
        preamble: String::new(),
        abbreviations: raw.abbreviations.clone(),
        entries: raw
            .entries
            .iter()
            .filter_map(|entry| {
                let fields = entry
                    .v
                    .fields
                    .iter()
                    .filter_map(|field| {
                        let name = field.key.v.to_ascii_lowercase();
                        let guard_name = match name.as_str() {
                            "crossref" => "__refkit_guard_crossref",
                            "xdata" => "__refkit_guard_xdata",
                            _ if is_date_parser_field(&name) => field.key.v,
                            _ => return None,
                        };
                        let mut field = field.clone();
                        field.key.v = guard_name;
                        Some(field)
                    })
                    .collect::<Vec<_>>();
                (!fields.is_empty()).then(|| {
                    Spanned::new(
                        RawEntry {
                            key: entry.v.key.clone(),
                            kind: entry.v.kind.clone(),
                            fields,
                        },
                        entry.span.clone(),
                    )
                })
            })
            .collect(),
    }
}

fn invalid_date(key: &str, name: &str, span: Range<usize>, message: String) -> Diagnostic {
    let mut diagnostic = Diagnostic::error("invalid_field", Some(span), message);
    diagnostic.entry = Some(key.to_string());
    diagnostic.field = Some(name.to_string());
    diagnostic
}

fn validate_normalized_dates(
    raw: &RawBibliography<'_>,
    normalized: &biblatex::Bibliography,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for entry in &raw.entries {
        let Some(normalized_entry) = normalized.get(entry.v.key.v) else {
            continue;
        };
        let mut seen = HashSet::new();
        for field in entry.v.fields.iter().rev() {
            let name = field.key.v.to_ascii_lowercase();
            if !is_date_parser_field(&name) || !seen.insert(name.clone()) {
                continue;
            }
            let Some(chunks) = normalized_entry.fields.get(&name) else {
                continue;
            };
            if let Err(message) = validate_date_parser_input(&name, &chunks.format_verbatim()) {
                let diagnostic =
                    invalid_date(entry.v.key.v, &name, field.value.span.clone(), message);
                diagnostics.push(diagnostic);
            }
        }
    }
    diagnostics
}

fn is_reference(name: &str) -> bool {
    name.eq_ignore_ascii_case("crossref") || name.eq_ignore_ascii_case("xdata")
}

type ReferenceGraph = cycles::Graph;

#[expect(
    clippy::indexing_slicing,
    reason = "The graph and weights use the raw entry enumeration. Every target is resolved through that enumeration before traversal, and recursive calls use only those targets."
)]
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
